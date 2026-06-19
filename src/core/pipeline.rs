use self::locks::{CommitLock, RenameLock};
use crate::core::{
    reg::{
        ArchReg,
        int::IntReg,
        phys::{Prf, Rat},
    },
    uop::{decode::Uop as DUop, rename::Uop as RUop},
};

pub mod locks;

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
    pub fn rename(&mut self) -> RenameLock<'_> {
        RenameLock::new(self)
    }

    pub fn commit(&mut self) -> CommitLock<'_> {
        CommitLock::new(self)
    }
}
