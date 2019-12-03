use std::io;

use tokio::net::ToSocketAddrs;

use crate::netconn::Netconn;
use crate::tcp::TcpListener;

pub struct StackBuilder(Netconn);

impl StackBuilder {
    pub fn new() -> Self {
        Self(Netconn::new_tcp())
    }

    pub fn bind_to(self, port: u16) -> io::Result<TcpListener> {
        let netconn = self.0;
        netconn.bind_port(port)?;
        netconn.listen()?;
        Ok(TcpListener::new(netconn))
    }

    pub fn bind_any(self) -> io::Result<TcpListener> {
        self.bind_to(0)
    }

    pub async fn bind_local<T: ToSocketAddrs>(self, host: T) -> io::Result<Self> {
        let netconn = self.0;

        let host = match host.to_socket_addrs().await?.next() {
            Some(host) => host,
            None => return Err(io::Error::new(io::ErrorKind::Other, "unable to resolve host")),
        };

        netconn.bind_ip_port(host.ip(), host.port())?;
        Ok(StackBuilder(netconn))
    }
}
