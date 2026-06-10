use self::lock::FetchLock;
use super::{AddrGenLock, predecode::PredecodeLock};
use crate::core::uop::{Exception, fetch::BtbPred, predecode::Predecoded};
use crate::mem::{LdReq, Mem, MemConn, ReqData, ReqHandle};
use crate::seq::{Sequential, locks::StageLock};
use std::collections::VecDeque;

mod lock;

#[derive(Debug)]
pub struct FetchUnit<C = MemConn, H = ReqHandle> {
    mem_conn: C,
    req: Option<(H, BtbPred)>,
    out_queue: VecDeque<Predecoded>,
    pkt_width: usize,
    flushed: bool,
    #[cfg(debug_assertions)]
    fetched: bool,
}

impl<C, H> FetchUnit<C, H> {
    pub fn new<M>(mem: &mut M, pkt_width: usize) -> Self
    where
        M: Mem<LdReq<Exception>, Conn = C, Handle = H, Output = Result<ReqData, Exception>>,
    {
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

    pub fn fetch<'f, M>(
        &'f mut self,
        mem: &mut M,
        addr_gen: AddrGenLock<'_>,
        predecode: PredecodeLock<'_>,
    ) -> impl StageLock<'f, (), &'f Predecoded>
    where
        M: Mem<LdReq<Exception>, Conn = C, Handle = H, Output = Result<ReqData, Exception>>,
    {
        #[cfg(debug_assertions)]
        assert!(!self.fetched, "cannot fetch more than once per clock cycle");
        assert!(!self.flushed, "cannot fetch after flushing");
        FetchLock::new(self, mem, addr_gen, predecode)
    }

    pub fn clear<M>(&mut self, mem: &mut M)
    where
        M: Mem<LdReq<Exception>, Conn = C, Handle = H, Output = Result<ReqData, Exception>>,
    {
        self.flushed = true;
        if let Some((h, _)) = self.req.take() {
            mem.kill(h);
        }
    }
}

impl<C, H> Sequential for FetchUnit<C, H> {
    fn finish_cycle(&mut self) {
        self.flushed = false;
        #[cfg(debug_assertions)]
        {
            self.fetched = false;
        }
    }
}
