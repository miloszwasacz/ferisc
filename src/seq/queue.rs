//TODO Better docs
//TODO Add notes about the debug assertions

pub use self::pop::Pop;
pub use self::push::Push;
use super::{Clearable, Outcome, Sequential, SimpleSubStage, Substage};
use derive_where::derive_where;
use std::collections::VecDeque;
use std::num::NonZeroUsize;

/// An in-order queue.
#[derive(Debug, Clone)]
pub struct Queue<T> {
    buf: VecDeque<T>,
    size: NonZeroUsize,
    state: State<T>,
}

#[derive(Debug, Clone)]
#[derive_where(Default)]
struct State<T> {
    pushed: VecDeque<T>,
    popped: usize,
    cleared: bool,
    drained: Option<NonZeroUsize>,
}

impl<T> Queue<T> {
    /// Creates a new in-order queue with the given `size`.
    pub fn new(size: NonZeroUsize) -> Self {
        Self {
            buf: VecDeque::with_capacity(size.get()),
            size,
            state: Default::default(),
        }
    }

    /// Returns the maximal capacity of the queue.
    pub fn size(&self) -> NonZeroUsize {
        self.size
    }

    /// Returns an iterator over all elements in the queue.
    ///
    /// Note that this does not include any new [pushed](Self::push) elements
    /// and still includes all [popped](Self::pop) elements.
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &T> + ExactSizeIterator {
        self.buf.iter()
    }

    /// Returns a [`Substage`] used for popping elements from the queue.
    ///
    /// The elements are popped in the same order of as they were previously
    /// [pushed](Self::push). This is useful when chaining in-order pipeline stages,
    /// such as decoding or register renaming, with queues between them acting
    /// as pipeline registers.
    ///
    /// Note that the elements are only dropped at the _END_ of the current
    /// [clock cycle](Sequential::finish_cycle).
    pub fn pop(&mut self, width: NonZeroUsize) -> Pop<'_, T> {
        Pop::new(self, width)
    }

    /// Returns a [`Substage`] used for pushing new elements into the queue.
    ///
    /// The elements are pushed in the order of their arrival. It is up to the
    /// user of this substage to ensure that the incoming elements are in the correct
    /// order[^1].
    ///
    /// Note that the elements are only pushed at the _END_ of the current
    /// [clock cycle](Sequential::finish_cycle).
    ///
    /// [^1]: [Popping](Self::pop) from another in-order queue can guarantee this.
    pub fn push(&mut self) -> Push<'_, T> {
        Push::new(self)
    }

    /// Schedules the last `count` elements to be removed, returning an iterator
    /// over them.
    ///
    /// Note that this iterator goes backwards, i.e. the first element yielded
    /// is the _LAST_ element of the queue, and that the elements are only dropped
    /// at the _END_ of the current [clock cycle](Sequential::finish_cycle).
    pub fn drain_back(&mut self, count: usize) -> impl ExactSizeIterator<Item = &T> {
        debug_assert!(
            self.state.drained.is_none(),
            "cannot drain more than once in a clock cycle"
        );
        debug_assert!(
            self.state.popped <= self.buf.len().saturating_sub(count),
            "some elements are both popped and drained"
        );

        self.state.drained = NonZeroUsize::new(count.min(self.buf.len()));
        self.buf.iter().rev().take(count)
    }
}

impl<T: Sequential> Queue<T> {
    /// Marks the end of the clock cycle, finalizing any outstanding operations
    /// on both the queue and the contained elements.
    pub fn finish_cycle(&mut self) {
        Sequential::finish_cycle(self);
        self.buf.iter_mut().for_each(Sequential::finish_cycle);
    }
}

impl<T> Sequential for Queue<T> {
    /// Marks the end of the clock cycle, finalizing any outstanding operations.
    ///
    /// <div class="warning">
    ///
    /// This method updates the state of _ONLY_ the queue, not the contained elements.
    /// If `T` also implements [`Sequential`], then the [specialized version](Queue::finish_cycle)
    /// should be used instead of this one.
    ///
    /// </div>
    fn finish_cycle(&mut self) {
        use std::mem::take;

        // Drain back
        if let Some(drained) = self.state.drained.take() {
            self.buf.truncate(self.buf.len() - drained.get());
        }

        // Pop
        {
            let popped = take(&mut self.state.popped);
            drop(self.buf.drain(..popped));
        }

        // Push
        {
            self.buf.append(&mut self.state.pushed);
        }

        // Clear
        if take(&mut self.state.cleared) {
            self.buf.clear();
        }
    }
}

impl<T> Clearable for Queue<T> {
    fn clear(&mut self) {
        self.state.cleared = true;
    }
}

mod pop {
    use super::*;
    use std::collections::vec_deque::Iter;
    use std::iter::Take;

    pub struct Pop<'a, T> {
        items: Take<Iter<'a, T>>,
        popped: &'a mut usize,
        #[cfg(debug_assertions)]
        max: usize,
    }

    impl<'a, T> Pop<'a, T> {
        pub(super) fn new(queue: &'a mut Queue<T>, width: NonZeroUsize) -> Self {
            debug_assert_eq!(
                queue.state.popped, 0,
                "cannot pop more than once in a clock cycle"
            );

            Self {
                items: queue.buf.iter().take(width.get()),
                popped: &mut queue.state.popped,
                #[cfg(debug_assertions)]
                max: queue.buf.len() - queue.state.drained.map_or(0, NonZeroUsize::get),
            }
        }
    }

    impl<'a, T> SimpleSubStage for Pop<'a, T> {
        type In = ();
        type Out = &'a T;

        fn start(&mut self, (): Self::In) -> Option<Self::Out> {
            self.items.next()
        }

        fn finish(&mut self) {
            *self.popped += 1;
        }
    }

    #[cfg(debug_assertions)]
    impl<T> Drop for Pop<'_, T> {
        fn drop(&mut self) {
            debug_assert!(
                *self.popped <= self.max,
                "some elements are both popped and drained"
            );
        }
    }
}

mod push {
    use super::*;
    use std::convert::Infallible;

    pub struct Push<'a, T> {
        new: &'a mut VecDeque<T>,
        max_size: usize,
    }

    impl<'a, T> Push<'a, T> {
        pub(super) fn new(queue: &'a mut Queue<T>) -> Self {
            debug_assert!(
                queue.state.pushed.is_empty(),
                "cannot push more than once in a clock cycle"
            );

            Self {
                new: &mut queue.state.pushed,
                max_size: queue.size.get() - queue.buf.len() + queue.state.popped,
            }
        }
    }

    impl<'a, T> Substage for Push<'a, T> {
        type In = T;
        type Out = ();
        type Extra = T;
        type Err = Infallible;

        fn start(&mut self, input: Self::In) -> Outcome<Self::Out, Self::Extra, Self::Err> {
            if self.new.len() < self.max_size {
                Outcome::Ok((), input)
            } else {
                Outcome::Stop
            }
        }

        fn finish(&mut self, extra: Self::Extra) {
            self.new.push_back(extra);
            debug_assert!(self.new.len() <= self.max_size);
        }
    }
}
