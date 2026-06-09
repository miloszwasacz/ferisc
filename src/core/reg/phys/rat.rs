use self::free_list::FreeList;
use self::locks::RenameLock;
use super::{Prf, RegIdx};
use crate::core::reg::ArchReg;
use crate::seq::{Buf, Sequential};
use bitflags::bitflags;
use generic_array::{ArrayLength, GenericArray};
use std::ops::{Deref, DerefMut, Index, IndexMut};

mod free_list;
pub mod locks;

//#region RAT

#[derive(Debug)]
pub struct Rat<T: ArchReg> {
    #[cfg(debug_assertions)]
    id: super::id::Id,
    mapping: Buf<Mapping<RegIdx, T>>,
    free_list: FreeList,
    committed: Buf<Mapping<RegIdx, T>>,
    #[cfg(debug_assertions)]
    state: State,
}

impl<T: ArchReg> Rat<T> {
    pub(super) fn new(#[cfg(debug_assertions)] id: super::id::Id, prf_size: usize) -> Self {
        let mut mapping = GenericArray::<_, T::COUNT>::default();
        let free_list = FreeList::new::<T>(prf_size, &mut mapping);
        let mapping = Buf::new(mapping.into());
        let committed = mapping.clone();

        Self {
            #[cfg(debug_assertions)]
            id,
            mapping,
            free_list,
            committed,
            #[cfg(debug_assertions)]
            state: State::empty(),
        }
    }

    pub fn rename<'a>(&'a mut self, prf: &'a mut Prf<T>) -> RenameLock<'a, T> {
        #[cfg(debug_assertions)]
        {
            assert_eq!(
                prf.id, self.id,
                "mismatched PRF and RAT IDs: \
                renaming should occur between corresponding PRF and RAT"
            );
            assert!(
                !self.state.contains(State::RENAMED),
                "cannot rename more than once per clock cycle"
            );
        }
        RenameLock::new(prf, self)
    }
}

impl<T: ArchReg> Clone for Rat<T>
where
    Mapping<RegIdx, T>: Copy,
{
    fn clone(&self) -> Self {
        Self {
            #[cfg(debug_assertions)]
            id: self.id,
            mapping: self.mapping.clone(),
            free_list: self.free_list.clone(),
            committed: self.committed.clone(),
            #[cfg(debug_assertions)]
            state: self.state,
        }
    }
}

impl<T: ArchReg> Sequential for Rat<T> {
    fn finish_cycle(&mut self) {
        self.mapping.finish_cycle();
        self.free_list.finish_cycle();
        self.committed.finish_cycle();
        #[cfg(debug_assertions)]
        {
            use bitflags::Flags;
            self.state.clear();
        }
    }
}

bitflags! {
    #[allow(unused)]
    #[derive(Debug, Clone, Copy)]
    struct State: u8 {
        const RENAMED = 1 << 0;
        const COMMITTED = 1 << 1;
        const FLUSHED = (1 << 2) | Self::RENAMED.bits() | Self::COMMITTED.bits();
    }
}

//#endregion

//#region Mapping

#[derive(Debug)]
struct Mapping<T, R: ArchReg>(GenericArray<T, R::COUNT>);

impl<T, R: ArchReg> Clone for Mapping<T, R>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T, R: ArchReg> Copy for Mapping<T, R>
where
    T: Copy,
    <R::COUNT as ArrayLength>::ArrayType<T>: Copy,
{
}

impl<T, R: ArchReg> From<GenericArray<T, R::COUNT>> for Mapping<T, R> {
    fn from(value: GenericArray<T, R::COUNT>) -> Self {
        Self(value)
    }
}

impl<T, R: ArchReg> Index<R> for Mapping<T, R> {
    type Output = T;

    fn index(&self, index: R) -> &Self::Output {
        self.0.index(Into::<usize>::into(index))
    }
}

impl<T, R: ArchReg> IndexMut<R> for Mapping<T, R> {
    fn index_mut(&mut self, index: R) -> &mut Self::Output {
        self.0.index_mut(Into::<usize>::into(index))
    }
}

impl<T, R: ArchReg> Deref for Mapping<T, R> {
    type Target = GenericArray<T, R::COUNT>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T, R: ArchReg> DerefMut for Mapping<T, R> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

//#endregion
