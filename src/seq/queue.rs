//TODO Better docs

pub use self::locks::{PopLock, PopTrans, PushLock, PushTrans};
use super::{Clearable, Sequential};
use std::collections::vec_deque::{self, VecDeque};

mod locks;

/// An in-order queue.
#[derive(Debug, Clone)]
pub struct Queue<T> {
    buf: VecDeque<T>,
    len: usize,
    size: usize,
    dealloc: usize,
    #[cfg(debug_assertions)]
    state: State,
}

impl<T> Queue<T> {
    /// Creates a new in-order queue with the given `size`.
    pub fn new(size: usize) -> Self {
        Self {
            buf: VecDeque::with_capacity(size),
            len: 0,
            size,
            dealloc: 0,
            #[cfg(debug_assertions)]
            state: State::empty(),
        }
    }

    /// Returns the current number of elements held in the queue.
    ///
    /// Note that this does not include any new elements that have been[pushed](Self::push),
    /// and still includes all elements that have been [popped](Self::pop).
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns the queue is currently empty.
    ///
    /// Note that this does not include any new elements that have been[pushed](Self::push),
    /// and still includes all elements that have been [popped](Self::pop).
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the maximal capacity of the queue.
    pub fn size(&self) -> usize {
        self.size
    }

    /// Returns an iterator of the elements currently stored within the queue.
    ///
    /// It is important to note that new elements that have been [pushed](Self::push)
    /// within a given clock cycle _will not_ be present in this iterator in this
    /// clock cycle - they will only appear in the next cycle (see [`Sequential`]).
    ///
    /// # Panics
    ///
    /// Panics if the queue has been [cleared](Self::clear) or [drained](Self::drain_from)
    /// in this clock cycle.
    ///
    /// <div class="warning">
    ///
    /// Note that this is only enforced when `debug_assertions`
    /// are enabled to increase performance of release builds.
    ///
    /// </div>
    pub fn iter(&self) -> Iter<'_, T> {
        #[cfg(debug_assertions)]
        assert!(
            !self.state.contains(State::CLEARED),
            "cannot iterate over a cleared queue"
        );

        self.buf.iter().take(self.len)
    }

    /// Returns a [`StageLock`] used for pushing new elements into the queue.
    ///
    /// The elements are pushed in the order of their arrival. It is up to the
    /// user of this lock to ensure that the incoming elements are in the correct
    /// order[^1].
    ///
    /// [^1]: [Popping](Self::pop) from another in-order queue can guarantee this.
    pub fn push(&mut self) -> PushLock<'_, T> {
        #[cfg(debug_assertions)]
        assert!(
            !self.state.contains(State::PUSHED),
            "cannot push new elements more than once per clock cycle"
        );
        PushLock::new(self)
    }

    /// Returns a [`StageLock`] used for popping elements from the queue.
    ///
    /// The elements are popped in the same order of as they were previously
    /// [pushed](Self::push). This is useful when chaining in-order pipeline stages,
    /// such as decoding or register renaming, with queues between them acting
    /// as pipeline registers.
    pub fn pop(&mut self, max_count: usize) -> PopLock<'_, T> {
        #[cfg(debug_assertions)]
        assert!(
            !self.state.contains(State::POPPED),
            "cannot remove elements more than once per clock cycle"
        );
        PopLock::new(self, max_count)
    }

    /// [Drains](VecDeque::drain) the queue, starting at `index` and returning
    /// an iterator over the removed elements. The elements are yielded int the
    /// same order as they were in the queue.
    ///
    /// # Panics
    ///
    /// See the _Panics_ section in [`VecDeque::drain`].
    ///
    /// # Leaking
    ///
    /// See the _Leaking_ section in [`VecDeque::drain`].
    pub fn drain_from(&mut self, index: usize) -> impl Iterator<Item = T> {
        #[cfg(debug_assertions)]
        {
            assert!(
                !self.state.intersects(State::CLEARED),
                "cannot drain and push or pop elements in the same clock cycle"
            );
            self.state.insert(State::CLEARED);
        }
        self.buf.drain(index..)
    }
}

impl<T> Sequential for Queue<T> {
    fn finish_cycle(&mut self) {
        drop(self.buf.drain(..self.dealloc));
        self.len = self.buf.len();
        self.dealloc = 0;
        #[cfg(debug_assertions)]
        {
            use bitflags::Flags;
            self.state.clear();
        }
    }
}

impl<T> Clearable for Queue<T> {
    fn clear(&mut self) {
        #[cfg(debug_assertions)]
        {
            assert!(
                !self.state.intersects(State::CLEARED),
                "cannot clear queue and push or pop elements in the same clock cycle"
            );
            self.state.insert(State::CLEARED);
        }
        self.buf.clear();
        debug_assert_eq!(self.dealloc, 0);
    }
}

#[cfg(debug_assertions)]
bitflags::bitflags! {
    #[derive(Debug, Clone, Copy)]
    struct State: u8 {
        const PUSHED = 1 << 0;
        const POPPED = 1 << 1;
        const CLEARED = (1 << 2) | Self::PUSHED.bits() | Self::POPPED.bits();
    }
}

type Iter<'a, T> = std::iter::Take<vec_deque::Iter<'a, T>>;
