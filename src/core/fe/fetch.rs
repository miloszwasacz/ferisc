use self::lock::FetchLock;
use super::{AddrGenLock, predecode::PredecodeLock};
use crate::core::{
    mem::IMem,
    uop::{fetch::BtbPred, predecode::Predecoded},
};
use crate::seq::{Sequential, locks::StageLock};
use std::collections::VecDeque;

mod lock;

#[derive(Debug)]
pub struct FetchUnit<M: IMem> {
    mem_conn: M::Conn,
    req: Option<(M::Handle, BtbPred)>,
    out_queue: VecDeque<Predecoded>,
    pkt_width: usize,
    flushed: bool,
    #[cfg(debug_assertions)]
    fetched: bool,
}

impl<M: IMem> FetchUnit<M> {
    pub fn new(mem: &mut M, pkt_width: usize) -> Self {
        Self {
            mem_conn: mem.register_conn(),
            req: None,
            out_queue: VecDeque::new(),
            pkt_width,
            flushed: false,
            #[cfg(debug_assertions)]
            fetched: false,
        }
    }

    pub fn fetch<'f>(
        &'f mut self,
        mem: &mut M,
        addr_gen: AddrGenLock<'_>,
        predecode: PredecodeLock<'_>,
    ) -> impl StageLock<'f, (), &'f Predecoded> {
        #[cfg(debug_assertions)]
        assert!(!self.fetched, "cannot fetch more than once per clock cycle");
        assert!(!self.flushed, "cannot fetch after flushing");
        FetchLock::new(self, mem, addr_gen, predecode)
    }

    pub fn clear(&mut self, mem: &mut M) {
        self.flushed = true;
        if let Some((h, _)) = self.req.take() {
            mem.kill(h);
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
