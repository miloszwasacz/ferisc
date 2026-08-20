pub use self::entry::{
    AddrHandle, Ld, ReadyStats as ReadyEntryStats, Src, St, Stats as EntryStats,
};
use self::entry::{Entry, ReadyEntry};
use self::fence::FenceQueue;
use self::hierarchy::DMem;
use self::queue::Queue;
pub use self::substage::{Commit, Dispatch};
use crate::core::{
    ee::rob::{self, Flushed},
    uop::{Exception, commit::Dest},
};
use crate::mem::{Bytes, LdReq, Mem, ReqData, Size, StReq};
use crate::seq::{Clearable, Sequential};
use itertools::{EitherOrBoth, Itertools};
use std::num::NonZeroUsize;

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

mod entry;
mod fence;
pub mod hierarchy;
mod queue;
pub mod substage;

//#region Memory Subsystem

//TODO MMIO
pub struct MemSubsystem<M: DMem> {
    ld_conn: ld_mem!(<M>::Conn),
    st_conn: st_mem!(<M>::Conn),
    ldq: Queue<Ld>,
    stq: Queue<St>,
    req: Req<M>,
    fences: FenceQueue,
}

#[derive(Default)]
enum Req<M: DMem> {
    #[default]
    None,
    Ld(<M as Mem<LdReq<Exception>>>::Handle, ReadyEntry<Ld>),
    St(<M as Mem<StReq<Exception>>>::Handle, ReadyEntry<St>),
}

#[derive(Debug, Clone)]
pub struct LdResult {
    pub dest: Dest,
    pub signed: bool,
    pub bytes: Bytes,
}

impl<M: DMem> MemSubsystem<M> {
    pub fn new(config: &Config, dmem: &mut M) -> Self {
        Self {
            ld_conn: ld_mem!(<M>::register_conn(dmem)),
            st_conn: st_mem!(<M>::register_conn(dmem)),
            ldq: Queue::new(config.ldq_size),
            stq: Queue::new(config.stq_size),
            req: Req::None,
            fences: FenceQueue::new(config.max_fences),
        }
    }

    pub fn dispatch(&mut self) -> Dispatch<'_> {
        Dispatch::new(self)
    }

    pub fn tick(
        &mut self,
        dmem: &mut M,
        try_read_bypassed: impl Fn(Src, Size) -> Option<ReqData>,
        write: impl FnOnce(LdResult),
    ) {
        // Update dependencies
        self.ldq.update_deps();
        self.stq.update_deps();

        // Handle currently processed request
        let ld_last = match std::mem::take(&mut self.req) {
            Req::Ld(handle, entry) => {
                match ld_mem!(<M>::take_if_ready(dmem, handle)) {
                    Ok(Ok(bytes)) => {
                        write(LdResult {
                            dest: entry.dest(),
                            signed: entry.signed(),
                            bytes,
                        });
                        entry.into_rob_entry().mark_done();
                    }
                    Ok(Err(ex)) => entry.into_rob_entry().throw(ex),
                    Err(handle) => {
                        self.req = Req::Ld(handle, entry);
                        return;
                    }
                }
                true
            }
            Req::St(handle, entry) => {
                match st_mem!(<M>::take_if_ready(dmem, handle)) {
                    Ok(Ok(())) => {
                        entry.into_rob_entry().mark_done();
                    }
                    Ok(Err(ex)) => entry.into_rob_entry().throw(ex),
                    Err(handle) => {
                        self.req = Req::St(handle, entry);
                        return;
                    }
                }
                false
            }
            Req::None => false,
        };

        // Schedule new request
        debug_assert!(matches!(self.req, Req::None));
        let mut schedule_ready = |try_ld| {
            if try_ld {
                self.ldq.schedule_ready(dmem, self.ld_conn)
            } else {
                self.stq
                    .schedule_ready(dmem, self.st_conn, &try_read_bypassed)
            }
        };
        let try_ld = !ld_last;
        self.req = schedule_ready(try_ld);
        if matches!(self.req, Req::None) {
            self.req = schedule_ready(!try_ld);
        }
    }

    pub fn commit(&mut self) -> Commit<'_> {
        Commit::new(self)
    }

    pub fn flush(&mut self, dmem: &mut M, flush: &rob::lock::Flush<'_>) {
        // Queues
        self.ldq.flush(flush);
        self.stq.flush(flush);

        // Current request
        match std::mem::take(&mut self.req) {
            Req::Ld(handle, entry) if entry.is_flushed(flush) => {
                ld_mem!(<M>::kill(dmem, handle));
            }
            Req::St(handle, entry) if entry.is_flushed(flush) => {
                st_mem!(<M>::kill(dmem, handle));
            }
            req => self.req = req,
        }

        // Fences
        let flushed_fences = flush
            .flushed()
            .filter_map(|flushed| match flushed {
                Flushed::MemOrd(fence) => Some(fence),
                _ => None,
            })
            .zip_longest(self.fences.iter().rev())
            .inspect(|fences| match fences {
                EitherOrBoth::Both(flushed, fence) => {
                    debug_assert_eq!(***fence, **flushed, "inconsistent fences")
                }
                EitherOrBoth::Right(_) => {
                    // Not all fences flushed
                }
                EitherOrBoth::Left(_) => debug_assert!(
                    false,
                    "more fences flushed than tracked by the memory subsystem"
                ),
            })
            .count();
        let _ = self.fences.drain_back(flushed_fences);
    }

    pub fn flush_all(&mut self, dmem: &mut M) {
        // Queues
        self.ldq.clear();
        self.stq.clear();

        // Current request
        match std::mem::take(&mut self.req) {
            Req::Ld(handle, _) => {
                ld_mem!(<M>::kill(dmem, handle));
            }
            Req::St(handle, _) => {
                st_mem!(<M>::kill(dmem, handle));
            }
            Req::None => {}
        }

        // Fences
        self.fences.clear();
    }
}

impl<M: DMem> Sequential for MemSubsystem<M> {
    fn finish_cycle(&mut self) {
        self.ldq.finish_cycle();
        self.stq.finish_cycle();
        self.fences.finish_cycle();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    pub ldq_size: NonZeroUsize,
    pub stq_size: NonZeroUsize,
    pub max_fences: NonZeroUsize,
}

#[derive(Debug, Clone)]
pub struct Stats {
    pub req: ReqStats,
    ldq: Box<[EntryStats<Ld>]>,
    stq: Box<[EntryStats<St>]>,
}

#[derive(Debug, Clone, Copy)]
pub enum ReqStats {
    None,
    Ld(ReadyEntryStats<Ld>),
    St(ReadyEntryStats<St>),
}

impl Stats {
    pub fn loads(&self) -> &[EntryStats<Ld>] {
        &self.ldq
    }

    pub fn stores(&self) -> &[EntryStats<St>] {
        &self.stq
    }
}

impl<M: DMem> MemSubsystem<M> {
    pub fn stats(&self) -> Stats {
        Stats {
            req: match &self.req {
                Req::None => ReqStats::None,
                Req::Ld(_, e) => ReqStats::Ld(e.stats()),
                Req::St(_, e) => ReqStats::St(e.stats()),
            },
            ldq: self.ldq.iter().map(Entry::stats).collect(),
            stq: self.stq.iter().map(Entry::stats).collect(),
        }
    }
}

//#endregion
