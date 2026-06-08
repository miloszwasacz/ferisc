// pub use self::cache::Cache;
pub use self::dram::Dram;
pub use self::req::*;

// mod cache;
mod dram;
mod req;

/// Physical address in memory.
pub type Addr = u64;
const _: () = {
    assert!(
        size_of::<Addr>() <= size_of::<usize>(),
        "address must fit in usize"
    );
};

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
