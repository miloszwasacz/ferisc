use crate::mem::Addr;
use std::sync::Arc;

pub type BtbPred = Option<Addr>;

#[derive(Debug, Clone)]
pub struct FetchPacket {
    pub addr: Addr,
    pub bytes: Arc<[u8]>,
    pub btb_pred: BtbPred,
}
