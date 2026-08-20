use self::substage::Out;
use super::addr_gen;
use crate::core::{
    mem::hierarchy::IMem,
    uop::{
        Exception,
        fetch::{BtbPred, FetchPacket},
    },
};
use crate::mem::{Addr, Size};
use crate::seq::Sequential;

pub mod substage;

#[derive(derive_more::Debug)]
pub struct FetchUnit<M: IMem> {
    mem_conn: M::Conn,
    req: ReqState<M>,
    pkt_width: Size,
    #[cfg(debug_assertions)]
    state: State,
}

#[derive(derive_more::Debug)]
struct Req<M: IMem> {
    addr: Addr,
    handle: M::Handle,
    pred: BtbPred,
}

#[derive(derive_more::Debug, Default)]
enum ReqState<M: IMem> {
    #[default]
    None,
    InProgress(Req<M>),
    Cached(FetchPacket),
}

#[cfg(debug_assertions)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
enum State {
    #[default]
    Ready,
    Fetched,
    Flushed,
}

impl<M: IMem> FetchUnit<M> {
    pub fn new(config: &Config, imem: &mut M) -> Self {
        Self {
            mem_conn: imem.register_conn(),
            req: Default::default(),
            pkt_width: config.packet_width,
            state: Default::default(),
        }
    }

    pub fn out<'a, 'g>(
        &'a mut self,
        addr_gen: addr_gen::Lock<'g>,
        imem: &'a mut M,
    ) -> Out<'a, 'g, M> {
        Out::new(self, addr_gen, imem)
    }

    pub fn flush(&mut self, imem: &mut M) {
        #[cfg(debug_assertions)]
        {
            match self.state {
                State::Ready => {}
                State::Fetched => {}
                State::Flushed => panic!("cannot flush more than once per clock cycle"),
            }
            self.state = State::Flushed;
        }

        if let ReqState::InProgress(Req { handle, .. }) = std::mem::take(&mut self.req) {
            imem.kill(handle);
        }
    }
}

impl<M: IMem> Sequential for FetchUnit<M> {
    fn finish_cycle(&mut self) {
        #[cfg(debug_assertions)]
        {
            self.state = Default::default();
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    pub packet_width: Size,
}

#[derive(Debug, Clone)]
pub enum Stats {
    Idle,
    Fetching(Addr),
    Fetched {
        addr: Addr,
        exception: Option<Exception>,
    },
}

impl<M: IMem> FetchUnit<M> {
    pub fn stats(&self) -> Stats {
        match &self.req {
            ReqState::None => Stats::Idle,
            ReqState::InProgress(Req { addr, .. }) => Stats::Fetching(*addr),
            ReqState::Cached(pkt) => Stats::Fetched {
                addr: pkt.addr,
                exception: pkt.bytes.as_ref().err().copied(),
            },
        }
    }
}
