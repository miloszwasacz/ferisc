use super::ee::{pipeline, rob};
use super::fe::{decode, fetch};
use super::mem;
use crate::mem::Addr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub entrypoint: Addr,
    pub fetch: fetch::Config,
    pub decode: decode::Config,
    pub rob: rob::Config,
    pub pipelines: pipeline::Configs,
    pub mem: mem::Config,
}
