// pub use self::cached::CachedHierarchy;
pub use self::direct::DirectHierarchy;
use crate::mem::{LdReq, Mem, StReq};
use std::ops::DerefMut;

// mod cached;
mod direct;

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
