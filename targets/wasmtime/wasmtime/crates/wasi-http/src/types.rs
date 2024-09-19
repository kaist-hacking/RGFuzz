//! Implements the base structure (i.e. [WasiHttpCtx]) that will provide the
//! implementation of the wasi-http API.

use crate::io::TokioIo;
use crate::{
    bindings::http::types::{self, Method, Scheme},
    body::{HostIncomingBody, HyperIncomingBody, HyperOutgoingBody},
    dns_error, hyper_request_error,
};
use http_body_util::BodyExt;
use hyper::header::HeaderName;
use std::any::Any;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;
use wasmtime::component::{Resource, ResourceTable};
use wasmtime_wasi::preview2::{self, AbortOnDropJoinHandle, Subscribe};

/// Capture the state necessary for use in the wasi-http API implementation.
pub struct WasiHttpCtx;

pub struct OutgoingRequest {
    pub use_tls: bool,
    pub authority: String,
    pub request: hyper::Request<HyperOutgoingBody>,
    pub connect_timeout: Duration,
    pub first_byte_timeout: Duration,
    pub between_bytes_timeout: Duration,
}

pub trait WasiHttpView: Send {
    fn ctx(&mut self) -> &mut WasiHttpCtx;
    fn table(&mut self) -> &mut ResourceTable;

    fn new_incoming_request(
        &mut self,
        req: hyper::Request<HyperIncomingBody>,
    ) -> wasmtime::Result<Resource<HostIncomingRequest>>
    where
        Self: Sized,
    {
        let (parts, body) = req.into_parts();
        let body = HostIncomingBody::new(
            body,
            // TODO: this needs to be plumbed through
            std::time::Duration::from_millis(600 * 1000),
        );
        let incoming_req = HostIncomingRequest::new(self, parts, Some(body));
        Ok(self.table().push(incoming_req)?)
    }

    fn new_response_outparam(
        &mut self,
        result: tokio::sync::oneshot::Sender<
            Result<hyper::Response<HyperOutgoingBody>, types::ErrorCode>,
        >,
    ) -> wasmtime::Result<Resource<HostResponseOutparam>> {
        let id = self.table().push(HostResponseOutparam { result })?;
        Ok(id)
    }

    fn send_request(
        &mut self,
        request: OutgoingRequest,
    ) -> wasmtime::Result<Resource<HostFutureIncomingResponse>>
    where
        Self: Sized,
    {
        default_send_request(self, request)
    }

    fn is_forbidden_header(&mut self, _name: &HeaderName) -> bool {
        false
    }
}

/// Returns `true` when the header is forbidden according to this [`WasiHttpView`] implementation.
pub(crate) fn is_forbidden_header(view: &mut dyn WasiHttpView, name: &HeaderName) -> bool {
    static FORBIDDEN_HEADERS: [HeaderName; 9] = [
        hyper::header::CONNECTION,
        HeaderName::from_static("keep-alive"),
        hyper::header::PROXY_AUTHENTICATE,
        hyper::header::PROXY_AUTHORIZATION,
        HeaderName::from_static("proxy-connection"),
        hyper::header::TE,
        hyper::header::TRANSFER_ENCODING,
        hyper::header::UPGRADE,
        HeaderName::from_static("http2-settings"),
    ];

    FORBIDDEN_HEADERS.contains(name) || view.is_forbidden_header(name)
}

/// Removes forbidden headers from a [`hyper::HeaderMap`].
pub(crate) fn remove_forbidden_headers(
    view: &mut dyn WasiHttpView,
    headers: &mut hyper::HeaderMap,
) {
    let forbidden_keys = Vec::from_iter(headers.keys().filter_map(|name| {
        if is_forbidden_header(view, name) {
            Some(name.clone())
        } else {
            None
        }
    }));

    for name in forbidden_keys {
        headers.remove(name);
    }
}

pub fn default_send_request(
    view: &mut dyn WasiHttpView,
    OutgoingRequest {
        use_tls,
        authority,
        request,
        connect_timeout,
        first_byte_timeout,
        between_bytes_timeout,
    }: OutgoingRequest,
) -> wasmtime::Result<Resource<HostFutureIncomingResponse>> {
    let handle = preview2::spawn(async move {
        let resp = handler(
            authority,
            use_tls,
            connect_timeout,
            first_byte_timeout,
            request,
            between_bytes_timeout,
        )
        .await;
        Ok(resp)
    });

    let fut = view.table().push(HostFutureIncomingResponse::new(handle))?;

    Ok(fut)
}

async fn handler(
    authority: String,
    use_tls: bool,
    connect_timeout: Duration,
    first_byte_timeout: Duration,
    mut request: http::Request<HyperOutgoingBody>,
    between_bytes_timeout: Duration,
) -> Result<IncomingResponseInternal, types::ErrorCode> {
    let tcp_stream = TcpStream::connect(authority.clone())
        .await
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::AddrNotAvailable => {
                dns_error("address not available".to_string(), 0)
            }

            _ => {
                if e.to_string()
                    .starts_with("failed to lookup address information")
                {
                    dns_error("address not available".to_string(), 0)
                } else {
                    types::ErrorCode::ConnectionRefused
                }
            }
        })?;

    let (mut sender, worker) = if use_tls {
        #[cfg(any(target_arch = "riscv64", target_arch = "s390x"))]
        {
            return Err(crate::bindings::http::types::ErrorCode::InternalError(
                Some("unsupported architecture for SSL".to_string()),
            ));
        }

        #[cfg(not(any(target_arch = "riscv64", target_arch = "s390x")))]
        {
            use tokio_rustls::rustls::OwnedTrustAnchor;

            // derived from https://github.com/tokio-rs/tls/blob/master/tokio-rustls/examples/client/src/main.rs
            let mut root_cert_store = rustls::RootCertStore::empty();
            root_cert_store.add_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.iter().map(|ta| {
                OwnedTrustAnchor::from_subject_spki_name_constraints(
                    ta.subject,
                    ta.spki,
                    ta.name_constraints,
                )
            }));
            let config = rustls::ClientConfig::builder()
                .with_safe_defaults()
                .with_root_certificates(root_cert_store)
                .with_no_client_auth();
            let connector = tokio_rustls::TlsConnector::from(std::sync::Arc::new(config));
            let mut parts = authority.split(":");
            let host = parts.next().unwrap_or(&authority);
            let domain = rustls::ServerName::try_from(host).map_err(|e| {
                tracing::warn!("dns lookup error: {e:?}");
                dns_error("invalid dns name".to_string(), 0)
            })?;
            let stream = connector.connect(domain, tcp_stream).await.map_err(|e| {
                tracing::warn!("tls protocol error: {e:?}");
                types::ErrorCode::TlsProtocolError
            })?;
            let stream = TokioIo::new(stream);

            let (sender, conn) = timeout(
                connect_timeout,
                hyper::client::conn::http1::handshake(stream),
            )
            .await
            .map_err(|_| types::ErrorCode::ConnectionTimeout)?
            .map_err(hyper_request_error)?;

            let worker = preview2::spawn(async move {
                match conn.await {
                    Ok(()) => {}
                    // TODO: shouldn't throw away this error and ideally should
                    // surface somewhere.
                    Err(e) => tracing::warn!("dropping error {e}"),
                }
            });

            (sender, worker)
        }
    } else {
        let tcp_stream = TokioIo::new(tcp_stream);
        let (sender, conn) = timeout(
            connect_timeout,
            // TODO: we should plumb the builder through the http context, and use it here
            hyper::client::conn::http1::handshake(tcp_stream),
        )
        .await
        .map_err(|_| types::ErrorCode::ConnectionTimeout)?
        .map_err(hyper_request_error)?;

        let worker = preview2::spawn(async move {
            match conn.await {
                Ok(()) => {}
                // TODO: same as above, shouldn't throw this error away.
                Err(e) => tracing::warn!("dropping error {e}"),
            }
        });

        (sender, worker)
    };

    // at this point, the request contains the scheme and the authority, but
    // the http packet should only include those if addressing a proxy, so
    // remove them here, since SendRequest::send_request does not do it for us
    *request.uri_mut() = http::Uri::builder()
        .path_and_query(
            request
                .uri()
                .path_and_query()
                .map(|p| p.as_str())
                .unwrap_or("/"),
        )
        .build()
        .expect("comes from valid request");

    let resp = timeout(first_byte_timeout, sender.send_request(request))
        .await
        .map_err(|_| types::ErrorCode::ConnectionReadTimeout)?
        .map_err(hyper_request_error)?
        .map(|body| body.map_err(hyper_request_error).boxed());

    Ok(IncomingResponseInternal {
        resp,
        worker: Arc::new(worker),
        between_bytes_timeout,
    })
}

impl From<http::Method> for types::Method {
    fn from(method: http::Method) -> Self {
        if method == http::Method::GET {
            types::Method::Get
        } else if method == hyper::Method::HEAD {
            types::Method::Head
        } else if method == hyper::Method::POST {
            types::Method::Post
        } else if method == hyper::Method::PUT {
            types::Method::Put
        } else if method == hyper::Method::DELETE {
            types::Method::Delete
        } else if method == hyper::Method::CONNECT {
            types::Method::Connect
        } else if method == hyper::Method::OPTIONS {
            types::Method::Options
        } else if method == hyper::Method::TRACE {
            types::Method::Trace
        } else if method == hyper::Method::PATCH {
            types::Method::Patch
        } else {
            types::Method::Other(method.to_string())
        }
    }
}

impl TryInto<http::Method> for types::Method {
    type Error = http::method::InvalidMethod;

    fn try_into(self) -> Result<http::Method, Self::Error> {
        match self {
            Method::Get => Ok(http::Method::GET),
            Method::Head => Ok(http::Method::HEAD),
            Method::Post => Ok(http::Method::POST),
            Method::Put => Ok(http::Method::PUT),
            Method::Delete => Ok(http::Method::DELETE),
            Method::Connect => Ok(http::Method::CONNECT),
            Method::Options => Ok(http::Method::OPTIONS),
            Method::Trace => Ok(http::Method::TRACE),
            Method::Patch => Ok(http::Method::PATCH),
            Method::Other(s) => http::Method::from_bytes(s.as_bytes()),
        }
    }
}

pub struct HostIncomingRequest {
    pub(crate) parts: http::request::Parts,
    pub body: Option<HostIncomingBody>,
}

impl HostIncomingRequest {
    pub fn new(
        view: &mut dyn WasiHttpView,
        mut parts: http::request::Parts,
        body: Option<HostIncomingBody>,
    ) -> Self {
        remove_forbidden_headers(view, &mut parts.headers);
        Self { parts, body }
    }
}

pub struct HostResponseOutparam {
    pub result:
        tokio::sync::oneshot::Sender<Result<hyper::Response<HyperOutgoingBody>, types::ErrorCode>>,
}

pub struct HostOutgoingRequest {
    pub method: Method,
    pub scheme: Option<Scheme>,
    pub path_with_query: Option<String>,
    pub authority: Option<String>,
    pub headers: FieldMap,
    pub body: Option<HyperOutgoingBody>,
}

#[derive(Default)]
pub struct HostRequestOptions {
    pub connect_timeout: Option<std::time::Duration>,
    pub first_byte_timeout: Option<std::time::Duration>,
    pub between_bytes_timeout: Option<std::time::Duration>,
}

pub struct HostIncomingResponse {
    pub status: u16,
    pub headers: FieldMap,
    pub body: Option<HostIncomingBody>,
    pub worker: Arc<AbortOnDropJoinHandle<()>>,
}

pub struct HostOutgoingResponse {
    pub status: http::StatusCode,
    pub headers: FieldMap,
    pub body: Option<HyperOutgoingBody>,
}

impl TryFrom<HostOutgoingResponse> for hyper::Response<HyperOutgoingBody> {
    type Error = http::Error;

    fn try_from(
        resp: HostOutgoingResponse,
    ) -> Result<hyper::Response<HyperOutgoingBody>, Self::Error> {
        use http_body_util::Empty;

        let mut builder = hyper::Response::builder().status(resp.status);

        *builder.headers_mut().unwrap() = resp.headers;

        match resp.body {
            Some(body) => builder.body(body),
            None => builder.body(
                Empty::<bytes::Bytes>::new()
                    .map_err(|_| unreachable!("Infallible error"))
                    .boxed(),
            ),
        }
    }
}

pub type FieldMap = hyper::HeaderMap;

pub enum HostFields {
    Ref {
        parent: u32,

        // NOTE: there's not failure in the result here because we assume that HostFields will
        // always be registered as a child of the entry with the `parent` id. This ensures that the
        // entry will always exist while this `HostFields::Ref` entry exists in the table, thus we
        // don't need to account for failure when fetching the fields ref from the parent.
        get_fields: for<'a> fn(elem: &'a mut (dyn Any + 'static)) -> &'a mut FieldMap,
    },
    Owned {
        fields: FieldMap,
    },
}

pub struct IncomingResponseInternal {
    pub resp: hyper::Response<HyperIncomingBody>,
    pub worker: Arc<AbortOnDropJoinHandle<()>>,
    pub between_bytes_timeout: std::time::Duration,
}

type FutureIncomingResponseHandle =
    AbortOnDropJoinHandle<anyhow::Result<Result<IncomingResponseInternal, types::ErrorCode>>>;

pub enum HostFutureIncomingResponse {
    Pending(FutureIncomingResponseHandle),
    Ready(anyhow::Result<Result<IncomingResponseInternal, types::ErrorCode>>),
    Consumed,
}

impl HostFutureIncomingResponse {
    pub fn new(handle: FutureIncomingResponseHandle) -> Self {
        Self::Pending(handle)
    }

    pub fn is_ready(&self) -> bool {
        matches!(self, Self::Ready(_))
    }

    pub fn unwrap_ready(
        self,
    ) -> anyhow::Result<Result<IncomingResponseInternal, types::ErrorCode>> {
        match self {
            Self::Ready(res) => res,
            Self::Pending(_) | Self::Consumed => {
                panic!("unwrap_ready called on a pending HostFutureIncomingResponse")
            }
        }
    }
}

#[async_trait::async_trait]
impl Subscribe for HostFutureIncomingResponse {
    async fn ready(&mut self) {
        if let Self::Pending(handle) = self {
            *self = Self::Ready(handle.await);
        }
    }
}
