use crate::mem::{Addr, MemErr};
use std::sync::Arc;

pub type BtbPred = Option<Addr>;

#[derive(Debug, Clone)]
pub struct FetchPacket {
    pub addr: Addr,
    pub bytes: Result<Arc<[u8]>, MemErr>,
    pub btb_pred: BtbPred,
}
