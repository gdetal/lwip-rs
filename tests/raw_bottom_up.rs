#[macro_use]
extern crate rusty_fork;

use std::future::Future;
use std::io;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use ipnetwork::{Ipv4Network, Ipv6Network};
use packet::{builder::Builder as PBuilder, ip};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite};
use tokio::time::timeout;
use tokio_test::*;

use lwip::Device;

rusty_fork_test! {
#[test]
fn bottom_up_icmp4() {
    let pkt = ip::v4::Builder::default()
        .id(0x42)
        .unwrap()
        .ttl(64)
        .unwrap()
        .source("1.2.3.4".parse().unwrap())
        .unwrap()
        .destination("5.6.7.8".parse().unwrap())
        .unwrap()
        .icmp()
        .unwrap()
        .echo()
        .unwrap()
        .request()
        .unwrap()
        .build()
        .unwrap();

        block_on(bottom_up(lwip::Proto::Icmp, pkt));
}
}

rusty_fork_test! {
#[test]
#[should_panic]
fn bottom_up_icmp4_on_udp() {
    let pkt = ip::v4::Builder::default()
        .id(0x42)
        .unwrap()
        .ttl(64)
        .unwrap()
        .source("1.2.3.4".parse().unwrap())
        .unwrap()
        .destination("5.6.7.8".parse().unwrap())
        .unwrap()
        .icmp()
        .unwrap()
        .echo()
        .unwrap()
        .request()
        .unwrap()
        .build()
        .unwrap();

        block_on(bottom_up(lwip::Proto::Udp, pkt));
}
}

rusty_fork_test! {
#[test]
fn bottom_up_udp6() {
    let pkt = ip::v6::Builder::default()
        .hop_limit(64)
        .unwrap()
        .source("2001:db8::1".parse().unwrap())
        .unwrap()
        .destination("2001:db8:1::1".parse().unwrap())
        .unwrap()
        .udp()
        .unwrap()
        .source(1234)
        .unwrap()
        .destination(5678)
        .unwrap()
        .build()
        .unwrap();

        block_on(bottom_up(lwip::Proto::Udp, pkt));
}
}

rusty_fork_test! {
#[test]
#[should_panic]
fn bottom_up_udp6_on_tcp() {
    let pkt = ip::v6::Builder::default()
        .hop_limit(64)
        .unwrap()
        .source("2001:db8::1".parse().unwrap())
        .unwrap()
        .destination("2001:db8:1::1".parse().unwrap())
        .unwrap()
        .udp()
        .unwrap()
        .source(1234)
        .unwrap()
        .destination(5678)
        .unwrap()
        .build()
        .unwrap();

    block_on(bottom_up(lwip::Proto::Tcp, pkt));
}
}

#[derive(Debug)]
struct PacketExpectSend(Option<Vec<u8>>);

impl PacketExpectSend {
    fn done(&self) -> bool {
        self.0 == None
    }
}

impl AsyncRead for PacketExpectSend {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        if let Some(pkt) = self.0.take() {
            let len = pkt.len();
            buf[..len].copy_from_slice(&pkt);
            Poll::Ready(Ok(len))
        } else {
            Poll::Ready(Ok(0)) // EOF
        }
    }
}

impl AsyncWrite for PacketExpectSend {
    fn poll_write(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        _: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        // Do not expect a write !
        panic!();
    }
    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        Poll::Ready(Ok(()))
    }
}

impl Device for PacketExpectSend {
    fn ipv4(&self) -> Ipv4Network {
        Ipv4Network::new(Ipv4Addr::LOCALHOST, 8).unwrap()
    }

    fn ipv6(&self) -> Vec<Ipv6Network> {
        vec![Ipv6Network::new(Ipv6Addr::LOCALHOST, 8).unwrap()]
    }

    fn mtu(&self) -> u16 {
        std::u16::MAX
    }
}

async fn bottom_up(proto: lwip::Proto, pkt: Vec<u8>) {
    let dev = PacketExpectSend(Some(pkt.to_vec()));

    let dev = lwip::NetDevice::new(dev).unwrap();
    let mut raw = lwip::RawSocket::bind_proto(proto, &dev).unwrap();

    let mut dev = dev.drive();

    let mut task = task::spawn(());
    assert_pending!(task.enter(|cx, _| Pin::new(&mut dev).poll(cx))); // packet is sent up the stack

    let mut buf: Vec<u8> = vec![0; pkt.len()];

    // for safety: wait for max 100 ms
    let len = timeout(Duration::from_millis(100), raw.read(&mut buf))
        .await
        .unwrap()
        .unwrap();

    assert_eq!(len, pkt.len());
    assert_eq!(pkt, buf);

    let (_, pe) = dev.into_inner();
    assert!(pe.done());
}
