use super::fetch::BtbPred;
use crate::mem::{Addr, MemErr};

#[derive(Debug, Clone)]
pub struct Predecoded {
    pub addr: Addr,
    pub bytes: Result<PdcdBytes, MemErr>,
    pub btb_pred: BtbPred,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PdcdBytes {
    Normal(u32),
    Compressed(u16),
}
