use self::entry::{Entry, UopKind};
use self::flush::{Flush, FlushPriority};
use self::locks::{CommitLock, DispatchLock, ExecuteLock};
use crate::core::{
    Mode,
    csr::CsrFile,
    fe::AddrGenLock,
    pipeline::locks::CommitLock as PipelineCommitLock,
    uop::{Exception, rename},
};
use crate::seq::{Buf, Pbuf, Queue, locks::StageLock};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

mod entry;
mod flush;
pub mod locks;

type Slot = Rc<RefCell<Entry>>;

#[derive(Debug)]
pub struct Rob {
    queue: Queue<Slot>,
    flush: Pbuf<Flush, FlushPriority>,
    commit_width: usize,
}

impl Rob {
    pub fn dispatch(&mut self) -> impl StageLock<'_, rename::Uop, (rename::Uop, EntryHandle)> {
        DispatchLock::new(self)
    }

    pub fn execute<'r, 'g>(&'r mut self, addr_gen: AddrGenLock<'g>) -> ExecuteLock<'r, 'g> {
        ExecuteLock::new(self, addr_gen)
    }

    pub fn commit(
        &mut self,
        pipelines: PipelineCommitLock<'_>,
        csrf: &mut CsrFile,
        addr_gen: AddrGenLock<'_>,
        mode: &mut Buf<Mode>,
    ) -> impl StageLock<'_, (), ()> {
        CommitLock::new(self, pipelines, csrf, addr_gen, mode)
    }
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
    pub fn commited(&self) -> bool {
        self.0.upgrade().is_none()
    }

    pub fn flushed(&self) -> bool {
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
                UopKind::Reg(_) | UopKind::Csr(_, _) | UopKind::MemOrd | UopKind::Except(_) => {
                    return;
                }
            };
            entry.kind.write(kind);
        });
    }

    pub fn throw(&mut self, exception: Exception) {
        with_entry!(self, |entry| entry.kind.write(UopKind::Except(exception)));
    }

    pub fn flush(&mut self) {
        with_entry!(self, |entry| entry.flush = true);
    }
}
