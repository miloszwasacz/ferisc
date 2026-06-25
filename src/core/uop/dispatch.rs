use crate::core::ee::rob;

pub mod int;

#[derive(Debug, Clone)]
pub struct Uop {
    pub kind: UopKind,
    pub rob_entry: rob::EntryHandle,
}

#[derive(Debug, Clone)]
pub enum UopKind {
    Int(int::Uop),
    Handled,
}
