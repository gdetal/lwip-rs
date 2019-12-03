#[macro_use]
extern crate rusty_fork;

use tokio::io::AsyncReadExt;
use tokio_test::*;

rusty_fork_test! {
#[test]
fn pending() {
    let dev = lwip::DeviceBuilder::loopback().unwrap();

    let mut raw = lwip::RawSocket::bind_proto(lwip::Proto::Icmp, &dev).unwrap();

    let mut buf: Vec<u8> = vec![0; 100];
    let mut t = task::spawn(raw.read(&mut buf));

    assert_pending!(t.poll());
}
}
