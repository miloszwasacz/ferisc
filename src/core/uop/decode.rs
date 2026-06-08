use super::InferUop;
use crate::mem::Addr;

#[derive(Debug, Clone, Copy)]
pub struct Uop {
    pub kind: UopKind,
    pub addr: Addr,
    pub comp: bool,
}

pub type UopKind = super::UopKind<int::Uop>;

pub mod int {
    use super::*;
    use crate::core::{
        reg::int::IntReg,
        uop::int::{self, Src},
    };

    pub type Uop = int::Uop<IntReg, Src<IntReg>, IntReg>;

    impl InferUop<IntReg> for super::Uop {
        type Kind = Uop;
    }
}
