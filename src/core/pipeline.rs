use self::locks::RenameLock;
use crate::core::{
    reg::{
        ArchReg,
        int::IntReg,
        phys::{Prf, Rat},
    },
    uop::{decode::Uop as DUop, rename::Uop as RUop},
};
use crate::seq::locks::StageLock;

mod locks;

#[derive(derive_more::Debug)]
pub struct Pipeline<T: ArchReg> {
    rat: Rat<T>,
    prf: Prf<T>,
}

#[derive(Debug)]
pub struct Pipelines {
    int: Pipeline<IntReg>,
}

impl Pipelines {
    pub fn rename(&mut self) -> impl StageLock<'_, DUop, RUop> {
        RenameLock::new(self)
    }
}
