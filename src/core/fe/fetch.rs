pub use self::lock::FetchLock;
use super::AddrGenLock;
use crate::core::uop::fetch::BtbPred;
use crate::mem::{Addr, LdReq, Mem, MemConn, MemErr, ReqData, ReqHandle};
use crate::seq::Sequential;

mod lock;

pub struct FetchUnit<C = MemConn, H = ReqHandle> {
    mem_conn: C,
    req: Option<(H, BtbPred)>,
    last: Option<(Addr, Result<ReqData, MemErr>, BtbPred)>,
    pkt_width: usize,
    flushed: bool,
    #[cfg(debug_assertions)]
    fetched: bool,
}

impl<C, H> FetchUnit<C, H> {
    pub fn new<M>(mem: &mut M, pkt_width: usize) -> Self
    where
        M: Mem<LdReq<MemErr>, Conn = C, Handle = H, Output = Result<ReqData, MemErr>>,
    {
        Self {
            mem_conn: mem.register_conn(),
            req: None,
            last: None,
            pkt_width,
            flushed: false,
            #[cfg(debug_assertions)]
            fetched: false,
        }
    }

    pub fn fetch<'f, 'm, 'a, 'g, M>(
        &'f mut self,
        mem: &'m mut M,
        addr_gen: &'a mut AddrGenLock<'g>,
    ) -> FetchLock<'f, 'm, 'a, 'g, M>
    where
        M: Mem<LdReq<MemErr>, Conn = C, Handle = H, Output = Result<ReqData, MemErr>>,
    {
        #[cfg(debug_assertions)]
        assert!(!self.fetched, "cannot fetch more than once per clock cycle");
        assert!(!self.flushed, "cannot fetch after flushing");
        FetchLock::new(self, mem, addr_gen)
    }

    pub fn clear<M>(&mut self, mem: &mut M)
    where
        M: Mem<LdReq<MemErr>, Conn = C, Handle = H, Output = Result<ReqData, MemErr>>,
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
