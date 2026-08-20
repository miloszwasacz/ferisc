pub(super) use self::push::Push;
use super::{DMem, Entry, Ld, Req, St};
use crate::seq::{Clearable, Sequential};
use std::collections::VecDeque;
use std::debug_assert_matches;
use std::num::NonZeroUsize;
use std::ops::{Deref, DerefMut};

#[derive(Debug, Clone)]
pub(super) struct Queue<T> {
    buf: VecDeque<QueueEntry<T>>,
    size: NonZeroUsize,
    state: State,
}

#[derive(Debug, Clone, derive_more::Deref, derive_more::DerefMut)]
struct QueueEntry<T> {
    #[deref]
    #[deref_mut]
    entry: Entry<T>,
    flushed: bool,
}

#[derive(Debug, Clone, Copy, Default)]
struct State {
    pushed: usize,
    scheduled: Option<usize>,
    flushed: Flush,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
enum Flush {
    #[default]
    None,
    Some,
    All,
}

impl<T> Queue<T> {
    pub fn new(size: NonZeroUsize) -> Self {
        Self {
            buf: VecDeque::with_capacity(size.get()),
            size,
            state: Default::default(),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &Entry<T>> {
        self.buf
            .iter()
            .take(self.buf.len() - self.state.pushed)
            .map(Deref::deref)
    }

    pub fn push(&mut self) -> Push<'_, T> {
        Push::new(self)
    }
}

impl<T> Sequential for Queue<T> {
    fn finish_cycle(&mut self) {
        use std::mem::take;
        self.buf
            .iter_mut()
            .map(DerefMut::deref_mut)
            .for_each(Sequential::finish_cycle);

        // Schedule
        if let Some(i) = take(&mut self.state.scheduled) {
            self.buf.remove(i);
        }

        // Push
        take(&mut self.state.pushed);

        // Clear
        match take(&mut self.state.flushed) {
            Flush::None => {}
            Flush::Some => self.buf.retain(|entry| !entry.flushed),
            Flush::All => self.buf.clear(),
        }
    }
}

impl<T> From<Entry<T>> for QueueEntry<T> {
    fn from(value: Entry<T>) -> Self {
        QueueEntry {
            entry: value,
            flushed: false,
        }
    }
}

mod schedule {
    use super::*;
    use crate::core::mem::{ReadyEntry, Src};
    use crate::mem::{LdReq, ReqData, Size, StReq};

    impl<T: Clone> Queue<T> {
        #[inline]
        fn _schedule_ready<M: DMem>(
            &mut self,
            mut find_map: impl FnMut(ReadyEntry<T>) -> Option<Req<M>>,
        ) -> Req<M> {
            debug_assert!(
                self.state.scheduled.is_none(),
                "cannot schedule more than once in a clock cycle"
            );

            self.buf
                .iter()
                .enumerate()
                .find_map(|(i, e)| {
                    let ready = e.deref().try_into().ok()?;
                    find_map(ready).inspect(|_| self.state.scheduled = Some(i))
                })
                .unwrap_or(Req::None)
        }
    }

    impl Queue<Ld> {
        pub fn update_deps(&mut self) {
            self.buf
                .iter_mut()
                .map(DerefMut::deref_mut)
                .for_each(Entry::<Ld>::update_deps)
        }

        pub fn schedule_ready<M: DMem>(&mut self, mem: &mut M, conn: ld_mem!(<M>::Conn)) -> Req<M> {
            self._schedule_ready(|entry| {
                let req = LdReq::new(entry.addr(), entry.size());
                Some(Req::Ld(ld_mem!(<M>::req(mem, req, conn)), entry))
            })
        }
    }

    impl Queue<St> {
        pub fn update_deps(&mut self) {
            self.buf
                .iter_mut()
                .map(DerefMut::deref_mut)
                .for_each(Entry::<St>::update_deps)
        }

        pub fn schedule_ready<M: DMem>(
            &mut self,
            mem: &mut M,
            conn: st_mem!(<M>::Conn),
            try_read_bypassed: impl Fn(Src, Size) -> Option<ReqData>,
        ) -> Req<M> {
            self._schedule_ready(|entry| {
                let data = try_read_bypassed(entry.src(), entry.size())?;
                let req = StReq::new(entry.addr(), data);
                Some(Req::St(st_mem!(<M>::req(mem, req, conn)), entry))
            })
        }
    }
}

mod push {
    use super::*;
    use crate::core::mem::entry::EntryBuilder;
    use crate::seq::{Outcome, Substage};
    use std::convert::Infallible;

    pub(in crate::core::mem) struct Push<'a, T> {
        buf: &'a mut VecDeque<QueueEntry<T>>,
        max_size: usize,
    }

    impl<'a, T> Push<'a, T> {
        pub(super) fn new(queue: &'a mut Queue<T>) -> Self {
            debug_assert!(
                queue.state.pushed == 0,
                "cannot push more than once in a clock cycle"
            );

            Self {
                max_size: queue.size.get() - queue.buf.len(),
                buf: &mut queue.buf,
            }
        }
    }

    macro_rules! impl_substage {
        ($( $meta:ty ),*) => {
            $(
            impl<'a> Substage for Push<'a, $meta> {
                type In = EntryBuilder<$meta>;
                type Out = ();
                type Extra = Entry<$meta>;
                type Err = Infallible;

                fn start(&mut self, input: Self::In) -> Outcome<Self::Out, Self::Extra, Self::Err> {
                    if self.buf.len() < self.max_size {
                        Outcome::Ok((), input.build(self.buf.iter().map(|e| e.handle())))
                    } else {
                        Outcome::Stop
                    }
                }

                fn finish(&mut self, extra: Self::Extra) {
                    self.buf.push_back(extra.into());
                    debug_assert!(self.buf.len() <= self.max_size);
                }
            }
            )*
        };
    }
    impl_substage!(Ld, St);
}

mod flush {
    use super::*;
    use crate::core::ee::rob;

    impl<T> Queue<T> {
        pub fn flush(&mut self, flush: &rob::lock::Flush<'_>) {
            debug_assert_matches!(self.state.flushed, Flush::None);
            self.state.flushed = Flush::Some;
            for entry in &mut self.buf {
                entry.flushed = entry.flushed || entry.is_flushed(flush);
            }
        }
    }

    impl<T> Clearable for Queue<T> {
        fn clear(&mut self) {
            debug_assert_matches!(self.state.flushed, Flush::None);
            self.state.flushed = Flush::All;
        }
    }
}
