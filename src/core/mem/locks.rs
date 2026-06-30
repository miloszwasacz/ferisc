use self::deps::Deps;
use self::into::IntoDKind;
use super::{
    AddrHandle, DMem, Entry, LdEntry, MemSubsystem, Slot, StEntry,
    entry::{Handle, Src},
};
use crate::core::{
    ee::rob,
    uop::{
        commit::Dest,
        dispatch::{Uop as DUop, UopKind as DKind, int as dint},
        fence::Fence,
        rename::{Uop as RUop, UopKind as RKind, int as rint},
    },
};
use crate::seq::locks::{CircuitLock, Transformer};
use std::cell::{Cell, RefCell};
use std::debug_assert_matches;
use std::marker::PhantomData;

mod deps;
mod into;

//#region Dispatch

type LdDep = Handle<StEntry>;
type StDep = AddrHandle;

pub struct DispatchLock<'a, M: DMem> {
    mem: &'a mut MemSubsystem<M>,
    ld_deps: Deps<LdDep>,
    st_deps: Deps<StDep>,
    fence: Fence,
}

impl<'a, M: DMem> DispatchLock<'a, M> {
    pub(super) fn new(mem: &'a mut MemSubsystem<M>) -> Self {
        let fence = mem.fence.read_copied();
        Self {
            mem,
            ld_deps: Default::default(),
            st_deps: Default::default(),
            fence,
        }
    }
}

impl<'a, M: DMem> CircuitLock<'a> for DispatchLock<'a, M> {
    type Trans = DispatchTrans<'a, M>;

    fn make_trans(&mut self) -> Self::Trans {
        DispatchTrans {
            dispatched: Dispatched::None,
            fence: self.fence,
            _conn: PhantomData,
        }
    }
}

pub struct DispatchTrans<'a, M: DMem> {
    dispatched: Dispatched,
    fence: Fence,
    _conn: PhantomData<&'a mut MemSubsystem<M>>,
}

#[derive(Debug, Clone, Default)]
enum Dispatched {
    #[default]
    None,
    Ld(Slot<LdEntry>),
    St(Slot<StEntry>),
}

impl<'a, M: DMem> Transformer<'a> for DispatchTrans<'a, M> {
    type In = (RUop, rob::EntryHandle);
    type Out = DUop;
    type Lock = DispatchLock<'a, M>;

    fn transform(&mut self, lock: &Self::Lock, input: Self::In) -> Option<Self::Out> {
        let (uop, rob_entry) = input;

        let dispatched = Cell::default();
        let add_ld = |size, signed, dest| {
            if lock.mem.ldq.len() >= lock.mem.ldq_size {
                return None;
            }

            let entry = Entry::new(
                rob_entry.clone(),
                size,
                LdEntry {
                    signed,
                    dest,
                    deps: lock.ld_deps.get(lock.mem),
                },
            );
            let slot = Slot::new(RefCell::new(entry));
            let handle = Handle::new(&slot);
            dispatched.set(Dispatched::Ld(slot));

            Some(handle.into())
        };
        let add_st = |size, src| {
            if lock.mem.stq.len() >= lock.mem.stq_size {
                return None;
            }

            let entry = Entry::new(
                rob_entry.clone(),
                size,
                StEntry {
                    src,
                    deps: lock.st_deps.get(lock.mem),
                },
            );
            let slot = Slot::new(RefCell::new(entry));
            let handle = Handle::new(&slot);
            dispatched.set(Dispatched::St(slot));

            Some(handle.into())
        };
        let kind = match uop.kind {
            RKind::Int(uop) => uop.into_dkind(
                |size, signed, dest| add_ld(size, signed, Dest::Int(dest)),
                |size, src| add_st(size, Src::Int(src)),
            )?,
            RKind::MemOrd(fence) if lock.fence.is_none() => {
                debug_assert!(self.fence.is_none());
                self.fence = fence;
                DKind::Handled
            }
            RKind::MemOrd(_) => return None,
            RKind::Except(_) => DKind::Handled,
        };
        debug_assert_matches!(self.dispatched, Dispatched::None);
        self.dispatched = dispatched.into_inner();

        Some(DUop { kind, rob_entry })
    }

    fn commit(self, lock: &mut Self::Lock) {
        debug_assert!(lock.fence.is_none());
        lock.fence = self.fence;

        match self.dispatched {
            Dispatched::None => {}
            Dispatched::Ld(s) => {
                lock.st_deps.push(Handle::new(&s).into());
                lock.mem.ldq.push_back(s);
            }
            Dispatched::St(s) => {
                let handle = Handle::new(&s);
                lock.ld_deps.push(handle.clone());
                lock.st_deps.push(handle.into());
                lock.mem.stq.push_back(s);
            }
        }

        #[cfg(debug_assertions)]
        lock.mem.state.insert(super::State::DISPATCHED);
    }
}

//#endregion
