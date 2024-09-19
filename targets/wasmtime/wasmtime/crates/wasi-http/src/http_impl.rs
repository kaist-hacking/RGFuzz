use crate::{
    bindings::http::{
        outgoing_handler,
        types::{self, Scheme},
    },
    http_request_error, internal_error,
    types::{HostFutureIncomingResponse, HostOutgoingRequest, OutgoingRequest},
    WasiHttpView,
};
use bytes::Bytes;
use http_body_util::{BodyExt, Empty};
use hyper::Method;
use wasmtime::component::Resource;

impl<T: WasiHttpView> outgoing_handler::Host for T {
    fn handle(
        &mut self,
        request_id: Resource<HostOutgoingRequest>,
        options: Option<Resource<types::RequestOptions>>,
    ) -> wasmtime::Result<Result<Resource<HostFutureIncomingResponse>, types::ErrorCode>> {
        let opts = options.and_then(|opts| self.table().get(&opts).ok());

        let connect_timeout = opts
            .and_then(|opts| opts.connect_timeout)
            .unwrap_or(std::time::Duration::from_millis(600 * 1000));

        let first_byte_timeout = opts
            .and_then(|opts| opts.first_byte_timeout)
            .unwrap_or(std::time::Duration::from_millis(600 * 1000));

        let between_bytes_timeout = opts
            .and_then(|opts| opts.between_bytes_timeout)
            .unwrap_or(std::time::Duration::from_millis(600 * 1000));

        let req = self.table().delete(request_id)?;
        let mut builder = hyper::Request::builder();

        builder = builder.method(match req.method {
            types::Method::Get => Method::GET,
            types::Method::Head => Method::HEAD,
            types::Method::Post => Method::POST,
            types::Method::Put => Method::PUT,
            types::Method::Delete => Method::DELETE,
            types::Method::Connect => Method::CONNECT,
            types::Method::Options => Method::OPTIONS,
            types::Method::Trace => Method::TRACE,
            types::Method::Patch => Method::PATCH,
            types::Method::Other(m) => match hyper::Method::from_bytes(m.as_bytes()) {
                Ok(method) => method,
                Err(_) => return Ok(Err(types::ErrorCode::HttpRequestMethodInvalid)),
            },
        });

        let (use_tls, scheme, port) = match req.scheme.unwrap_or(Scheme::Https) {
            Scheme::Http => (false, http::uri::Scheme::HTTP, 80),
            Scheme::Https => (true, http::uri::Scheme::HTTPS, 443),

            // We can only support http/https
            Scheme::Other(_) => return Ok(Err(types::ErrorCode::HttpProtocolError)),
        };

        let authority = if let Some(authority) = req.authority {
            if authority.find(':').is_some() {
                authority
            } else {
                format!("{}:{port}", authority)
            }
        } else {
            String::new()
        };
        builder = builder.header(hyper::header::HOST, &authority);

        let mut uri = http::Uri::builder()
            .scheme(scheme)
            .authority(authority.clone());

        if let Some(path) = req.path_with_query {
            uri = uri.path_and_query(path);
        }

        builder = builder.uri(uri.build().map_err(http_request_error)?);

        for (k, v) in req.headers.iter() {
            builder = builder.header(k, v);
        }

        let body = req.body.unwrap_or_else(|| {
            Empty::<Bytes>::new()
                .map_err(|_| unreachable!("Infallible error"))
                .boxed()
        });

        let request = builder
            .body(body)
            .map_err(|err| internal_error(err.to_string()))?;

        Ok(Ok(self.send_request(OutgoingRequest {
            use_tls,
            authority,
            request,
            connect_timeout,
            first_byte_timeout,
            between_bytes_timeout,
        })?))
    }
}
