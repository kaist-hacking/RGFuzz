use crate::preview2::bindings::sockets::instance_network;
use crate::preview2::network::Network;
use crate::preview2::WasiView;
use wasmtime::component::Resource;

impl<T: WasiView> instance_network::Host for T {
    fn instance_network(&mut self) -> Result<Resource<Network>, anyhow::Error> {
        let network = Network {
            socket_addr_check: self.ctx().socket_addr_check.clone(),
            allow_ip_name_lookup: self.ctx().allowed_network_uses.ip_name_lookup,
        };
        let network = self.table().push(network)?;
        Ok(network)
    }
}
