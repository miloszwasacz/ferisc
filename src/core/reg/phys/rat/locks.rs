use super::{Mapping, Rat, free_list::AllocLock};
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
            },
            |reg, value| {
                if reg.is_hardwired() {
                    return Some(RegDest {
                        oper: RegOper {
                            #[cfg(debug_assertions)]
                            id: lock.rat.id,
                            idx: HARDWIRED_IDX,
                        },
                        arch: reg,
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
                        },
                        arch: reg,
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
    }
}

//#endregion
