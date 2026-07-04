use super::uop::{FenceHandle, Uop as DUop, UopKind as DKind};
use crate::core::{
    csr::CsrAddr,
    uop::{Exception, commit::Dest, fence::Fence, rename},
};
use crate::mem::Addr;
use crate::seq::Buf;
use std::rc::Rc;

pub mod int;

#[derive(Debug, Clone)]
pub(super) struct Entry {
    pub kind: Buf<UopKind>,
    pub addr: Addr,
    pub comp: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum UopKind<F = Rc<Fence>> {
    Reg(Dest),
    Br { done: bool },
    Ld { dest: Dest, done: bool },
    St { done: bool },
    Csr(Dest, CsrAddr),
    MemOrd(F),
    Except(Exception),
}

impl From<rename::Uop> for (Entry, DUop) {
    fn from(rename::Uop { kind, addr, comp }: rename::Uop) -> Self {
        let (ekind, dkind) = match kind {
            rename::UopKind::Int(int) => (int.into(), DKind::Int(int)),
            rename::UopKind::MemOrd(fence) => {
                let fence = Rc::new(fence);
                let handle = FenceHandle::new(&fence);
                (UopKind::MemOrd(fence), DKind::MemOrd(handle))
            }
            rename::UopKind::Except(ex) => (UopKind::Except(ex), DKind::Handled),
        };

        let entry = Entry {
            kind: Buf::new(ekind),
            addr,
            comp,
        };
        let duop = DUop {
            kind: dkind,
            addr,
            comp,
        };
        (entry, duop)
    }
}
