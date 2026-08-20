use crate::core::uop::fence::Fence;
use crate::seq::queue::{Pop, Push, Queue};
use std::rc::{Rc, Weak};

type OwnedFence = Rc<Fence>;
pub type FenceQueue = Queue<OwnedFence>;
pub type FencePush<'a> = Push<'a, OwnedFence>;
pub type FencePop<'a> = Pop<'a, OwnedFence>;

#[derive(Debug, Clone, Default)]
pub struct FenceHandle(Weak<Fence>);

impl FenceHandle {
    pub fn new(fence: &Rc<Fence>) -> Self {
        Self(Rc::downgrade(fence))
    }

    pub fn get(&self) -> Fence {
        self.0.upgrade().map(|fence| *fence).unwrap_or(Fence::NONE)
    }
}
