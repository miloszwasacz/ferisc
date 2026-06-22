use crate::core::{
    csr::CsrAddr,
    mem::AddrHandle,
    reg::{
        int::{IntReg, IntVal},
        phys::{self, RegOper},
    },
    uop::{InferUop, fetch::BtbPred, int::*},
};
use crate::mem::Addr;

type RegDest = phys::RegDest<IntReg>;

#[derive(Debug, Clone)]
pub enum Uop {
    Alu {
        op: AluOp,
        src1: Src<RegOper>,
        src2: Src<RegOper>,
        dest: RegDest,
    },
    Mul {
        op: MulOp,
        src1: RegOper,
        src2: RegOper,
        dest: RegDest,
    },
    Jalr {
        base: RegOper,
        offset: IntVal,
        pred: BtbPred,
    },
    Br {
        cmp: BrCmp,
        src1: RegOper,
        src2: RegOper,
        target: Addr,
        pred: bool,
        btb_pred: BtbPred,
    },
    Lda {
        base: RegOper,
        offset: IntVal,
        dest: AddrHandle,
    },
    Sta {
        base: RegOper,
        offset: IntVal,
        dest: AddrHandle,
    },
    Csr {
        op: CsrOp,
        csr: CsrAddr,
        src: Src<RegOper>,
        dest: RegDest,
    },
}

impl InferUop<IntReg> for super::Uop {
    type Kind = Uop;
}
