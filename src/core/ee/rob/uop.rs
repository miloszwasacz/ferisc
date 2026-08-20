use super::Handle;
use crate::core::{
    csr::CsrAddr,
    uop::{commit::Dest, fence::Fence},
};
use derive_more::{Deref, DerefMut};

#[derive(Debug, Clone, Deref, DerefMut)]
pub struct Uop<U> {
    #[deref]
    #[deref_mut]
    pub uop: U,
    pub handle: Handle,
}

#[derive(Debug, Clone)]
pub enum Committed {
    Handled,
    Reg(Dest),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flushed {
    Reg(Dest),
    Ld(Dest),
    St,
    Csr(Dest, CsrAddr),
    MemOrd(Fence),
    Other,
}
