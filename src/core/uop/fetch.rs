use super::Exception;
use crate::mem::Addr;
use std::sync::Arc;

pub type BtbPred = Option<Addr>;

#[derive(Debug, Clone)]
pub struct FetchPacket {
    pub addr: Addr,
    pub bytes: Result<Arc<[u8]>, Exception>,
    pub pred: BtbPred,
}
