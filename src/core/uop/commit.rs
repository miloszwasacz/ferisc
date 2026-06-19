use crate::core::reg::{int::IntReg, phys::RegDest};

#[derive(Debug, Clone, Copy)]
pub enum Dest {
    Int(RegDest<IntReg>),
}
