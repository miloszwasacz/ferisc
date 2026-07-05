use self::lock::FetchLock;
use super::{addr_gen, predecode::PredecodeLock};
use crate::core::{
    mem::IMem,
    uop::{fetch::BtbPred, predecode::Predecoded},
};
use crate::mem::Addr;
use crate::seq::{Sequential, locks::StageLock};
use std::collections::VecDeque;
use std::num::NonZeroU8;
use std::ops::{Deref, DerefMut};

mod lock;

#[derive(Debug)]
pub struct FetchUnit<M: IMem> {
    mem_conn: M::Conn,
    req: Option<(M::Handle, BtbPred)>,
    out_queue: VecDeque<Predecoded>,
    pkt_width: NonZeroU8,
    flushed: bool,
    #[cfg(debug_assertions)]
    fetched: bool,
}

impl<M: IMem> FetchUnit<M> {
    pub fn new(config: &Config, mut mem: impl DerefMut<Target = M>) -> Self {
        Self {
            mem_conn: mem.deref_mut().register_conn(),
            req: None,
            out_queue: VecDeque::new(),
            pkt_width: config.packet_width,
            flushed: false,
            #[cfg(debug_assertions)]
            fetched: false,
        }
    }

    pub fn fetch<'f>(
        &'f mut self,
        mem: impl DerefMut<Target = M>,
        addr_gen: addr_gen::Lock<'_>,
        predecode: PredecodeLock<'_>,
    ) -> impl StageLock<'f, (), &'f Predecoded> {
        #[cfg(debug_assertions)]
        assert!(!self.fetched, "cannot fetch more than once per clock cycle");
        assert!(!self.flushed, "cannot fetch after flushing");
        FetchLock::new(self, mem, addr_gen, predecode)
    }

    pub fn clear(&mut self, mut mem: impl DerefMut<Target = M>) {
        self.flushed = true;
        if let Some((h, _)) = self.req.take() {
            mem.deref_mut().kill(h);
        }
    }
}

impl<M: IMem> Sequential for FetchUnit<M> {
    fn finish_cycle(&mut self) {
        self.flushed = false;
        #[cfg(debug_assertions)]
        {
            self.fetched = false;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    pub packet_width: NonZeroU8,
}

#[derive(Debug, Clone)]
pub struct Stats {
    pub fetching: Option<Addr>,
}

impl<M: IMem> FetchUnit<M> {
    pub fn stats(&self, imem: impl Deref<Target = M>) -> Stats {
        Stats {
            fetching: self.req.as_ref().map(|(h, _)| imem.req_addr(h)),
        }
    }

    pub(super) fn predecode_stats(&self) -> &VecDeque<Predecoded> {
        &self.out_queue
    }
}
