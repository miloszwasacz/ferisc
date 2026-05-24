use super::FetchUnit;
use crate::core::{fe::AddrGenLock, uop::fetch::FetchPacket};
use crate::mem::{Addr, LdReq, Mem, MemErr, ReqData};
use crate::seq::{CircuitLock, Transformer};
use std::marker::PhantomData;

pub struct FetchLock<'u, 'm, 'a, 'g, M>
where
    M: Mem<LdReq<MemErr>, Output = Result<ReqData, MemErr>>,
{
    unit: &'u mut FetchUnit<M::Conn, M::Handle>,
    mem: &'m mut M,
    addr_gen: &'a mut AddrGenLock<'g>,
    commited: bool,
}

impl<'u, 'm, 'a, 'g, M> FetchLock<'u, 'm, 'a, 'g, M>
where
    M: Mem<LdReq<MemErr>, Output = Result<ReqData, MemErr>>,
{
    pub(super) fn new(
        unit: &'u mut FetchUnit<M::Conn, M::Handle>,
        mem: &'m mut M,
        addr_gen: &'a mut AddrGenLock<'g>,
    ) -> FetchLock<'u, 'm, 'a, 'g, M> {
        Self {
            unit,
            mem,
            addr_gen,
            commited: false,
        }
    }
}

impl<'u, 'm, 'a, 'g, M> CircuitLock<'_> for FetchLock<'u, 'm, 'a, 'g, M>
where
    M: Mem<LdReq<MemErr>, Output = Result<ReqData, MemErr>>,
{
    type Trans = FetchTrans<'u, 'm, 'a, 'g, M>;

    fn make_trans(&mut self) -> Self::Trans {
        Self::Trans {
            finished: false,
            _lock: PhantomData,
        }
    }
}

impl<M> Drop for FetchLock<'_, '_, '_, '_, M>
where
    M: Mem<LdReq<MemErr>, Output = Result<ReqData, MemErr>>,
{
    fn drop(&mut self) {
        if !self.commited {
            // Next pipeline stage stalls
            self.addr_gen.fetch(self.addr_gen.next_pc());
            return;
        }

        let req = self
            .unit
            .req
            .take()
            .and_then(|(h, pred)| {
                let addr = self.mem.req_addr(&h);
                match self.mem.take_if_ready(h) {
                    Ok(out) => {
                        debug_assert!(
                            self.unit.last.is_none(),
                            "output not cleared even though next pipeline stage does not stall"
                        );
                        self.unit.last = Some((addr, out, pred));
                        None
                    }
                    Err(h) => Some((h, pred)),
                }
            })
            .unwrap_or_else(|| {
                let req = LdReq::new(self.addr_gen.next_pc(), self.unit.pkt_width);
                let h = self.mem.req(req, self.unit.mem_conn);
                //TODO BP and BTB predictions
                (h, None)
            });

        let next_pc = self.mem.req_addr(&req.0) + self.unit.pkt_width as Addr;
        self.addr_gen.fetch(next_pc);
        self.unit.req = Some(req);
        #[cfg(debug_assertions)]
        {
            self.unit.fetched = true;
        }
    }
}

pub struct FetchTrans<'u, 'm, 'a, 'g, M>
where
    M: Mem<LdReq<MemErr>, Output = Result<ReqData, MemErr>>,
{
    finished: bool,
    _lock: PhantomData<FetchLock<'u, 'm, 'a, 'g, M>>,
}

impl<'u, 'm, 'a, 'g, M> Transformer<'_> for FetchTrans<'u, 'm, 'a, 'g, M>
where
    M: Mem<LdReq<MemErr>, Output = Result<ReqData, MemErr>>,
{
    type In = ();
    type Out = Result<FetchPacket, MemErr>;
    type Lock = FetchLock<'u, 'm, 'a, 'g, M>;

    fn transform(&mut self, lock: &Self::Lock, _: Self::In) -> Option<Self::Out> {
        if self.finished {
            return None;
        }

        self.finished = true;
        lock.unit.last.clone().map(|(addr, res, btb_pred)| {
            res.map(|bytes| FetchPacket {
                addr,
                bytes,
                btb_pred,
            })
        })
    }

    fn commit(self, lock: &mut Self::Lock) {
        lock.unit.last = None;
        lock.commited = true;
    }
}
