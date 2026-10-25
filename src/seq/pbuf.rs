#[cfg(debug_assertions)]
use self::debug::*;
use super::{Buf, Clearable, Sequential, Stallable};

/// A clock-based priority [buffer](Buf).
///
/// Unlike [`Buf`], which accepts at most one value to be written per clock cycle,
/// `Pbuf` allows multiple writes with different priorities. The value with the
/// highest priority wins.
///
/// # Examples
///
/// ```
/// # use ferisc::seq::{Pbuf, Clearable, Sequential, Stallable};
/// // We can construct a buffer with an initial value
/// let mut pbuf = Pbuf::new(0u32);
///
/// // We can read the buffer
/// assert_eq!(*pbuf.read(), 0);
///
/// // Multiple writes with different priorities are allowed.
/// // The highest priority value wins.
/// pbuf.write(10, 1u8);
/// pbuf.write(20, 2u8);
/// pbuf.write(5, 0u8);
/// pbuf.finish_cycle();
/// assert_eq!(*pbuf.read(), 20);
///
/// // We can stall the buffer to suppress any writes.
/// pbuf.write(30, 3u8);
/// pbuf.stall();
/// pbuf.finish_cycle();
/// assert_eq!(*pbuf.read(), 20);
///
/// // We can clear the buffer
/// pbuf.clear();
/// pbuf.finish_cycle();
/// assert_eq!(*pbuf.read(), Default::default());
/// ```
#[derive(Debug)]
pub struct Pbuf<T, P = u8> {
    buf: Buf<T>,
    new: Option<(T, P)>,
    #[cfg(debug_assertions)]
    debug: Debug<P>,
}

impl<T, P> Pbuf<T, P> {
    /// Creates a new priority buffer with the specified initial value.
    pub fn new(value: T) -> Self {
        Self {
            buf: Buf::new(value),
            new: None,
            #[cfg(debug_assertions)]
            debug: Default::default(),
        }
    }

    /// Returns a reference to the data currently stored in the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ferisc::seq::Pbuf;
    /// let pbuf: Pbuf<u32> = Pbuf::new(42);
    /// assert_eq!(*pbuf.read(), 42);
    /// ```
    pub fn read(&self) -> &T {
        self.buf.read()
    }

    /// Returns a copy of the data currently stored in the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ferisc::seq::Pbuf;
    /// let pbuf: Pbuf<u32> = Pbuf::new(42);
    /// assert_eq!(pbuf.read_copied(), 42);
    /// ```
    pub fn read_copied(&self) -> T
    where
        T: Copy,
    {
        self.buf.read_copied()
    }

    /// Returns a reference to the data currently stored in the buffer,
    /// bypassing any data [written](Self::write) in this clock cycle.
    ///
    /// If at least one value has been written in the current cycle, this returns
    /// the highest-priority pending value. Otherwise, it returns the current
    /// buffered value.
    ///
    /// Once a value has been bypassed, only higher-priority values can be
    /// written in the same cycle. This is useful when `read_bypassed` is
    /// called inside a loop that may issue additional writes.
    ///
    /// # Imperativeness
    ///
    /// This method is inherently imperative - in order for a value to be bypassed,
    /// it has to be [written](Self::write) first. In debug builds, this is enforced
    /// by panicking if a non-higher-priority value is written after
    /// this method has been called.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ferisc::seq::{Pbuf, Sequential};
    /// let mut pbuf = Pbuf::new(0u32);
    /// pbuf.write(10, 1u8);
    ///
    /// // Regular read yields the old data
    /// assert_eq!(*pbuf.read(), 0);
    ///
    /// // Bypassing yields the new data
    /// assert_eq!(*pbuf.read_bypassed(), 10);
    ///
    /// // Writing a new value with higher priority is allowed
    /// // and can also be bypassed
    /// pbuf.write(20, 2u8);
    /// assert_eq!(*pbuf.read_bypassed(), 20);
    /// ```
    pub fn read_bypassed(&self) -> &T {
        #[cfg(debug_assertions)]
        self.debug.mark_bypassed();

        self.new
            .as_ref()
            .map(|(s, _)| s)
            .unwrap_or_else(|| self.read())
    }

    /// Schedules a new value to be written into the buffer at the end
    /// of the current clock cycle, using the given priority.
    ///
    /// If no value has been written yet, or if the new priority is higher
    /// than the previously written priority, the new value replaces the
    /// pending value. Otherwise, the new value is silently discarded.
    ///
    /// <div class="warning">
    ///
    /// Writing multiple values with the same priority is not allowed and
    /// will panic in debug builds.
    ///
    /// </div>
    ///
    /// # Examples
    ///
    /// ```
    /// # use ferisc::seq::{Pbuf, Sequential};
    /// let mut pbuf = Pbuf::new(0u32);
    ///
    /// // Lower priority write
    /// pbuf.write(10, 1u8);
    ///
    /// // Higher priority write replaces it
    /// pbuf.write(20, 2u8);
    ///
    /// // Lower priority write is discarded
    /// pbuf.write(5, 0u8);
    ///
    /// pbuf.finish_cycle();
    /// assert_eq!(*pbuf.read(), 20);
    /// ```
    pub fn write(&mut self, value: T, priority: P)
    where
        P: Ord,
    {
        #[cfg(debug_assertions)]
        assert!(
            self.is_new_priority(&priority),
            "cannot write multiple values with the same priority"
        );

        if self.new.as_ref().is_none_or(|(_, p)| *p < priority) {
            let _old = self.new.replace((value, priority));
            #[cfg(debug_assertions)]
            if let Some((_, p)) = _old {
                self.debug.push_old_priority(p);
            }
        } else {
            #[cfg(debug_assertions)]
            assert!(
                !self.debug.bypassed(),
                "the buffer has already been bypassed"
            );
        }
    }
}

impl<T: Clone, P> Clone for Pbuf<T, P> {
    /// Creates a duplicate of the priority buffer with the currently stored data.
    ///
    /// Note that the new instance only has the same current value, not any new
    /// data scheduled with [`Self::write`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use ferisc::seq::{Pbuf, Sequential};
    /// let mut b1 = Pbuf::new(0u32);
    /// b1.write(10, 1u8);
    /// let mut b2 = b1.clone();
    ///
    /// // b2 has the same value as b1
    /// assert_eq!(b1.read_copied(), b2.read_copied());
    ///
    /// // Only the current data is duplicated
    /// b1.finish_cycle();
    /// b2.finish_cycle();
    /// assert_eq!(b1.read_copied(), 10);
    /// assert_eq!(b2.read_copied(), 0);
    /// ```
    fn clone(&self) -> Self {
        Self {
            buf: self.buf.clone(),
            new: None,
            #[cfg(debug_assertions)]
            debug: Default::default(),
        }
    }
}

impl<T: Default, P> Default for Pbuf<T, P> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T, P> Sequential for Pbuf<T, P> {
    fn finish_cycle(&mut self) {
        #[cfg(debug_assertions)]
        self.debug.finish_cycle();

        if let Some((s, _)) = self.new.take() {
            self.buf.write(s);
        }
        self.buf.finish_cycle();
    }
}

impl<T, P> Stallable for Pbuf<T, P> {
    /// Disables all writes that are performed in the current clock cycle.
    ///
    /// If the buffer is both stalled and cleared in the same cycle,
    /// clearing takes precedence.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ferisc::seq::{Pbuf, Clearable, Sequential, Stallable};
    /// let mut pbuf = Pbuf::new(0u32);
    ///
    /// // Stalling disables any writes that occur both before the call...
    /// pbuf.write(10, 1u8);
    /// pbuf.stall();
    /// pbuf.finish_cycle();
    /// assert_eq!(*pbuf.read(), 0);
    ///
    /// // ...and after the call
    /// pbuf.stall();
    /// pbuf.write(20, 2u8);
    /// pbuf.finish_cycle();
    /// assert_eq!(*pbuf.read(), 0);
    ///
    /// // Clearing takes precedence over stalling
    /// pbuf.stall();
    /// pbuf.clear();
    /// pbuf.finish_cycle();
    /// assert_eq!(*pbuf.read(), Default::default());
    /// ```
    fn stall(&mut self) {
        self.buf.stall();
    }
}

impl<T: Default, P> Clearable for Pbuf<T, P> {
    /// Clears the buffer at the end of the current clock cycle,
    /// setting it to the default value of the held type.
    ///
    /// Clearing takes precedence over [stalling](Pbuf::stall).
    ///
    /// # Examples
    ///
    /// ```
    /// # use ferisc::seq::{Pbuf, Clearable, Sequential, Stallable};
    /// // Clearing only takes effect at the end of the clock cycle
    /// let mut pbuf = Pbuf::new(0u32);
    /// pbuf.write(10, 1u8);
    /// pbuf.clear();
    /// assert_eq!(*pbuf.read(), 0);
    /// pbuf.finish_cycle();
    /// assert_eq!(*pbuf.read(), Default::default());
    ///
    /// // Clearing takes precedence over stalling
    /// let mut pbuf: Pbuf<u32> = Pbuf::new(0);
    /// pbuf.stall();
    /// pbuf.clear();
    /// pbuf.finish_cycle();
    /// assert_eq!(*pbuf.read(), Default::default());
    /// ```
    fn clear(&mut self) {
        self.buf.clear()
    }
}

#[cfg(debug_assertions)]
mod debug {
    use super::*;
    use educe::Educe;
    use std::cell::Cell;

    /// Internal debug state for [`Pbuf`] that enforces write constraints in debug builds.
    ///
    /// Tracks which priorities have been used in the current clock cycle and whether
    /// [`read_bypassed`](Pbuf::read_bypassed) has been called, so that invalid writes
    /// (duplicate priorities or writes after bypass) can be caught early.
    #[derive(Debug, Educe)]
    #[educe(Default)]
    pub struct Debug<P> {
        /// Priority history
        ///
        /// Records all priorities written in the current clock cycle
        /// to detect duplicate priority writes.
        phist: Vec<P>,
        /// Flag set when [`read_bypassed`](Pbuf::read_bypassed) is called, to prevent
        /// lower-priority writes afterward.
        bypassed: Cell<bool>,
    }

    impl<P> Debug<P> {
        /// Returns whether [`read_bypassed`](Pbuf::read_bypassed) has been called
        /// in the current clock cycle.
        pub fn bypassed(&self) -> bool {
            self.bypassed.get()
        }

        /// Marks that [`read_bypassed`](Pbuf::read_bypassed) has been called in the
        /// current clock cycle, preventing lower-priority writes afterward.
        pub fn mark_bypassed(&self) {
            self.bypassed.set(true);
        }

        /// Records a previously pending priority just replaced by a higher-priority
        /// write, so it can be checked against future writes for duplicates.
        pub fn push_old_priority(&mut self, priority: P) {
            self.phist.push(priority);
        }
    }

    impl<T, P> Pbuf<T, P> {
        /// Returns whether the given priority has not been used in the current clock cycle,
        /// either as the current pending write or in the priority history.
        pub(super) fn is_new_priority(&self, priority: &P) -> bool
        where
            P: PartialEq,
        {
            let is_current_p = |p: &P| *p == *priority;
            let is_old_p = || self.debug.phist.contains(priority);
            self.new
                .as_ref()
                .is_none_or(|(_, p)| !is_current_p(p) && !is_old_p())
        }
    }

    impl<P> Sequential for Debug<P> {
        fn finish_cycle(&mut self) {
            self.phist.clear();
            self.bypassed.take();
        }
    }
}
