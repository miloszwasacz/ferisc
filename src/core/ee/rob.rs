use self::entry::Entry;
pub use self::entry::Handle;
use self::flush::{Flush, FlushPriority};
pub use self::uop::{Committed, Flushed, Uop};
use crate::seq::{Pbuf, Queue, Sequential};
use derive_more::Deref;
use std::num::{NonZeroU8, NonZeroUsize};

pub mod entry;
mod flush;
pub mod lock;
pub mod substage;
mod uop;

#[derive(Debug)]
pub struct Rob {
    queue: Queue<Entry>,
    flush: Pbuf<Flush, FlushPriority>,
    commit_width: NonZeroU8,
    flush_width: NonZeroUsize,
}

impl Rob {
    pub fn new(config: &Config) -> Self {
        Self {
            queue: Queue::new(config.size),
            flush: Default::default(),
            commit_width: config.commit_width,
            flush_width: config.flush_width.unwrap_or(config.size),
        }
    }
}

impl Sequential for Rob {
    fn finish_cycle(&mut self) {
        self.queue.finish_cycle();
        self.flush.finish_cycle();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    pub size: NonZeroUsize,
    pub commit_width: NonZeroU8,
    pub flush_width: Option<NonZeroUsize>,
}

#[derive(Debug, Clone, Deref)]
#[deref(forward)]
pub struct Stats(Box<[entry::Stats]>);

impl Rob {
    pub fn stats(&self) -> Stats {
        Stats(match self.flush.read() {
            Flush::None => self.queue.iter().map(|entry| entry.stats(false)).collect(),
            Flush::Partial(cause) => {
                let mut flushed = false;
                self.queue
                    .iter()
                    .map(|entry| {
                        let stats = entry.stats(flushed);
                        flushed = flushed || entry.handle_eq(cause);
                        stats
                    })
                    .collect()
            }
            Flush::All => self.queue.iter().map(|entry| entry.stats(true)).collect(),
        })
    }
}
