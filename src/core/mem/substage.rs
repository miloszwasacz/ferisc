pub use self::commit::Commit;
pub use self::dispatch::{DUop, Dispatch, Meta as DispatchMeta};
use super::entry::{Entry, Ld, St};
use super::{DMem, MemSubsystem};
use crate::core::{ee::rob, uop::fence::Fence};
use crate::seq::{Outcome, Substage};
use std::convert::Infallible;

mod dispatch {
    use super::*;
    use crate::core::{
        mem::{
            AddrHandle, Src,
            entry::EntryBuilder,
            fence::{FenceHandle, FencePush},
            queue::Push,
        },
        uop::commit::Dest,
    };
    use crate::mem::Size;
    use std::rc::Rc;

    pub struct Dispatch<'a> {
        ld: Push<'a, Ld>,
        st: Push<'a, St>,
        fences: FencePush<'a>,
        last_fence: FenceHandle,
    }

    #[derive(Debug, Clone)]
    pub enum DUop {
        Ld {
            size: Size,
            signed: bool,
            dest: Dest,
        },
        St {
            size: Size,
            src: Src,
        },
        Fence(Fence),
    }

    #[allow(private_interfaces)]
    pub enum Meta {
        Ld(Entry<Ld>),
        St(Entry<St>),
        Fence(Rc<Fence>),
    }

    impl<'a> Dispatch<'a> {
        pub(in crate::core::mem) fn new<M: DMem>(mem: &'a mut MemSubsystem<M>) -> Self {
            let last_fence = mem
                .fences
                .iter()
                .last()
                .map(FenceHandle::new)
                .unwrap_or_default();

            Self {
                ld: mem.ldq.push(),
                st: mem.stq.push(),
                fences: mem.fences.push(),
                last_fence,
            }
        }
    }

    impl<'a> Substage for Dispatch<'a> {
        type In = rob::Uop<DUop>;
        type Out = Option<AddrHandle>;
        type Extra = Meta;
        type Err = Infallible;

        fn start(&mut self, input: Self::In) -> Outcome<Self::Out, Self::Extra, Self::Err> {
            let Self::In { uop, handle } = input;
            match uop {
                DUop::Ld { size, signed, dest } => {
                    let entry = EntryBuilder::<Ld>::new(
                        handle,
                        size,
                        signed,
                        dest,
                        self.last_fence.clone(),
                    );
                    let addr = entry.handle();
                    self.ld.start(entry).map(|()| Some(addr)).map_meta(Meta::Ld)
                }
                DUop::St { size, src } => {
                    let entry = EntryBuilder::<St>::new(handle, size, src, self.last_fence.clone());
                    let addr = entry.handle();
                    self.st.start(entry).map(|()| Some(addr)).map_meta(Meta::St)
                }
                DUop::Fence(fence) => self
                    .fences
                    .start(Rc::new(fence))
                    .map(|()| None)
                    .map_meta(Meta::Fence),
            }
        }

        fn finish(&mut self, extra: Self::Extra) {
            match extra {
                Meta::Ld(entry) => self.ld.finish(entry),
                Meta::St(entry) => self.st.finish(entry),
                Meta::Fence(fence) => {
                    self.last_fence = FenceHandle::new(&fence);
                    self.fences.finish(fence);
                }
            }
        }
    }
}

mod commit {
    use super::*;
    use crate::core::mem::fence::FencePop;
    use crate::seq::SimpleSubStage;

    pub struct Commit<'a>(FencePop<'a>);

    impl<'a> Commit<'a> {
        pub(in crate::core::mem) fn new<M: DMem>(mem: &'a mut MemSubsystem<M>) -> Self {
            Self(mem.fences.pop(mem.fences.size()))
        }
    }

    impl<'a> SimpleSubStage for Commit<'a> {
        type In = Fence;
        type Out = ();

        fn start(&mut self, input: Self::In) -> Option<Self::Out> {
            SimpleSubStage::start(&mut self.0, ())
                .map(|fence| debug_assert_eq!(**fence, input, "inconsistent order of fences"))
        }

        fn finish(&mut self) {
            SimpleSubStage::finish(&mut self.0)
        }
    }
}
