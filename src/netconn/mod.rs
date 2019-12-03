use std::convert::TryInto;
use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use tokio::sync::mpsc;

use bytes::{BufMut, Bytes, BytesMut};

use crate::lwip;

mod socket;
pub use self::socket::*;

#[derive(Debug)]
struct NetconnCState(
    (
        mpsc::UnboundedSender<NetconnEvent>,
        mpsc::UnboundedSender<NetconnEvent>,
    ),
);

#[derive(Debug)]
struct NetconnInner {
    conn: *mut lwip::netconn,
    ntype: NetconnType,
    rxevents: mpsc::UnboundedReceiver<NetconnEvent>,
    txevents: mpsc::UnboundedReceiver<NetconnEvent>,
}

#[derive(Debug)]
pub struct Netconn {
    inner: Arc<Mutex<NetconnInner>>,
}

#[derive(Debug)]
enum NetconnEvent {
    Data(usize),
    Error,
}

pub(crate) type NetconnType = lwip::netconn_type;

unsafe extern "C" fn netconn_callback(
    netconn: *mut lwip::netconn,
    evt: lwip::netconn_evt,
    len: u16,
) {
    let ptr = (*netconn).callback_ctx;
    if ptr == std::ptr::null_mut()
    /* TODO ?*/
    {
        return;
    }

    let state = &mut *(ptr as *mut NetconnCState);
    let inner = &state.0;

    if evt == lwip::netconn_evt::NETCONN_EVT_SENDMINUS
        || evt == lwip::netconn_evt::NETCONN_EVT_RCVMINUS
    {
        return;
    }

    match evt {
        lwip::netconn_evt::NETCONN_EVT_RCVPLUS => {
            if len == 0 && (*netconn).state != lwip::netconn_state::NETCONN_LISTEN {
                (*netconn).callback_ctx = std::ptr::null_mut();
                (*netconn).callback = None;
                Box::from_raw(state);
            } else {
                inner.0.send(NetconnEvent::Data(len as usize)).unwrap();
            }
        }
        lwip::netconn_evt::NETCONN_EVT_SENDPLUS => {
            inner.1.send(NetconnEvent::Data(len as usize)).unwrap();
        }
        lwip::netconn_evt::NETCONN_EVT_ERROR => {
            inner.0.send(NetconnEvent::Error).unwrap();
            inner.1.send(NetconnEvent::Error).unwrap();
        }
        _ => {}
    }
}

impl Netconn {
    fn new(conn: *mut lwip::netconn, ntype: NetconnType) -> Self {
        let (txtx, rxtx) = mpsc::unbounded_channel();
        let (txrx, rxrx) = mpsc::unbounded_channel();

        let mut inner = NetconnInner {
            conn: conn,
            ntype: ntype,
            txevents: rxtx,
            rxevents: rxrx,
        };

        let state = Box::into_raw(Box::new(NetconnCState((txrx, txtx))));
        unsafe {
            (*inner.conn).callback_ctx = state as *mut _;
        }

        Netconn {
            inner: Arc::new(Mutex::new(inner)),
        }
    }
    fn new_from_type(ntype: NetconnType, proto: u8) -> Self {
        crate::stack_init();

        let conn = unsafe {
            lwip::netconn_new_with_proto_and_callback(ntype, proto, Some(netconn_callback))
        };
        Self::new(conn, ntype)
    }

    pub(crate) fn new_tcp() -> Self {
        Self::new_from_type(NetconnType::NETCONN_TCP, 0)
    }

    pub(crate) fn new_raw(proto: u8) -> Self {
        Self::new_from_type(NetconnType::NETCONN_RAW_IPV6_HDRINCL, proto)
    }

    pub(crate) fn bind_if(&self, index: u8) -> io::Result<()> {
        let inner = self.inner.lock().unwrap();

        unsafe { lwip::netconn_bind_if(inner.conn, index) }.into()
    }

    pub(crate) fn bind_port(&self, port: u16) -> io::Result<()> {
        self.bind_ip_port(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port)
    }

    pub(crate) fn bind_ip_port(&self, ip: IpAddr, port: u16) -> io::Result<()> {
        let ip: lwip::ip_addr_t = ip.into();
        let inner = self.inner.lock().unwrap();
        unsafe { lwip::netconn_bind(inner.conn, &ip, port) }.into()
    }

    pub(crate) fn connect(&self, ip: IpAddr, port: u16) -> io::Result<()> {
        let ip: lwip::ip_addr_t = ip.into();
        let inner = self.inner.lock().unwrap();

        inner.set_nonblocking();

        let res: io::Result<()> = unsafe { lwip::netconn_connect(inner.conn, &ip, port) }.into();

        match res {
            Err(ref e) if e.raw_os_error() == Some(115) => Ok(()),
            res => res,
        }
    }

    pub(crate) fn listen(&self) -> io::Result<()> {
        let inner = self.inner.lock().unwrap();
        inner.set_nonblocking();

        unsafe { lwip::netconn_listen_with_backlog(inner.conn, 0xff) }.into()
    }

    pub(crate) fn accept(&self) -> io::Result<Self> {
        let mut newconn: *mut lwip::netconn = std::ptr::null_mut();
        let inner = self.inner.lock().unwrap();

        inner.set_nonblocking();

        let ret: io::Result<()> =
            unsafe { lwip::netconn_accept(inner.conn, &mut newconn as *mut *mut lwip::netconn) }
                .into();
        ret?;
        Ok(Self::new(newconn, inner.ntype))
    }

    pub(crate) fn recv(&self) -> io::Result<Bytes> {
        let mut netbuf: *mut lwip::netbuf = std::ptr::null_mut();
        let inner = self.inner.lock().unwrap();

        inner.set_nonblocking();

        let ret: io::Result<()> =
            unsafe { lwip::netconn_recv(inner.conn, &mut netbuf as *mut *mut lwip::netbuf) }.into();
        ret?;

        // netbuf to Bytes: TODO improve this :)
        let mut buffer = BytesMut::new();

        unsafe {
            while {
                let mut data: *mut u8 = std::ptr::null_mut();
                let ptr = &mut data as *mut *mut _;
                let mut len: u16 = 0;

                lwip::netbuf_data(netbuf, ptr as *mut *mut ::std::os::raw::c_void, &mut len);

                buffer.put(std::slice::from_raw_parts(data, len as usize));
                lwip::netbuf_next(netbuf) >= 0
            }
            /* do-while(netbuf_next() >= 0) */
            {}

            lwip::netbuf_delete(netbuf);
        }

        Ok(buffer.freeze())
    }

    pub(crate) fn send(&self, buf: &[u8]) -> io::Result<usize> {
        let inner = self.inner.lock().unwrap();

        inner.set_nonblocking();

        match inner.ntype {
            NetconnType::NETCONN_TCP => {
                let mut len = 0 as usize;

                let ret: io::Result<()> = unsafe {
                    lwip::netconn_write_partly(
                        inner.conn,
                        buf.as_ptr() as *const ::std::os::raw::c_void,
                        buf.len(),
                        0,
                        &mut len,
                    )
                }
                .into();
                ret?;
                Ok(len)
            }
            _ => unsafe {
                let netbuf = lwip::netbuf_new();
                let ret: io::Result<()> = lwip::netbuf_ref(
                    netbuf,
                    buf.as_ptr() as *const ::std::os::raw::c_void,
                    buf.len() as u16,
                )
                .into();
                ret?;

                let ret: io::Result<()> = lwip::netconn_send(inner.conn, netbuf).into();
                ret?;
                Ok(buf.len())
            },
        }
    }

    pub(crate) fn shutdown_tx(&self) -> io::Result<()> {
        let inner = self.inner.lock().unwrap();

        inner.set_nonblocking();

        unsafe { lwip::netconn_shutdown(inner.conn, 0, 1) }.into()
    }

    pub(crate) fn local(&self) -> io::Result<SocketAddr> {
        let inner = self.inner.lock().unwrap();

        let mut ip: lwip::ip_addr_t = IpAddr::V4(Ipv4Addr::UNSPECIFIED).into();
        let mut port: u16 = 0;

        unsafe {
            let ret: io::Result<()> =
                lwip::netconn_getaddr(inner.conn, &mut ip, &mut port, 1).into();
            ret?;
        }

        Ok(SocketAddr::new(ip.try_into()?, port))
    }

    pub(crate) fn poll_rx(&self, cx: &mut Context<'_>) -> Poll<io::Result<usize>> {
        let mut inner = self.inner.lock().unwrap();

        let poll = inner.rxevents.poll_recv(cx);
        match poll {
            Poll::Ready(Some(NetconnEvent::Data(len))) => Poll::Ready(Ok(len)),
            Poll::Ready(Some(NetconnEvent::Error)) => Poll::Ready(Err(inner.error())),
            _ => Poll::Pending,
            // TODO handle close of channel
        }
    }

    pub(crate) fn poll_tx(&self, cx: &mut Context<'_>) -> Poll<io::Result<usize>> {
        let mut inner = self.inner.lock().unwrap();

        match inner.txevents.poll_recv(cx) {
            Poll::Ready(Some(NetconnEvent::Data(len))) => Poll::Ready(Ok(len)),
            Poll::Ready(Some(NetconnEvent::Error)) => Poll::Ready(Err(inner.error())),
            _ => Poll::Pending,
            // TODO handle close of channel
        }
    }
}

impl NetconnInner {
    fn set_nonblocking(&self) {
        unsafe {
            (*self.conn).flags |= 0x2 /* TODO NETCONN_FLAG_NON_BLOCKING */ | 0x4 /* TODO NETCONN_FLAG_IN_NONBLOCKING_CONNECT */;
        }
    }

    pub(crate) fn error(&self) -> io::Error {
        unsafe { lwip::netconn_err(self.conn) }.into()
    }
}

impl Drop for NetconnInner {
    fn drop(&mut self) {
        unsafe {
            self.set_nonblocking();
            lwip::netconn_delete(self.conn);
        }
    }
}
