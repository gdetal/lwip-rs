use std::io;

use crate::raw::Proto;
use crate::{NetDevice, Netconn, NetconnSocket};

pub type RawSocket = NetconnSocket;

impl RawSocket {
    pub fn bind_proto<D>(proto: Proto, dev: &NetDevice<D>) -> io::Result<RawSocket> {
        let raw = Netconn::new_raw(proto.value());
        raw.bind_if(dev.netif_as_ref().index())?;
        Ok(NetconnSocket::new(raw))
    }
}
