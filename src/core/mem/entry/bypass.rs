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

    #[inline(always)]
    fn _update_deps(&mut self, is_load: bool) {
        let Some(addr_range) = self.addr_range() else {
            return;
        };
        let fence = self.fence.get();
        let fenced = fence.is_some_and(|fence| {
            //TODO Distinguish between MEM and MMIO addresses
            if is_load {
                fence.si() || fence.sr()
            } else {
                fence.so() || fence.sw()
            }
        });
        macro_rules! is_fenced {
            (|$fence:ident| $fence_chk:expr) => {{ fenced && fence.is_some_and(|$fence| $fence_chk) }};
        }
        self.deps.retain(|h| {
            match &h.0 {
                HandleType::Ld(h) => {
                    let Some(s) = h.0.upgrade() else {
                        // Flushed or already executed
                        return false;
                    };
                    let e = s.borrow();
                    let Some(dep_range) = e.addr_range() else {
                        // Prevent WAR + respect fences
                        return !is_load || is_fenced!(|fence| fence.pld());
                    };
                    addr_ranges_intersect(&addr_range, &dep_range)
                        || is_fenced!(|fence| {
                            //TODO Distinguish between MEM and MMIO addresses
                            fence.pi() || fence.pr()
                        })
                }
                HandleType::St(h) => {
                    let Some(s) = h.0.upgrade() else {
                        // Flushed or already executed
                        return false;
                    };
                    let e = s.borrow();
                    let Some(dep_range) = e.addr_range() else {
                        // Prevent RAW & WAW
                        return true;
                    };
                    addr_ranges_intersect(&addr_range, &dep_range)
                        || is_fenced!(|fence| {
                            //TODO Distinguish between MEM and MMIO addresses
                            fence.po() || fence.pw()
                        })
                }
            }
        });
    }
}

impl Entry<LdEntry> {
    pub(in crate::core::mem) fn update_deps(&mut self) {
        self._update_deps(true);
    }
}

impl Entry<StEntry> {
    pub(in crate::core::mem) fn update_deps(&mut self) {
        self._update_deps(false);
    }
}

fn addr_ranges_intersect(a: &Range<Addr>, b: &Range<Addr>) -> bool {
    a.start < b.end && b.start < a.end
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
