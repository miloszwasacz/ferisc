use crate::core::{
    ee::pipeline,
    reg::{int::IntReg, phys::RegDest},
};

pub type Dest = pipeline::Kind<RegDest<IntReg>>;
