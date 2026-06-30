// pub use self::cached::CachedHierarchy;
pub use self::direct::DirectHierarchy;
pub use self::entry::AddrHandle;
use self::entry::{Entry, LdEntry, StEntry};
use self::locks::DispatchLock;
use crate::seq::{Buf, Queue};
use crate::core::uop::{Exception, fence::Fence};
use crate::mem::{LdReq, Mem, ReqData, StReq};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::ops::DerefMut;
use std::rc::Rc;

// mod cached;
mod direct;
mod entry;
pub mod locks;

//#region Hierarchy

pub trait MemHierarchy {
    type IMem: IMem;
    type DMem: DMem;

    fn imem(&mut self) -> impl DerefMut<Target = Self::IMem>;
    fn dmem(&mut self) -> impl DerefMut<Target = Self::DMem>;
}

pub trait IMem: Mem<LdReq<Exception>, Output = Result<ReqData, Exception>> {}
impl<T: Mem<LdReq<Exception>, Output = Result<ReqData, Exception>>> IMem for T {}

pub trait DMem: Mem<LdReq<Exception>> + Mem<StReq<Exception>> {}
impl<T: Mem<LdReq<Exception>> + Mem<StReq<Exception>>> DMem for T {}

//#endregion

//#region Memory Subsystem

type Slot<T> = Rc<RefCell<Entry<T>>>;

pub struct MemSubsystem<M: DMem> {
    ld_conn: <M as Mem<LdReq<Exception>>>::Conn,
    st_conn: <M as Mem<StReq<Exception>>>::Conn,
    ldq: VecDeque<Slot<LdEntry>>,
    stq: VecDeque<Slot<StEntry>>,
    fence: Buf<Fence>,
    ldq_size: usize,
    stq_size: usize,
    #[cfg(debug_assertions)]
    state: State,
}

impl<M: DMem> MemSubsystem<M> {
    pub fn new(mut dmem: impl DerefMut<Target = M>, ldq_size: usize, stq_size: usize) -> Self {
        let dmem = dmem.deref_mut();
        Self {
            ld_conn: <M as Mem<LdReq<Exception>>>::register_conn(dmem),
            st_conn: <M as Mem<StReq<Exception>>>::register_conn(dmem),
            ldq: VecDeque::new(),
            stq: VecDeque::new(),
            req: Req::None,
            fence: Buf::new(Fence::none()),
            ldq_size,
            stq_size,
            #[cfg(debug_assertions)]
            state: State::empty(),
        }
    }

    pub fn dispatch(&mut self) -> DispatchLock<'_, M> {
        #[cfg(debug_assertions)]
        assert!(
            !self.state.contains(State::DISPATCHED),
            "cannot dispatch more than once in a clock cycle"
        );

        DispatchLock::new(self)
    }
}

#[cfg(debug_assertions)]
bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct State: u8 {
        const DISPATCHED = 1 << 0;
        const FLUSHED = 1 << 1 | Self::DISPATCHED.bits();
    }
}

//#endregion
