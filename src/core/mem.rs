// pub use self::cached::CachedHierarchy;
pub use self::direct::DirectHierarchy;
pub use self::entry::AddrHandle;
pub use self::locks::DispatchLock;
use crate::core::uop::fence::Fence;
use crate::mem::{LdReq, Mem, StReq};
use crate::seq::Buf;
use std::ops::DerefMut;

// mod cached;
mod direct;
mod entry;
mod locks;

pub trait MemHierarchy {
    type Err;
    type IMem<'a>: DerefMut<Target: Mem<LdReq<Self::Err>>> + 'a
    where
        Self: 'a;
    type DMem<'a>: DerefMut<Target: Mem<LdReq<Self::Err>> + Mem<StReq<Self::Err>>> + 'a
    where
        Self: 'a;

    fn imem(&mut self) -> Self::IMem<'_>;
    fn dmem(&mut self) -> Self::DMem<'_>;
}

#[derive(Debug)]
pub struct MemSubsystem<H> {
    hier: H,
    fence: Buf<Fence>,
}

impl<H: MemHierarchy> MemSubsystem<H> {
    pub fn imem(&mut self) -> H::IMem<'_> {
        self.hier.imem()
    }

    pub fn dispatch(&mut self) -> DispatchLock<'_, H> {
        todo!()
    }
}
