use std::fmt;

#[derive(Copy, Clone, PartialEq)]
pub enum Proto {
    HopByHopOpts,
    Icmp,
    Igmp,
    Udp,
    UdpLite,
    Tcp,
    Unknown(u8),
}

impl Proto {
    pub fn value(&self) -> u8 {
        match *self {
            Proto::HopByHopOpts => 0,
            Proto::Icmp => 1,
            Proto::Igmp => 2,
            Proto::Udp => 17,
            Proto::UdpLite => 136,
            Proto::Tcp => 6,
            Proto::Unknown(value) => value,
        }
    }
}

impl fmt::Debug for Proto {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Proto::HopByHopOpts => write!(f, "Hop-by-Hop Options"),
            Proto::Icmp => write!(f, "ICMP"),
            Proto::Igmp => write!(f, "IGMP"),
            Proto::Udp => write!(f, "UDP"),
            Proto::UdpLite => write!(f, "UDPLite"),
            Proto::Tcp => write!(f, "TCP"),
            Proto::Unknown(value) => write!(f, "Unknown ({})", value),
        }
    }
}
