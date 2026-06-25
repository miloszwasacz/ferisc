use crate::mem::Addr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum Flush {
    #[default]
    None,
    After(usize),
    All,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) enum FlushPriority {
    Execute,
    Commit,
}

#[derive(Debug, Clone, Copy)]
pub struct Uop {
    pub kind: UopKind,
    pub addr: Addr,
    pub comp: bool,
}

pub type UopKind = super::UopKind;
