use super::{AddrHandle, Entry, Handle, HandleType, LdEntry, StEntry};
use crate::mem::Addr;
use crate::seq::{Buf, Sequential};
use std::ops::Range;

//#region AddrBuf

#[derive(Debug, Clone, Default)]
pub struct AddrBuf {
    buf: Buf<Option<Addr>>,
    bypassed: Option<Addr>,
}

impl AddrBuf {
    pub fn read(&self) -> Option<Addr> {
        self.buf.read_copied()
    }

    fn write(&mut self, addr: Addr) {
        let old = self.bypassed.replace(addr);
        debug_assert!(old.is_none());
        self.buf.write(Some(addr));
    }
}

impl Sequential for AddrBuf {
    fn finish_cycle(&mut self) {
        self.buf.finish_cycle();
        self.bypassed = None;
    }
}

//#endregion

//#region Bypass

#[derive(Debug)]
pub struct AddrBypass(());

impl AddrBypass {
    pub fn read(&self, handle: &AddrHandle) -> Option<Addr> {
        match &handle.0 {
            HandleType::Ld(h) => self.read_ld(h),
            HandleType::St(h) => self.read_st(h),
        }
    }

    pub(in crate::core::mem) fn read_ld(&self, handle: &Handle<LdEntry>) -> Option<Addr> {
        self.read_entry(&handle.expect().borrow())
    }

    pub(in crate::core::mem) fn read_st(&self, handle: &Handle<StEntry>) -> Option<Addr> {
        self.read_entry(&handle.expect().borrow())
    }

    pub(in crate::core::mem) fn read_entry<T>(&self, entry: &Entry<T>) -> Option<Addr> {
        entry.addr.bypassed.or_else(|| entry.addr.buf.read_copied())
    }
}

impl<M> Entry<M> {
    fn addr_range(&self) -> Option<Range<Addr>> {
        self.addr
            .bypassed
            .or_else(|| self.addr.buf.read_copied())
            .map(|addr| addr..(addr + self.size as Addr))
    }
}

fn addr_ranges_intersect(a: &Range<Addr>, b: &Range<Addr>) -> bool {
    a.start < b.end && b.start < a.end
}

macro_rules! check_dep {
    ($addr_range:expr, $handle:expr) => {{
        let Some(s) = $handle.0.upgrade() else {
            return false;
        };
        let e = s.borrow();
        let Some(dep_range) = e.addr_range() else {
            return true;
        };
        addr_ranges_intersect(&$addr_range, &dep_range)
    }};
}

impl Entry<LdEntry> {
    pub(in crate::core::mem) fn update_deps(&mut self, addr_bypass: &AddrBypass) {
        let _ = addr_bypass;
        let Some(addr_range) = self.addr_range() else {
            return;
        };
        self.deps.retain(|h| check_dep!(addr_range, h));
    }
}

impl Entry<StEntry> {
    pub(in crate::core::mem) fn update_deps(&mut self, addr_bypass: &AddrBypass) {
        let _ = addr_bypass;
        let Some(addr_range) = self.addr_range() else {
            return;
        };
        self.deps
            .retain(|h| apply_to_handle!(&h.0, |h| check_dep!(addr_range, h)));
    }
}

#[derive(Debug, Default)]
pub struct AddrBypassBuilder(());

impl AddrBypassBuilder {
    pub fn write_addr(&mut self, handle: AddrHandle, addr: Addr) {
        apply_to_handle!(handle.0, |handle| {
            let s = handle.expect();
            let mut e = s.borrow_mut();

            let buf = &mut e.addr;
            debug_assert!(buf.read().is_none(), "address already computed");
            buf.write(addr);
        });
    }

    pub fn build(self) -> AddrBypass {
        AddrBypass(())
    }
}

//#endregion
