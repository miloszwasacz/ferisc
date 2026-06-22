use super::*;
use crate::core::{
    reg::int::{IntReg, IntVal},
    uop::int::{self, Src},
};

pub type Uop = int::Uop<RegOper<IntReg>, Src<RegOper<IntReg>>, RegDest<IntReg>>;

impl InferUop<IntReg> for super::Uop {
    type Kind = Uop;
}

impl Rename for decode::int::Uop {
    type Reg = IntReg;
    type Renamed = Uop;

    fn rename(
        self,
        get_oper: impl Fn(Self::Reg) -> RegOper<Self::Reg>,
        mut alloc_dest: impl FnMut(Self::Reg, Option<Repr<Self::Reg>>) -> Option<RegDest<Self::Reg>>,
    ) -> Option<Self::Renamed> {
        Some(match self {
            Self::Alu {
                op,
                src1,
                src2,
                dest,
            } => Self::Renamed::Alu {
                op,
                src1: src1.map_reg(&get_oper),
                src2: src2.map_reg(&get_oper),
                dest: alloc_dest(dest, None)?,
            },
            Self::Mul {
                op,
                src1,
                src2,
                dest,
            } => Self::Renamed::Mul {
                op,
                src1: get_oper(src1),
                src2: get_oper(src2),
                dest: alloc_dest(dest, None)?,
            },
            Self::Jal {
                target,
                link,
                link_addr,
            } => Self::Renamed::Jal {
                target,
                link: alloc_dest(link, Some(IntVal::address(link_addr)))?,
                link_addr,
            },
            Self::Jalr {
                base,
                offset,
                link,
                link_addr,
                pred,
            } => Self::Renamed::Jalr {
                base: get_oper(base),
                offset,
                link: alloc_dest(link, Some(IntVal::address(link_addr)))?,
                link_addr,
                pred,
            },
            Self::Br {
                cmp,
                src1,
                src2,
                target,
                pred,
                btb_pred,
            } => Self::Renamed::Br {
                cmp,
                src1: get_oper(src1),
                src2: get_oper(src2),
                target,
                pred,
                btb_pred,
            },
            Self::Ld {
                size,
                signed,
                base,
                offset,
                dest,
            } => Self::Renamed::Ld {
                size,
                signed,
                base: get_oper(base),
                offset,
                dest: alloc_dest(dest, None)?,
            },
            Self::St {
                size,
                src,
                base,
                offset,
            } => Self::Renamed::St {
                size,
                src: get_oper(src),
                base: get_oper(base),
                offset,
            },
            Self::Csr { op, csr, src, dest } => Self::Renamed::Csr {
                op,
                csr,
                src: src.map_reg(&get_oper),
                dest: alloc_dest(dest, None)?,
            },
        })
    }
}
