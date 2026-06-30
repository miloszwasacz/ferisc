use super::{Handle, LdDep, MemSubsystem, StDep};
use crate::core::mem::DMem;
use std::cell::RefCell;

pub(super) struct Deps<T>(RefCell<Option<Vec<T>>>);

impl<T> Default for Deps<T> {
    fn default() -> Self {
        Self(RefCell::new(None))
    }
}

impl Deps<LdDep> {
    pub fn get<M: DMem>(&self, mem: &MemSubsystem<M>) -> Vec<LdDep> {
        self.0
            .borrow_mut()
            .get_or_insert_with(|| mem.stq.iter().map(Handle::new).collect())
            .clone()
    }
}

impl Deps<StDep> {
    pub fn get<M: DMem>(&self, mem: &MemSubsystem<M>) -> Vec<StDep> {
        self.0
            .borrow_mut()
            .get_or_insert_with(|| {
                mem.ldq
                    .iter()
                    .map(|s| Handle::new(s).into())
                    .chain(mem.stq.iter().map(|s| Handle::new(s).into()))
                    .collect()
            })
            .clone()
    }
}

impl<T> Deps<T> {
    pub fn push(&mut self, dep: T) {
        self.0
            .borrow_mut()
            .as_mut()
            .expect("dependencies should already be initialized")
            .push(dep);
    }
}
