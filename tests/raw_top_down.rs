#[macro_use]
extern crate rusty_fork;

use std::future::Future;
use std::io;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::pin::Pin;
use std::task::{Context, Poll};

use ipnetwork::{Ipv4Network, Ipv6Network};
use packet::{builder::Builder as PBuilder, ip};
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use tokio_test::*;

use lwip::Device;

rusty_fork_test! {
#[test]
fn top_down_udp6() {
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

    block_on(top_down(lwip::Proto::Udp, pkt));
}
}

rusty_fork_test! {
#[test]
fn top_down_icmp4() {
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

        block_on(top_down(lwip::Proto::Icmp, pkt));
}
}

#[derive(Debug)]
struct PacketExpectRecv(Option<Vec<u8>>);

impl PacketExpectRecv {
    fn done(&self) -> bool {
        self.0 == None
    }
}

impl AsyncRead for PacketExpectRecv {
    fn poll_read(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        _: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        Poll::Ready(Ok(0)) // EOF
    }
}

impl AsyncWrite for PacketExpectRecv {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        if let Some(pkt) = self.0.take() {
            assert_eq!(pkt.len(), buf.len());
            assert_eq!(pkt, buf);
            Poll::Ready(Ok(buf.len()))
        } else {
            // Do not expect another write !
            panic!();
        }
    }
    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        Poll::Ready(Ok(()))
    }
}

impl Device for PacketExpectRecv {
    fn ipv4(&self) -> Ipv4Network {
        Ipv4Network::new(Ipv4Addr::UNSPECIFIED, 8).unwrap()
    }

    fn ipv6(&self) -> Vec<Ipv6Network> {
        vec![Ipv6Network::new(Ipv6Addr::UNSPECIFIED, 8).unwrap()]
    }

    fn mtu(&self) -> u16 {
        std::u16::MAX
    }
}

async fn top_down(proto: lwip::Proto, pkt: Vec<u8>) {
    let dev = PacketExpectRecv(Some(pkt.to_vec()));

    let dev = lwip::NetDevice::new(dev).unwrap();

    let mut raw = lwip::RawSocket::bind_proto(proto, &dev).unwrap();

    let v = pkt.to_vec();
    let mut t = task::spawn(raw.write(&v));

    let len = assert_ready_ok!(t.poll()); // packet is sent down the stack
    assert_eq!(len, pkt.len());

    let mut task = task::spawn(());
    let mut dev = dev.drive();
    assert_pending!(task.enter(|cx, _| Pin::new(&mut dev).poll(cx))); // packet should be received

    let (_, pe) = dev.into_inner();
    assert!(pe.done());
}
