use super::FetchUnit;
use crate::core::{
    fe::{addr_gen, predecode::PredecodeLock},
    mem::IMem,
    uop::{fetch::FetchPacket, predecode::Predecoded},
};
use crate::mem::{Addr, LdReq};
use crate::seq::locks::{CircuitLock, Transformer};
use std::collections::vec_deque::Iter;
use std::marker::PhantomData;
use std::ops::DerefMut;

pub struct FetchLock<'u, 'g, 'd, M, D>(Option<LockImpl<'u, 'g, 'd, M, D>>)
where
    M: IMem,
    D: DerefMut<Target = M>;

struct LockImpl<'u, 'g, 'd, M, D>
where
    M: IMem,
    D: DerefMut<Target = M>,
{
    unit: *mut FetchUnit<M>,
    mem: D,
    addr_gen: addr_gen::Lock<'g>,
    predecode: PredecodeLock<'d>,
    iter: Iter<'u, Predecoded>,
    popped: usize,
    _unit: PhantomData<&'u mut FetchUnit<M>>,
}

impl<'u, 'g, 'd, M, D> FetchLock<'u, 'g, 'd, M, D>
where
    M: IMem,
    D: DerefMut<Target = M>,
{
    pub(super) fn new(
        unit: &'u mut FetchUnit<M>,
        mem: D,
        addr_gen: addr_gen::Lock<'g>,
        predecode: PredecodeLock<'d>,
    ) -> FetchLock<'u, 'g, 'd, M, D> {
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

impl<'u, 'g, 'd, M, D> CircuitLock<'u> for FetchLock<'u, 'g, 'd, M, D>
where
    M: IMem,
    D: DerefMut<Target = M>,
{
    type Trans = FetchTrans<'u, 'g, 'd, M, D>;

    fn make_trans(&mut self) -> Self::Trans {
        Self::Trans {
            item: self.0.as_mut().unwrap().iter.next(),
            _lock: PhantomData,
        }
    }
}

impl<M, D> Drop for FetchLock<'_, '_, '_, M, D>
where
    M: IMem,
    D: DerefMut<Target = M>,
{
    fn drop(&mut self) {
        let (unit, mut mem, mut addr_gen, mut predecode, popped) = unsafe {
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

        let mem = mem.deref_mut();
        let (req, addr) = unit
            .req
            .take()
            .and_then(|(h, pred)| {
                let addr = mem.req_addr(&h);
                match mem.take_if_ready(h) {
                    Ok(bytes) => {
                        debug_assert!(unit.out_queue.is_empty());
                        unit.out_queue = predecode.transform(FetchPacket { addr, bytes, pred });
                        None
                    }
                    Err(h) => Some(((h, pred), addr)),
                }
            })
            .unwrap_or_else(|| {
                let addr = addr_gen.next_pc();
                let req = LdReq::new(addr, unit.pkt_width.get() as _);
                let h = mem.req(req, unit.mem_conn);
                //TODO BTB prediction
                ((h, None), addr)
            });

        let next_pc = addr + unit.pkt_width.get() as Addr;
        addr_gen.fetch(next_pc);
        unit.req = Some(req);
        #[cfg(debug_assertions)]
        {
            unit.fetched = true;
        }
    }
}

pub struct FetchTrans<'u, 'g, 'd, M, D>
where
    M: IMem,
    D: DerefMut<Target = M>,
{
    item: Option<&'u Predecoded>,
    _lock: PhantomData<FetchLock<'u, 'g, 'd, M, D>>,
}

impl<'u, 'g, 'd, M, D> Transformer<'u> for FetchTrans<'u, 'g, 'd, M, D>
where
    M: IMem,
    D: DerefMut<Target = M>,
{
    type In = ();
    type Out = &'u Predecoded;
    type Lock = FetchLock<'u, 'g, 'd, M, D>;

    fn transform(&mut self, _: &Self::Lock, _: Self::In) -> Option<Self::Out> {
        self.item
    }

    fn commit(self, lock: &mut Self::Lock) {
        lock.0.as_mut().unwrap().popped += 1;
    }
}
