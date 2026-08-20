use super::*;
use crate::core::{
    reg::int::{IntReg, IntVal},
    uop::{dispatch::int::Uop as DUop, int::*},
};

pub type Uop = EeUop<IntVal, IntVal>;

impl Issue for DUop {
    type Reg = IntReg;
    type Issued = Uop;

    fn issue(self, read: impl TryReadReg<Self::Reg>) -> Option<Self::Issued> {
        Some(match self {
            DUop::Alu {
                op,
                src1,
                src2,
                dest,
            } => Uop::Alu {
                op,
                src1: unwrap_src!(read, src1),
                src2: unwrap_src!(read, src2),
                dest,
            },
            DUop::Mul {
                op,
                src1,
                src2,
                dest,
            } => Uop::Mul {
                op,
                src1: try_read!(read, src1),
                src2: try_read!(read, src2),
                dest,
            },
            DUop::Jalr { base, offset, pred } => Uop::Jalr {
                base: try_read!(read, base),
                offset,
                pred,
            },
            DUop::Br {
                cmp,
                src1,
                src2,
                next,
                target,
                pred,
            } => Uop::Br {
                cmp,
                src1: try_read!(read, src1),
                src2: try_read!(read, src2),
                next,
                target,
                pred,
            },
            DUop::Lda { base, offset, dest } => Uop::Lda {
                base: try_read!(read, base),
                offset,
                dest,
            },
            DUop::Sta { base, offset, dest } => Uop::Sta {
                base: try_read!(read, base),
                offset,
                dest,
            },
            DUop::Csr { op, csr, src, dest } => Uop::Csr {
                op,
                csr,
                src: unwrap_src!(read, src),
                dest,
            },
        })
    }
}
