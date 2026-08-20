use crate::core::uop::Exception;
use crate::mem::{LdReq, Mem, ReqData, StReq};
use crate::seq::Sequential;
use std::ops::{Deref, DerefMut};

// pub mod cached;
pub mod direct;

pub trait MemHierarchy: Sequential {
    type IMem: IMem;
    type DMem: DMem;

    fn imem(&self) -> impl Deref<Target = Self::IMem>;
    fn imem_mut(&mut self) -> impl DerefMut<Target = Self::IMem>;
    fn dmem(&self) -> impl Deref<Target = Self::DMem>;
    fn dmem_mut(&mut self) -> impl DerefMut<Target = Self::DMem>;
}

pub trait IMem: Mem<LdReq<Exception>, Output = Result<ReqData, Exception>> {}
impl<T: Mem<LdReq<Exception>, Output = Result<ReqData, Exception>>> IMem for T {}

pub trait DMem:
    Mem<LdReq<Exception>, Output = Result<ReqData, Exception>>
    + Mem<StReq<Exception>, Output = Result<(), Exception>>
{
}
impl<T> DMem for T where
    T: Mem<LdReq<Exception>, Output = Result<ReqData, Exception>>
        + Mem<StReq<Exception>, Output = Result<(), Exception>>
{
}
