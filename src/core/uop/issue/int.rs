use super::*;
use crate::core::{
    reg::int::{IntReg, IntVal},
    uop::{dispatch::int::Uop as DUop, int::*},
};

pub type Uop = EeUop<IntVal, IntVal>;

impl Issue for DUop {
    type Reg = IntReg;
    type Issued = Uop;

    fn issue(self, prf: &Prf<Self::Reg>) -> Option<Self::Issued> {
        macro_rules! unwrap_src {
            ($src:expr) => {{
                match $src {
                    Src::Reg(reg) => prf[reg].try_read_copied()?,
                    Src::Imm(imm) => imm,
                }
            }};
        }
        Some(match self {
            DUop::Alu {
                op,
                src1,
                src2,
                dest,
            } => Self::Issued::Alu {
                op,
                src1: unwrap_src!(src1),
                src2: unwrap_src!(src2),
                dest,
            },
            DUop::Mul {
                op,
                src1,
                src2,
                dest,
            } => Self::Issued::Mul {
                op,
                src1: prf[src1].try_read_copied()?,
                src2: prf[src2].try_read_copied()?,
                dest,
            },
            DUop::Jalr { base, offset, pred } => Self::Issued::Jalr {
                base: prf[base].try_read_copied()?,
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
            } => Self::Issued::Br {
                cmp,
                src1: prf[src1].try_read_copied()?,
                src2: prf[src2].try_read_copied()?,
                next,
                target,
                pred,
            },
            DUop::Lda { base, offset, dest } => Self::Issued::Lda {
                base: prf[base].try_read_copied()?,
                offset,
                dest,
            },
            DUop::Sta { base, offset, dest } => Self::Issued::Sta {
                base: prf[base].try_read_copied()?,
                offset,
                dest,
            },
            DUop::Csr { op, csr, src, dest } => Self::Issued::Csr {
                op,
                csr,
                src: unwrap_src!(src),
                dest,
            },
        })
    }
}
