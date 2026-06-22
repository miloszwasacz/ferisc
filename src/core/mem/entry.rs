use crate::core::{
    reg::{int::IntReg, phys::RegOper},
    rob,
    uop::commit::Dest,
};
use crate::mem::Addr;
use crate::seq::Buf;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

type AddrBuf = Buf<Option<Addr>>;

#[derive(Debug)]
pub(super) struct Entry<M> {
    pub rob_entry: rob::EntryHandle,
    pub addr: Rc<RefCell<AddrBuf>>,
    pub meta: M,
}

impl<M: Clone> Clone for Entry<M> {
    fn clone(&self) -> Self {
        Self {
            rob_entry: self.rob_entry.clone(),
            addr: Rc::new((*self.addr).clone()),
            meta: self.meta.clone(),
        }
    }
}

//#region Load

#[derive(Debug, Clone)]
pub(super) struct LdEntry {
    pub size: usize,
    pub signed: bool,
    pub dest: Dest,
}

//#endregion

//#region Store

#[derive(Debug, Clone)]
pub(super) struct StEntry {
    pub size: usize,
    pub src: Src,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum Src {
    Int(RegOper<IntReg>),
}

//#endregion

//#region Handle

#[derive(Debug, Clone)]
pub struct AddrHandle(Weak<RefCell<AddrBuf>>);

impl AddrHandle {
    pub fn write(&mut self, addr: Addr) {
        if let Some(c) = self.0.upgrade() {
            let mut buf = c.borrow_mut();
            debug_assert!(buf.read().is_none(), "address already computed");
            buf.write(Some(addr));
        }
    }
}

//#endregion
