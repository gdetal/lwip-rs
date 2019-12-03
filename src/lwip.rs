#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(unused)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use std::convert::Into;
use std::convert::TryInto;
use std::io;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::os::raw::c_void;

use byteorder::{ByteOrder, NativeEndian, NetworkEndian};
use bytes::{Bytes, BytesMut};

impl Into<io::Result<()>> for err_t {
    fn into(self) -> io::Result<()> {
        if self == err_t::ERR_OK {
            Ok(())
        } else {
            Err(self.into())
        }
    }
}

impl Into<io::Error> for err_t {
    fn into(self) -> io::Error {
        io::Error::from_raw_os_error(unsafe { err_to_errno(self) })
    }
}

pub trait IntoPbuf {
    fn into_pbuf(self) -> *mut pbuf;
}

pub trait FromPbuf {
    fn from_pbuf(p: *mut pbuf) -> Self;
}

impl FromPbuf for Bytes {
    fn from_pbuf(p: *mut pbuf) -> Self {
        unsafe {
            let mut buf: Vec<u8> = vec![0; (&mut *p).tot_len as usize];
            let len =
                pbuf_copy_partial(p, buf.as_mut_ptr() as *mut c_void, buf.len() as u16, 0) as usize;
            //pbuf_free(p); // TODO handle free correctly !
            buf.truncate(len);
            Bytes::from(buf)
        }
    }
}

impl IntoPbuf for Bytes {
    fn into_pbuf(self) -> *mut pbuf {
        unsafe {
            let p = pbuf_alloc(pbuf_layer::PBUF_IP, self.len() as u16, pbuf_type::PBUF_RAM);
            let result: io::Result<()> =
                pbuf_take(p, self.as_ptr() as *const c_void, self.len() as u16).into();
            result.unwrap();
            p
        }
    }
}

impl TryInto<IpAddr> for ip_addr {
    type Error = io::Error;

    fn try_into(self) -> Result<IpAddr, Self::Error> {
        match self.type_ as lwip_ip_addr_type {
            lwip_ip_addr_type_IPADDR_TYPE_V4 => Ok(unsafe { self.u_addr.ip4 }.into()),
            lwip_ip_addr_type_IPADDR_TYPE_V6 => Ok(unsafe { self.u_addr.ip6 }.into()),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "neither IPv4 or IPv6 address",
            )),
        }
    }
}

impl ip_addr {
    pub(crate) fn unspecified() -> Self {
        ip_addr {
            type_: lwip_ip_addr_type_IPADDR_TYPE_ANY as u8,
            u_addr: unsafe { std::mem::zeroed() },
        }
    }
}

impl Into<ip_addr> for IpAddr {
    fn into(self) -> ip_addr {
        if self.is_unspecified() {
            ip_addr::unspecified()
        } else {
            match self {
                IpAddr::V4(addr) => addr.into(),
                IpAddr::V6(addr) => addr.into(),
            }
        }
    }
}

impl Into<Ipv4Addr> for ip4_addr {
    fn into(self) -> Ipv4Addr {
        let mut buf = [0; 4];
        NativeEndian::write_u32(&mut buf, self.addr);
        Ipv4Addr::from(buf)
    }
}

impl Into<ip4_addr> for Ipv4Addr {
    fn into(self) -> ip4_addr {
        ip4_addr {
            addr: NativeEndian::read_u32(&self.octets()),
        }
    }
}

impl Into<ip6_addr> for Ipv4Addr {
    fn into(self) -> ip6_addr {
        ip6_addr {
            zone: 0,
            addr: [0, 0, 0xffffffff, NativeEndian::read_u32(&self.octets())],
        }
    }
}

impl Into<IpAddr> for ip4_addr {
    fn into(self) -> IpAddr {
        IpAddr::V4(self.into())
    }
}

impl Into<ip_addr> for Ipv4Addr {
    fn into(self) -> ip_addr {
        ip_addr {
            type_: lwip_ip_addr_type_IPADDR_TYPE_V4 as u8,
            u_addr: ip_addr__bindgen_ty_1 { ip4: self.into() },
        }
    }
}

impl Into<Ipv6Addr> for ip6_addr {
    fn into(self) -> Ipv6Addr {
        let mut buf = [0; 16];
        for n in 0..=3 {
            NativeEndian::write_u32(&mut buf[(n + 0)..(n + 4)], self.addr[n]);
        }
        Ipv6Addr::from(buf)
    }
}

impl Into<ip6_addr> for Ipv6Addr {
    fn into(self) -> ip6_addr {
        let octets = self.octets();
        ip6_addr {
            zone: 0,
            addr: [
                NativeEndian::read_u32(&octets[0..4]),
                NativeEndian::read_u32(&octets[4..8]),
                NativeEndian::read_u32(&octets[8..12]),
                NativeEndian::read_u32(&octets[12..16]),
            ],
        }
    }
}

impl Into<IpAddr> for ip6_addr {
    fn into(self) -> IpAddr {
        IpAddr::V6(self.into())
    }
}

impl Into<ip_addr> for Ipv6Addr {
    fn into(self) -> ip_addr {
        ip_addr {
            type_: lwip_ip_addr_type_IPADDR_TYPE_V6 as u8,
            u_addr: ip_addr__bindgen_ty_1 { ip6: self.into() },
        }
    }
}
