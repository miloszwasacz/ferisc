use super::{EntryHandle, Flush, FlushPriority, Rob, Slot, UopKind};
use crate::core::{
    Mode,
    csr::{CsrAddr, CsrFile},
    fe::AddrGenLock,
    pipeline::locks::{CommitLock as PipelineCommitLock, CommitTrans as PipelineCommitTrans},
    reg::int::Uint,
    uop::{Exception, rename},
};
use crate::mem::Addr;
use crate::seq::{
    Buf, Pbuf,
    locks::{
        CircuitLock, Transformer,
        queue::{PopLock, PopTrans, PushLock, PushTrans},
    },
};
use std::cell::RefCell;
use std::debug_assert_matches;
use std::marker::PhantomData;
use std::rc::Rc;

//#region Dispatch

pub struct DispatchLock<'a>(PushLock<'a, Slot>);

impl<'a> DispatchLock<'a> {
    pub(super) fn new(rob: &'a mut Rob) -> Self {
        Self(rob.queue.push())
    }
}

impl<'a> CircuitLock<'a> for DispatchLock<'a> {
    type Trans = DispatchTrans;

    fn make_trans(&mut self) -> Self::Trans {
        DispatchTrans(self.0.make_trans())
    }
}

pub struct DispatchTrans(PushTrans<Slot>);

impl<'a> Transformer<'a> for DispatchTrans {
    type In = rename::Uop;
    type Out = (rename::Uop, EntryHandle);
    type Lock = DispatchLock<'a>;

    fn transform(&mut self, lock: &Self::Lock, input: Self::In) -> Option<Self::Out> {
        let entry = Rc::new(RefCell::new(input.into()));
        let handle = EntryHandle(Rc::downgrade(&entry));
        self.0.transform(&lock.0, entry).map(|_| (input, handle))
    }

    fn commit(self, lock: &mut Self::Lock) {
        self.0.commit(&mut lock.0)
    }
}

//#endregion

//#region Execute

pub struct ExecuteLock<'r, 'g> {
    rob: &'r mut Rob,
    addr_gen: AddrGenLock<'g>,
    cause: Option<(Addr, usize)>,
}

impl<'r, 'g> ExecuteLock<'r, 'g> {
    pub(super) fn new(rob: &'r mut Rob, addr_gen: AddrGenLock<'g>) -> Self {
        Self {
            rob,
            addr_gen,
            cause: None,
        }
    }

    pub fn jump(&mut self, target: Addr, cause: &EntryHandle) {
        // Save only the index of the earliest possible cause
        self.cause = self
            .rob
            .queue
            .iter()
            .enumerate()
            .take(self.cause.map(|(_, i)| i).unwrap_or(usize::MAX))
            // We use `rfind` because the causes will most likely be towards the
            // end of the queue, but it relies on the assumption that there are
            // no duplicates (which would be nonsensical in this context).
            .rfind(|(_, s)| cause.0.ptr_eq(&Rc::downgrade(s)))
            .map(|(i, _)| (target, i))
            // We don't need to check whether the new index is smaller because
            // we only search up to the currently earliest cause.
            .or(self.cause);

        debug_assert!(self.cause.is_some(), "flush cause not found in the ROB");
    }
}

impl Drop for ExecuteLock<'_, '_> {
    fn drop(&mut self) {
        if let Some((target, cause)) = self.cause {
            self.addr_gen.execute(target);
            self.rob
                .flush
                .write(Flush::After(cause), FlushPriority::Execute);
        }
    }
}

//#endregion

//#region Commit

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Special {
    #[default]
    None,
    Csr(CsrAddr),
    Exception {
        ex: Exception,
        pc: Addr,
    },
}

pub struct CommitLock<'r, 'p, 'c, 'g, 'm> {
    pop: PopLock<'r, Slot>,
    commit: PipelineCommitLock<'p>,
    csrf: &'c mut CsrFile,
    addr_gen: AddrGenLock<'g>,
    mode: &'m mut Buf<Mode>,
    flush: &'r mut Pbuf<Flush, FlushPriority>,
    count: Uint,
    special: Special,
}

impl<'r, 'p, 'c, 'g, 'm> CommitLock<'r, 'p, 'c, 'g, 'm> {
    pub(super) fn new(
        rob: &'r mut Rob,
        pipelines: PipelineCommitLock<'p>,
        csrf: &'c mut CsrFile,
        addr_gen: AddrGenLock<'g>,
        mode: &'m mut Buf<Mode>,
    ) -> Self {
        Self {
            pop: rob.queue.pop(rob.commit_width),
            commit: pipelines,
            csrf,
            addr_gen,
            mode,
            flush: &mut rob.flush,
            count: 0,
            special: Default::default(),
        }
    }
}

impl Drop for CommitLock<'_, '_, '_, '_, '_> {
    fn drop(&mut self) {
        let count = match self.special {
            Special::None => self.count,
            Special::Csr(csr) => {
                self.csrf.commit(csr);
                self.count
            }
            Special::Exception { ex, pc } => {
                debug_assert_matches!(
                    self.flush.read(),
                    Flush::None,
                    "cannot commit and flush in the same clock cycle"
                );
                todo!("trap handling");
                self.csrf.mcause.implicit_write(ex.into());
                self.csrf.mepc.implicit_write(pc.into());

                //TODO Check ISA if this privilege elevation is correct
                self.mode.write(Mode::M);
                self.flush.write(Flush::All, FlushPriority::Commit);

                self.count - 1
            }
        };
        self.csrf
            .minstret
            .implicit_write(self.csrf.minstret.read() + count);
    }
}

impl<'r, 'p, 'c, 'g, 'm> CircuitLock<'r> for CommitLock<'r, 'p, 'c, 'g, 'm> {
    type Trans = CommitTrans<'r, 'p, 'c, 'g, 'm>;

    fn make_trans(&mut self) -> Self::Trans {
        let trans = matches!(self.special, Special::None).then_some(CommitTransImpl {
            pop: self.pop.make_trans(),
            commit: self.commit.make_trans(),
            special: Default::default(),
        });

        CommitTrans(trans, PhantomData)
    }
}

pub struct CommitTrans<'r, 'p, 'c, 'g, 'm>(
    Option<CommitTransImpl<'r>>,
    PhantomData<CommitLock<'r, 'p, 'c, 'g, 'm>>,
);

struct CommitTransImpl<'a> {
    pop: PopTrans<'a, Slot>,
    commit: PipelineCommitTrans,
    special: Special,
}

impl<'r, 'p, 'c, 'g, 'm> Transformer<'r> for CommitTrans<'r, 'p, 'c, 'g, 'm> {
    type In = ();
    type Out = ();
    type Lock = CommitLock<'r, 'p, 'c, 'g, 'm>;

    fn transform(&mut self, lock: &Self::Lock, input: Self::In) -> Option<Self::Out> {
        let trans = self.0.as_mut()?;
        trans.pop.transform(&lock.pop, input).and_then(|slot| {
            let entry = slot.borrow();
            match entry.kind.read_copied() {
                UopKind::Ld { done: false, .. }
                | UopKind::Br { done: false }
                | UopKind::St { done: false } => None,
                UopKind::Reg(dest) | UopKind::Ld { dest, done: true } => {
                    trans.commit.transform(&lock.commit, dest)
                }
                UopKind::Br { done: true } | UopKind::St { done: true } | UopKind::MemOrd => {
                    Some(())
                }
                UopKind::Csr(dest, csr) => {
                    trans.commit.transform(&lock.commit, dest).inspect(|_| {
                        let old = std::mem::replace(&mut trans.special, Special::Csr(csr));
                        debug_assert_matches!(old, Special::None);
                    })
                }
                UopKind::Except(ex) => {
                    let old = std::mem::replace(
                        &mut trans.special,
                        Special::Exception { ex, pc: entry.addr },
                    );
                    debug_assert_matches!(old, Special::None);
                    Some(())
                }
            }
        })
    }

    fn commit(self, lock: &mut Self::Lock) {
        let CommitTransImpl {
            pop,
            commit,
            special,
        } = self.0.unwrap();

        pop.commit(&mut lock.pop);
        commit.commit(&mut lock.commit);
        lock.count += 1;
        debug_assert_matches!(lock.special, Special::None);
        lock.special = special;
    }
}

//#endregion
