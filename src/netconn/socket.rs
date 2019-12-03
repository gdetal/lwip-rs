use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use futures::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite};

use crate::Netconn;

#[derive(Debug)]
struct NetconnSocketInner(Netconn);

#[derive(Debug)]
pub struct NetconnSocket {
    inner: Arc<Mutex<NetconnSocketInner>>,
}

impl NetconnSocket {
    pub(crate) fn new(conn: Netconn) -> Self {
        let inner = NetconnSocketInner(conn);

        Self {
            inner: Arc::new(Mutex::new(inner)),
        }
    }

    pub fn local(&self) -> io::Result<SocketAddr> {
        let inner = self.inner.lock().unwrap();

        inner.0.local()
    }

    pub fn close(self) {
        let inner = self.inner.lock().unwrap();
        drop(inner)
    }
}

impl AsyncRead for NetconnSocket {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        let inner = self.inner.lock().unwrap();

        loop {
            return match inner.0.recv() {
                Ok(data) => {
                    let len = data.len();
                    buf[..len].clone_from_slice(&data);
                    Poll::Ready(Ok(len))
                }
                Err(ref e) if e.kind() == io::ErrorKind::NotConnected => {
                    /* EOF */
                    Poll::Ready(Ok(0))
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    match inner.0.poll_rx(cx) {
                        Poll::Ready(Ok(_)) => continue, /* more data received since first-call retry. */
                        Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
                        _ => Poll::Pending,
                    }
                }
                Err(e) => Poll::Ready(Err(e)),
            };
        }
    }
}

impl AsyncWrite for NetconnSocket {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        let inner = self.inner.lock().unwrap();

        match inner.0.send(buf) {
            Ok(len) => Poll::Ready(Ok(len)),
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => match inner.0.poll_tx(cx) {
                Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
                _ => Poll::Pending,
            },
            Err(e) => Poll::Ready(Err(e)),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        let inner = self.inner.lock().unwrap();
        Poll::Ready(inner.0.shutdown_tx())
    }
}

unsafe impl Send for NetconnSocket {}
