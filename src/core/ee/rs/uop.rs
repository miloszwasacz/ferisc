use crate::core::{
    ee::rob::lock::Flush,
    reg::ArchReg,
    uop::issue::{self, Issue, TryReadReg},
};
use std::ops::Deref;

pub trait Uop: Deref<Target: Issue<Reg = Self::Reg>> {
    type Reg: ArchReg;
    type Issued;

    fn is_flushed(&self, flush: &Flush<'_>) -> bool;
    fn issue(&self, read: impl TryReadReg<Self::Reg>) -> Option<Self::Issued>;
}

impl<T: Issue + Clone> Uop for issue::Uop<T> {
    type Reg = T::Reg;
    type Issued = issue::Uop<T::Issued>;

    fn is_flushed(&self, flush: &Flush<'_>) -> bool {
        self.handle.is_flushed(flush)
    }

    fn issue(&self, read: impl TryReadReg<Self::Reg>) -> Option<Self::Issued> {
        self.uop.clone().issue(read).map(|issued| issue::Uop {
            uop: issued,
            handle: self.handle.clone(),
        })
    }
}
