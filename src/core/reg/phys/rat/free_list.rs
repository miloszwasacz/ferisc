use super::{Mapping, RegIdx};
#[cfg(debug_assertions)]
use crate::core::reg::phys::id::Id;
use crate::core::reg::{
    ArchReg,
    phys::{HARDWIRED_IDX, Prf},
};
use crate::seq::Pbuf;
use generic_array::GenericArray;
use generic_array::functional::FunctionalSequence;
use static_assertions::const_assert_eq;
use std::collections::VecDeque;

//#region FreeList

const FIRST_IDX: usize = 0;

#[derive(Debug, Clone)]
pub struct FreeList {
    #[cfg(debug_assertions)]
    id: Id,
    queue: VecDeque<RegIdx>,
}

impl FreeList {
    pub fn new<T: ArchReg>(
        #[cfg(debug_assertions)] id: Id,
        prf_size: usize,
    ) -> (Self, Mapping<RegIdx, T>) {
        let mut queue = (FIRST_IDX..prf_size).collect::<VecDeque<_>>();
        let mut mapping = GenericArray::<_, T::COUNT>::default();
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

        (
            Self {
                #[cfg(debug_assertions)]
                id,
                queue,
            },
            mapping.into(),
        )
    }

    pub fn try_alloc(&mut self) -> Option<AllocLock> {
        self.queue.front().copied().map(|idx| AllocLock {
            #[cfg(debug_assertions)]
            id: self.id,
            idx,
        })
    }

    pub fn dealloc(&mut self, idx: RegIdx) {
        self.queue.push_back(idx)
    }

    pub(super) fn reset_from_committed<T: ArchReg, P>(
        &mut self,
        mapping: &Mapping<Pbuf<RegIdx, P>, T>,
        prf: &mut Prf<T>,
    ) {
        let any_hardwired = T::VARIANTS.iter().any(|reg| reg.is_hardwired());
        let mapping = {
            let mut sorted = mapping.as_refs().map(Pbuf::read_copied);
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
            .collect();
    }
}

//#endregion

//#region Locks

pub struct AllocLock {
    #[cfg(debug_assertions)]
    id: Id,
    idx: RegIdx,
}

impl AllocLock {
    pub fn idx(&self) -> RegIdx {
        self.idx
    }

    pub fn commit(self, free_list: &mut FreeList) {
        #[cfg(debug_assertions)]
        assert_eq!(free_list.id, self.id);

        let idx = free_list.queue.pop_front();
        debug_assert!(idx.is_some_and(|idx| idx == self.idx()));
    }
}

//#endregion
