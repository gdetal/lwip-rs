use std::collections::VecDeque;
use std::io;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

use tokio::io::{AsyncRead, AsyncWrite};

#[derive(Debug)]
struct LoopbackInner {
    queue: VecDeque<Vec<u8>>,
    task: Option<Waker>,
}

#[derive(Debug)]
pub struct Loopback(Arc<Mutex<LoopbackInner>>);

impl Loopback {
    pub fn new() -> Self {
        let inner = LoopbackInner {
            queue: VecDeque::new(),
            task: None,
        };

        Loopback(Arc::new(Mutex::new(inner)))
    }
}

impl AsyncRead for Loopback {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        let mut inner = self.0.lock().unwrap();
        if let Some(pkt) = inner.queue.pop_front() {
            buf[..pkt.len()].copy_from_slice(&pkt);
            Poll::Ready(Ok(pkt.len()))
        } else {
            inner.task = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

impl AsyncWrite for Loopback {
    fn poll_write(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        let mut inner = self.0.lock().unwrap();

        inner.queue.push_back(buf.to_vec());

        if let Some(task) = inner.task.take() {
            task.wake();
        }

        Poll::Ready(Ok(buf.len()))
    }
    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        Poll::Ready(Ok(()))
    }
}
