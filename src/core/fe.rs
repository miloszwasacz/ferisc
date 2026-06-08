//! The Front End of the core

pub use self::addr_gen::{AddrGen, AddrGenLock};

mod addr_gen;
pub mod decode;
pub mod fetch;
pub mod predecode;
