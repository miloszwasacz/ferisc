use super::execute::Execute;
use crate::core::reg::{ArchReg, phys::Prf};

pub mod int;

pub trait Issue {
    type Reg: ArchReg;
    type Issued: Execute<Reg = Self::Reg>;

    fn issue(self, prf: &Prf<Self::Reg>) -> Option<Self::Issued>;
}
