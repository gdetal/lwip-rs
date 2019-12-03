use std::net::Ipv4Addr;

use futures::{SinkExt, StreamExt};
use packet::{builder::Builder, icmp, ip, Packet};
use tokio::io::{copy, split, AsyncReadExt, AsyncWriteExt};
use tokio_util::codec::{BytesCodec, Framed};
use tun::Configuration;

#[tokio::main]
async fn main() {
    let mut config = Configuration::default();

    config
        .address((10, 0, 0, 1))
        .netmask((255, 255, 255, 0))
        .up();

    let dev = tun::create_as_async(&config).unwrap();

    let dev = lwip::DeviceBuilder::default()
        .mtu(1500)
        .ipv4(Ipv4Addr::new(10, 0, 0, 1), 24)
        .build(dev)
        .unwrap();

    // Create a raw socket:
    let raw = lwip::RawSocket::bind_proto(lwip::Proto::Icmp, &dev).unwrap();

    let echo = lwip::TcpListener::bind_to(1234).expect("Unable to bind TCP socket");
    tokio::spawn(echo_loop(echo));

    let serve = lwip::TcpListener::bind_to(1235).expect("Unable to bind TCP socket");
    tokio::spawn(serve_loop(serve));

    // read packet from the socket
    let mut stream = Framed::new(raw, BytesCodec::new());

    // bind tun to lwip netif
    tokio::spawn(dev.drive());

    while let Some(Ok(pkt)) = stream.next().await {
        // if the packet is an ICMP request generate a valid ICMP reply
        match ip::Packet::new(pkt) {
            Ok(ip::Packet::V4(pkt)) => match icmp::Packet::new(pkt.payload()) {
                Ok(icmp) => match icmp.echo() {
                    Ok(icmp) => {
                        println!("request: {:#?}", pkt);
                        let reply = ip::v4::Builder::default()
                            .id(0x42)
                            .unwrap()
                            .ttl(64)
                            .unwrap()
                            .source(pkt.destination())
                            .unwrap()
                            .destination(pkt.source())
                            .unwrap()
                            .icmp()
                            .unwrap()
                            .echo()
                            .unwrap()
                            .reply()
                            .unwrap()
                            .identifier(icmp.identifier())
                            .unwrap()
                            .sequence(icmp.sequence())
                            .unwrap()
                            .payload(icmp.payload())
                            .unwrap()
                            .build()
                            .unwrap();
                        stream.send(reply.into()).await.unwrap();
                    }
                    _ => {}
                },
                _ => {}
            },
            Err(err) => println!("Received an invalid packet: {:?}", err),
            _ => {}
        }
    }
}

async fn echo_loop(mut listener: lwip::tcp::TcpListener) {
    while let Some(Ok(conn)) = listener.next().await {
        println!("new connection: {:?}", conn);
        tokio::spawn(echo(conn));
    }
}

async fn echo(conn: lwip::tcp::TcpStream) {
    let (mut r, mut w) = split(conn);
    copy(&mut r, &mut w).await.unwrap();
}

async fn serve_loop(mut listener: lwip::tcp::TcpListener) {
    while let Some(Ok(conn)) = listener.next().await {
        println!("new connection: {:?}", conn);
        tokio::spawn(serve(conn));
    }
}

async fn serve(mut conn: lwip::tcp::TcpStream) {
    conn.write(b"HELO").await.unwrap();
    conn.shutdown().await.unwrap();

    let mut buf = [0; 1000];
    let r = conn.read(&mut buf).await.unwrap();

    println!("connection done! {:?}", &buf[..r]);
}
