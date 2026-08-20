use super::fence::FenceHandle;
use crate::core::{
    ee::{pipeline::Kind, rob},
    reg::{int::IntReg, phys::RegOper},
    uop::commit::Dest,
};
use crate::mem::{Addr, Size};
use crate::seq::{Buf, Sequential};
use derive_more::{Deref, DerefMut};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

type AddrBuf = Buf<Option<Addr>>;

#[derive(Debug, Clone, Deref, DerefMut)]
pub(super) struct Entry<M> {
    #[deref]
    #[deref_mut]
    inner: InnerEntry<M>,
    fence: FenceHandle,
    deps: Vec<AddrHandle>,
}

#[derive(Debug, Clone)]
pub(super) struct InnerEntry<M> {
    rob_entry: rob::Handle,
    addr: Rc<RefCell<AddrBuf>>,
    size: Size,
    meta: M,
}

#[derive(Debug, Clone, Deref, DerefMut)]
pub(super) struct ReadyEntry<M> {
    #[deref]
    #[deref_mut]
    inner: InnerEntry<M>,
    addr: Addr,
}

#[derive(Deref, DerefMut)]
pub(super) struct EntryBuilder<M> {
    #[deref]
    #[deref_mut]
    inner: InnerEntry<M>,
    fence: FenceHandle,
}

#[derive(Debug, Clone, Copy)]
pub struct Ld {
    pub signed: bool,
    pub dest: Dest,
}

#[derive(Debug, Clone, Copy)]
pub struct St {
    pub src: Src,
}

macro_rules! src {
    ($( $kind:ident ),*) => {
        ::paste::paste!(Kind<$( RegOper<[< $kind Reg >]> ),*>)
    };
}
pub type Src = src!(Int);

impl<M> InnerEntry<M> {
    fn _handle(&self, kind: HandleKind) -> AddrHandle {
        AddrHandle {
            buf: Rc::downgrade(&self.addr),
            size: self.size,
            kind,
        }
    }

    pub fn is_flushed(&self, flush: &rob::lock::Flush<'_>) -> bool {
        self.rob_entry.is_flushed(flush)
    }

    pub fn size(&self) -> Size {
        self.size
    }
}

impl InnerEntry<Ld> {
    pub fn handle(&self) -> AddrHandle {
        self._handle(HandleKind::Ld)
    }

    pub fn signed(&self) -> bool {
        self.meta.signed
    }

    pub fn dest(&self) -> Dest {
        self.meta.dest
    }
}

impl InnerEntry<St> {
    pub fn handle(&self) -> AddrHandle {
        self._handle(HandleKind::St)
    }

    pub fn src(&self) -> Src {
        self.meta.src
    }
}

impl<M> Entry<M> {
    #[inline]
    fn _update_deps(&mut self, kind: HandleKind) {
        use std::ops::Range;

        let fence = self.fence.get();
        let Some(addr) = *self.addr.borrow().read_bypassed() else {
            return;
        };
        let addr_range = self.size.addr_range(addr);
        let intersect =
            |other: Range<_>| addr_range.start < other.end && other.start < addr_range.end;

        self.deps.retain(|dep| {
            let Some(dep_addr) = dep.buf.upgrade() else {
                // Flushed or already committed
                return false;
            };
            dep_addr.borrow().read_bypassed().is_none_or(|dep_addr| {
                let intersect = intersect(dep.size.addr_range(dep_addr));
                //TODO MMIO
                match (dep.kind, kind) {
                    (HandleKind::Ld, HandleKind::Ld) => fence.pld() && fence.sld(),
                    (HandleKind::Ld, HandleKind::St) => intersect || (fence.pld() && fence.sst()),
                    (HandleKind::St, HandleKind::Ld) => intersect || (fence.pst() && fence.sld()),
                    (HandleKind::St, HandleKind::St) => intersect || (fence.pst() && fence.sst()),
                }
            })
        })
    }
}

impl Entry<Ld> {
    pub fn update_deps(&mut self) {
        self._update_deps(HandleKind::Ld)
    }
}

impl Entry<St> {
    pub fn update_deps(&mut self) {
        self._update_deps(HandleKind::St)
    }
}

impl<M> Sequential for Entry<M> {
    fn finish_cycle(&mut self) {
        self.addr.borrow_mut().finish_cycle();
    }
}

impl<M> ReadyEntry<M> {
    pub fn into_rob_entry(self) -> rob::Handle {
        self.inner.rob_entry
    }

    pub fn addr(&self) -> Addr {
        self.addr
    }
}

impl<M: Clone> TryFrom<&Entry<M>> for ReadyEntry<M> {
    type Error = ();

    fn try_from(value: &Entry<M>) -> Result<Self, Self::Error> {
        //TODO Check for Fence as well?
        if !value.deps.is_empty() {
            return Err(());
        }

        let addr = *value.addr.borrow().read_bypassed();
        addr.map(|addr| ReadyEntry {
            inner: value.inner.clone(),
            addr,
        })
        .ok_or(())
    }
}

impl EntryBuilder<Ld> {
    pub fn new(
        rob_entry: rob::Handle,
        size: Size,
        signed: bool,
        dest: Dest,
        fence: FenceHandle,
    ) -> Self {
        Self {
            inner: InnerEntry {
                rob_entry,
                addr: Default::default(),
                size,
                meta: Ld { signed, dest },
            },
            fence,
        }
    }

    pub fn handle(&self) -> AddrHandle {
        self._handle(HandleKind::Ld)
    }

    pub fn build(self, deps: impl IntoIterator<Item = AddrHandle>) -> Entry<Ld> {
        self._build(deps, HandleKind::Ld)
    }
}

impl EntryBuilder<St> {
    pub fn new(rob_entry: rob::Handle, size: Size, src: Src, fence: FenceHandle) -> Self {
        Self {
            inner: InnerEntry {
                rob_entry,
                addr: Default::default(),
                size,
                meta: St { src },
            },
            fence,
        }
    }

    pub fn handle(&self) -> AddrHandle {
        self._handle(HandleKind::St)
    }

    pub fn build(self, deps: impl IntoIterator<Item = AddrHandle>) -> Entry<St> {
        self._build(deps, HandleKind::St)
    }
}

impl<M> EntryBuilder<M> {
    #[inline]
    fn _handle(&self, kind: HandleKind) -> AddrHandle {
        AddrHandle {
            buf: Rc::downgrade(&self.addr),
            size: self.size,
            kind,
        }
    }

    #[inline]
    fn _build(self, deps: impl IntoIterator<Item = AddrHandle>, kind: HandleKind) -> Entry<M> {
        let mut entry = Entry {
            inner: self.inner,
            fence: self.fence,
            deps: deps.into_iter().collect(),
        };
        entry._update_deps(kind);
        entry
    }
}

//#region Handle

#[derive(Debug, Clone)]
pub struct AddrHandle {
    buf: Weak<RefCell<AddrBuf>>,
    size: Size,
    kind: HandleKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum HandleKind {
    Ld,
    St,
}

impl AddrHandle {
    fn unwrap(&self) -> Rc<RefCell<AddrBuf>> {
        self.buf.upgrade().expect("handle should be valid")
    }

    pub fn write(self, addr: Addr) {
        let rc = self.unwrap();
        let mut buf = rc.borrow_mut();
        debug_assert!(buf.read_bypassed().is_none(), "address already computed");
        buf.write(Some(addr));
    }
}

//#endregion

//#region Stats

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deref)]
pub struct Stats<M> {
    pub addr: Option<Addr>,
    pub size: Size,
    pub waiting: bool,
    #[deref]
    pub meta: M,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deref)]
pub struct ReadyStats<M> {
    pub addr: Addr,
    pub size: Size,
    #[deref]
    pub meta: M,
}

impl<M: Clone> Entry<M> {
    pub fn stats(&self) -> Stats<M> {
        Stats {
            addr: self.addr.borrow().read_copied(),
            size: self.size(),
            waiting: !self.deps.is_empty(),
            meta: self.meta.clone(),
        }
    }
}

impl<M: Clone> ReadyEntry<M> {
    pub fn stats(&self) -> ReadyStats<M> {
        ReadyStats {
            addr: self.addr,
            size: self.size(),
            meta: self.meta.clone(),
        }
    }
}

//#endregion
