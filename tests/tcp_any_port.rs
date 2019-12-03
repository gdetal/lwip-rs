use std::io;
use std::net::Ipv4Addr;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::Duration;

use tokio::io::{AsyncRead, AsyncWrite};
use tokio::runtime;
use tokio::stream::StreamExt;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::time::timeout;

#[derive(Debug)]
struct SubDeviceInner {
    txqueue: UnboundedSender<Vec<u8>>,
    rxqueue: UnboundedReceiver<Vec<u8>>,
}

#[derive(Debug)]
pub struct SubDevice(Arc<Mutex<SubDeviceInner>>);

pub struct DevicePair;

impl DevicePair {
    pub fn new() -> (SubDevice, SubDevice) {
        let (tx0, rx0) = unbounded_channel();
        let (tx1, rx1) = unbounded_channel();

        let inner0 = SubDeviceInner {
            txqueue: tx0,
            rxqueue: rx1,
        };

        let inner1 = SubDeviceInner {
            txqueue: tx1,
            rxqueue: rx0,
        };

        (
            SubDevice(Arc::new(Mutex::new(inner0))),
            SubDevice(Arc::new(Mutex::new(inner1))),
        )
    }
}

impl AsyncRead for SubDevice {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        let mut inner = self.0.lock().unwrap();

        match inner.rxqueue.poll_recv(cx) {
            Poll::Ready(Some(pkt)) => {
                buf[..pkt.len()].copy_from_slice(&pkt);
                Poll::Ready(Ok(pkt.len()))
            }
            _ => Poll::Pending,
        }
    }
}

impl AsyncWrite for SubDevice {
    fn poll_write(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        let inner = self.0.lock().unwrap();

        Poll::Ready(match inner.txqueue.send(buf.to_vec()) {
            Ok(()) => Ok(buf.len()),
            _ => Err(io::Error::new(
                io::ErrorKind::Other,
                "unable to transmit pkt",
            )),
        })
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        Poll::Ready(Ok(()))
    }
}

#[test]
fn tcp_any_port() {
    let mut rt = runtime::Builder::new()
        .threaded_scheduler()
        .enable_time()
        .build()
        .unwrap();
    rt.block_on(async { timeout(Duration::from_secs(30), tcp_any_port_async()).await })
        .unwrap()
}

async fn tcp_any_port_async() {
    let (dev0, dev1) = DevicePair::new();

    let dev0 = lwip::DeviceBuilder::default()
        .mtu(1500)
        .ipv4(Ipv4Addr::new(10, 0, 0, 1), 24)
        .build(dev0)
        .unwrap();

    let dev1 = lwip::DeviceBuilder::default()
        .mtu(1500)
        .ipv4(Ipv4Addr::new(192, 168, 0, 1), 24)
        .build(dev1)
        .unwrap();

    tokio::spawn(dev0.drive());
    tokio::spawn(dev1.drive());

    // start server:
    let echo = lwip::TcpListener::bind("10.0.0.1:0").await.unwrap();
    tokio::spawn(dummy_loop(echo)); // any loop.

    for port in (1000..=20000).step_by(1000) {
        let conn = lwip::TcpStream::connect_from(
            format!("192.168.0.1:{}", port),
            format!("10.0.0.1:{}", port),
        )
        .await
        .unwrap();

        tokio::task::spawn_blocking(move || drop(conn))
            .await
            .unwrap();
    }

    // TODO: improve this:
    // DevicePair can cause a deadlock if drops happens in a specific order.
    // Sleeping for a while solves this issue:
    std::thread::sleep(std::time::Duration::from_secs(1));
}

pub async fn dummy_loop(mut listener: lwip::tcp::TcpListener) {
    while let Some(Ok(_conn)) = listener.next().await {}
}
