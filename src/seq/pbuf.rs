use super::{Buf, Clearable, Sequential, Stallable};

//TODO Example usage in docs

/// A clock-based priority [buffer](Buf).
#[derive(Debug)]
pub struct Pbuf<T, P = u8> {
    buf: Buf<T>,
    new: Option<(T, P)>,
}

impl<T, P> Pbuf<T, P> {
    /// Creates a new priority buffer with the specified initial value.
    pub fn new(value: T) -> Self {
        Self {
            buf: Buf::new(value),
            new: None,
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

    pub fn write(&mut self, value: T, priority: P)
    where
        P: Ord,
    {
        if self.new.as_ref().is_none_or(|(_, p)| *p < priority) {
            self.new = Some((value, priority));
        }
    }
}

//TODO Add a note that the state is not preserved (i.e. only the current value is cloned)
impl<T: Clone, P> Clone for Pbuf<T, P> {
    fn clone(&self) -> Self {
        Self {
            buf: self.buf.clone(),
            new: None,
        }
    }
}

impl<T: Default, P> Default for Pbuf<T, P> {
    fn default() -> Self {
        Self {
            buf: Default::default(),
            new: None,
        }
    }
}

impl<T, P> Sequential for Pbuf<T, P> {
    fn finish_cycle(&mut self) {
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
