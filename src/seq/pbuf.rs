#[cfg(debug_assertions)]
use self::debug::*;
use super::{Buf, Clearable, Sequential, Stallable};

//TODO Example usage in docs

/// A clock-based priority [buffer](Buf).
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

    pub fn read(&self) -> &T {
        self.buf.read()
    }

    pub fn read_copied(&self) -> T
    where
        T: Copy,
    {
        self.buf.read_copied()
    }

    //TODO Add a note that once a value has been bypassed, only higher priority
    //     values can be written (useful when used in a loop)
    pub fn read_bypassed(&self) -> &T {
        #[cfg(debug_assertions)]
        self.debug.mark_bypassed();

        self.new
            .as_ref()
            .map(|(s, _)| s)
            .unwrap_or_else(|| self.read())
    }

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

//TODO Add a note that the state is not preserved (i.e. only the current value is cloned)
impl<T: Clone, P> Clone for Pbuf<T, P> {
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
    fn stall(&mut self) {
        self.buf.stall();
    }
}

impl<T: Default, P> Clearable for Pbuf<T, P> {
    fn clear(&mut self) {
        self.buf.clear()
    }
}

#[cfg(debug_assertions)]
mod debug {
    use super::*;
    use derive_where::derive_where;
    use std::cell::Cell;

    #[derive(Debug)]
    #[derive_where(Default)]
    pub struct Debug<P> {
        /// Priority History
        ///
        /// Used for checking which priorities have already been written.
        phist: Vec<P>,
        bypassed: Cell<bool>,
    }

    impl<P> Debug<P> {
        pub fn bypassed(&self) -> bool {
            self.bypassed.get()
        }

        pub fn mark_bypassed(&self) {
            self.bypassed.set(true);
        }

        pub fn push_old_priority(&mut self, priority: P) {
            self.phist.push(priority);
        }
    }

    impl<T, P> Pbuf<T, P> {
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
