use std::io;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::pin::Pin;
use std::task::{Context, Poll};

use ipnetwork::{Ipv4Network, Ipv6Network};
use tokio::io::{AsyncRead, AsyncWrite};

use crate::{Loopback, NetDevice};

pub trait Device {
    fn ipv4(&self) -> Ipv4Network;
    fn ipv6(&self) -> Vec<Ipv6Network>;
    fn mtu(&self) -> u16;
}

#[derive(Debug)]
pub struct DeviceBuilder {
    mtu: u16,
    ipv4: Ipv4Network,
    ipv6: Vec<Ipv6Network>,
}

impl Default for DeviceBuilder {
    fn default() -> Self {
        DeviceBuilder {
            mtu: 1500,
            ipv4: Ipv4Network::new(Ipv4Addr::UNSPECIFIED, 0).unwrap(),
            ipv6: Vec::new(),
        }
    }
}

impl DeviceBuilder {
    pub fn loopback() -> io::Result<NetDevice<DeviceWrapper<Loopback>>> {
        let dev = Loopback::new();
        Self::default()
            .mtu(std::u16::MAX)
            .ipv4(Ipv4Addr::LOCALHOST, 8)
            .ipv6(Ipv6Addr::LOCALHOST, 8)
            .build(dev)
    }

    pub fn mtu(mut self, mtu: u16) -> Self {
        self.mtu = mtu;
        self
    }

    pub fn ipv4(mut self, addr: Ipv4Addr, prefix: u8) -> Self {
        self.ipv4 = Ipv4Network::new(addr, prefix).unwrap();
        self
    }

    pub fn ipv6(mut self, addr: Ipv6Addr, prefix: u8) -> Self {
        self.ipv6.push(Ipv6Network::new(addr, prefix).unwrap());
        self
    }

    pub fn build<D: AsyncRead + AsyncWrite>(
        self,
        underlying: D,
    ) -> io::Result<NetDevice<DeviceWrapper<D>>> {
        let dev = DeviceWrapper {
            underlying: underlying,
            builder: self,
        };

        NetDevice::new(dev)
    }
}

pin_project_lite::pin_project! {
    #[derive(Debug)]
    pub struct DeviceWrapper<D>{
        #[pin]
        underlying: D,
        builder: DeviceBuilder,
    }
}

impl<D> DeviceWrapper<D> {
    pub fn into_inner(self) -> D {
        self.underlying
    }
}

impl<D> AsyncRead for DeviceWrapper<D>
where
    D: AsyncRead,
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        self.project().underlying.poll_read(cx, buf)
    }
}

impl<D> AsyncWrite for DeviceWrapper<D>
where
    D: AsyncWrite,
{
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        self.project().underlying.poll_write(cx, buf)
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        self.project().underlying.poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        self.project().underlying.poll_shutdown(cx)
    }
}

impl<D> Device for DeviceWrapper<D> {
    fn ipv4(&self) -> Ipv4Network {
        self.builder.ipv4
    }

    fn ipv6(&self) -> Vec<Ipv6Network> {
        self.builder.ipv6.clone()
    }

    fn mtu(&self) -> u16 {
        self.builder.mtu
    }
}
