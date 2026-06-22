use crate::core::{
    csr::CsrAddr,
    uop::{Exception, commit::Dest, rename},
};
use crate::mem::Addr;
use crate::seq::Buf;

pub mod int;

#[derive(Debug, Clone)]
pub(super) struct Entry {
    pub kind: Buf<UopKind>,
    pub addr: Addr,
    pub comp: bool,
    pub flush: bool,
}

//TODO If the Jump is resolved at the EXECUTE stage, not the COMMIT stage,
//     flushing has to be done per instruction unless extra state is stored.
#[derive(Debug, Clone, Copy)]
pub enum UopKind {
    Reg(Dest),
    Br { done: bool },
    Ld { dest: Dest, done: bool },
    St { done: bool },
    Csr(Dest, CsrAddr),
    MemOrd,
    Except(Exception),
}

impl From<rename::Uop> for Entry {
    fn from(value: rename::Uop) -> Self {
        let rename::Uop { kind, addr, comp } = value;
        let kind = match kind {
            rename::UopKind::Int(int) => int.into(),
            rename::UopKind::MemOrd(_) => UopKind::MemOrd,
            rename::UopKind::Except(ex) => UopKind::Except(ex),
        };

        Self {
            kind: Buf::new(kind),
            addr,
            comp,
            flush: false,
        }
    }
}
