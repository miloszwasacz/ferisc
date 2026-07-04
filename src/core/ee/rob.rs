use self::entry::{Entry, UopKind};
use self::flush::{Flush, FlushPriority};
pub use self::flush::{Uop as FlushedUop, UopKind as FlushedUopKind};
use self::locks::{AccessLock, FlushAllLock, FlushLock};
use crate::core::uop::{Exception, fence::Fence};
use crate::seq::{Pbuf, Queue, Sequential};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

mod entry;
mod flush;
pub mod locks;
pub mod uop;

type Slot = Rc<RefCell<Entry>>;

#[derive(Debug)]
pub struct Rob {
    queue: Queue<Slot>,
    /// A list of fences committed in this clock cycle.
    ///
    /// We need to keep track of them because the [`MemSubsystem`](crate::core::MemSubsystem)
    /// might still need them until the end of the clock cycle, keeping the `Rc` alive.
    committed_fences: Vec<Rc<Fence>>,
    flush: Pbuf<Flush, FlushPriority>,
    commit_width: usize,
    flush_width: usize,
}

impl Rob {
    pub fn access(&mut self) -> RobAccess<'_> {
        match self.flush.read_copied() {
            Flush::None => RobAccess::Ok(AccessLock::new(self)),
            Flush::After(i) => RobAccess::FlushSome(FlushLock::new(self, i)),
            Flush::All => RobAccess::FlushAll(FlushAllLock::new(self)),
        }
    }
}

impl Sequential for Rob {
    fn finish_cycle(&mut self) {
        self.queue.finish_cycle();
        self.flush.finish_cycle();
        drop(self.committed_fences.drain(..))
    }
}

pub enum RobAccess<'a> {
    Ok(AccessLock<'a>),
    FlushSome(FlushLock<'a>),
    FlushAll(FlushAllLock<'a>),
}

#[derive(Debug, Clone)]
pub struct EntryHandle(Weak<RefCell<Entry>>);

macro_rules! with_entry {
    ($self:expr, |$entry:ident| $body:stmt) => {{
        if let Some(e) = $self.0.upgrade() {
            let mut $entry = e.borrow_mut();
            { $body }
        }
    }};
}

impl EntryHandle {
    pub fn is_flushed(&self) -> bool {
        self.0.upgrade().is_none()
    }

    pub fn mark_done(&mut self) {
        with_entry!(self, |entry| {
            let kind = match entry.kind.read() {
                UopKind::Br { .. } => UopKind::Br { done: true },
                UopKind::Ld { dest, .. } => UopKind::Ld {
                    dest: *dest,
                    done: true,
                },
                UopKind::St { .. } => UopKind::St { done: true },
                UopKind::Reg(_) | UopKind::Csr(_, _) | UopKind::MemOrd(_) | UopKind::Except(_) => {
                    return;
                }
            };
            entry.kind.write(kind);
        });
    }

    pub fn throw(&mut self, exception: Exception) {
        with_entry!(self, |entry| entry.kind.write(UopKind::Except(exception)));
    }
}
