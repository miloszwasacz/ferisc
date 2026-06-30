use super::Slot;
use crate::core::{
    ee::rob,
    reg::{int::IntReg, phys::RegOper},
    uop::commit::Dest,
};
use crate::mem::Addr;
use crate::seq::Buf;
use std::cell::RefCell;
use std::ops::{Deref, DerefMut, Range};
use std::rc::Weak;

type AddrBuf = Buf<Option<Addr>>;

#[derive(Debug, Clone)]
pub(super) struct Entry<M> {
    pub rob_entry: rob::EntryHandle,
    pub addr: AddrBuf,
    pub size: usize,
    pub meta: M,
}

impl<M> Entry<M> {
    pub fn new(rob_entry: rob::EntryHandle, size: usize, meta: M) -> Self {
        Self {
            rob_entry,
            addr: Default::default(),
            size,
            meta,
        }
    }
}

impl<M> Deref for Entry<M> {
    type Target = M;

    fn deref(&self) -> &Self::Target {
        &self.meta
    }
}

impl<M> DerefMut for Entry<M> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.meta
    }
}

//#region Load

#[derive(Debug, Clone)]
pub(super) struct LdEntry {
    pub signed: bool,
    pub dest: Dest,
    pub deps: Vec<Handle<StEntry>>,
}

//#endregion

//#region Store

#[derive(Debug, Clone)]
pub(super) struct StEntry {
    pub src: Src,
    pub deps: Vec<AddrHandle>,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum Src {
    Int(RegOper<IntReg>),
}

//#endregion

//#region Handles

#[derive(Debug, Clone)]
pub(super) struct Handle<M>(Weak<RefCell<Entry<M>>>);

impl<M> Handle<M> {
    pub fn new(slot: &Slot<M>) -> Self {
        Self(Slot::downgrade(slot))
    }

    pub(super) fn addr_range(&self) -> Option<Range<Addr>> {
        let e = self
            .0
            .upgrade()
            .expect("handle should not be flushed or already executed");
        let e = e.borrow();

        let start = e.addr.read_copied()?;
        let end = start + (e.size as Addr);
        Some(start..end)
    }
}

#[derive(Debug, Clone)]
pub struct AddrHandle(pub(super) HandleType);

#[derive(Debug, Clone)]
pub(super) enum HandleType {
    Ld(Handle<LdEntry>),
    St(Handle<StEntry>),
}

macro_rules! apply_to_handle {
    ($handle:expr, |$h:ident| $body:expr) => {
        match $handle {
            HandleType::Ld($h) => $body,
            HandleType::St($h) => $body,
        }
    };
}

impl AddrHandle {
    pub fn write(&mut self, addr: Addr) {
        apply_to_handle!(&mut self.0, |handle| {
            let e = handle
                .0
                .upgrade()
                .expect("handle should not be flushed or already executed");
            let mut e = e.borrow_mut();

            let buf = &mut e.addr;
            debug_assert!(buf.read().is_none(), "address already computed");
            buf.write(Some(addr));
        });
    }

    pub(super) fn addr_range(&self) -> Option<Range<Addr>> {
        apply_to_handle!(&self.0, |handle| handle.addr_range())
    }
}

impl From<Handle<LdEntry>> for AddrHandle {
    fn from(value: Handle<LdEntry>) -> Self {
        Self(HandleType::Ld(value))
    }
}

impl From<Handle<StEntry>> for AddrHandle {
    fn from(value: Handle<StEntry>) -> Self {
        Self(HandleType::St(value))
    }
}

//#endregion
