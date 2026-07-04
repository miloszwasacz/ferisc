use super::{CircuitLock, Transformer};
use std::marker::PhantomData;

pub struct TransLock<I, O, F = fn(I) -> O>(F, PhantomData<fn(I) -> O>);

impl<I, O, F> TransLock<I, O, F> {
    pub fn new(f: F) -> Self
    where
        F: Fn(I) -> O,
    {
        Self(f, PhantomData)
    }
}

impl<T: Copy> CopiedLock<'_, T> {
    pub(super) fn copied() -> Self {
        Self(T::clone, PhantomData)
    }
}

impl<I, O, F> CircuitLock<'_> for TransLock<I, O, F>
where
    F: Fn(I) -> O,
{
    type Trans = Transform<I, O, F>;

    fn make_trans(&mut self) -> Self::Trans {
        Transform(PhantomData, PhantomData)
    }
}

pub struct Transform<I, O, F = fn(I) -> O>(PhantomData<F>, PhantomData<fn(I) -> O>);

impl<I, O, F> Transformer<'_> for Transform<I, O, F>
where
    F: Fn(I) -> O,
{
    type In = I;
    type Out = O;
    type Lock = TransLock<I, O, F>;

    fn transform(&mut self, lock: &Self::Lock, input: Self::In) -> Option<Self::Out> {
        Some(lock.0(input))
    }

    fn commit(self, _: &mut Self::Lock) {}
}

pub type CopiedLock<'a, T> = TransLock<&'a T, T>;
