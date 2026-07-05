pub use self::lock::PredecodeLock;
use crate::core::{fe::fetch::FetchUnit, mem::IMem, uop::predecode::Predecoded};
use crate::seq::{Buf, Clearable, Sequential};
use const_format::str_repeat;
use core::fmt;
use std::collections::VecDeque;

mod lock;

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

#[derive(Debug, Clone)]
pub struct Stats {
    predecoded: VecDeque<Predecoded>,
    cross_block: CrossBlock,
}

impl Stats {
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &Predecoded> + DoubleEndedIterator {
        self.predecoded.iter()
    }

    pub fn cross_block(&self) -> Option<CBDisplay<'_>> {
        self.cross_block.as_ref().map(CBDisplay)
    }
}

impl Predecoder {
    pub fn stats<M: IMem>(&self, fetch: &FetchUnit<M>) -> Stats {
        Stats {
            predecoded: fetch.predecode_stats().clone(),
            cross_block: self.0.read_copied(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CBDisplay<'a>(&'a [u8; 2]);

impl fmt::Display for CBDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}{:08b}{:08b}",
            str_repeat!("X", 16),
            self.0[1],
            self.0[0]
        )
    }
}
