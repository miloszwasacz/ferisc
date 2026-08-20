pub mod int;

#[derive(Debug, Clone)]
pub struct Uop {
    pub kind: UopKind,
}

pub type UopKind = super::UopKind<int::Uop, Handled>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Handled;
