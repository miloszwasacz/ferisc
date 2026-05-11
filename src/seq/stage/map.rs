use super::{CircuitLock, Transformer};
use std::marker::PhantomData;

pub struct TransLock<F, I, O>(F, PhantomData<fn(I) -> O>);

impl<F, I, O> TransLock<F, I, O> {
    pub(super) fn new(f: F) -> Self
    where
        F: Fn(I) -> O,
    {
        Self(f, PhantomData)
    }
}

impl<F, I, O> CircuitLock<'_> for TransLock<F, I, O>
where
    F: Fn(I) -> O,
{
    type Trans = Transform<F, I, O>;

    fn make_trans(&mut self) -> Self::Trans {
        Transform(PhantomData, PhantomData)
    }
}

pub struct Transform<F, I, O>(PhantomData<F>, PhantomData<fn(I) -> O>);

impl<F, I, O> Transformer<'_> for Transform<F, I, O>
where
    F: Fn(I) -> O,
{
    type In = I;
    type Out = O;
    type Lock = TransLock<F, I, O>;

    fn transform(&mut self, lock: &Self::Lock, input: Self::In) -> Option<Self::Out> {
        Some(lock.0(input))
    }

    fn commit(self, _: &mut Self::Lock) {}
}
