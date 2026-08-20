pub use self::access::Access;
pub use self::execute::Execute;
pub use self::flush::Flush;
pub use self::flush_all::FlushAll;
pub use self::tick::Tick;
use super::substage::{Commit, Dispatch};
use super::{Handle, Rob};
use crate::core::{
    csr::CsrFile,
    ee::rob::{entry::UopKind, flush::FlushPriority},
    fe::addr_gen,
};
use crate::mem::Addr;
use std::num::NonZeroUsize;

mod access {
    use super::*;
    use crate::core::ee::rob::flush::Flush as FlushInfo;

    pub enum Access<'a> {
        Tick(Tick<'a>),
        FlushSome(Flush<'a>),
        FlushAll(FlushAll<'a>),
    }

    impl Rob {
        pub fn access(&mut self, csrf: &mut CsrFile) -> Access<'_> {
            match self.flush.read() {
                FlushInfo::None => Access::Tick(Tick::new(self)),
                FlushInfo::Partial { .. } => Access::FlushSome(Flush::new(self, csrf)),
                FlushInfo::All => Access::FlushAll(FlushAll::new(self, csrf)),
            }
        }
    }
}

mod tick {
    use super::*;
    use crate::core::{Mode, mem};

    pub struct Tick<'r>(&'r mut Rob);

    impl<'r> Tick<'r> {
        pub(super) fn new(rob: &'r mut Rob) -> Self {
            Self(rob)
        }

        pub fn dispatch<'a>(&'a mut self, mem: mem::Dispatch<'a>) -> Dispatch<'a> {
            Dispatch::new(self.0, mem)
        }

        pub fn execute<'a, 'g>(&'a mut self, addr_gen: addr_gen::Lock<'g>) -> Execute<'a, 'g> {
            Execute::new(self.0, addr_gen)
        }

        pub fn commit<'a, 'g, F: FnOnce(Mode)>(
            &'a mut self,
            addr_gen: addr_gen::Lock<'g>,
            mem: mem::Commit<'a>,
            csrf: &'a mut CsrFile,
            write_mode: F,
        ) -> Commit<'a, 'g, F> {
            Commit::new(self.0, addr_gen, mem, csrf, write_mode)
        }
    }
}

mod execute {
    use super::*;
    use crate::core::ee::rob::flush::Flush;
    use std::debug_assert_matches;

    pub struct Execute<'a, 'g> {
        rob: &'a mut Rob,
        addr_gen: addr_gen::Lock<'g>,
        drained: Option<Drained>,
    }

    #[derive(Debug, Clone)]
    struct Drained {
        cause: Handle,
        count: NonZeroUsize,
        target: Addr,
    }

    impl<'a, 'g> Execute<'a, 'g> {
        pub(super) fn new(rob: &'a mut Rob, addr_gen: addr_gen::Lock<'g>) -> Self {
            Self {
                rob,
                addr_gen,
                drained: None,
            }
        }

        pub fn jump(&mut self, target: Addr, cause: &Handle) {
            let count = self.drained.as_ref().map_or(0, |d| d.count.get());
            self.drained = self
                .rob
                .queue
                .iter()
                .rev()
                .skip(count)
                .position(|entry| entry.handle_eq(cause))
                .map(|i| Drained {
                    cause: cause.clone(),
                    count: NonZeroUsize::new(count + i + 1).unwrap(),
                    target,
                })
                .or(self.drained.take());

            debug_assert!(self.drained.is_some(), "jump cause not found in the ROB");
        }
    }

    impl Drop for Execute<'_, '_> {
        fn drop(&mut self) {
            let Some(Drained { cause, target, .. }) = self.drained.take() else {
                return;
            };

            debug_assert_matches!(
                self.rob.flush.read(),
                Flush::None,
                "ROB is already flushing"
            );
            self.rob
                .flush
                .write(Flush::Partial(cause), FlushPriority::Execute);

            self.addr_gen.execute(target);
            //TODO Notify BP
        }
    }
}

mod flush {
    use super::*;
    use crate::core::ee::rob::{flush::Flush as FlushInfo, uop::Flushed};
    use std::marker::PhantomData;

    pub struct Flush<'a>(Box<[Flushed]>, PhantomData<&'a mut Rob>);

    impl<'a> Flush<'a> {
        pub(super) fn new(rob: &'a mut Rob, csrf: &mut CsrFile) -> Self {
            let FlushInfo::Partial(cause) = rob.flush.read() else {
                panic!("only partial flush is supported");
            };

            let mut count = rob
                .queue
                .iter()
                .rev()
                .take_while(|entry| !entry.handle_eq(cause))
                .count();
            debug_assert!(count < rob.queue.iter().len(), "flush cause not found");

            if count > rob.flush_width.get() {
                count = rob.flush_width.get();
            } else {
                rob.flush.write(FlushInfo::None, FlushPriority::Flush);
            }

            let flushed = rob
                .queue
                .drain_back(count)
                .map(|entry| entry.flush(csrf))
                .collect();

            Self(flushed, PhantomData)
        }

        //TODO Add a note that the iterator is in reverse order to how the elements
        //     were originally pushed into the ROB
        pub fn flushed(&self) -> impl ExactSizeIterator<Item = &Flushed> {
            self.0.iter()
        }
    }
}

mod flush_all {
    use super::*;
    use crate::core::ee::rob::flush::Flush;
    use crate::seq::Clearable;
    use std::debug_assert_matches;
    use std::marker::PhantomData;

    pub struct FlushAll<'a>(PhantomData<&'a mut Rob>);

    impl<'a> FlushAll<'a> {
        pub(super) fn new(rob: &'a mut Rob, csrf: &mut CsrFile) -> Self {
            debug_assert_matches!(rob.flush.read(), Flush::All, "only full flush is supported");

            // Flush CSRs
            for entry in rob.queue.iter() {
                if let UopKind::Csr(_, csr) = entry.kind() {
                    csrf.flush(csr)
                }
            }

            // Flush ROB
            rob.queue.clear();
            rob.flush.clear();

            Self(PhantomData)
        }
    }
}
