use crate::seq::Sequential;

#[derive(Debug, Clone)]
pub struct CsrBuf<T> {
    val: T,
    new: Option<T>,
    implicit: Option<T>,
}

impl<T> CsrBuf<T> {
    pub fn read(&self) -> &T {
        &self.val
    }

    pub fn write(&mut self, val: T) {
        debug_assert!(self.new.is_none());
        self.new = Some(val);
    }

    pub fn implicit_write(&mut self, val: T) {
        debug_assert!(self.implicit.is_none());
        self.implicit = Some(val);
    }
}

impl<T> Sequential for CsrBuf<T> {
    fn finish_cycle(&mut self) {
        if let Some(new) = self.new.take().or(self.implicit.take()) {
            self.new = Some(new);
        }
    }
}
