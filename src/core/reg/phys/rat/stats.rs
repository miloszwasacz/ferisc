use super::Rat;
use crate::core::reg::{ArchReg, phys::Prf};
use derive_where::derive_where;
use generic_array::functional::FunctionalSequence;
use generic_array::{ArrayLength, GenericArray};

#[derive_where(Debug, Clone, PartialEq, Eq; T, T::Repr)]
#[derive(derive_more::Deref)]
pub struct Stats<T: ArchReg>(GenericArray<RegStats<T>, T::COUNT>);

#[derive_where(Debug, Clone, Copy, PartialEq, Eq; T, T::Repr)]
pub struct RegStats<T: ArchReg> {
    pub arch: T,
    pub speculative: Option<T::Repr>,
    pub committed: T::Repr,
}

impl<T> Copy for Stats<T>
where
    T: ArchReg,
    T::Repr: Copy,
    <T::COUNT as ArrayLength>::ArrayType<RegStats<T>>: Copy,
{
}

impl<T: ArchReg> Rat<T> {
    pub fn stats(&self, prf: &Prf<T>) -> Stats<T>
    where
        T::Repr: Clone,
    {
        let variants = const { GenericArray::<_, T::COUNT>::from_slice(T::VARIANTS) };
        let values = self
            .mapping
            .as_refs()
            .zip(self.committed.as_refs(), |s, c| {
                let s = prf.regs[s.read_copied()].try_read().cloned();
                let c = prf.regs[c.read_copied()]
                    .try_read()
                    .cloned()
                    .expect("committed register should not be busy");

                (s, c)
            });

        Stats(
            variants.zip(values, |arch, (speculative, committed)| RegStats {
                arch: *arch,
                speculative,
                committed,
            }),
        )
    }
}
