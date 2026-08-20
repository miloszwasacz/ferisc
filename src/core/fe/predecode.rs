use self::substage::{In, Out};
use crate::core::uop::predecode::Predecoded;
use crate::seq::{Buf, Clearable, Sequential};
use const_format::str_repeat;
use core::fmt;
use std::collections::VecDeque;
use std::num::NonZeroUsize;

pub mod substage;

type CrossBlock = Option<[u8; 2]>;

#[derive(Debug)]
pub struct PredecodeUnit {
    cross_block: Buf<CrossBlock>,
    out_queue: Queue,
    //TODO Different IN and OUT widths
    width: NonZeroUsize,
    state: State,
}

#[derive(Debug, derive_more::Deref, derive_more::DerefMut)]
struct Queue {
    #[deref]
    #[deref_mut]
    buf: VecDeque<Predecoded>,
    len: usize,
}

#[derive(Debug, Default)]
struct State {
    popped: usize,
    cleared: bool,
}

impl PredecodeUnit {
    pub fn new(config: &Config) -> Self {
        Self {
            cross_block: Default::default(),
            out_queue: Queue::new(config.queue_size.unwrap_or(config.width)),
            width: config.width,
            state: Default::default(),
        }
    }

    pub fn r#in(&mut self) -> In<'_> {
        In::new(self)
    }

    pub fn out(&mut self) -> Out<'_> {
        Out::new(self)
    }
}

impl Queue {
    fn new(size: NonZeroUsize) -> Self {
        Self {
            buf: VecDeque::with_capacity(size.get()),
            len: 0,
        }
    }
}

impl Sequential for PredecodeUnit {
    fn finish_cycle(&mut self) {
        use std::mem::take;

        // Pop
        {
            let popped = take(&mut self.state.popped);
            drop(self.out_queue.drain(..popped));
        }

        // Clear
        if take(&mut self.state.cleared) {
            self.out_queue.clear();
            self.cross_block.clear();
        }

        // Misc
        self.out_queue.len = self.out_queue.len();
        self.cross_block.finish_cycle();
    }
}

impl Clearable for PredecodeUnit {
    fn clear(&mut self) {
        self.state.cleared = true;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    pub width: NonZeroUsize,
    //TODO Add a note that queue_size >= width
    pub queue_size: Option<NonZeroUsize>,
}

#[derive(Debug, Clone)]
pub struct Stats {
    predecoded: Box<[Predecoded]>,
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

impl PredecodeUnit {
    pub fn stats(&self) -> Stats {
        Stats {
            predecoded: self
                .out_queue
                .iter()
                .take(self.out_queue.len)
                .copied()
                .collect(),
            cross_block: self.cross_block.read_copied(),
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
