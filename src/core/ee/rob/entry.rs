use crate::core::{
    csr::{CsrAddr, CsrFile},
    ee::rob::{Flushed, lock::Flush},
    uop::{Exception, commit::Dest, decode::UopKind as EntryDisplay, fence::Fence, rename},
};
use crate::mem::Addr;
use crate::seq::{Buf, Sequential};
use std::cell::RefCell;
use std::debug_assert_matches;
use std::rc::{Rc, Weak};

pub mod pipelines;

type StateCell = RefCell<Option<Buf<EntryState>>>;

#[derive(Debug, Clone)]
pub(super) struct Entry {
    kind: UopKind,
    addr: Addr,
    comp: bool,
    state: Rc<StateCell>,
    #[cfg(feature = "tui")]
    display: EntryDisplay,
}

#[derive(Debug, Clone, Copy)]
pub enum UopKind {
    Reg(Dest),
    Br,
    Ld(Dest),
    St,
    Csr(Dest, CsrAddr),
    MemOrd(Fence),
    Except(Exception),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum EntryState {
    #[default]
    NotDone,
    Done,
    Exception(Exception),
}

impl Entry {
    pub fn new(uop: rename::Uop) -> Self {
        let rename::Uop {
            kind,
            addr,
            comp,
            #[cfg(feature = "tui")]
            display,
        } = uop;

        let kind = kind.into();
        let state = match kind {
            UopKind::Reg(_) | UopKind::Br | UopKind::Ld(_) | UopKind::St | UopKind::Csr(_, _) => {
                EntryState::NotDone
            }
            UopKind::MemOrd(_) | UopKind::Except(_) => EntryState::Done,
        };

        Self {
            kind,
            addr,
            comp,
            state: Rc::new(RefCell::new(Some(Buf::new(state)))),
            #[cfg(feature = "tui")]
            display,
        }
    }

    pub fn kind(&self) -> UopKind {
        self.kind
    }

    pub fn addr(&self) -> Addr {
        self.addr
    }

    pub fn comp(&self) -> bool {
        self.comp
    }

    pub fn handle(&self) -> Handle {
        Handle(Rc::downgrade(&self.state))
    }

    pub fn handle_eq(&self, other: &Handle) -> bool {
        other.0.ptr_eq(&Rc::downgrade(&self.state))
    }

    pub fn is_done(&self) -> bool {
        let Some(state) = &*self.state.borrow() else {
            // Flushed
            return false;
        };
        match state.read_copied() {
            EntryState::NotDone => false,
            EntryState::Done => true,
            EntryState::Exception(_) => {
                panic!("invalid ROB entry state: exception should be committed to `kind`")
            }
        }
    }

    pub(super) fn flush(&self, csrf: &mut CsrFile) -> Flushed {
        {
            let mut state = self.state.borrow_mut();
            debug_assert!(state.is_some(), "entry already flushed");
            *state = None;
        }
        match self.kind() {
            UopKind::Reg(dest) => Flushed::Reg(dest),
            UopKind::Ld(dest) => Flushed::Ld(dest),
            UopKind::St => Flushed::St,
            UopKind::Csr(dest, csr) => {
                csrf.flush(csr);
                Flushed::Csr(dest, csr)
            }
            UopKind::MemOrd(fence) => Flushed::MemOrd(fence),
            UopKind::Br | UopKind::Except(_) => Flushed::Other,
        }
    }
}

impl Sequential for Entry {
    fn finish_cycle(&mut self) {
        let mut state_guard = self.state.borrow_mut();
        let Some(state) = state_guard.as_mut() else {
            return;
        };

        state.finish_cycle();
        if let EntryState::Exception(ex) = state.read_copied() {
            self.kind = UopKind::Except(ex);
            state.write(EntryState::Done);
            state.finish_cycle();
        }
    }
}

//#region Handle

#[derive(Debug, Clone)]
pub struct Handle(Weak<StateCell>);

impl Handle {
    fn unwrap(&self) -> Rc<StateCell> {
        self.0.upgrade().expect("handle should be valid")
    }

    pub fn is_flushed(&self, flush: &Flush<'_>) -> bool {
        // Ignored field to enforce that the ROB is flushed before any handles
        // are checked if they have been flushed.
        let _ = flush;

        self.unwrap().borrow().is_none()
    }

    #[inline(always)]
    fn set(self, new_state: EntryState) {
        let state_rc = self.unwrap();
        let mut state = state_rc.borrow_mut();
        let Some(state) = state.as_mut() else {
            panic!("entry flushed")
        };
        debug_assert_matches!(
            state.read_bypassed(),
            EntryState::NotDone,
            "entry already processed"
        );
        state.write(new_state)
    }

    pub fn mark_done(self) {
        self.set(EntryState::Done);
    }

    pub fn throw(self, exception: Exception) {
        self.set(EntryState::Exception(exception));
    }
}

//#endregion

//#region Stats

#[derive(Debug, Clone, Copy)]
pub struct Stats {
    pub kind: UopKind,
    pub addr: Addr,
    pub comp: bool,
    pub state: State,
    #[cfg(feature = "tui")]
    pub display: EntryDisplay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    NotDone,
    Done,
    Flushed,
}

impl Entry {
    pub fn stats(&self, is_flushed: bool) -> Stats {
        Stats {
            kind: self.kind(),
            addr: self.addr(),
            comp: self.comp(),
            state: if is_flushed {
                State::Flushed
            } else if self.is_done() {
                State::Done
            } else {
                State::NotDone
            },
            display: self.display,
        }
    }
}

//#endregion
