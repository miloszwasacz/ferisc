pub use self::lock::PredecodeLock;
use crate::seq::{Buf, Clearable, Sequential};

type CrossBlock = Option<[u8; 2]>;

#[derive(Debug, Default)]
pub struct Predecoder(Buf<CrossBlock>);

impl Predecoder {
    pub fn lock(&mut self) -> PredecodeLock<'_> {
        let cross = self.0.read_copied();
        PredecodeLock::new(self, cross)
    }
}

impl Sequential for Predecoder {
    fn finish_cycle(&mut self) {
        self.0.finish_cycle();
    }
}

impl Clearable for Predecoder {
    fn clear(&mut self) {
        self.0.clear();
    }
}
