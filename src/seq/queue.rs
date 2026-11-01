pub use self::pop::Pop;
pub use self::push::Push;
use super::{Clearable, Outcome, Sequential, SimpleSubStage, Substage};
use educe::Educe;
use std::collections::VecDeque;
use std::num::NonZeroUsize;

/// A [`Sequential`] in-order queue.
///
/// Elements are pushed and popped in FIFO order. Pushes and pops take effect
/// at the end of the current clock cycle, making the queue suitable as a
/// pipeline register between in-order pipeline stages.
///
/// # Examples
///
/// ```
/// # use ferisc::seq::{Queue, Clearable, Sequential, Substage, IntoPipelineStage};
/// use std::num::NonZeroUsize;
/// # mod __private {
/// #     use ferisc::seq::{Queue, Sequential, SimpleSubStage, IntoPipelineStage};
/// #     include!("queue/test_helpers.rs");
/// # }
/// # use __private::*;
///
/// // We can create a queue with capacity 4
/// let mut queue = Queue::new(NonZeroUsize::new(4).unwrap());
///
/// // We can push elements into the queue from a producer
/// Producer([1, 2].into_iter())
///     // earlier substages...
/// #    .into_pipeline_stage()
///     .substage(queue.push())
///     .execute();
///
/// // Elements are not visible until the end of the cycle
/// assert_eq!(queue.iter().len(), 0);
///
/// queue.finish_cycle();
/// assert_eq!(queue.iter().copied().collect::<Vec<_>>(), vec![1, 2]);
///
/// // We can pop elements in FIFO order
///  let mut queue2 = Queue::new(NonZeroUsize::new(4).unwrap());
///  queue
///     .pop(NonZeroUsize::new(1).unwrap())
///     .into_pipeline_stage()
///     // yields references to the popped elements, which can be cloned or copied
///     .copied()
///     .substage(queue2.push())
///     .execute();
///
/// queue.finish_cycle();
/// queue2.finish_cycle();
/// assert_eq!(queue.iter().copied().collect::<Vec<_>>(), vec![2]);
/// assert_eq!(queue2.iter().copied().collect::<Vec<_>>(), vec![1]);
/// ```
#[derive(Debug, Clone)]
pub struct Queue<T> {
    buf: VecDeque<T>,
    size: NonZeroUsize,
    state: State<T>,
}

/// Internal state tracking pending operations for the current clock cycle.
#[derive(Debug, Clone, Educe)]
#[educe(Default)]
struct State<T> {
    /// Elements scheduled to be pushed at the end of the current cycle.
    pushed: VecDeque<T>,
    /// Number of elements scheduled to be popped at the end of the current cycle.
    popped: usize,
    /// Whether the queue is scheduled to be cleared at the end of the current cycle.
    cleared: bool,
    /// Number of elements scheduled to be drained from the back at the end of the current cycle.
    drained: Option<NonZeroUsize>,
}

impl<T> Queue<T> {
    /// Creates a new queue with the given maximum `size`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ferisc::seq::Queue;
    /// use std::num::NonZeroUsize;
    ///
    /// let queue = Queue::<u32>::new(NonZeroUsize::new(4).unwrap());
    /// assert_eq!(queue.size(), NonZeroUsize::new(4).unwrap());
    /// ```
    pub fn new(size: NonZeroUsize) -> Self {
        Self {
            buf: VecDeque::with_capacity(size.get()),
            size,
            state: Default::default(),
        }
    }

    /// Returns the maximal capacity of the queue.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ferisc::seq::Queue;
    /// use std::num::NonZeroUsize;
    ///
    /// let queue = Queue::<u32>::new(NonZeroUsize::new(4).unwrap());
    /// assert_eq!(queue.size(), NonZeroUsize::new(4).unwrap());
    /// ```
    pub fn size(&self) -> NonZeroUsize {
        self.size
    }

    /// Returns an iterator over all elements currently stored in the queue.
    ///
    /// Note that this does not include any new [pushed](Self::push) elements
    /// and still includes all [popped](Self::pop) elements.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ferisc::seq::{Queue, Sequential, Substage, IntoPipelineStage};
    /// use std::num::NonZeroUsize;
    /// # mod __private {
    /// #     use ferisc::seq::{Queue, Sequential, SimpleSubStage, IntoPipelineStage};
    /// #     include!("queue/test_helpers.rs");
    /// # }
    /// # use __private::*;
    ///
    /// let mut queue = Queue::new(NonZeroUsize::new(4).unwrap());
    /// Producer([10, 20].into_iter())
    ///     // earlier substages...
    /// #    .into_pipeline_stage()
    ///     .substage(queue.push())
    ///     .execute();
    ///
    /// // Pushed elements are not visible yet
    /// assert_eq!(queue.iter().len(), 0);
    ///
    /// queue.finish_cycle();
    /// assert_eq!(queue.iter().copied().collect::<Vec<_>>(), vec![10, 20]);
    /// ```
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &T> + ExactSizeIterator {
        self.buf.iter()
    }

    /// Returns a [`Pop`] substage for popping elements from the front of the queue.
    ///
    /// Elements are popped in FIFO order, matching the order they were
    /// [pushed](Self::push). This is useful when chaining in-order pipeline stages,
    /// such as decoding or register renaming, with queues between them acting
    /// as pipeline registers.
    ///
    /// The `width` parameter limits how many elements can be popped in a single
    /// clock cycle.
    ///
    /// Note that elements are only removed at the _END_ of the current
    /// [clock cycle](Sequential::finish_cycle).
    ///
    /// <div class="warning">
    ///
    /// This method should only be called once per clock cycle, but for
    /// performance reasons, this is only enforced when compiling with
    /// `debug_assertions` enabled. Make sure all call sites conform to this
    /// requirement.
    ///
    /// </div>
    ///
    /// # Examples
    ///
    /// ```
    /// # use ferisc::seq::{Queue, Sequential, Substage, IntoPipelineStage};
    /// use std::num::NonZeroUsize;
    /// # mod __private {
    /// #     use ferisc::seq::{Queue, Sequential, SimpleSubStage, IntoPipelineStage};
    /// #     include!("queue/test_helpers.rs");
    /// # }
    /// # use __private::*;
    ///
    /// let mut queue1 = Queue::new(NonZeroUsize::new(4).unwrap());
    /// // Initialize the queue with [10, 20]...
    /// # init_queue_with(&mut queue1, [10, 20]);
    ///
    /// // Pop up to 1 element
    /// let width = NonZeroUsize::new(1).unwrap();
    /// let mut queue2 = Queue::new(NonZeroUsize::new(4).unwrap());
    /// queue1
    ///    .pop(width)
    ///    .into_pipeline_stage()
    ///    .copied()
    ///    .substage(queue2.push())
    ///    .execute();
    ///
    /// queue1.finish_cycle();
    /// queue2.finish_cycle();
    /// assert_eq!(queue1.iter().copied().collect::<Vec<_>>(), vec![20]);
    /// assert_eq!(queue2.iter().copied().collect::<Vec<_>>(), vec![10]);
    /// ```
    pub fn pop(&mut self, width: NonZeroUsize) -> Pop<'_, T> {
        Pop::new(self, width)
    }

    /// Returns a [`Push`] substage for pushing new elements into the back of the queue.
    ///
    /// Elements are pushed in the order of their arrival. It is up to the
    /// user of this substage to ensure that the incoming elements are in the correct
    /// order[^1].
    ///
    /// The push substage will automatically stop accepting new elements when the
    /// queue reaches its capacity, returning [`Outcome::Stop`].
    ///
    /// Note that elements are only committed at the _END_ of the current
    /// [clock cycle](Sequential::finish_cycle).
    ///
    /// <div class="warning">
    ///
    /// This method should only be called once per clock cycle, but for
    /// performance reasons, this is only enforced when compiling with
    /// `debug_assertions` enabled. Make sure all call sites conform to this
    /// requirement.
    ///
    /// </div>
    ///
    /// [^1]: [Popping](Self::pop) from another in-order queue can guarantee this.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ferisc::seq::{Queue, Sequential, Substage, IntoPipelineStage};
    /// use std::num::NonZeroUsize;
    /// # mod __private {
    /// #     use ferisc::seq::{Queue, Sequential, SimpleSubStage, IntoPipelineStage};
    /// #     include!("queue/test_helpers.rs");
    /// # }
    /// # use __private::*;
    ///
    /// let size = NonZeroUsize::new(2).unwrap();
    /// let mut queue = Queue::new(size);
    /// Producer([10, 20, 30].into_iter())
    ///     // earlier substages...
    /// #    .into_pipeline_stage()
    ///     .substage(queue.push())
    ///     .execute();
    /// queue.finish_cycle();
    ///
    /// // Only up to `size` elements are pushed.
    /// assert_eq!(queue.iter().copied().collect::<Vec<_>>(), vec![10, 20]);
    /// ```
    pub fn push(&mut self) -> Push<'_, T> {
        Push::new(self)
    }

    /// Schedules the last `count` elements to be removed from the back of the queue,
    /// returning an iterator over them.
    ///
    /// The iterator yields elements in reverse order: the first element yielded
    /// is the _LAST_ element of the queue.
    ///
    /// Elements are only removed at the _END_ of the current
    /// [clock cycle](Sequential::finish_cycle).
    ///
    /// <div class="warning">
    ///
    /// This method should only be called once per clock cycle, but for
    /// performance reasons, this is only enforced when compiling with
    /// `debug_assertions` enabled. Make sure all call sites conform to this
    /// requirement.
    ///
    /// Additionally, the same element must not be both drained and popped
    ///
    /// </div>
    ///
    /// # Examples
    ///
    /// ```
    /// # use ferisc::seq::{Queue, Sequential, Substage, IntoPipelineStage};
    /// use std::num::NonZeroUsize;
    /// # mod __private {
    /// #     use ferisc::seq::{Queue, Sequential, SimpleSubStage, IntoPipelineStage};
    /// #     include!("queue/test_helpers.rs");
    /// # }
    /// # use __private::*;
    ///
    /// let mut queue = Queue::new(NonZeroUsize::new(4).unwrap());
    /// // Initialize the queue with [10, 20, 30]...
    /// # init_queue_with(&mut queue, [10, 20, 30]);
    ///
    /// // Drain the last 2 elements
    /// let drained: Vec<_> = queue.drain_back(2).copied().collect();
    /// assert_eq!(drained, vec![30, 20]);
    ///
    /// queue.finish_cycle();
    /// assert_eq!(queue.iter().copied().collect::<Vec<_>>(), vec![10]);
    /// ```
    pub fn drain_back(&mut self, count: usize) -> impl ExactSizeIterator<Item = &T> {
        debug_assert!(
            self.state.drained.is_none(),
            "cannot drain more than once in a clock cycle"
        );
        debug_assert!(
            self.state.popped <= self.buf.len().saturating_sub(count),
            "some elements are both popped and drained"
        );

        let drained = self.buf.iter().rev().take(count);
        self.state.drained = NonZeroUsize::new(drained.len());
        drained
    }
}

//TODO Replace with specialization once it gets stabilized
impl<T: Sequential> Queue<T> {
    /// Marks the end of the clock cycle, finalizing any outstanding operations
    /// on both the queue and the contained elements.
    ///
    /// This method first finalizes the queue itself, then calls
    /// [`Sequential::finish_cycle`] on each contained element. Use this version
    /// when `T` implements [`Sequential`]; otherwise, use the trait method directly.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ferisc::seq::{Buf, Queue, Sequential, Substage, IntoPipelineStage};
    /// use std::num::NonZeroUsize;
    /// # mod __private {
    /// #     use ferisc::seq::{Queue, Sequential, SimpleSubStage, IntoPipelineStage};
    /// #     include!("queue/test_helpers.rs");
    /// # }
    /// # use __private::*;
    ///
    /// let mut buf = Buf::new(10);
    /// buf.write(20);
    ///
    /// let mut queue = Queue::new(NonZeroUsize::new(2).unwrap());
    /// Producer([buf].into_iter())
    ///     // earlier substages...
    /// #    .into_pipeline_stage()
    ///     .substage(queue.push())
    ///     .execute();
    ///
    /// queue.finish_cycle();
    /// assert_eq!(queue.iter().map(Buf::read_copied).collect::<Vec<_>>(), vec![20]);
    /// ```
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
    /// Clears the queue at the end of the current clock cycle,
    /// removing all elements.
    ///
    /// Clearing takes precedence over any pending pushes, pops, or drains.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ferisc::seq::{Queue, Clearable, Sequential, Substage, IntoPipelineStage};
    /// use std::num::NonZeroUsize;
    /// # mod __private {
    /// #     use ferisc::seq::{Queue, Sequential, SimpleSubStage, IntoPipelineStage};
    /// #     include!("queue/test_helpers.rs");
    /// # }
    /// # use __private::*;
    ///
    /// let mut queue = Queue::new(NonZeroUsize::new(4).unwrap());
    /// // Initialize the queue with [10, 20]...
    /// # init_queue_with(&mut queue, [10, 20]);
    ///
    /// queue.clear();
    /// queue.finish_cycle();
    /// assert_eq!(queue.iter().len(), 0);
    /// ```
    fn clear(&mut self) {
        self.state.cleared = true;
    }
}

mod pop {
    use super::*;
    use std::collections::vec_deque::Iter;
    use std::iter::Take;

    /// A pipeline [`Substage`] for popping items from a [`Queue`].
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

    /// A pipeline [`Substage`] for pushing items into a [`Queue`].
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
