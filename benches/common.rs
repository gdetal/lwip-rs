use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::runtime;
use tokio::stream::StreamExt;

use criterion::*;

#[allow(dead_code)]
pub fn criterion() -> Criterion {
    Criterion::default()
        .warm_up_time(Duration::from_secs(5))
        .sample_size(100)
        .measurement_time(Duration::from_secs(10))
        .confidence_level(0.80)
        .without_plots()
}

pub fn benchmark_futures<T: std::future::Future>(benchmark: T) {
    let mut rt = runtime::Builder::new()
        .threaded_scheduler() // basic_scheduler() causes a deadlock due to Criterion.
        .core_threads(1)
        .max_threads(1)
        .build()
        .unwrap();
    rt.block_on(benchmark);
}

pub async fn benchmark_async(c: &mut Criterion, name: &str, dst: &str) {
    let dev = lwip::DeviceBuilder::loopback().unwrap();

    tokio::spawn(dev.drive());

    let mut port = 0;
    c.bench_function(&name, |b| {
        b.iter_batched(
            || {
                port += 1;
                init_server(port);
                format!("{}:{}", dst, port)
            },
            |dst| {
                run_client(black_box(&dst), black_box(20));
            },
            BatchSize::PerIteration,
        )
    });
}

pub fn init_server(port: u16) {
    let echo = lwip::TcpListener::bind_to(port).unwrap();
    tokio::spawn(echo_loop(echo));
}

pub fn run_client(dst: &str, n: u32) {
    futures::executor::block_on(run_client_async(dst, n));
}

pub async fn run_client_async(dst: &str, n: u32) {
    let mut conn = lwip::TcpStream::connect(dst).await.unwrap();

    for _ in 1..=n {
        conn.write(b"hello").await.unwrap();

        let mut buf = vec![0; 5];
        conn.read(&mut buf).await.unwrap();
        assert_eq!(buf, b"hello".to_owned());
    }
}

pub async fn echo_loop(mut listener: lwip::tcp::TcpListener) {
    if let Some(Ok(conn)) = listener.next().await {
        // expect a single connection.
        tokio::spawn(echo(conn));
    }
}

async fn echo(conn: lwip::tcp::TcpStream) {
    let (mut r, mut w) = tokio::io::split(conn);
    tokio::io::copy(&mut r, &mut w).await.unwrap();
}
