use super::{InferUop, decode};
use crate::core::reg::{
    ArchReg, RegRepr,
    phys::{RegDest, RegOper},
};
use crate::mem::Addr;

pub mod int;

#[allow(type_alias_bounds)]
pub type Repr<T: RegRepr> = <T as RegRepr>::Repr;

pub trait Rename {
    type Reg: ArchReg;
    type Renamed;

    fn rename(
        self,
        get_oper: impl Fn(Self::Reg) -> RegOper<Self::Reg>,
        alloc_dest: impl FnMut(Self::Reg, Option<Repr<Self::Reg>>) -> Option<RegDest<Self::Reg>>,
    ) -> Option<Self::Renamed>;
}

#[derive(Debug, Clone, Copy)]
pub struct Uop {
    pub kind: UopKind,
    pub addr: Addr,
    pub comp: bool,
}

pub type UopKind = super::UopKind<int::Uop>;
