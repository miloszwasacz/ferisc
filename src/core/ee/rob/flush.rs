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

pub type UopKind = super::UopKind<()>;

impl From<&super::UopKind> for UopKind {
    fn from(value: &super::UopKind) -> Self {
        use super::UopKind;
        match value {
            UopKind::Reg(dest) => Self::Reg(*dest),
            UopKind::Br { done } => Self::Br { done: *done },
            UopKind::Ld { dest, done } => Self::Ld {
                dest: *dest,
                done: *done,
            },
            UopKind::St { done } => Self::St { done: *done },
            UopKind::Csr(dest, csr) => Self::Csr(*dest, *csr),
            UopKind::MemOrd(_) => Self::MemOrd(()),
            UopKind::Except(ex) => Self::Except(*ex),
        }
    }
}
