use self::substage::{Dispatch, Issue};
pub use self::uop::Uop;
use crate::core::{ee::rob, uop::issue::TryReadReg};
use crate::seq::{Clearable, Sequential};
use derive_where::derive_where;
use std::collections::VecDeque;
use std::debug_assert_matches;
use std::num::NonZeroUsize;
use std::ops::Deref;

pub mod substage;
mod uop;

#[derive(Debug, Clone)]
pub struct Rs<U> {
    entries: VecDeque<RsEntry<U>>,
    size: NonZeroUsize,
    state: State<U>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, derive_more::Deref, derive_more::DerefMut)]
pub struct RsEntry<U> {
    #[deref]
    #[deref_mut]
    pub uop: U,
    pub flushed: bool,
}

#[derive(Debug, Clone)]
#[derive_where(Default)]
struct State<U> {
    pushed: VecDeque<RsEntry<U>>,
    removed: Option<usize>,
    flushed: Flush,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
enum Flush {
    #[default]
    None,
    Some,
    All,
}

impl<U: Uop> Rs<U> {
    pub fn new(config: &Config) -> Self {
        Self {
            entries: VecDeque::with_capacity(config.size.get()),
            size: config.size,
            state: Default::default(),
        }
    }

    pub fn dispatch(&mut self) -> Dispatch<'_, U> {
        Dispatch::new(self)
    }

    pub fn issue<R: TryReadReg<U::Reg>>(&mut self, read: R) -> Issue<'_, U, R> {
        Issue::new(self, read)
    }

    //TODO Document how `flush` is different from `clear` (i.e. flush vs flush all)
    pub fn flush(&mut self, flush: &rob::lock::Flush<'_>) {
        let prev = std::mem::replace(&mut self.state.flushed, Flush::Some);
        debug_assert_matches!(
            prev,
            Flush::None,
            "cannot flush more than once per clock cycle"
        );
        for entry in &mut self.entries {
            entry.flushed = entry.flushed || entry.is_flushed(flush);
        }
    }
}

impl<U: Uop> Sequential for Rs<U> {
    /// Marks the end of the clock cycle, finalizing any outstanding operations.
    ///
    /// <div class="warning">
    ///
    /// This method updates the state of _ONLY_ the reservation station, not
    /// the contained elements.
    ///
    /// </div>
    fn finish_cycle(&mut self) {
        // Remove
        if let Some(removed) = self.state.removed.take() {
            self.entries.remove(removed);
        }

        // Push
        {
            self.entries.append(&mut self.state.pushed);
        }

        // Flush
        match std::mem::take(&mut self.state.flushed) {
            Flush::None => {}
            Flush::Some => self.entries.retain(|entry| !entry.flushed),
            Flush::All => self.entries.clear(),
        }
    }
}

impl<U: Uop> Clearable for Rs<U> {
    fn clear(&mut self) {
        self.state.flushed = Flush::All;
    }
}

impl<U> From<U> for RsEntry<U> {
    fn from(value: U) -> Self {
        RsEntry {
            uop: value,
            flushed: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    pub size: NonZeroUsize,
}

#[derive(Debug, Clone)]
pub struct Stats<U>(VecDeque<RsEntry<U>>);

impl<U> Stats<U> {
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &U> + ExactSizeIterator {
        self.0.iter().map(Deref::deref)
    }
}

impl<U: Clone> Rs<U> {
    pub fn stats(&self) -> Stats<U> {
        Stats(self.entries.clone())
    }
}
