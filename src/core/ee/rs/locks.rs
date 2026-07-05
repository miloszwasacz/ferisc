use super::{IUop, Rs};
use crate::core::uop::{
    execute::Uop as EUop,
    issue::{Issue, TryReadReg},
};
use crate::seq::locks::{CircuitLock, Transformer};
use std::collections::VecDeque;
use std::marker::PhantomData;

//#region Dispatch

pub struct DispatchLock<'a, U> {
    rs: &'a mut Rs<U>,
    free_space: usize,
}

impl<'a, U> DispatchLock<'a, U> {
    pub(super) fn new(rs: &'a mut Rs<U>) -> Self {
        let free_space = rs.size.get() - rs.len.read_copied();
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

pub struct DispatchTrans<U>(Option<IUop<U>>);

impl<'a, U: Issue + 'a> Transformer<'a> for DispatchTrans<U> {
    type In = IUop<U>;
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

pub struct IssueLock<'a, U, R> {
    rs: &'a mut Rs<U>,
    try_read_reg: R,
    issued: VecDeque<usize>,
}

impl<'a, U, R> IssueLock<'a, U, R>
where
    U: Issue + Clone + 'a,
    R: TryReadReg<U::Reg>,
{
    pub(super) fn new(rs: &'a mut Rs<U>, try_read_reg: R) -> Self {
        Self {
            rs,
            try_read_reg,
            issued: VecDeque::new(),
        }
    }
}

impl<U, R> Drop for IssueLock<'_, U, R> {
    fn drop(&mut self) {
        {
            let mut i = 0;
            self.rs.entries.retain(|_| {
                // Retain only the elements that are not in `self.issued` - since we iterate
                // in order and the indices in `self.issued` are sorted, we only need to check
                // the first element for a match.
                let issued = self.issued.pop_front_if(|issued| i == *issued);
                i += 1;
                issued.is_none()
            });
            debug_assert!(self.issued.is_empty());
        }

        #[cfg(debug_assertions)]
        self.rs.state.insert(super::State::ISSUED);
    }
}

impl<'a, U, R> CircuitLock<'a> for IssueLock<'a, U, R>
where
    U: Issue + Clone + 'a,
    for<'b> &'b R: TryReadReg<U::Reg>,
{
    type Trans = IssueTrans<U, R>;

    fn make_trans(&mut self) -> Self::Trans {
        IssueTrans(None, PhantomData)
    }
}

pub struct IssueTrans<U, R>(Option<usize>, PhantomData<(U, R)>);

impl<'a, U, R> Transformer<'a> for IssueTrans<U, R>
where
    U: Issue + Clone + 'a,
    for<'b> &'b R: TryReadReg<U::Reg>,
{
    type In = ();
    type Out = EUop<U::Issued>;
    type Lock = IssueLock<'a, U, R>;

    fn transform(&mut self, lock: &Self::Lock, _: Self::In) -> Option<Self::Out> {
        lock.rs
            .entries
            .iter()
            .take(lock.rs.len.read_copied())
            .cloned()
            .enumerate()
            // We can skip the entries up to and including the last issued one
            // because they have already been checked
            .skip(lock.issued.back().map(|i| *i + 1).unwrap_or(0))
            .find_map(|(i, entry)| {
                let IUop {
                    kind: uop,
                    rob_entry,
                } = entry;

                uop.issue(&lock.try_read_reg).map(|uop| {
                    let old = self.0.replace(i);
                    debug_assert!(old.is_none());

                    EUop {
                        kind: uop,
                        rob_entry,
                    }
                })
            })
    }

    fn commit(self, lock: &mut Self::Lock) {
        lock.issued.push_back(self.0.unwrap());
    }
}

//#endregion
