use super::execute::Execute;
use crate::core::{
    reg::{ArchReg, phys::RegOper},
    rob,
};

pub mod int;

#[derive(Debug, Clone)]
pub struct Uop<K> {
    pub kind: K,
    pub rob_entry: rob::EntryHandle,
}

#[allow(type_alias_bounds)]
pub type RegRepr<R: ArchReg> = R::Repr;

pub trait Issue {
    type Reg: ArchReg;
    type Issued: Execute<Reg = Self::Reg>;

    fn issue(self, try_read_reg: impl TryReadReg<Self::Reg>) -> Option<Self::Issued>;
}

pub trait TryReadReg<R: ArchReg> {
    fn try_read(&self, reg: RegOper<R>) -> Option<R::Repr>;
}
impl<R, F> TryReadReg<R> for F
where
    R: ArchReg,
    F: Fn(RegOper<R>) -> Option<R::Repr>,
{
    fn try_read(&self, reg: RegOper<R>) -> Option<R::Repr> {
        self(reg)
    }
}
