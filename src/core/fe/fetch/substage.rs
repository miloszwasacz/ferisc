use super::{FetchUnit, Req, ReqState, State};
use crate::core::{fe::addr_gen, mem::hierarchy::IMem, uop::fetch::FetchPacket};
use crate::mem::{Addr, LdReq};
use crate::seq::SimpleSubStage;
use std::mem::{replace, take};

pub struct Out<'a, 'g, M: IMem> {
    fetch: &'a mut FetchUnit<M>,
    addr_gen: addr_gen::Lock<'g>,
    imem: &'a mut M,
    done: bool,
}

impl<'a, 'g, M: IMem> Out<'a, 'g, M> {
    pub(super) fn new(
        fetch: &'a mut FetchUnit<M>,
        addr_gen: addr_gen::Lock<'g>,
        imem: &'a mut M,
    ) -> Self {
        #[cfg(debug_assertions)]
        {
            match fetch.state {
                State::Ready => {}
                State::Fetched => panic!("cannot fetch more than once per clock cycle"),
                State::Flushed => panic!("cannot fetch after flushing"),
            }
            fetch.state = State::Fetched;
        }

        Self {
            fetch,
            addr_gen,
            imem,
            done: false,
        }
    }
}

impl<'a, 'g, M: IMem> SimpleSubStage for Out<'a, 'g, M> {
    type In = ();
    type Out = FetchPacket;

    fn start(&mut self, (): Self::In) -> Option<Self::Out> {
        macro_rules! set_req {
            ($v:expr) => {{
                let old = replace(&mut self.fetch.req, $v);
                debug_assert!(matches!(old, ReqState::None));
            }};
        }
        if self.done {
            return None;
        }

        match take(&mut self.fetch.req) {
            ReqState::None => None,
            ReqState::InProgress(Req { addr, handle, pred }) => {
                match self.imem.take_if_ready(handle) {
                    Ok(bytes) => {
                        let pkt = FetchPacket { addr, bytes, pred };

                        // Cache in case further stages stall
                        set_req!(ReqState::Cached(pkt.clone()));

                        Some(pkt)
                    }
                    Err(handle) => {
                        set_req!(ReqState::InProgress(Req { addr, handle, pred }));
                        None
                    }
                }
            }
            ReqState::Cached(pkt) => {
                // Cache in case further stages stall
                set_req!(ReqState::Cached(pkt.clone()));

                Some(pkt)
            }
        }
    }

    fn finish(&mut self) {
        let req = take(&mut self.fetch.req);
        debug_assert!(matches!(req, ReqState::Cached(_)));
        self.done = true;
    }
}

impl<M: IMem> Drop for Out<'_, '_, M> {
    fn drop(&mut self) {
        let (req, next_pc) = match take(&mut self.fetch.req) {
            ReqState::None => {
                // Make a new request
                let addr = self.addr_gen.next_pc();
                let ld = LdReq::new(addr, self.fetch.pkt_width);
                let handle = self.imem.req(ld, self.fetch.mem_conn);
                let pred = None; //TODO BTB prediction

                let req = ReqState::InProgress(Req { addr, handle, pred });
                let next_pc = addr + self.fetch.pkt_width.as_usize() as Addr;
                (req, next_pc)
            }
            req @ ReqState::InProgress(_) | req @ ReqState::Cached(_) => {
                // Wait until the request is handled
                (req, self.addr_gen.next_pc())
            }
        };
        self.fetch.req = req;
        self.addr_gen.fetch(next_pc);
    }
}
