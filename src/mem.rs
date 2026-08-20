// pub use self::cache::Cache;
pub use self::dram::Dram;
pub use self::req::*;
pub use self::size::{Bytes, ScalarSize, Size, VectorSize};
use std::fmt;

// mod cache;
mod dram;
mod req;
mod size;

/// Physical address in memory.
pub type Addr = u64;
const _: () = {
    assert!(
        size_of::<Addr>() <= size_of::<usize>(),
        "address must fit in usize"
    );
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AddrView(pub Addr);

impl AddrView {
    /// The minimal width of the hexadecimal representation of the address
    /// (excluding the _0x_ prefix).
    pub const PRETTY_WIDTH: usize = 7;
}

impl fmt::Display for AddrView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !f.alternate() {
            write!(f, "0x{:0width$x}", self.0, width = f.width().unwrap_or(0))
        } else {
            write!(
                f,
                "{:0width$x}",
                self.0,
                width = f
                    .width()
                    .map(|w| w.max(Self::PRETTY_WIDTH))
                    .unwrap_or(Self::PRETTY_WIDTH)
            )
        }
    }
}

pub trait Mem<R> {
    type Conn: Copy;
    type Handle: Sized;
    type Output;

    fn register_conn(&mut self) -> Self::Conn;
    fn req(&mut self, req: R, conn: Self::Conn) -> Self::Handle;
    fn req_addr(&self, handle: &Self::Handle) -> Addr;
    fn take_if_ready(&mut self, handle: Self::Handle) -> Result<Self::Output, Self::Handle>;
    fn kill(&mut self, handle: Self::Handle);
}

pub trait FromBytes {
    fn from_bytes(bytes: Bytes) -> Self;
}
impl<T: From<Bytes>> FromBytes for T {
    fn from_bytes(bytes: Bytes) -> Self {
        T::from(bytes)
    }
}

pub trait FromBytesSigned {
    fn from_bytes(bytes: Bytes, signed: bool) -> Self;
}

pub trait ToBytes {
    fn to_bytes(&self, size: Size) -> Bytes;
}
