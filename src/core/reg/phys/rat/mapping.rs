use crate::core::reg::ArchReg;
use derive_more::{Deref, DerefMut};
use generic_array::functional::FunctionalSequence;
use generic_array::{ArrayLength, GenericArray};
use std::ops::{Index, IndexMut};

#[derive(Debug, Deref, DerefMut)]
pub struct Mapping<T, R: ArchReg>(GenericArray<T, R::COUNT>);

impl<T, R: ArchReg> Mapping<T, R> {
    pub fn map<U>(self, f: impl FnMut(T) -> U) -> Mapping<U, R> {
        Mapping(self.0.map(f))
    }

    pub fn as_refs(&self) -> GenericArray<&T, R::COUNT> {
        self.0.each_ref()
    }
}

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
