use super::*;
use crate::core::{
    reg::int::{IntReg, IntVal},
    uop::{dispatch::int::Uop as DUop, int::*},
};

pub type Uop = EeUop<IntVal, IntVal>;

impl Issue for DUop {
    type Reg = IntReg;
    type Issued = Uop;

    fn issue(self, try_read_reg: impl TryReadReg<Self::Reg>) -> Option<Self::Issued> {
        macro_rules! read_copied {
            ($reg:expr) => {
                try_read_reg.try_read($reg)?
            };
        }
        macro_rules! unwrap_src {
            ($src:expr) => {{
                match $src {
                    Src::Reg(reg) => read_copied!(reg),
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
                src1: read_copied!(src1),
                src2: read_copied!(src2),
                dest,
            },
            DUop::Jalr { base, offset, pred } => Self::Issued::Jalr {
                base: read_copied!(base),
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
                src1: read_copied!(src1),
                src2: read_copied!(src2),
                next,
                target,
                pred,
            },
            DUop::Lda { base, offset, dest } => Self::Issued::Lda {
                base: read_copied!(base),
                offset,
                dest,
            },
            DUop::Sta { base, offset, dest } => Self::Issued::Sta {
                base: read_copied!(base),
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
