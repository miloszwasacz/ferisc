use super::Exception;
use crate::mem::{Addr, ReqData};

pub type BtbPred = Option<Addr>;

#[derive(Debug, Clone)]
pub struct FetchPacket {
    pub addr: Addr,
    pub bytes: Result<ReqData, Exception>,
    pub pred: BtbPred,
}
