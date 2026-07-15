use super::{Mapping, RegIdx, RenameLock};
use crate::core::reg::{
    ArchReg,
    phys::{HARDWIRED_IDX, Prf},
};
use crate::seq::Sequential;
use generic_array::GenericArray;
use static_assertions::const_assert_eq;
use std::collections::VecDeque;

//#region FreeList

pub(super) type Queue = VecDeque<RegIdx>;
const FIRST_IDX: usize = 0;

#[derive(Debug, Clone)]
pub struct FreeList {
    queue: Queue,
    #[cfg(debug_assertions)]
    state: super::State,
}

impl FreeList {
    pub fn new<T: ArchReg>(prf_size: usize, mapping: &mut GenericArray<RegIdx, T::COUNT>) -> Self {
        let mut queue = (FIRST_IDX..prf_size).collect::<Queue>();
        for (i, reg) in T::VARIANTS.iter().enumerate() {
            if !reg.is_hardwired() {
                mapping[i] = queue
                    .pop_back()
                    .expect("prf should be big enough to fit all architectural registers");
            } else {
                mapping[i] = HARDWIRED_IDX;

                // The hardwired should never be able to be allocated
                const_assert_eq!(HARDWIRED_IDX, FIRST_IDX);
                assert!(
                    queue.pop_front_if(|idx| *idx == HARDWIRED_IDX).is_some(),
                    "not enough physical registers to accommodate for a hardwired register"
                );
            }
        }

        Self {
            queue,
            #[cfg(debug_assertions)]
            state: super::State::empty(),
        }
    }

    pub(super) fn allocate(&mut self, count: usize) {
        drop(self.queue.drain(..count));
        #[cfg(debug_assertions)]
        self.state.insert(super::State::RENAMED);
    }

    pub(super) fn deallocate(&mut self, mut deallocated: Queue) {
        self.queue.append(&mut deallocated);
        #[cfg(debug_assertions)]
        self.state.insert(super::State::COMMITTED);
    }

    pub(super) fn reset_from_mapping<T: ArchReg>(
        &mut self,
        mapping: &Mapping<RegIdx, T>,
        prf: &mut Prf<T>,
    ) {
        #[cfg(debug_assertions)]
        {
            assert!(
                !self.state.intersects(super::State::FLUSHED),
                "cannot flush and rename or commit in the same clock cycle"
            );
            self.state.insert(super::State::FLUSHED);
        }

        let any_hardwired = T::VARIANTS.iter().any(|reg| reg.is_hardwired());
        let mapping = {
            let mut sorted = *mapping;
            sorted.sort();
            sorted
        };

        let mut i = 0;
        self.queue = (FIRST_IDX..prf.size())
            .filter(|idx| match mapping.get(i) {
                Some(mapped) if *mapped == *idx => {
                    i += 1;
                    false
                }
                _ if *idx == HARDWIRED_IDX && any_hardwired => false,
                None | Some(_) => {
                    if prf.regs[*idx].try_read().is_none() {
                        prf.regs[*idx].write(Default::default());
                    }
                    true
                }
            })
            .collect::<Queue>();
    }
}

impl Sequential for FreeList {
    fn finish_cycle(&mut self) {
        #[cfg(debug_assertions)]
        {
            use bitflags::Flags;
            self.state.clear();
        }
    }
}

//#endregion

//#region Locks

pub struct AllocLock {
    allocated: usize,
}

impl<'a, T: ArchReg> RenameLock<'a, T> {
    pub fn alloc_lock(&mut self) -> AllocLock {
        #[cfg(debug_assertions)]
        assert!(
            !self.rat().free_list.state.contains(super::State::RENAMED),
            "cannot rename more than once per clock cycle"
        );
        AllocLock {
            allocated: self.alloc_count(),
        }
    }
}

impl AllocLock {
    pub fn try_alloc<'a, T: ArchReg>(&mut self, rename_lock: &RenameLock<'a, T>) -> Option<RegIdx> {
        rename_lock
            .rat()
            .free_list
            .queue
            .get(self.allocated)
            .copied()
            .inspect(|_| self.allocated += 1)
    }
}

//#endregion
