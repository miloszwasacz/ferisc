#[cfg(debug_assertions)]
use self::debug::Debug;
use self::free_list::FreeList;
use self::mapping::Mapping;
pub use self::stats::{RegStats, Stats};
use self::substage::{Commit, Rename};
#[cfg(debug_assertions)]
use super::id::Id;
use super::{Prf, RegDest, RegIdx};
use crate::core::reg::ArchReg;
use crate::seq::{Pbuf, Sequential};
use std::ops::DerefMut;

mod free_list;
mod mapping;
mod stats;
pub mod substage;

macro_rules! assert_ids_eq {
    ($rat:expr, $prf:expr) => {
        assert_eq!($rat.id, $prf.id, "mismatched PRF and RAT IDs")
    };
}

//#region RAT

#[derive(Debug)]
pub struct Rat<T: ArchReg> {
    #[cfg(debug_assertions)]
    id: Id,
    mapping: Mapping<RegBuf, T>,
    free_list: FreeList,
    committed: Mapping<RegBuf, T>,
    #[cfg(debug_assertions)]
    debug: Debug,
}

impl<T: ArchReg> Rat<T> {
    pub(super) fn new(#[cfg(debug_assertions)] id: Id, prf_size: usize) -> Self {
        let (free_list, mapping) = FreeList::new::<T>(
            #[cfg(debug_assertions)]
            id,
            prf_size,
        );
        let mapping = mapping.map(Pbuf::new);
        let committed = mapping.clone();

        Self {
            #[cfg(debug_assertions)]
            id,
            mapping,
            free_list,
            committed,
            #[cfg(debug_assertions)]
            debug: Default::default(),
        }
    }

    pub fn rename<'a>(&'a mut self, prf: &'a mut Prf<T>) -> Rename<'a, T> {
        #[cfg(debug_assertions)]
        assert_ids_eq!(self, prf);
        #[cfg(debug_assertions)]
        self.debug.rename();

        Rename::new(self, prf)
    }

    pub fn commit<'a>(&'a mut self, prf: &'a mut Prf<T>) -> Commit<'a, T> {
        #[cfg(debug_assertions)]
        assert_ids_eq!(self, prf);
        #[cfg(debug_assertions)]
        self.debug.commit();

        Commit::new(self, prf)
    }

    //TODO Add a note that the iterator should be in reverse order to what was returned by `rename`
    pub fn flush(&mut self, prf: &mut Prf<T>, flushed: impl IntoIterator<Item = RegDest<T>>) {
        #[cfg(debug_assertions)]
        assert_ids_eq!(self, prf);
        #[cfg(debug_assertions)]
        self.debug.flush();

        for (priority, dest) in flushed
            .into_iter()
            .enumerate()
            .filter(|(_, dest)| !dest.arch.is_hardwired())
        {
            // Update mapping
            self.mapping[dest.arch].write(dest.prev, Priority::Flush(priority));

            // Update free list
            self.free_list.dealloc(dest.idx);

            // Update PRF
            prf[*dest].reset();
        }
    }

    pub fn flush_all(&mut self, prf: &mut Prf<T>) {
        #[cfg(debug_assertions)]
        assert_ids_eq!(self, prf);
        #[cfg(debug_assertions)]
        self.debug.flush();

        // Update mapping
        self.committed
            .iter()
            .map(Pbuf::read_copied)
            .enumerate()
            .for_each(|(i, idx)| {
                self.mapping.deref_mut()[i].write(idx, Priority::Flush(usize::MAX))
            });

        // Update free list and PRF
        self.free_list.reset_from_committed(&self.committed, prf);
    }
}

impl<T: ArchReg> Clone for Rat<T>
where
    Mapping<RegIdx, T>: Clone,
{
    fn clone(&self) -> Self {
        Self {
            #[cfg(debug_assertions)]
            id: self.id,
            mapping: self.mapping.clone(),
            free_list: self.free_list.clone(),
            committed: self.committed.clone(),
            #[cfg(debug_assertions)]
            debug: self.debug,
        }
    }
}

impl<T: ArchReg> Sequential for Rat<T> {
    fn finish_cycle(&mut self) {
        #[cfg(debug_assertions)]
        self.debug.finish_cycle();

        self.mapping.iter_mut().for_each(Sequential::finish_cycle);
        self.committed.iter_mut().for_each(Sequential::finish_cycle);
    }
}

#[cfg(debug_assertions)]
mod debug {
    use crate::seq::Sequential;
    use bitflags::{Flags, bitflags};

    macro_rules! assert_not_twice {
        ($self:expr, $state:ident, $name:expr) => {
            assert!(
                !$self.0.contains(State::$state),
                concat!(
                    "cannot ",
                    stringify!($name),
                    " twice in the same clock cycle"
                ),
            )
        };
    }
    macro_rules! assert_not_flushed {
        ($self:expr, $action:expr) => {
            assert!(
                !$self.0.contains(State::FLUSHED),
                concat!("cannot ", $action, " and flush in the same clock cycle"),
            )
        };
    }
    macro_rules! states {
        ($(
            $name:ident, $state:ident
        ),*) => {
            impl Debug {
                $(
                pub fn $name(&mut self) {
                    assert_not_twice!(self, $state, stringify!($name));
                    assert_not_flushed!(self, stringify!($name));
                    self.0.insert(State::$state);
                }
                )*
            }
        };
    }

    #[derive(Debug, Clone, Copy)]
    pub(super) struct Debug(State);

    impl Default for Debug {
        fn default() -> Self {
            Self(State::empty())
        }
    }

    bitflags! {
        #[derive(Debug, Clone, Copy)]
        struct State: u8 {
            const RENAMED = 1 << 0;
            const COMMITTED = 1 << 1;
            const FLUSHED = (1 << 2) | Self::RENAMED.bits() | Self::COMMITTED.bits();
        }
    }

    states!(rename, RENAMED, commit, COMMITTED);

    impl Debug {
        pub fn flush(&mut self) {
            assert!(
                !self.0.intersects(State::FLUSHED),
                "cannot flush and perform another operation in the same clock cycle",
            );
            self.0.insert(State::FLUSHED);
        }
    }

    impl Sequential for Debug {
        fn finish_cycle(&mut self) {
            self.0.clear()
        }
    }
}

//#endregion

//#region RegBuf

type RegBuf = Pbuf<RegIdx, Priority>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Priority {
    Commit(usize),
    Rename(usize),
    Flush(usize),
}

//#endregion
