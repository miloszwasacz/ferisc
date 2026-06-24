use self::locks::{DispatchLock, IssueLock};
use crate::core::rob;
use crate::seq::{Buf, Sequential};
use std::collections::VecDeque;

pub mod locks;

#[derive(Debug, Clone)]
pub struct Rs<U> {
    entries: VecDeque<(U, rob::EntryHandle)>,
    len: Buf<usize>,
    size: usize,
    #[cfg(debug_assertions)]
    state: State,
}

impl<U> Rs<U> {
    pub fn dispatch(&mut self) -> DispatchLock<'_, U> {
        #[cfg(debug_assertions)]
        assert!(
            !self.state.contains(State::DISPATCHED),
            "cannot dispatch more than once per clock cycle"
        );

        DispatchLock::new(self)
    }

    pub fn issue(&mut self) -> IssueLock<'_, U> {
        #[cfg(debug_assertions)]
        assert!(
            !self.state.contains(State::ISSUED),
            "cannot issue more than once per clock cycle"
        );

        todo!()
    }
}

impl<U> Sequential for Rs<U> {
    fn finish_cycle(&mut self) {
        debug_assert!(self.entries.len() <= self.size);
        self.len.write(self.entries.len());
        self.len.finish_cycle();

        #[cfg(debug_assertions)]
        {
            use bitflags::Flags;
            self.state.clear();
        }
    }
}

#[cfg(debug_assertions)]
bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct State: u8 {
        const DISPATCHED = 1 << 0;
        const ISSUED = 1 << 1;
    }
}
