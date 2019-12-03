mod lwip;

mod raw;
pub use raw::*;

pub mod tcp;
pub use tcp::*;

mod netconn;
pub use netconn::*;

mod dev;
pub use dev::*;

use std::io;
use std::sync::Once;

static mut BUILDER_INIT: bool = false;
static BUILDER_INIT_ONCE: Once = Once::new();

pub(crate) fn stack_init() {
    BUILDER_INIT_ONCE.call_once(|| unsafe {
        let err: io::Result<()> = lwip::tcpip_init_block().into();
        err.expect("unable to initialise the TCP/IP stack");
        BUILDER_INIT = true;
    });
}
