use super::{
    AddrBypass, DMem, Req, Slot,
    entry::{LdEntry, StEntry},
};
use crate::core::ee::pipeline::Prfs;
use crate::core::mem::entry::Src;
use crate::mem::{LdReq, ReqData, StReq};
use crate::seq::{Buf, Sequential};
use std::ops::{Index, IndexMut};

//TODO Add notes that this struct does not guard against the "imperativeness" of the code

#[derive(Debug, Clone)]
pub(super) struct Queue<T> {
    queue: Vec<Slot<T>>,
    len: Buf<usize>,
    size: usize,
}

impl<T> Queue<T> {
    pub fn new(size: usize) -> Self {
        Self {
            queue: Vec::with_capacity(size),
            len: Buf::new(0),
            size,
        }
    }

    //TODO Add docs explaining how `free_space` reports the actual current unbuffered free space
    pub fn free_space(&self) -> usize {
        self.size - self.queue.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Slot<T>> {
        self.queue.iter().take(self.len.read_copied())
    }

    pub fn push(&mut self, slot: Slot<T>) {
        debug_assert!(self.queue.len() < self.size);
        self.queue.push(slot);
    }

    pub fn flush(&mut self) {
        self.queue.retain(|s| !s.borrow().rob_entry.is_flushed());
    }

    pub fn flush_all(&mut self) {
        self.queue.clear();
        self.len.write(0);
    }
}

macro_rules! update_deps {
    ($queue:expr, $bypass:expr) => {{
        $queue
            .iter_mut()
            .enumerate()
            .for_each(|(_, s)| s.borrow_mut().update_deps($bypass));
    }};
}

impl Queue<LdEntry> {
    pub fn update_deps(&mut self, addr_bypass: &AddrBypass) {
        update_deps!(self.queue, addr_bypass);
    }

    //TODO Add docs explaining how this can remove an element which influences `free_space`
    pub fn schedule_ready<M: DMem>(
        &mut self,
        mem: &mut M,
        conn: ld_mem!(<M>::Conn),
        addr_bypass: &AddrBypass,
    ) -> Req<M> {
        let mut req = Req::None;
        self.queue.retain(|s| {
            if !matches!(req, Req::None) {
                return true;
            }
            let ld_req = {
                let e = s.borrow();
                let addr = match addr_bypass.read_entry(&e) {
                    Some(addr) if e.deps.is_empty() => addr,
                    Some(_) | None => return true,
                };
                LdReq::new(addr, e.size)
            };
            req = Req::Ld(ld_mem!(<M>::req(mem, ld_req, conn)), s.clone());
            false
        });
        req
    }
}

impl Queue<StEntry> {
    pub fn update_deps(&mut self, addr_bypass: &AddrBypass) {
        update_deps!(self.queue, addr_bypass);
    }

    //TODO Add docs explaining how this can remove an element which influences `free_space`
    pub fn schedule_ready<M: DMem>(
        &mut self,
        mem: &mut M,
        conn: st_mem!(<M>::Conn),
        addr_bypass: &AddrBypass,
        prfs: &Prfs<'_>,
    ) -> Req<M> {
        let mut req = Req::None;
        self.queue.retain(|s| {
            if !matches!(req, Req::None) {
                return true;
            }
            let st_req = {
                let e = s.borrow();
                let addr = match addr_bypass.read_entry(&e) {
                    Some(addr) if e.deps.is_empty() => addr,
                    Some(_) | None => return true,
                };
                let data = match e.src {
                    Src::Int(reg) => {
                        let Some(data) = prfs.int[reg].try_read_copied() else {
                            return true;
                        };
                        ReqData::from(&data.u().to_le_bytes()[..e.size])
                    }
                };
                StReq::new(addr, data)
            };
            req = Req::St(st_mem!(<M>::req(mem, st_req, conn)), s.clone());
            false
        });
        req
    }
}

impl<T> Index<usize> for Queue<T> {
    type Output = Slot<T>;

    fn index(&self, index: usize) -> &Self::Output {
        self.queue.index(index)
    }
}

impl<T> IndexMut<usize> for Queue<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.queue.index_mut(index)
    }
}

impl<T> Sequential for Queue<T> {
    fn finish_cycle(&mut self) {
        self.len.write(self.queue.len());
        self.len.finish_cycle();
    }
}
