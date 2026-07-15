use self::into::IntoDKind;
use super::{
    AddrHandle, DMem, Entry, LdEntry, MemSubsystem, Slot, StEntry,
    entry::{Handle, Src},
};
use crate::core::{
    ee::rob::{
        self, FlushedUop,
        uop::{FenceHandle, Uop as RUop, UopKind as RKind},
    },
    uop::{
        commit::Dest,
        dispatch::{Uop as DUop, UopKind as DKind, int as dint},
        rename::int as rint,
    },
};
use crate::seq::locks::{CircuitLock, Transformer};
use std::cell::{Cell, RefCell};
use std::debug_assert_matches;
use std::marker::PhantomData;
use std::ops::DerefMut;

mod into;

//#region Dispatch

pub struct DispatchLock<'a, M: DMem> {
    mem: &'a mut MemSubsystem<M>,
    loads: Vec<Handle<LdEntry>>,
    stores: Vec<Handle<StEntry>>,
    fence: FenceHandle,
}

impl<'a, M: DMem> DispatchLock<'a, M> {
    pub(super) fn new(mem: &'a mut MemSubsystem<M>) -> Self {
        let loads = mem.ldq.iter().map(Handle::new).collect();
        let stores = mem.stq.iter().map(Handle::new).collect();
        let fence = mem.fence.read().clone();

        Self {
            mem,
            loads,
            stores,
            fence,
        }
    }
}

impl<'a, M: DMem> Drop for DispatchLock<'a, M> {
    fn drop(&mut self) {
        if !self.fence.is_none() {
            self.mem.fence.write(self.fence.clone());
        }
    }
}

impl<'a, M: DMem> CircuitLock<'a> for DispatchLock<'a, M> {
    type Trans = DispatchTrans<'a, M>;

    fn make_trans(&mut self) -> Self::Trans {
        DispatchTrans {
            dispatched: Dispatched::None,
            fence: self.fence.clone(),
            _conn: PhantomData,
        }
    }
}

pub struct DispatchTrans<'a, M: DMem> {
    dispatched: Dispatched,
    fence: FenceHandle,
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
        let fence = lock.fence.get();
        let add_ld = |size, signed, dest| {
            if lock.mem.ldq.free_space() == 0 {
                return None;
            }
            let deps = lock
                .stores
                .iter()
                .cloned()
                .map(Into::into)
                .chain(
                    fence
                        .is_some_and(|fence| fence.pld() && fence.sld())
                        .then_some(lock.loads.iter())
                        .into_iter()
                        .flatten()
                        .cloned()
                        .map(Into::into),
                )
                .collect();

            let entry = Entry::new(
                rob_entry.clone(),
                size,
                deps,
                lock.fence.clone(),
                LdEntry { signed, dest },
            );
            let slot = Slot::new(RefCell::new(entry));
            let handle = Handle::new(&slot);
            dispatched.set(Dispatched::Ld(slot));

            Some(handle.into())
        };
        let add_st = |size, src| {
            if lock.mem.stq.free_space() == 0 {
                return None;
            }
            let deps = lock
                .stores
                .iter()
                .map(|h| h.clone().into())
                .chain(lock.loads.iter().map(|h| h.clone().into()))
                .collect();

            let entry = Entry::new(
                rob_entry.clone(),
                size,
                deps,
                lock.fence.clone(),
                StEntry { src },
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
            RKind::Handled => DKind::Handled,
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
                lock.loads.push(Handle::new(&s));
                lock.mem.ldq.push(s);
            }
            Dispatched::St(s) => {
                lock.stores.push(Handle::new(&s));
                lock.mem.stq.push(s);
            }
        }

        #[cfg(debug_assertions)]
        lock.mem.state.insert(super::State::DISPATCHED);
    }
}

//#endregion

//#region Flush

pub struct FlushLock<'a, M: DMem, D: DerefMut<Target = M>> {
    mem: &'a mut MemSubsystem<M>,
    dmem: D,
}

impl<'a, M: DMem, D: DerefMut<Target = M>> FlushLock<'a, M, D> {
    pub(super) fn new(mem: &'a mut MemSubsystem<M>, dmem: D) -> Self {
        Self { mem, dmem }
    }
}

impl<M: DMem, D: DerefMut<Target = M>> Drop for FlushLock<'_, M, D> {
    fn drop(&mut self) {
        self.mem.ldq.flush();
        self.mem.stq.flush();
        self.mem.flush_req(self.dmem.deref_mut());
        debug_assert!(self.mem.fence.read().is_none());

        #[cfg(debug_assertions)]
        self.mem.state.insert(super::State::FLUSHED);
    }
}

impl<'a, M: DMem, D: DerefMut<Target = M>> CircuitLock<'a> for FlushLock<'a, M, D> {
    type Trans = FlushTrans<'a, M, D>;

    fn make_trans(&mut self) -> Self::Trans {
        FlushTrans(PhantomData, PhantomData)
    }
}

pub struct FlushTrans<'a, M: DMem, D>(PhantomData<&'a mut MemSubsystem<M>>, PhantomData<D>);

impl<'a, M: DMem, D: DerefMut<Target = M>> Transformer<'a> for FlushTrans<'a, M, D> {
    type In = FlushedUop;
    type Out = FlushedUop;
    type Lock = FlushLock<'a, M, D>;

    fn transform(&mut self, _: &Self::Lock, input: Self::In) -> Option<Self::Out> {
        Some(input)
    }

    fn commit(self, _: &mut Self::Lock) {}
}

//#endregion
