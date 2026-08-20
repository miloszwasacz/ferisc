use super::{Clearable, Sequential, Stallable};
use std::mem;

/// A clock-based buffer for holding arbitrary data.
///
/// # Examples
///
/// ```
/// # use ferisc::seq::{Buf, Clearable, Sequential, Stallable};
/// // We can construct a buffer with an initial value
/// let mut buf = Buf::new(1);
///
/// // We can read the buffer
/// assert_eq!(*buf.read(), 1);
///
/// // We can write to the buffer
/// buf.write(2);
/// buf.finish_cycle();
/// assert_eq!(*buf.read(), 2);
///
/// // We can stall the buffer to suppress any writes.
/// buf.write(3);
/// buf.stall();
/// buf.finish_cycle();
/// assert_eq!(*buf.read(), 2);
///
/// // We can clear the buffer
/// buf.clear();
/// buf.finish_cycle();
/// assert_eq!(*buf.read(), Default::default());
/// ```
#[derive(Debug)]
pub struct Buf<T> {
    value: T,
    state: BufState<T>,
    new: Option<T>,
    #[cfg(debug_assertions)]
    bypassed: std::cell::Cell<bool>,
}

#[derive(Debug, Clone, Copy, Default)]
enum BufState<T> {
    /// Default state, the buffer accepts writes.
    #[default]
    Enabled,
    /// The buffer is stalled, i.e. it is disabled.
    Stalled,
    /// The buffer is being cleared.
    Cleared(fn() -> T),
}

impl<T> Buf<T> {
    /// Creates a new buffer with the specified initial value.
    pub fn new(value: T) -> Self {
        Self {
            value,
            state: BufState::default(),
            new: None,
            #[cfg(debug_assertions)]
            bypassed: Default::default(),
        }
    }

    /// Returns a reference to the data currently stored in the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ferisc::seq::Buf;
    /// let buf = Buf::new(1);
    /// assert_eq!(*buf.read(), 1);
    /// ```
    pub fn read(&self) -> &T {
        &self.value
    }

    /// Returns a copy of the data currently stored in the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ferisc::seq::Buf;
    /// let buf = Buf::new(1);
    /// assert_eq!(buf.read_copied(), 1);
    /// ```
    pub fn read_copied(&self) -> T
    where
        T: Copy,
    {
        self.value
    }

    /// Returns a reference to the data currently stored in the buffer,
    /// bypassing any data [written](Self::write) in this clock cycle.
    ///
    /// # Imperativeness
    ///
    /// This method is inherently imperative - in order for a value to be bypassed,
    /// it has to be [written](Self::write) first. In debug builds, this is enforced
    /// by panicking if within a given clock cycle a new value is written after
    /// this method has been called.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ferisc::seq::{Buf, Sequential};
    /// let mut buf = Buf::new(1);
    /// buf.write(2);
    ///
    /// // Regular read yields the old data
    /// assert_eq!(*buf.read(), 1);
    ///
    /// // Bypassing yields the new data
    /// assert_eq!(*buf.read_bypassed(), 2);
    /// ```
    pub fn read_bypassed(&self) -> &T {
        self.new
            .as_ref()
            .inspect(|_| {
                #[cfg(debug_assertions)]
                self.bypassed.set(true);
            })
            .unwrap_or_else(|| self.read())
    }

    /// Schedules a new value to be written into the buffer at the end
    /// of the current clock cycle.
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
    /// # use ferisc::seq::{Buf, Sequential};
    /// let mut buf = Buf::new(1);
    /// buf.write(2);
    ///
    /// // Reading the buffer before the end of
    /// // a clock cycle yields the old data
    /// assert_eq!(*buf.read(), 1);
    ///
    /// buf.finish_cycle();
    /// assert_eq!(*buf.read(), 2);
    /// ```
    pub fn write(&mut self, value: T) {
        debug_assert!(self.new.is_none(), "buffer has already been written to");
        #[cfg(debug_assertions)]
        assert!(!self.bypassed.get(), "the buffer has already been bypassed");

        self.new = Some(value);
    }
}

impl<T: Clone> Clone for Buf<T> {
    /// Creates of duplicate of the buffer with the currently stored data.
    ///
    /// Note that the new instance only has the same current value, not any new
    /// data scheduled with [`Self::write`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use ferisc::seq::{Buf, Sequential};
    /// let mut b1 = Buf::new(1);
    /// b1.write(2);
    /// let mut b2 = b1.clone();
    ///
    /// // b2 has the same value as b1
    /// assert_eq!(b1.read_copied(), b2.read_copied());
    ///
    /// // Only the current data is duplicated
    /// b1.finish_cycle();
    /// b2.finish_cycle();
    /// assert_eq!(b1.read_copied(), 2);
    /// assert_eq!(b2.read_copied(), 1);
    /// ```
    fn clone(&self) -> Self {
        Self::new(self.value.clone())
    }
}

impl<T: Default> Default for Buf<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T> Sequential for Buf<T> {
    fn finish_cycle(&mut self) {
        #[cfg(debug_assertions)]
        self.bypassed.take();

        let new = self.new.take();
        self.value = match mem::take(&mut self.state) {
            BufState::Enabled => match new {
                Some(new) => new,
                None => return,
            },
            BufState::Stalled => return,
            BufState::Cleared(default) => default(),
        };
    }
}

impl<T> Stallable for Buf<T> {
    /// Disables all writes that are performed in the current clock cycle.
    ///
    /// If the buffer is both stalled and cleared in the same cycle,
    /// clearing takes precedence.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ferisc::seq::{Buf, Clearable, Sequential, Stallable};
    /// let mut buf = Buf::new(1);
    ///
    /// // Stalling disables any writes that occur both before the call...
    /// buf.write(2);
    /// buf.stall();
    /// buf.finish_cycle();
    /// assert_eq!(*buf.read(), 1);
    ///
    /// // ...and after the call
    /// buf.stall();
    /// buf.write(3);
    /// buf.finish_cycle();
    /// assert_eq!(*buf.read(), 1);
    ///
    /// // Clearing takes precedence over stalling
    /// buf.stall();
    /// buf.clear();
    /// buf.finish_cycle();
    /// assert_eq!(*buf.read(), Default::default());
    /// ```
    fn stall(&mut self) {
        self.state = match self.state {
            BufState::Enabled | BufState::Stalled => BufState::Stalled,
            BufState::Cleared(default) => BufState::Cleared(default),
        }
    }
}

impl<T: Default> Clearable for Buf<T> {
    /// Clears the buffer at the end of the current clock cycle,
    /// setting it to the default value of the held type.
    ///
    /// Clearing takes precedence over [stalling](Buf::stall).
    ///
    /// # Examples
    ///
    /// ```
    /// # use ferisc::seq::{Buf, Clearable, Sequential, Stallable};
    /// // Clearing only takes effect at the end of the clock cycle
    /// let mut buf = Buf::new(1);
    /// buf.clear();
    /// assert_eq!(*buf.read(), 1);
    /// buf.finish_cycle();
    /// assert_eq!(*buf.read(), Default::default());
    ///
    /// // Clearing takes precedence over stalling
    /// let mut buf = Buf::new(1);
    /// buf.stall();
    /// buf.clear();
    /// buf.finish_cycle();
    /// assert_eq!(*buf.read(), Default::default());
    /// ```
    fn clear(&mut self) {
        self.state = BufState::Cleared(T::default);
    }
}
