use std::io;

use futures::future::poll_fn;
use tokio::net::ToSocketAddrs;

use crate::{Netconn, NetconnSocket};

pub type TcpStream = NetconnSocket;

impl TcpStream {
    pub async fn connect<D: ToSocketAddrs>(host: D) -> io::Result<Self> {
        let netconn = Netconn::new_tcp();
        Self::connect_priv(netconn, host).await
    }

    async fn connect_priv<D: ToSocketAddrs>(netconn: Netconn, host: D) -> io::Result<Self> {
        let host = match host.to_socket_addrs().await?.next() {
            Some(host) => host,
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "unable to resolve host",
                ))
            }
        };

        netconn.connect(host.ip(), host.port())?;
        poll_fn(|cx| netconn.poll_tx(cx)).await?;
        Ok(TcpStream::new(netconn))
    }

    pub async fn connect_from<S: ToSocketAddrs, D: ToSocketAddrs>(
        src: S,
        host: D,
    ) -> io::Result<Self> {
        let src = match src.to_socket_addrs().await?.next() {
            Some(src) => src,
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "unable to resolve host",
                ))
            }
        };

        let netconn = Netconn::new_tcp();
        netconn.bind_ip_port(src.ip(), src.port())?;
        Self::connect_priv(netconn, host).await
    }
}
