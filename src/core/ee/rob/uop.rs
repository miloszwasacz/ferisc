use crate::core::uop::{fence::Fence, rename};
use crate::mem::Addr;
use std::rc::{Rc, Weak};

#[derive(Debug, Clone)]
pub struct Uop {
    pub kind: UopKind,
    pub addr: Addr,
    pub comp: bool,
}

#[derive(Debug, Clone)]
pub enum UopKind {
    Int(rename::int::Uop),
    MemOrd(FenceHandle),
    Handled,
}

#[derive(Debug, Clone)]
pub struct FenceHandle(Weak<Fence>);

impl FenceHandle {
    pub(super) fn new(fence: &Rc<Fence>) -> Self {
        Self(Rc::downgrade(fence))
    }

    pub const fn detached() -> Self {
        Self(Weak::new())
    }

    pub fn get(&self) -> Option<Fence> {
        self.0.upgrade().map(|f| *f)
    }

    pub fn is_none(&self) -> bool {
        self.0.upgrade().is_none()
    }
}
