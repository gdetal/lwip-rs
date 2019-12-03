use std::io;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use futures::task::{Context, Poll};
use futures::Stream;
use tokio::net::ToSocketAddrs;

use crate::netconn::Netconn;
use crate::tcp::TcpStream;

#[derive(Debug)]
pub struct TcpListenerInner {
    conn: Netconn,
}

#[derive(Debug)]
pub struct TcpListener {
    inner: Arc<Mutex<TcpListenerInner>>,
}

impl TcpListener {
    pub async fn bind<T: ToSocketAddrs>(host: T) -> io::Result<Self> {
        let host = match host.to_socket_addrs().await?.next() {
            Some(host) => host,
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "unable to resolve host",
                ))
            }
        };

        let netconn = Netconn::new_tcp();
        netconn.bind_ip_port(host.ip(), host.port())?;
        netconn.listen()?;
        Ok(TcpListener::new(netconn))
    }

    pub fn bind_to(port: u16) -> io::Result<Self> {
        let netconn = Netconn::new_tcp();
        netconn.bind_port(port)?;
        netconn.listen()?;
        Ok(TcpListener::new(netconn))
    }

    pub fn bind_any() -> io::Result<Self> {
        TcpListener::bind_to(0)
    }

    pub(crate) fn new(conn: Netconn) -> Self {
        Self {
            inner: Arc::new(Mutex::new(TcpListenerInner { conn })),
        }
    }
}

impl Stream for TcpListener {
    type Item = io::Result<TcpStream>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let inner = self.inner.lock().unwrap();

        match inner.conn.poll_rx(cx) {
            Poll::Ready(Ok(_)) => match inner.conn.accept() {
                Ok(conn) => {
                    let stream = TcpStream::new(conn);
                    Poll::Ready(Some(Ok(stream)))
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => unreachable!(),
                Err(e) => Poll::Ready(Some(Err(e))),
            },
            Poll::Ready(Err(e)) => Poll::Ready(Some(Err(e))),
            _ => Poll::Pending,
        }
    }
}

unsafe impl Send for TcpListener {}
