#[macro_use]
extern crate rusty_fork;

use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::runtime;
use tokio::stream::StreamExt;
use tokio::time::timeout;

rusty_fork_test! {
#[test]
fn tcp_echo() {
    let mut rt = runtime::Builder::new()
        .basic_scheduler()
        .enable_time()
        .build()
        .unwrap();
    rt.block_on(async { timeout(Duration::from_secs(30), tcp_echo_async()).await })
        .unwrap();
}
}

async fn tcp_echo_async() {
    let dev = lwip::DeviceBuilder::loopback().unwrap();

    // start server:
    let echo = lwip::TcpListener::bind("0.0.0.0:1234").await.unwrap();
    tokio::spawn(echo_loop(echo));

    tokio::spawn(dev.drive());

    let mut conn = lwip::TcpStream::connect("[::1]:1234").await.unwrap();

    for _ in 1..=20 {
        conn.write(b"hello").await.unwrap();

        let mut buf = vec![0; 5];
        conn.read(&mut buf).await.unwrap();
        assert_eq!(buf, b"hello".to_owned());
    }
}

pub async fn echo_loop(mut listener: lwip::tcp::TcpListener) {
    while let Some(Ok(conn)) = listener.next().await {
        tokio::spawn(echo(conn));
    }
}

async fn echo(conn: lwip::tcp::TcpStream) {
    let (mut r, mut w) = tokio::io::split(conn);
    tokio::io::copy(&mut r, &mut w).await.unwrap();
}

#[test]
fn tcp_closed_port() {
    let mut rt = runtime::Builder::new()
        .threaded_scheduler()
        .enable_time()
        .build()
        .unwrap();
    rt.block_on(async { timeout(Duration::from_secs(30), tcp_closed_port_async()).await })
        .unwrap()
}

async fn tcp_closed_port_async() {
    let dev = lwip::DeviceBuilder::loopback().unwrap();

    tokio::spawn(dev.drive());

    for port in (1000..=20000).step_by(100) {
        let conn = lwip::TcpStream::connect(format!("127.0.0.1:{}", port))
            .await
            .expect_err("should not work");

        tokio::task::spawn_blocking(move || drop(conn))
            .await
            .unwrap();
    }
}
