use super::{Mapping, Rat, State, free_list::AllocLock};
use crate::core::{
    reg::{
        ArchReg,
        phys::{HARDWIRED_IDX, Prf, RegDest, RegIdx, RegOper},
    },
    uop::{
        Infer, InferUop,
        decode::Uop as DUop,
        rename::{Rename, Repr, Uop as RUop},
    },
};
use crate::seq::locks::{CircuitLock, Transformer};
use std::collections::VecDeque;

//#region Rename

pub struct RenameLock<'a, T: ArchReg> {
    prf: &'a mut Prf<T>,
    rat: &'a mut Rat<T>,
    mapping: Mapping<RegIdx, T>,
    allocated: Vec<(RegIdx, Option<Repr<T>>)>,
}

impl<'a, T: ArchReg> RenameLock<'a, T> {
    pub(super) fn new(prf: &'a mut Prf<T>, rat: &'a mut Rat<T>) -> Self {
        let mapping = rat.mapping.read_copied();
        Self {
            prf,
            rat,
            mapping,
            allocated: Vec::new(),
        }
    }

    pub(super) fn rat(&self) -> &Rat<T> {
        self.rat
    }

    pub(super) fn alloc_count(&self) -> usize {
        self.allocated.len()
    }
}

impl<'a, T: ArchReg> CircuitLock<'a> for RenameLock<'a, T>
where
    DUop: InferUop<T>,
    RUop: InferUop<T>,
    Infer<DUop, T>: Rename<Reg = T, Renamed = Infer<RUop, T>>,
{
    type Trans = RenameTrans<T>;

    fn make_trans(&mut self) -> Self::Trans {
        RenameTrans {
            alloc_lock: self.alloc_lock(),
            allocated: Vec::new(),
        }
    }
}

impl<'a, T: ArchReg> Drop for RenameLock<'a, T> {
    fn drop(&mut self) {
        self.rat.mapping.write(self.mapping);
        self.rat.free_list.allocate(self.allocated.len());
        for (preg, value) in std::mem::take(&mut self.allocated) {
            let reg = &mut self.prf.regs[preg];
            match value {
                None => reg.mark_busy(),
                Some(value) => reg.write(value),
            }
        }
    }
}

pub struct RenameTrans<T: ArchReg> {
    alloc_lock: AllocLock,
    allocated: Vec<(T, RegIdx, Option<Repr<T>>)>,
}

impl<'a, T: ArchReg + 'a> Transformer<'a> for RenameTrans<T>
where
    DUop: InferUop<T>,
    RUop: InferUop<T>,
    Infer<DUop, T>: Rename<Reg = T, Renamed = Infer<RUop, T>>,
{
    type In = Infer<DUop, T>;
    type Out = Infer<RUop, T>;
    type Lock = RenameLock<'a, T>;

    fn transform(&mut self, lock: &Self::Lock, input: Self::In) -> Option<Self::Out> {
        input.rename(
            |reg| RegOper {
                #[cfg(debug_assertions)]
                id: lock.rat.id,
                idx: lock.mapping[reg],
                arch: reg,
            },
            |reg, value| {
                if reg.is_hardwired() {
                    return Some(RegDest {
                        oper: RegOper {
                            #[cfg(debug_assertions)]
                            id: lock.rat.id,
                            idx: HARDWIRED_IDX,
                            arch: reg,
                        },
                        prev: HARDWIRED_IDX,
                    });
                }

                self.alloc_lock
                    .try_alloc(lock)
                    .map(|idx| RegDest {
                        oper: RegOper {
                            #[cfg(debug_assertions)]
                            id: lock.rat.id,
                            idx,
                            arch: reg,
                        },
                        prev: lock.mapping[reg],
                    })
                    .inspect(|dest| self.allocated.push((dest.arch, dest.idx, value)))
            },
        )
    }

    fn commit(self, lock: &mut Self::Lock) {
        #[cfg(debug_assertions)]
        let mut dests = std::collections::HashSet::<T>::with_capacity(self.allocated.len());
        lock.allocated.reserve(self.allocated.len());
        for (arch, phys, value) in self.allocated {
            #[cfg(debug_assertions)]
            assert!(
                dests.insert(arch),
                "a single uop cannot use the same architectural register for multiple destinations"
            );
            lock.mapping[arch] = phys;
            lock.allocated.push((phys, value));
        }

        #[cfg(debug_assertions)]
        lock.rat.state.insert(State::RENAMED);
    }
}

//#endregion

//#region Commit

pub struct CommitLock<'a, T: ArchReg> {
    prf: &'a mut Prf<T>,
    rat: &'a mut Rat<T>,
    mapping: Mapping<RegIdx, T>,
    deallocated: VecDeque<RegIdx>,
}

impl<'a, T: ArchReg> CommitLock<'a, T> {
    pub(super) fn new(prf: &'a mut Prf<T>, rat: &'a mut Rat<T>) -> Self {
        let mapping = rat.committed.read_copied();
        Self {
            prf,
            rat,
            mapping,
            deallocated: VecDeque::new(),
        }
    }
}

impl<'a, T: ArchReg> CircuitLock<'a> for CommitLock<'a, T> {
    type Trans = CommitTrans<T>;

    fn make_trans(&mut self) -> Self::Trans {
        CommitTrans(None)
    }
}

impl<T: ArchReg> Drop for CommitLock<'_, T> {
    fn drop(&mut self) {
        self.rat.committed.write(self.mapping);
        self.rat
            .free_list
            .deallocate(std::mem::take(&mut self.deallocated));

        #[cfg(debug_assertions)]
        self.rat.state.insert(State::COMMITTED);
    }
}

pub struct CommitTrans<T: ArchReg>(Option<RegDest<T>>);

impl<'a, T: ArchReg + 'a> Transformer<'a> for CommitTrans<T> {
    type In = RegDest<T>;
    type Out = ();
    type Lock = CommitLock<'a, T>;

    fn transform(&mut self, lock: &Self::Lock, input: Self::In) -> Option<Self::Out> {
        lock.prf[*input].try_read().map(|_| {
            let old = self.0.replace(input);
            debug_assert!(old.is_none());
        })
    }

    fn commit(self, lock: &mut Self::Lock) {
        let dest = self.0.unwrap();
        if !dest.arch.is_hardwired() {
            lock.mapping[dest.arch] = dest.idx;
            lock.deallocated.push_back(dest.prev);
        }
    }
}

//#endregion

//#region Flush

pub struct FlushLock<'a, T: ArchReg> {
    prf: &'a mut Prf<T>,
    rat: &'a mut Rat<T>,
    mapping: Mapping<RegIdx, T>,
    deallocated: VecDeque<RegIdx>,
}

impl<'a, T: ArchReg> FlushLock<'a, T> {
    pub(super) fn new(prf: &'a mut Prf<T>, rat: &'a mut Rat<T>) -> Self {
        let mapping = rat.mapping.read_copied();
        Self {
            prf,
            rat,
            mapping,
            deallocated: VecDeque::new(),
        }
    }
}

impl<'a, T: ArchReg> CircuitLock<'a> for FlushLock<'a, T> {
    type Trans = FlushTrans<T>;

    fn make_trans(&mut self) -> Self::Trans {
        FlushTrans(None)
    }
}

impl<T: ArchReg> Drop for FlushLock<'_, T> {
    fn drop(&mut self) {
        for reg in &self.deallocated {
            self.prf.regs[*reg].write(Default::default());
        }
        self.rat.mapping.write(self.mapping);
        self.rat
            .free_list
            .deallocate(std::mem::take(&mut self.deallocated));

        #[cfg(debug_assertions)]
        self.rat.state.insert(State::FLUSHED);
    }
}

pub struct FlushTrans<T: ArchReg>(Option<RegDest<T>>);

impl<'a, T: ArchReg + 'a> Transformer<'a> for FlushTrans<T> {
    type In = RegDest<T>;
    type Out = ();
    type Lock = FlushLock<'a, T>;

    fn transform(&mut self, _: &Self::Lock, input: Self::In) -> Option<Self::Out> {
        let old = self.0.replace(input);
        debug_assert!(old.is_none());
        Some(())
    }

    fn commit(self, lock: &mut Self::Lock) {
        let dest = self.0.unwrap();
        if !dest.arch.is_hardwired() {
            lock.mapping[dest.arch] = dest.prev;
            lock.deallocated.push_back(dest.idx)
        }
    }
}

//#endregion
