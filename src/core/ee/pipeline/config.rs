use crate::core::ee::{eu, rs};
use std::num::NonZeroUsize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub prf_size: NonZeroUsize,
    pub lines: Box<[(rs::Config, eu::Config)]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Configs {
    pub int: Config,
}
