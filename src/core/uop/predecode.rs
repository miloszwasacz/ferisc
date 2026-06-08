use super::Exception;
use crate::mem::Addr;

#[derive(Debug, Clone, Copy)]
pub struct Predecoded {
    pub meta: Meta,
    pub bytes: Result<PdcdBytes, Exception>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PdcdBytes {
    Normal(i32),
    Compressed(i16),
}

#[derive(Debug, Clone, Copy)]
pub struct Meta {
    pub addr: Addr,
    //TODO BP
    pub pred: bool,
}
