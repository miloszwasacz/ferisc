use super::{DUop, Pipelines, RUop};
use crate::core::{
    reg::{int::IntReg, phys::rat::locks as rat},
    uop::{commit::Dest, decode::UopKind as DKind, rename::UopKind as RKind},
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

//#region Commit

pub struct CommitLock<'a> {
    int: rat::CommitLock<'a, IntReg>,
}

impl<'a> CommitLock<'a> {
    pub(super) fn new(pipelines: &'a mut Pipelines) -> Self {
        let Pipelines { int } = pipelines;

        Self {
            int: int.rat.commit(&mut int.prf),
        }
    }
}

impl<'a> CircuitLock<'a> for CommitLock<'a> {
    type Trans = CommitTrans;

    fn make_trans(&mut self) -> Self::Trans {
        CommitTrans {
            int: self.int.make_trans(),

            dest: None,
        }
    }
}

pub struct CommitTrans {
    int: rat::CommitTrans<IntReg>,

    dest: Option<Dest>,
}

impl<'a> Transformer<'a> for CommitTrans {
    type In = Dest;
    type Out = ();
    type Lock = CommitLock<'a>;

    fn transform(&mut self, lock: &Self::Lock, input: Self::In) -> Option<Self::Out> {
        let Self { int, dest } = self;
        let old = dest.replace(input);
        debug_assert!(old.is_none());

        match input {
            Dest::Int(input) => int.transform(&lock.int, input),
        }
    }

    fn commit(self, lock: &mut Self::Lock) {
        let Self { int, dest } = self;

        match dest.unwrap() {
            Dest::Int(_) => int.commit(&mut lock.int),
        }
    }
}

//#endregion
