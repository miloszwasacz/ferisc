use crate::core::reg::{
    ArchReg,
    phys::{Prf, RegOper},
};
use std::collections::HashMap;

pub struct Bypass<R: ArchReg>(HashMap<RegOper<R>, R::Repr>);

impl<R: ArchReg> Bypass<R> {
    pub fn try_read<'a>(&'a self, prf: &'a Prf<R>, reg: RegOper<R>) -> Option<&'a R::Repr> {
        self.0.get(&reg).or_else(|| prf[reg].try_read())
    }

    pub fn try_read_copied(&self, prf: &Prf<R>, reg: RegOper<R>) -> Option<R::Repr>
    where
        R::Repr: Copy,
    {
        self.0
            .get(&reg)
            .copied()
            .or_else(|| prf[reg].try_read_copied())
    }
}

pub struct BypassBuilder<'p, R: ArchReg> {
    bypass: Bypass<R>,
    prf: &'p mut Prf<R>,
}

impl<'p, R: ArchReg<Repr: Clone>> BypassBuilder<'p, R> {
    pub fn new(prf: &'p mut Prf<R>) -> Self {
        Self {
            bypass: Bypass(HashMap::new()),
            prf,
        }
    }

    pub fn write(&mut self, reg: RegOper<R>, val: R::Repr) {
        self.bypass.0.insert(reg, val.clone());
        self.prf[reg].write(val);
    }

    pub fn build(self) -> Bypass<R> {
        self.bypass
    }
}
