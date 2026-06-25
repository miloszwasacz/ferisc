use super::MemSubsystem;
use crate::core::{
    ee::rob,
    uop::{dispatch, rename},
};
use crate::seq::locks::{CircuitLock, Transformer};
use std::marker::PhantomData;

//#region Dispatch

pub struct DispatchLock<'a, H>(PhantomData<&'a mut MemSubsystem<H>>);

impl<'a, H> CircuitLock<'a> for DispatchLock<'a, H> {
    type Trans = DispatchTrans<'a, H>;

    fn make_trans(&mut self) -> Self::Trans {
        todo!()
    }
}

pub struct DispatchTrans<'a, H>(PhantomData<&'a mut H>);

impl<'a, H> Transformer<'a> for DispatchTrans<'a, H> {
    type In = (rename::Uop, rob::EntryHandle);
    type Out = dispatch::Uop;
    type Lock = DispatchLock<'a, H>;

    fn transform(&mut self, lock: &Self::Lock, input: Self::In) -> Option<Self::Out> {
        todo!()
    }

    fn commit(self, lock: &mut Self::Lock) {
        todo!()
    }
}

//#endregion
