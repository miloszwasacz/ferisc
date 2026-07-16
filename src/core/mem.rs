// pub use self::cached::CachedHierarchy;
pub use self::direct::DirectHierarchy;
use self::entry::Entry;
pub use self::entry::{AddrBypass, AddrBypassBuilder, AddrHandle, LdEntry, Src, StEntry};
use self::locks::{DispatchLock, FlushLock};
use self::queue::Queue;
use crate::core::{
    ee::{pipeline::BypassNetwork, rob::uop::FenceHandle},
    reg::int::IntVal,
    uop::{Exception, commit::Dest},
};
use crate::mem::{Addr, LdReq, Mem, ReqData, StReq};
use crate::seq::{Buf, Sequential};
use static_assertions::const_assert_eq;
use std::cell::RefCell;
use std::num::NonZeroUsize;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

macro_rules! ld_mem {
    (<$mem:ty>::$( $tail:tt )*) => {
        <$mem as $crate::mem::Mem<$crate::mem::LdReq<$crate::core::uop::Exception>>>::$( $tail )*
    };
}
macro_rules! st_mem {
    (<$mem:ty>::$( $tail:tt )*) => {
        <$mem as $crate::mem::Mem<$crate::mem::StReq<$crate::core::uop::Exception>>>::$( $tail )*
    };
}

// mod cached;
mod direct;
mod entry;
pub mod locks;
mod queue;

//#region Hierarchy

pub trait MemHierarchy: Sequential {
    type IMem: IMem;
    type DMem: DMem;

    fn imem(&self) -> impl Deref<Target = Self::IMem>;
    fn imem_mut(&mut self) -> impl DerefMut<Target = Self::IMem>;
    fn dmem(&self) -> impl Deref<Target = Self::DMem>;
    fn dmem_mut(&mut self) -> impl DerefMut<Target = Self::DMem>;
}

pub trait IMem: Mem<LdReq<Exception>, Output = Result<ReqData, Exception>> {}
impl<T: Mem<LdReq<Exception>, Output = Result<ReqData, Exception>>> IMem for T {}

pub trait DMem:
    Mem<LdReq<Exception>, Output = Result<ReqData, Exception>>
    + Mem<StReq<Exception>, Output = Result<(), Exception>>
{
}
impl<T> DMem for T where
    T: Mem<LdReq<Exception>, Output = Result<ReqData, Exception>>
        + Mem<StReq<Exception>, Output = Result<(), Exception>>
{
}

//#endregion

//#region Memory Subsystem

type Slot<T> = Rc<RefCell<Entry<T>>>;

//TODO MMIO
pub struct MemSubsystem<M: DMem> {
    ld_conn: ld_mem!(<M>::Conn),
    st_conn: st_mem!(<M>::Conn),
    ldq: Queue<LdEntry>,
    stq: Queue<StEntry>,
    req: Req<M>,
    fence: Buf<FenceHandle>,
    #[cfg(debug_assertions)]
    state: State,
}

#[derive(Default)]
enum Req<M: DMem> {
    #[default]
    None,
    Ld(<M as Mem<LdReq<Exception>>>::Handle, Slot<LdEntry>),
    St(<M as Mem<StReq<Exception>>>::Handle, Slot<StEntry>),
}

impl<M: DMem> Req<M> {
    pub fn or_else(self, f: impl FnOnce() -> Self) -> Self {
        match self {
            Req::None => f(),
            req => req,
        }
    }
}

impl<M: DMem> MemSubsystem<M> {
    pub fn new(config: &Config, mut dmem: impl DerefMut<Target = M>) -> Self {
        let dmem = dmem.deref_mut();
        Self {
            ld_conn: ld_mem!(<M>::register_conn(dmem)),
            st_conn: st_mem!(<M>::register_conn(dmem)),
            ldq: Queue::new(config.ldq_size.get()),
            stq: Queue::new(config.stq_size.get()),
            req: Req::None,
            fence: Buf::new(FenceHandle::detached()),
            #[cfg(debug_assertions)]
            state: State::empty(),
        }
    }

    pub fn dispatch(&mut self) -> DispatchLock<'_, M> {
        #[cfg(debug_assertions)]
        assert!(
            !self.state.contains(State::DISPATCHED),
            "cannot dispatch more than once in a clock cycle"
        );

        DispatchLock::new(self)
    }

    pub fn tick(
        &mut self,
        mut dmem: impl DerefMut<Target = M>,
        addr_bypass: AddrBypass,
        prfs: &mut BypassNetwork<'_>,
    ) {
        let dmem = dmem.deref_mut();
        let ld_prev = matches!(self.req, Req::Ld(_, _));

        // Update dependencies
        self.ldq.update_deps(&addr_bypass);
        self.stq.update_deps(&addr_bypass);

        // Handle current request
        match std::mem::take(&mut self.req) {
            Req::Ld(h, s) => match ld_mem!(<M>::take_if_ready(dmem, h)) {
                Ok(Ok(data)) => {
                    let mut entry = s.borrow_mut();
                    write_ld_result(data, &entry, prfs);
                    entry.rob_entry.mark_done();
                }
                Ok(Err(ex)) => {
                    s.borrow_mut().rob_entry.throw(ex);
                }
                Err(h) => {
                    self.req = Req::Ld(h, s);
                    return;
                }
            },
            Req::St(h, s) => match st_mem!(<M>::take_if_ready(dmem, h)) {
                Ok(Ok(())) => {
                    s.borrow_mut().rob_entry.mark_done();
                }
                Ok(Err(ex)) => {
                    s.borrow_mut().rob_entry.throw(ex);
                }
                Err(h) => {
                    self.req = Req::St(h, s);
                    return;
                }
            },
            Req::None => {}
        }

        // Make new request
        debug_assert!(matches!(self.req, Req::None));
        self.req = if !ld_prev {
            self.ldq
                .schedule_ready(dmem, self.ld_conn, &addr_bypass)
                .or_else(|| {
                    self.stq
                        .schedule_ready(dmem, self.st_conn, &addr_bypass, prfs)
                })
        } else {
            self.stq
                .schedule_ready(dmem, self.st_conn, &addr_bypass, prfs)
                .or_else(|| self.ldq.schedule_ready(dmem, self.ld_conn, &addr_bypass))
        };
    }

    pub fn flush<D: DerefMut<Target = M>>(&mut self, dmem: D) -> FlushLock<'_, M, D> {
        #[cfg(debug_assertions)]
        assert!(
            !self.state.intersects(State::FLUSHED),
            "cannot flush and dispatch in the same clock cycle"
        );

        FlushLock::new(self, dmem)
    }

    pub fn flush_all(&mut self, mut dmem: impl DerefMut<Target = M>) {
        #[cfg(debug_assertions)]
        {
            assert!(
                !self.state.intersects(State::FLUSHED),
                "cannot flush and dispatch in the same clock cycle"
            );
            self.state.insert(State::FLUSHED);
        }

        let dmem = dmem.deref_mut();
        self.ldq.flush_all();
        self.stq.flush_all();
        self.flush_req(dmem);
        debug_assert!(self.fence.read().get().is_none());
    }

    fn flush_req(&mut self, dmem: &mut M) {
        match std::mem::take(&mut self.req) {
            Req::Ld(h, s) if s.borrow().rob_entry.is_flushed() => ld_mem!(<M>::kill(dmem, h)),
            Req::St(h, s) if s.borrow().rob_entry.is_flushed() => st_mem!(<M>::kill(dmem, h)),
            req => {
                self.req = req;
            }
        }
    }
}

impl<M: DMem> Sequential for MemSubsystem<M> {
    fn finish_cycle(&mut self) {
        self.ldq.finish_cycle();
        self.stq.finish_cycle();
        self.fence.finish_cycle();
        match &mut self.req {
            Req::None => {}
            Req::Ld(_, s) => s.borrow_mut().addr.finish_cycle(),
            Req::St(_, s) => s.borrow_mut().addr.finish_cycle(),
        }

        #[cfg(debug_assertions)]
        {
            use bitflags::Flags;
            self.state.clear();
        }
    }
}

fn write_ld_result(result: ReqData, entry: &Entry<LdEntry>, bypass: &mut BypassNetwork<'_>) {
    let data = result.as_ref();
    match entry.dest {
        Dest::Int(dest) => {
            macro_rules! into_intval {
                ($ctor:ident, $ty:ty) => {{
                    IntVal::$ctor(<$ty>::from_le_bytes(data.try_into().unwrap_or_else(|_| {
                        panic!(
                            "invalid data size (expected: {}, actual: {})",
                            size_of::<$ty>(),
                            data.len(),
                        )
                    })) as _)
                }};
            }
            let data = match entry.size {
                1 if entry.signed => into_intval!(signed, i8),
                1 => into_intval!(unsigned, u8),
                2 if entry.signed => into_intval!(signed, i16),
                2 => into_intval!(unsigned, u16),
                4 if entry.signed => into_intval!(signed, i32),
                4 => into_intval!(unsigned, u32),
                8 => {
                    const_assert_eq!(size_of::<Addr>(), 8);
                    into_intval!(unsigned, u64)
                }
                s => panic!("unsupported integer size: {s}"),
            };
            bypass.int.write(*dest, data);
        }
    }
}

#[cfg(debug_assertions)]
bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct State: u8 {
        const DISPATCHED = 1 << 0;
        const FLUSHED = 1 << 1 | Self::DISPATCHED.bits();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    pub ldq_size: NonZeroUsize,
    pub stq_size: NonZeroUsize,
}

#[derive(Debug, Clone)]
pub struct Stats {
    pub req: ReqStats,
    ldq: Box<[EntryStats<LdEntry>]>,
    stq: Box<[EntryStats<StEntry>]>,
}

#[derive(Debug, Clone, Copy)]
pub enum ReqStats {
    None,
    Ld(EntryStats<LdEntry>),
    St(EntryStats<StEntry>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, derive_more::Deref)]
pub struct EntryStats<M> {
    pub addr: Option<Addr>,
    pub size: usize,
    pub fenced: bool,
    #[deref]
    pub meta: M,
}

impl Stats {
    pub fn loads(&self) -> &[EntryStats<LdEntry>] {
        &self.ldq
    }

    pub fn stores(&self) -> &[EntryStats<StEntry>] {
        &self.stq
    }
}

impl<M: DMem> MemSubsystem<M> {
    pub fn stats(&self) -> Stats {
        #[cfg(debug_assertions)]
        assert!(
            self.state.is_empty(),
            "cannot create statistics in the middle of a clock cycle"
        );

        macro_rules! entry_stats {
            ($slot:expr) => {{
                let e = $slot.borrow();
                EntryStats {
                    addr: e.addr.read(),
                    size: e.size,
                    fenced: !e.fence.is_none(),
                    meta: e.meta,
                }
            }};
        }
        macro_rules! queue_stats {
            ($queue:expr) => {{ $queue.iter().map(|s| entry_stats!(s)).collect() }};
        }

        Stats {
            req: match &self.req {
                Req::None => ReqStats::None,
                Req::Ld(_, s) => ReqStats::Ld(entry_stats!(s)),
                Req::St(_, s) => ReqStats::St(entry_stats!(s)),
            },
            ldq: queue_stats!(self.ldq),
            stq: queue_stats!(self.stq),
        }
    }
}

//#endregion
