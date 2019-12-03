use std::io::{self, Read};
use std::mem;
use std::net::Ipv4Addr;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use bytes::{Bytes, BytesMut};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc;
use transfer_async::{transfer, Transfer};

use crate::lwip::{self, FromPbuf, IntoPbuf};
use crate::Device;

#[derive(Debug)]
struct NetIfCState(Arc<Mutex<mpsc::UnboundedSender<Bytes>>>);

#[derive(Debug)]
struct NetIfInner {
    pcb: *mut lwip::netif,
    rx: mpsc::UnboundedReceiver<Bytes>,
}

#[derive(Debug)]
pub struct NetIf {
    inner: Arc<Mutex<NetIfInner>>,
}

fn netif_common_output(netif: *mut lwip::netif, p: *mut lwip::pbuf) -> lwip::err_t {
    unsafe {
        let state: &mut NetIfCState = &mut *((&mut *netif).state as *mut NetIfCState);
        let state = state.0.lock().unwrap();

        // send to channel:
        state.send(Bytes::from_pbuf(p)).unwrap();
    }
    lwip::err_enum_t::ERR_OK
}

extern "C" fn netif_output(
    netif: *mut lwip::netif,
    p: *mut lwip::pbuf,
    _: *const lwip::ip4_addr_t,
) -> lwip::err_t {
    netif_common_output(netif, p)
}

extern "C" fn netif_output_ip6(
    netif: *mut lwip::netif,
    p: *mut lwip::pbuf,
    _: *const lwip::ip6_addr_t,
) -> lwip::err_t {
    netif_common_output(netif, p)
}

unsafe extern "C" fn netif_remove_ballback(netif: *mut lwip::netif) {
    Box::from_raw((&mut *netif).state as *mut NetIfCState);

    (*netif).state = std::ptr::null_mut();
}

extern "C" fn netif_init(netif: *mut lwip::netif) -> lwip::err_t {
    unsafe {
        (*netif).output = Some(netif_output);
        (*netif).output_ip6 = Some(netif_output_ip6);
        lwip::netif_set_remove_callback(netif, Some(netif_remove_ballback));
    }
    lwip::err_enum_t::ERR_OK
}

impl NetIf {
    pub fn new<D: Device>(device: &D) -> io::Result<Self> {
        crate::stack_init();
        let (tx, rx) = mpsc::unbounded_channel();
        let pcb: *mut lwip::netif = Box::into_raw(Box::new(unsafe { mem::zeroed() }));

        let addr: lwip::ip4_addr = device.ipv4().ip().into();
        let mask: lwip::ip4_addr = device.ipv4().mask().into();
        let default: lwip::ip4_addr = Ipv4Addr::UNSPECIFIED.into();

        let ret: io::Result<()> = unsafe {
            lwip::netifapi_netif_add(
                pcb,
                &addr,
                &mask,
                &default,
                std::ptr::null_mut(),
                Some(netif_init),
                Some(lwip::tcpip_input),
            )
        }
        .into();
        ret?; // TODO

        unsafe {
            lwip::netif_set_link_up(pcb);
            lwip::netif_set_up(pcb);
        }

        // TODO Add mtu !!

        for addr in device.ipv6() {
            let addr: lwip::ip6_addr = addr.ip().into();
            unsafe {
                lwip::netif_add_ip6_address(pcb, &addr, std::ptr::null_mut());
                lwip::netif_ip6_addr_set_state(pcb, 1, 0x10 /* IP6_ADDR_VALID */);
            }
        }

        let state = Box::into_raw(Box::new(NetIfCState(Arc::new(Mutex::new(tx)))));
        unsafe {
            (*pcb).state = state as *mut _;
        }

        let inner = NetIfInner { pcb: pcb, rx: rx };

        Ok(NetIf {
            inner: Arc::new(Mutex::new(inner)),
        })
    }

    pub(crate) fn index(&self) -> u8 {
        let inner = self.inner.lock().unwrap();

        unsafe { (*inner.pcb).num + 1 }
    }
}

impl Drop for NetIfInner {
    fn drop(&mut self) {
        unsafe {
            lwip::netifapi_netif_common(self.pcb, Some(lwip::netif_remove), None);
            // free the pointer
            Box::from_raw(self.pcb);
        }
    }
}

unsafe impl Send for NetIf {}
unsafe impl Sync for NetIf {}

impl Clone for NetIf {
    fn clone(&self) -> Self {
        NetIf {
            inner: self.inner.clone(),
        }
    }
}

impl AsyncRead for NetIf {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        let mut inner = self.inner.lock().unwrap();
        match inner.rx.poll_recv(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Some(p)) => Poll::Ready(p.as_ref().read(buf)),
            Poll::Ready(None) => Poll::Pending, // TODO: maybe error ?
        }
    }
}

impl AsyncWrite for NetIf {
    fn poll_write(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        let pbuf = BytesMut::from(buf).freeze().into_pbuf();

        let inner = self.inner.lock().unwrap();

        let ret: io::Result<()> = unsafe { lwip::tcpip_input(pbuf, inner.pcb) }.into();
        ret?;
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        Poll::Ready(Ok(()))
    }
    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        Poll::Ready(Ok(()))
    }
}

pub struct NetDevice<D> {
    netif: NetIf,
    device: D,
}

impl<D> NetDevice<D>
where
    D: Device,
{
    pub fn new(device: D) -> io::Result<NetDevice<D>> {
        let netif = NetIf::new(&device)?;

        Ok(NetDevice {
            netif: netif,
            device: device,
        })
    }
}

impl<D> NetDevice<D>
where
    D: AsyncRead + AsyncWrite,
{
    pub fn drive(self) -> Transfer<NetIf, D> {
        transfer(self.netif, self.device)
    }
}

impl<D> NetDevice<D> {
    pub fn netif_as_ref(&self) -> &NetIf {
        &self.netif
    }

    pub fn device_as_ref(&self) -> &D {
        &self.device
    }

    pub fn device_as_mut(&mut self) -> &mut D {
        &mut self.device
    }
}
