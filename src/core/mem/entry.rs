pub use self::bypass::{AddrBuf, AddrBypass, AddrBypassBuilder};
use super::Slot;
use crate::core::{
    ee::rob,
    reg::{int::IntReg, phys::RegOper},
    uop::commit::Dest,
};
use std::cell::RefCell;
use std::ops::{Deref, DerefMut};
use std::rc::Weak;

macro_rules! apply_to_handle {
    ($handle:expr, |$h:ident| $body:expr) => {
        match $handle {
            HandleType::Ld($h) => $body,
            HandleType::St($h) => $body,
        }
    };
}

mod bypass;

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

    fn expect(&self) -> Slot<M> {
        self.0
            .upgrade()
            .expect("handle should not be flushed or already executed")
    }
}

#[derive(Debug, Clone)]
pub struct AddrHandle(pub(super) HandleType);

#[derive(Debug, Clone)]
pub(super) enum HandleType {
    Ld(Handle<LdEntry>),
    St(Handle<StEntry>),
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
