use super::FetchUnit;
use crate::core::{
    fe::{AddrGenLock, predecode::PredecodeLock},
    uop::{Exception, fetch::FetchPacket, predecode::Predecoded},
};
use crate::mem::{Addr, LdReq, Mem, ReqData};
use crate::seq::locks::{CircuitLock, Transformer};
use std::collections::vec_deque::Iter;
use std::marker::PhantomData;

pub struct FetchLock<'u, 'm, 'a, 'g, 'd, M>(Option<LockImpl<'u, 'm, 'a, 'g, 'd, M>>)
where
    M: Mem<LdReq<Exception>, Output = Result<ReqData, Exception>>;

struct LockImpl<'u, 'm, 'a, 'g, 'd, M>
where
    M: Mem<LdReq<Exception>, Output = Result<ReqData, Exception>>,
{
    unit: *mut FetchUnit<M::Conn, M::Handle>,
    mem: &'m mut M,
    addr_gen: &'a mut AddrGenLock<'g>,
    predecode: PredecodeLock<'d>,
    iter: Iter<'u, Predecoded>,
    popped: usize,
    _unit: PhantomData<&'u mut FetchUnit<M::Conn, M::Handle>>,
}

impl<'u, 'm, 'a, 'g, 'd, M> FetchLock<'u, 'm, 'a, 'g, 'd, M>
where
    M: Mem<LdReq<Exception>, Output = Result<ReqData, Exception>>,
{
    pub(super) fn new(
        unit: &'u mut FetchUnit<M::Conn, M::Handle>,
        mem: &'m mut M,
        addr_gen: &'a mut AddrGenLock<'g>,
        predecode: PredecodeLock<'d>,
    ) -> FetchLock<'u, 'm, 'a, 'g, 'd, M> {
        Self(Some(LockImpl {
            unit: unit as _,
            mem,
            addr_gen,
            predecode,
            iter: unit.out_queue.iter(),
            popped: 0,
            _unit: PhantomData,
        }))
    }
}

impl<'u, 'm, 'a, 'g, 'd, M> CircuitLock<'u> for FetchLock<'u, 'm, 'a, 'g, 'd, M>
where
    M: Mem<LdReq<Exception>, Output = Result<ReqData, Exception>>,
{
    type Trans = FetchTrans<'u, 'm, 'a, 'g, 'd, M>;

    fn make_trans(&mut self) -> Self::Trans {
        Self::Trans {
            item: self.0.as_mut().unwrap().iter.next(),
            _lock: PhantomData,
        }
    }
}

impl<M> Drop for FetchLock<'_, '_, '_, '_, '_, M>
where
    M: Mem<LdReq<Exception>, Output = Result<ReqData, Exception>>,
{
    fn drop(&mut self) {
        let (unit, mem, addr_gen, mut predecode, popped) = unsafe {
            // SAFETY: iter is dropped so it is safe to interact with the fetch unit again
            let LockImpl {
                unit,
                mem,
                addr_gen,
                predecode,
                popped,
                ..
            } = self.0.take().unwrap();
            (&mut *unit, mem, addr_gen, predecode, popped)
        };
        drop(unit.out_queue.drain(..popped));

        if !unit.out_queue.is_empty() {
            addr_gen.fetch(addr_gen.next_pc());
            predecode.forget();
            return;
        }

        let req = unit
            .req
            .take()
            .and_then(|(h, btb_pred)| {
                let addr = mem.req_addr(&h);
                match mem.take_if_ready(h) {
                    Ok(bytes) => {
                        debug_assert!(unit.out_queue.is_empty());
                        unit.out_queue = predecode.transform(FetchPacket {
                            addr,
                            bytes,
                            btb_pred,
                        });
                        //TODO BP prediction
                        None
                    }
                    Err(h) => Some((h, btb_pred)),
                }
            })
            .unwrap_or_else(|| {
                let req = LdReq::new(addr_gen.next_pc(), unit.pkt_width);
                let h = mem.req(req, unit.mem_conn);
                //TODO BTB prediction
                (h, None)
            });

        let next_pc = mem.req_addr(&req.0) + unit.pkt_width as Addr;
        addr_gen.fetch(next_pc);
        unit.req = Some(req);
        #[cfg(debug_assertions)]
        {
            unit.fetched = true;
        }
    }
}

pub struct FetchTrans<'u, 'm, 'a, 'g, 'd, M>
where
    M: Mem<LdReq<Exception>, Output = Result<ReqData, Exception>>,
{
    item: Option<&'u Predecoded>,
    _lock: PhantomData<FetchLock<'u, 'm, 'a, 'g, 'd, M>>,
}

impl<'u, 'm, 'a, 'g, 'd, M> Transformer<'u> for FetchTrans<'u, 'm, 'a, 'g, 'd, M>
where
    M: Mem<LdReq<Exception>, Output = Result<ReqData, Exception>>,
{
    type In = ();
    type Out = &'u Predecoded;
    type Lock = FetchLock<'u, 'm, 'a, 'g, 'd, M>;

    fn transform(&mut self, _: &Self::Lock, _: Self::In) -> Option<Self::Out> {
        self.item
    }

    fn commit(self, lock: &mut Self::Lock) {
        lock.0.as_mut().unwrap().popped += 1;
    }
}
