pub use self::bypass::{Bypass, BypassBuilder};
use self::locks::{DispatchLock, IssueLock};
use crate::core::uop::issue::{Issue, TryReadReg, Uop as IUop};
use crate::seq::{Buf, Clearable, Sequential};
use std::collections::VecDeque;
use std::num::NonZeroUsize;

mod bypass;
pub mod locks;

#[derive(Debug, Clone)]
pub struct Rs<U> {
    entries: VecDeque<IUop<U>>,
    len: Buf<usize>,
    size: NonZeroUsize,
    #[cfg(debug_assertions)]
    state: State,
}

impl<U> Rs<U> {
    pub fn new(config: &Config) -> Self {
        Self {
            entries: VecDeque::with_capacity(config.size.get()),
            len: Buf::new(0),
            size: config.size,
            #[cfg(debug_assertions)]
            state: State::empty(),
        }
    }

    pub fn dispatch(&mut self) -> DispatchLock<'_, U> {
        #[cfg(debug_assertions)]
        assert!(
            !self.state.contains(State::DISPATCHED),
            "cannot dispatch more than once per clock cycle"
        );

        DispatchLock::new(self)
    }

    pub fn issue<'a, R>(&'a mut self, try_read_reg: R) -> IssueLock<'a, U, R>
    where
        U: Issue + Clone + 'a,
        R: TryReadReg<U::Reg>,
    {
        #[cfg(debug_assertions)]
        assert!(
            !self.state.contains(State::ISSUED),
            "cannot issue more than once per clock cycle"
        );

        IssueLock::new(self, try_read_reg)
    }

    //TODO Document how `flush` is different from `clear` (i.e. flush vs flush all)
    pub fn flush(&mut self) {
        #[cfg(debug_assertions)]
        {
            assert!(
                !self.state.intersects(State::CLEARED),
                "cannot flush and dispatch or issue in the same clock cycle"
            );
            self.state.insert(State::CLEARED);
        }

        self.entries.retain(|uop| !uop.rob_entry.is_flushed());
    }
}

impl<U> Sequential for Rs<U> {
    fn finish_cycle(&mut self) {
        debug_assert!(self.entries.len() <= self.size.get());
        self.len.write(self.entries.len());
        self.len.finish_cycle();

        #[cfg(debug_assertions)]
        {
            use bitflags::Flags;
            self.state.clear();
        }
    }
}

impl<U> Clearable for Rs<U> {
    fn clear(&mut self) {
        #[cfg(debug_assertions)]
        {
            assert!(
                !self.state.intersects(State::CLEARED),
                "cannot flush and dispatch or issue in the same clock cycle"
            );
            self.state.insert(State::CLEARED);
        }

        self.entries.clear();
    }
}

#[cfg(debug_assertions)]
bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct State: u8 {
        const DISPATCHED = 1 << 0;
        const ISSUED = 1 << 1;
        const CLEARED = 1 << 2 | Self::DISPATCHED.bits() | Self::ISSUED.bits();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    pub size: NonZeroUsize,
}

#[derive(Debug, Clone)]
pub struct Stats<U>(VecDeque<IUop<U>>);

impl<U> Stats<U> {
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &IUop<U>> + DoubleEndedIterator {
        self.0.iter()
    }
}

impl<U: Clone> Rs<U> {
    pub fn stats(&self) -> Stats<U> {
        #[cfg(debug_assertions)]
        assert!(
            self.state.is_empty(),
            "cannot create statistics in the middle of a clock cycle"
        );

        Stats(self.entries.clone())
    }
}
