use super::{EntryHandle, Flush, FlushPriority, FlushedUop, Rob, Slot, UopKind};
use crate::core::{
    Mode,
    csr::{CsrAddr, CsrFile},
    fe::AddrGenLock,
    reg::int::Uint,
    uop::{Exception, commit::Dest, rename},
};
use crate::mem::Addr;
use crate::seq::{
    Buf, Clearable, Pbuf,
    locks::{
        CircuitLock, StageLock, Transformer,
        queue::{PopLock, PopTrans, PushLock, PushTrans},
    },
};
use std::cell::RefCell;
use std::debug_assert_matches;
use std::iter::Rev;
use std::marker::PhantomData;
use std::rc::Rc;

//#region Access

pub struct AccessLock<'r>(&'r mut Rob);

impl<'r> AccessLock<'r> {
    pub(super) fn new(rob: &'r mut Rob) -> Self {
        Self(rob)
    }

    pub fn dispatch<'a>(&'a mut self) -> impl StageLock<'a, rename::Uop, (rename::Uop, EntryHandle)>
    where
        'r: 'a,
    {
        DispatchLock::new(self)
    }

    pub fn execute<'a, 'g>(&'a mut self, addr_gen: AddrGenLock<'g>) -> ExecuteLock<'a, 'r, 'g>
    where
        'r: 'a,
    {
        ExecuteLock::new(self, addr_gen)
    }

    pub fn commit<'a>(
        &'a mut self,
        csrf: &mut CsrFile,
        addr_gen: AddrGenLock<'_>,
        mode: &mut Buf<Mode>,
    ) -> impl StageLock<'a, (), Option<Dest>>
    where
        'r: 'a,
    {
        CommitLock::new(self, csrf, addr_gen, mode)
    }
}

//#endregion

//#region Dispatch

pub struct DispatchLock<'a>(PushLock<'a, Slot>);

impl<'a, 'r: 'a> DispatchLock<'a> {
    fn new(rob: &'a mut AccessLock<'r>) -> Self {
        Self(rob.0.queue.push())
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

pub struct ExecuteLock<'a, 'r, 'g> {
    rob: &'a mut AccessLock<'r>,
    addr_gen: AddrGenLock<'g>,
    cause: Option<(Addr, usize)>,
}

impl<'a, 'r: 'a, 'g> ExecuteLock<'a, 'r, 'g> {
    fn new(rob: &'a mut AccessLock<'r>, addr_gen: AddrGenLock<'g>) -> Self {
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
            .0
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

impl Drop for ExecuteLock<'_, '_, '_> {
    fn drop(&mut self) {
        if let Some((target, cause)) = self.cause {
            self.addr_gen.execute(target);
            self.rob
                .0
                .flush
                .write(Flush::After(cause), FlushPriority::Execute);
            //TODO Notify BP
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

pub struct CommitLock<'r, 'c, 'g, 'm> {
    pop: PopLock<'r, Slot>,
    csrf: &'c mut CsrFile,
    addr_gen: AddrGenLock<'g>,
    mode: &'m mut Buf<Mode>,
    flush: &'r mut Pbuf<Flush, FlushPriority>,
    count: Uint,
    special: Special,
}

impl<'a, 'r: 'a, 'c, 'g, 'm> CommitLock<'a, 'c, 'g, 'm> {
    fn new(
        rob: &'a mut AccessLock<'r>,
        csrf: &'c mut CsrFile,
        addr_gen: AddrGenLock<'g>,
        mode: &'m mut Buf<Mode>,
    ) -> Self {
        Self {
            pop: rob.0.queue.pop(rob.0.commit_width),
            csrf,
            addr_gen,
            mode,
            flush: &mut rob.0.flush,
            count: 0,
            special: Default::default(),
        }
    }
}

impl Drop for CommitLock<'_, '_, '_, '_> {
    fn drop(&mut self) {
        let count = match self.special {
            Special::None => self.count,
            Special::Csr(csr) => {
                self.csrf.commit(csr);
                self.count
            }
            Special::Exception { ex: _, pc: _ } => {
                debug_assert_matches!(
                    self.flush.read(),
                    Flush::None,
                    "cannot commit and flush in the same clock cycle"
                );
                todo!("trap handling");
                // self.csrf.mcause.implicit_write(ex.into());
                // self.csrf.mepc.implicit_write(pc.into());
                //
                // //TODO Check ISA if this privilege elevation is correct
                // self.mode.write(Mode::M);
                // self.flush.write(Flush::All, FlushPriority::Commit);
                //
                // self.count - 1
            }
        };
        self.csrf
            .minstret
            .implicit_write(self.csrf.minstret.read() + count);
    }
}

impl<'r, 'c, 'g, 'm> CircuitLock<'r> for CommitLock<'r, 'c, 'g, 'm> {
    type Trans = CommitTrans<'r, 'c, 'g, 'm>;

    fn make_trans(&mut self) -> Self::Trans {
        let trans = matches!(self.special, Special::None).then_some(CommitTransImpl {
            pop: self.pop.make_trans(),
            special: Default::default(),
        });

        CommitTrans(trans, PhantomData)
    }
}

pub struct CommitTrans<'r, 'c, 'g, 'm>(
    Option<CommitTransImpl<'r>>,
    PhantomData<CommitLock<'r, 'c, 'g, 'm>>,
);

struct CommitTransImpl<'a> {
    pop: PopTrans<'a, Slot>,
    special: Special,
}

impl<'r, 'c, 'g, 'm> Transformer<'r> for CommitTrans<'r, 'c, 'g, 'm> {
    type In = ();
    type Out = Option<Dest>;
    type Lock = CommitLock<'r, 'c, 'g, 'm>;

    fn transform(&mut self, lock: &Self::Lock, input: Self::In) -> Option<Self::Out> {
        let trans = self.0.as_mut()?;
        trans.pop.transform(&lock.pop, input).and_then(|slot| {
            let entry = slot.borrow();
            match entry.kind.read_copied() {
                UopKind::Ld { done: false, .. }
                | UopKind::Br { done: false }
                | UopKind::St { done: false } => None,
                UopKind::Reg(dest) | UopKind::Ld { dest, done: true } => Some(Some(dest)),
                UopKind::Br { done: true } | UopKind::St { done: true } | UopKind::MemOrd => {
                    Some(None)
                }
                UopKind::Csr(dest, csr) => {
                    let old = std::mem::replace(&mut trans.special, Special::Csr(csr));
                    debug_assert_matches!(old, Special::None);
                    Some(Some(dest))
                }
                UopKind::Except(ex) => {
                    let old = std::mem::replace(
                        &mut trans.special,
                        Special::Exception { ex, pc: entry.addr },
                    );
                    debug_assert_matches!(old, Special::None);
                    Some(None)
                }
            }
        })
    }

    fn commit(self, lock: &mut Self::Lock) {
        let CommitTransImpl { pop, special } = self.0.unwrap();

        pop.commit(&mut lock.pop);
        lock.count += 1;
        debug_assert_matches!(lock.special, Special::None);
        lock.special = special;
    }
}

//#endregion

//#region Flush

pub struct FlushLock<'r>(
    Rev<std::vec::IntoIter<FlushedUop>>,
    PhantomData<&'r mut Rob>,
);

impl<'a> FlushLock<'a> {
    pub(super) fn new(rob: &'a mut Rob, after: usize) -> Self {
        let from = (after + 1).max(rob.queue.len() - rob.flush_width);
        if from == after + 1 {
            rob.flush.clear();
        }
        let flushed = rob
            .queue
            .drain_from(from)
            .map(|slot| {
                let flushed = {
                    let entry = slot.borrow();
                    FlushedUop {
                        kind: entry.kind.read_copied(),
                        addr: entry.addr,
                        comp: entry.comp,
                    }
                };
                {
                    let check = Rc::downgrade(&slot);
                    drop(slot);
                    #[cfg(debug_assertions)]
                    if let Some(slot) = check.upgrade() {
                        panic!(
                            "ROB slot was not flushed properly (slot strong ref count: {})",
                            Rc::strong_count(&slot) - 1
                        );
                    }
                }
                flushed
            })
            .collect::<Box<[_]>>();

        Self(flushed.into_iter().rev(), PhantomData)
    }
}

impl<'a> CircuitLock<'a> for FlushLock<'a> {
    type Trans = FlushTrans;

    fn make_trans(&mut self) -> Self::Trans {
        FlushTrans(self.0.next())
    }
}

pub struct FlushTrans(Option<FlushedUop>);

impl<'a> Transformer<'a> for FlushTrans {
    type In = ();
    type Out = FlushedUop;
    type Lock = FlushLock<'a>;

    fn transform(&mut self, _: &Self::Lock, _: Self::In) -> Option<Self::Out> {
        self.0.take()
    }

    fn commit(self, _: &mut Self::Lock) {}
}

pub struct FlushAllLock<'a>(PhantomData<&'a mut Rob>);

impl<'a> FlushAllLock<'a> {
    pub(super) fn new(rob: &'a mut Rob) -> Self {
        rob.flush.clear();
        rob.queue.clear();

        Self(PhantomData)
    }
}

//#endregion
