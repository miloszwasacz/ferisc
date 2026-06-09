use super::{DUop, Pipelines, RUop};
use crate::core::{
    reg::{int::IntReg, phys::rat::locks as rat},
    uop::{decode::UopKind as DKind, rename::UopKind as RKind},
};
use crate::seq::locks::{CircuitLock, Transformer};

//#region Rename

pub struct RenameLock<'a> {
    int: rat::RenameLock<'a, IntReg>,
}

impl<'a> RenameLock<'a> {
    pub(super) fn new(pipelines: &'a mut Pipelines) -> Self {
        let Pipelines { int } = pipelines;

        Self {
            int: int.rat.rename(&mut int.prf),
        }
    }
}

impl<'a> CircuitLock<'a> for RenameLock<'a> {
    type Trans = RenameTrans;

    fn make_trans(&mut self) -> Self::Trans {
        let Self { int } = self;

        Self::Trans {
            int: int.make_trans(),
        }
    }
}

pub struct RenameTrans {
    int: rat::RenameTrans<IntReg>,
}

impl<'a> Transformer<'a> for RenameTrans {
    type In = DUop;
    type Out = RUop;
    type Lock = RenameLock<'a>;

    fn transform(
        &mut self,
        lock: &Self::Lock,
        DUop { kind, addr, comp }: Self::In,
    ) -> Option<Self::Out> {
        let kind = match kind {
            DKind::Int(uop) => RKind::Int(self.int.transform(&lock.int, uop)?),
            DKind::MemOrd(f) => RKind::MemOrd(f),
            DKind::Except(e) => RKind::Except(e),
        };

        Some(Self::Out { kind, addr, comp })
    }

    fn commit(self, lock: &mut Self::Lock) {
        let Self { int } = self;
        int.commit(&mut lock.int);
    }
}

//#endregion
