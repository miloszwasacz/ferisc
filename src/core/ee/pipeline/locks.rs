use super::{DUop, DecUop, IUop, Line, Pipeline, Pipelines, RUop};
use crate::core::{
    ee::{
        rob::{FlushedUop as FUop, FlushedUopKind as FKind},
        rs,
    },
    reg::{
        ArchReg,
        int::IntReg,
        phys::{RegDest, rat::locks as rat},
    },
    uop::{
        InferUop,
        commit::Dest,
        decode::UopKind as DecKind,
        dispatch::UopKind as DKind,
        issue::{self, Issue},
        rename::UopKind as RKind,
    },
};
use crate::seq::locks::{CircuitLock, Transformer};
use std::cmp::Reverse;
use std::debug_assert_matches;

macro_rules! commit_trans {
    ($self:expr, $lock:expr) => {{
        let Self { int, kind } = $self;
        match kind {
            UopKind::Int => int.commit(&mut $lock.int),
            UopKind::None => {}
        }
    }};
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
enum UopKind {
    #[default]
    None,
    Int,
}

impl Pipelines {
    pub fn input_locks(&mut self) -> (RenameLock<'_>, DispatchLock<'_>) {
        let Pipelines { int } = self;

        let rename = RenameLock {
            int: int.rat.rename(&mut int.prf),
        };
        let dispatch = DispatchLock {
            int: PipelineDispatchLock::new(int.lines.iter_mut().map(|(rs, _)| rs)),
        };
        (rename, dispatch)
    }

    pub fn commit(&mut self) -> CommitLock<'_> {
        CommitLock::new(self)
    }

    pub fn flush(&mut self) -> FlushLock<'_> {
        FlushLock::new(self)
    }
}

//#region Rename

pub struct RenameLock<'a> {
    int: rat::RenameLock<'a, IntReg>,
}

impl<'a> CircuitLock<'a> for RenameLock<'a> {
    type Trans = RenameTrans;

    fn make_trans(&mut self) -> Self::Trans {
        let Self { int } = self;

        Self::Trans {
            int: int.make_trans(),

            kind: UopKind::None,
        }
    }
}

pub struct RenameTrans {
    int: rat::RenameTrans<IntReg>,

    kind: UopKind,
}

impl<'a> Transformer<'a> for RenameTrans {
    type In = DecUop;
    type Out = RUop;
    type Lock = RenameLock<'a>;

    fn transform(
        &mut self,
        lock: &Self::Lock,
        DecUop { kind, addr, comp }: Self::In,
    ) -> Option<Self::Out> {
        debug_assert_matches!(self.kind, UopKind::None);
        let kind = match kind {
            DecKind::Int(uop) => {
                self.kind = UopKind::Int;
                RKind::Int(self.int.transform(&lock.int, uop)?)
            }
            DecKind::MemOrd(f) => RKind::MemOrd(f),
            DecKind::Except(e) => RKind::Except(e),
        };

        Some(Self::Out { kind, addr, comp })
    }

    fn commit(self, lock: &mut Self::Lock) {
        commit_trans!(self, lock);
    }
}

//#endregion

//#region Dispatch

pub struct DispatchLock<'a> {
    int: PipelineDispatchLock<'a, IntReg>,
}

impl<'a> CircuitLock<'a> for DispatchLock<'a> {
    type Trans = DispatchTrans;

    fn make_trans(&mut self) -> Self::Trans {
        let Self { int } = self;

        Self::Trans {
            int: int.make_trans(),

            kind: UopKind::None,
        }
    }
}

pub struct DispatchTrans {
    int: PipelineDispatchTrans<IntReg>,

    kind: UopKind,
}

impl<'a> Transformer<'a> for DispatchTrans {
    type In = DUop;
    type Out = ();
    type Lock = DispatchLock<'a>;

    fn transform(&mut self, lock: &Self::Lock, input: Self::In) -> Option<Self::Out> {
        let Self { int, kind } = self;
        debug_assert_matches!(kind, UopKind::None);

        let Self::In {
            kind: uop,
            rob_entry,
        } = input;
        match uop {
            DKind::Int(uop) => {
                *kind = UopKind::Int;
                int.transform(
                    &lock.int,
                    issue::Uop {
                        kind: uop,
                        rob_entry,
                    },
                )
            }
            DKind::Handled => Some(()),
        }
    }

    fn commit(self, lock: &mut Self::Lock) {
        commit_trans!(self, lock);
    }
}

type RsLock<'a, T> = rs::locks::DispatchLock<'a, IUop<T>>;
type RsTrans<T> = rs::locks::DispatchTrans<IUop<T>>;

pub struct PipelineDispatchLock<'a, T>(Box<[RsLock<'a, T>]>, Vec<Option<RsTrans<T>>>)
where
    T: ArchReg,
    DUop: InferUop<T>,
    IUop<T>: Issue;

impl<'a, T> PipelineDispatchLock<'a, T>
where
    T: ArchReg,
    DUop: InferUop<T>,
    IUop<T>: Issue,
{
    fn new(rss: impl IntoIterator<Item = &'a mut rs::Rs<IUop<T>>>) -> Self {
        let mut locks = rss
            .into_iter()
            .map(|rs| rs.dispatch())
            .collect::<Box<[_]>>();
        locks.sort_unstable_by_key(|l| Reverse(l.free_space()));
        let trans = Vec::with_capacity(locks.len());

        Self(locks, trans)
    }
}

impl<'a, T> CircuitLock<'a> for PipelineDispatchLock<'a, T>
where
    T: ArchReg + 'a,
    DUop: InferUop<T>,
    IUop<T>: Issue + Clone,
{
    type Trans = PipelineDispatchTrans<T>;

    fn make_trans(&mut self) -> Self::Trans {
        let mut trans = std::mem::take(&mut self.1);
        debug_assert!(trans.is_empty());
        for lock in &mut self.0 {
            trans.push(Some(lock.make_trans()));
        }

        PipelineDispatchTrans(trans, None)
    }
}

pub struct PipelineDispatchTrans<T>(Vec<Option<RsTrans<T>>>, Option<usize>)
where
    T: ArchReg,
    DUop: InferUop<T>,
    IUop<T>: Issue;

impl<'a, T> Transformer<'a> for PipelineDispatchTrans<T>
where
    T: ArchReg + 'a,
    DUop: InferUop<T>,
    IUop<T>: Issue + Clone,
{
    type In = issue::Uop<IUop<T>>;
    type Out = ();
    type Lock = PipelineDispatchLock<'a, T>;

    fn transform(&mut self, lock: &Self::Lock, input: Self::In) -> Option<Self::Out> {
        let i = self.0.iter_mut().enumerate().find_map(|(i, l)| {
            l.as_mut()
                .unwrap()
                .transform(&lock.0[i], input.clone())
                .map(|_| i)
        })?;

        let old = self.1.replace(i);
        debug_assert!(old.is_none());
        Some(())
    }

    fn commit(self, lock: &mut Self::Lock) {
        let Self(mut trans, i) = self;
        let i = i.unwrap();

        trans[i].take().unwrap().commit(&mut lock.0[i]);
        trans.clear();
        lock.1 = trans;

        // We update the locks so that the sorting over free space is maintained
        let free_space = lock.0[i].free_space();
        let mut i = i;
        while i < lock.0.len().saturating_sub(1) {
            if lock.0[i + 1].free_space() <= free_space {
                break;
            }

            lock.0.swap(i, i + 1);
            i += 1;
        }
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

            kind: UopKind::None,
        }
    }
}

pub struct CommitTrans {
    int: rat::CommitTrans<IntReg>,

    kind: UopKind,
}

impl<'a> Transformer<'a> for CommitTrans {
    type In = Option<Dest>;
    type Out = ();
    type Lock = CommitLock<'a>;

    fn transform(&mut self, lock: &Self::Lock, input: Self::In) -> Option<Self::Out> {
        let Self { int, kind } = self;

        debug_assert_matches!(kind, UopKind::None);
        *kind = match input {
            None => UopKind::None,
            Some(dest) => match dest {
                Dest::Int(dest) => int.transform(&lock.int, dest).map(|_| UopKind::Int),
            }?,
        };

        Some(())
    }

    fn commit(self, lock: &mut Self::Lock) {
        commit_trans!(self, lock);
    }
}

//#endregion

//#region Flush

pub struct FlushLock<'a> {
    int: PipelineFlushLock<'a, IntReg>,
}

impl<'a> FlushLock<'a> {
    pub(super) fn new(pipelines: &'a mut Pipelines) -> Self {
        let Pipelines { int } = pipelines;

        Self {
            int: PipelineFlushLock::new(int),
        }
    }
}

impl<'a> CircuitLock<'a> for FlushLock<'a> {
    type Trans = FlushTrans;

    fn make_trans(&mut self) -> Self::Trans {
        FlushTrans(None)
    }
}

pub struct FlushTrans(Option<Dest>);

impl<'a> Transformer<'a> for FlushTrans {
    type In = FUop;
    type Out = FUop;
    type Lock = FlushLock<'a>;

    fn transform(&mut self, _: &Self::Lock, input: Self::In) -> Option<Self::Out> {
        debug_assert!(self.0.is_none());
        self.0 = match input.kind {
            FKind::Reg(dest) | FKind::Ld { dest, .. } | FKind::Csr(dest, _) => Some(dest),
            FKind::Br { .. } | FKind::St { .. } | FKind::MemOrd | FKind::Except(_) => None,
        };

        Some(input)
    }

    fn commit(self, lock: &mut Self::Lock) {
        let Some(dest) = self.0 else {
            return;
        };
        macro_rules! flush {
            ($pipeline:expr, $dest:expr) => {{
                let mut trans = $pipeline.make_trans();
                trans.transform($pipeline, $dest).or_else(|| {
                    panic!(concat!("cannot flush ", stringify!($pipeline), " pipeline"))
                });
                trans.commit($pipeline);
            }};
        }

        let Self::Lock { int } = lock;
        match dest {
            Dest::Int(dest) => flush!(int, dest),
        }
    }
}

pub struct PipelineFlushLock<'a, T>
where
    T: ArchReg,
    DUop: InferUop<T>,
    IUop<T>: Issue,
{
    rat: rat::FlushLock<'a, T>,
    lines: &'a mut [Line<T>],
}

impl<'a, T> PipelineFlushLock<'a, T>
where
    T: ArchReg,
    DUop: InferUop<T>,
    IUop<T>: Issue,
{
    fn new(pipeline: &'a mut Pipeline<T>) -> Self {
        Self {
            rat: pipeline.rat.flush(&mut pipeline.prf),
            lines: &mut pipeline.lines,
        }
    }
}

impl<T> Drop for PipelineFlushLock<'_, T>
where
    T: ArchReg,
    DUop: InferUop<T>,
    IUop<T>: Issue,
{
    fn drop(&mut self) {
        for (rs, eu) in self.lines.iter_mut() {
            rs.flush();
            eu.flush();
        }
    }
}

impl<'a, T> CircuitLock<'a> for PipelineFlushLock<'a, T>
where
    T: ArchReg + 'a,
    DUop: InferUop<T>,
    IUop<T>: Issue,
{
    type Trans = PipelineFlushTrans<T>;

    fn make_trans(&mut self) -> Self::Trans {
        let Self { rat, lines: _ } = self;

        PipelineFlushTrans {
            rat: rat.make_trans(),
        }
    }
}

pub struct PipelineFlushTrans<T: ArchReg> {
    rat: rat::FlushTrans<T>,
}

impl<'a, T> Transformer<'a> for PipelineFlushTrans<T>
where
    T: ArchReg + 'a,
    DUop: InferUop<T>,
    IUop<T>: Issue,
{
    type In = RegDest<T>;
    type Out = ();
    type Lock = PipelineFlushLock<'a, T>;

    fn transform(&mut self, lock: &Self::Lock, input: Self::In) -> Option<Self::Out> {
        let Self { rat } = self;

        rat.transform(&lock.rat, input)
            .or_else(|| panic!("cannot flush RAT"));

        Some(())
    }

    fn commit(self, lock: &mut Self::Lock) {
        let Self { rat } = self;

        rat.commit(&mut lock.rat);
    }
}

//#endregion
