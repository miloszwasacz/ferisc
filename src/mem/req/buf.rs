use crate::seq::Sequential;

#[derive(Debug, Clone)]
pub struct ReqBuf<R> {
    value: Option<R>,
    new: NewReq<R>,
}

#[derive(Debug, Clone, Copy, Default)]
enum NewReq<R> {
    #[default]
    None,
    Some(R),
    Killed,
}

impl<R> NewReq<R> {
    pub fn take(&mut self) -> Self {
        std::mem::take(self)
    }
}

impl<R> ReqBuf<R> {
    pub fn read(&self) -> Option<&R> {
        self.value.as_ref()
    }

    pub fn write(&mut self, value: R) {
        self.new = match self.new.take() {
            NewReq::None => NewReq::Some(value),
            NewReq::Killed => NewReq::Killed,
            NewReq::Some(_) => panic!("buffer has already been written to"),
        }
    }

    pub fn kill(&mut self) {
        self.new = NewReq::Killed;
    }

    //TODO Add a note that this method calls `finish_cycle`
    pub fn try_complete<P: FnOnce(&R) -> bool, F: FnOnce(R) -> R>(
        &mut self,
        is_incomplete: P,
        complete: F,
    ) -> bool {
        self.finish_cycle();

        let mut completed = false;
        self.value = self.value.take().map(|req| {
            if is_incomplete(&req) {
                completed = true;
                complete(req)
            } else {
                req
            }
        });
        completed
    }
}

impl<R> Default for ReqBuf<R> {
    fn default() -> Self {
        Self {
            value: Default::default(),
            new: Default::default(),
        }
    }
}

impl<R> Sequential for ReqBuf<R> {
    fn finish_cycle(&mut self) {
        let value = self.value.take();
        self.value = match self.new.take() {
            NewReq::None => value,
            NewReq::Some(new) => Some(new),
            NewReq::Killed => None,
        }
    }
}
