use std::future::Future;
use std::io;
use std::pin::Pin;
use std::sync::Once;
use std::task::{Context, Poll};

use tokio::io::{AsyncRead, AsyncWrite};
use transfer_async::{transfer, Transfer};

use crate::lwip;
use crate::NetIf;
use crate::Device;
use crate::LoopbackNetIf;

static mut BUILDER_INIT: bool = false;
static BUILDER_INIT_ONCE: Once = Once::new();

pin_project_lite::pin_project! {
    pub struct Stack<S>
    {
        #[pin]
        tr: Transfer<NetIf, S>,
    }
}

impl Default for Stack<LoopbackNetIf> {
    fn default() -> Self {
        Self::new(LoopbackNetIf::new()).unwrap()
    }
}

impl<D> Stack<D>
where
    D: AsyncRead + AsyncWrite + Device,
{
    pub fn new(device: D) -> io::Result<Self> {
        if unsafe { BUILDER_INIT } {
            Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "Can only instanciate one builder",
            ))
        } else {
            Self::init(device)
        }
    }

    fn init(device: D) -> io::Result<Self> {
        BUILDER_INIT_ONCE.call_once(|| unsafe {
            let err: io::Result<()> = lwip::tcpip_init_block().into();
            err.expect("unable to initialise the TCP/IP stack");
            BUILDER_INIT = true;
        });
        Ok(Stack {
            tr: transfer(NetIf::new(&device)?, device),
        })
    }

    pub fn into_inner(self) -> D {
        let (_, device) = self.tr.into_inner();
        device
    }
}

impl<D> Future for Stack<D>
where
    D: AsyncRead + AsyncWrite + Device,
{
    type Output = io::Result<(u64, u64)>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.project().tr.poll(cx)
    }
}
