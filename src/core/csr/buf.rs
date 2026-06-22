use crate::seq::Sequential;

#[derive(Debug, Clone)]
pub struct CsrBuf<T> {
    val: T,
    new: New<T>,
    implicit: Option<T>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum New<T> {
    #[default]
    None,
    Written(T),
    Committed(T),
    Flushed,
}

impl<T> CsrBuf<T> {
    pub fn read(&self) -> &T {
        &self.val
    }

    pub fn write(&mut self, val: T) {
        debug_assert!(matches!(self.new, New::None));
        self.new = New::Written(val);
    }

    pub fn commit(&mut self) {
        self.new = match std::mem::take(&mut self.new) {
            New::Written(new) => New::Committed(new),
            New::None => panic!("cannot commit without writing"),
            New::Committed(_) => panic!("cannot commit more than once in the same clock cycle"),
            New::Flushed => panic!("cannot commit and flush in the same clock cycle"),
        };
    }

    pub fn flush(&mut self) {
        debug_assert!(
            !matches!(self.new, New::Committed(_)),
            "cannot commit and flush in the same clock cycle"
        );
        self.new = New::Flushed;
    }

    pub fn implicit_write(&mut self, val: T) {
        debug_assert!(self.implicit.is_none());
        self.implicit = Some(val);
    }
}

impl<T> Sequential for CsrBuf<T> {
    fn finish_cycle(&mut self) {
        if let Some(new) = self.implicit.take() {
            self.val = new;
        }
        match std::mem::take(&mut self.new) {
            New::None => {}
            New::Written(new) => {
                self.new = New::Written(new);
            }
            New::Committed(new) => {
                self.val = new;
            }
            New::Flushed => {}
        }
    }
}
