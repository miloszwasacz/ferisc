use super::Rs;
use crate::core::{
    rob,
    uop::{execute::Execute, issue::Issue},
};
use crate::seq::locks::{CircuitLock, Transformer};
use std::marker::PhantomData;

//#region Dispatch

pub struct DispatchLock<'a, U> {
    rs: &'a mut Rs<U>,
    free_space: usize,
}

impl<'a, U> DispatchLock<'a, U> {
    pub(super) fn new(rs: &'a mut Rs<U>) -> Self {
        let free_space = rs.size - rs.len.read_copied();
        Self { rs, free_space }
    }

    pub fn free_space(&self) -> usize {
        self.free_space
    }
}

impl<'a, U: Issue + 'a> CircuitLock<'a> for DispatchLock<'a, U> {
    type Trans = DispatchTrans<U>;

    fn make_trans(&mut self) -> Self::Trans {
        DispatchTrans(None)
    }
}

pub struct DispatchTrans<U>(Option<(U, rob::EntryHandle)>);

impl<'a, U: Issue + 'a> Transformer<'a> for DispatchTrans<U> {
    type In = (U, rob::EntryHandle);
    type Out = ();
    type Lock = DispatchLock<'a, U>;

    fn transform(&mut self, lock: &Self::Lock, input: Self::In) -> Option<Self::Out> {
        //TODO Make RSs operation-specific (ALU, AGU, BR, etc.)
        if lock.free_space > 0 {
            let old = self.0.replace(input);
            debug_assert!(old.is_none());
            Some(())
        } else {
            None
        }
    }

    fn commit(self, lock: &mut Self::Lock) {
        let new = self.0.unwrap();
        lock.rs.entries.push_back(new);
        lock.free_space -= 1;

        #[cfg(debug_assertions)]
        lock.rs.state.insert(super::State::DISPATCHED);
    }
}

//#endregion

//#region Issue

pub struct IssueLock<'a, U>(PhantomData<&'a U>);

impl<'a, U: Execute + 'a> CircuitLock<'a> for IssueLock<'a, U> {
    type Trans = IssueTrans<U>;

    fn make_trans(&mut self) -> Self::Trans {
        todo!()
    }
}

pub struct IssueTrans<U>(PhantomData<U>);

impl<'a, U: Execute + 'a> Transformer<'a> for IssueTrans<U> {
    type In = ();
    type Out = &'a (U, rob::EntryHandle);
    type Lock = IssueLock<'a, U>;

    fn transform(&mut self, lock: &Self::Lock, input: Self::In) -> Option<Self::Out> {
        todo!()
    }

    fn commit(self, lock: &mut Self::Lock) {
        todo!();

        // #[cfg(debug_assertions)]
        // lock.rs.state.insert(super::State::ISSUED);
    }
}

//#endregion
