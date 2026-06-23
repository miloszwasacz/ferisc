use crate::core::{
    reg::{int::IntReg, phys::RegOper},
    uop::{InferUop, int::*},
};

pub type Uop = EeUop<Src<RegOper<IntReg>>, RegOper<IntReg>>;

impl InferUop<IntReg> for super::Uop {
    type Kind = Uop;
}
