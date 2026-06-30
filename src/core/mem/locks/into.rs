use super::*;
use crate::core::reg::{
    ArchReg,
    int::IntReg,
    phys::{RegDest, RegOper},
};

type Size = usize;
type Signed = bool;

pub(super) trait IntoDKind {
    type Reg: ArchReg;

    fn into_dkind(
        self,
        add_ld: impl FnOnce(Size, Signed, RegDest<Self::Reg>) -> Option<AddrHandle>,
        add_st: impl FnOnce(Size, RegOper<Self::Reg>) -> Option<AddrHandle>,
    ) -> Option<DKind>;
}

impl IntoDKind for rint::Uop {
    type Reg = IntReg;

    fn into_dkind(
        self,
        add_ld: impl FnOnce(Size, Signed, RegDest<Self::Reg>) -> Option<AddrHandle>,
        add_st: impl FnOnce(Size, RegOper<Self::Reg>) -> Option<AddrHandle>,
    ) -> Option<DKind> {
        Some(DKind::Int(match self {
            Self::Alu {
                op,
                src1,
                src2,
                dest,
            } => dint::Uop::Alu {
                op,
                src1,
                src2,
                dest,
            },
            Self::Mul {
                op,
                src1,
                src2,
                dest,
            } => dint::Uop::Mul {
                op,
                src1,
                src2,
                dest,
            },
            Self::Jal { .. } => return Some(DKind::Handled),
            Self::Jalr {
                base, offset, pred, ..
            } => dint::Uop::Jalr { base, offset, pred },
            Self::Br {
                cmp,
                src1,
                src2,
                next,
                target,
                pred,
                ..
            } => dint::Uop::Br {
                cmp,
                src1,
                src2,
                next,
                target,
                pred,
            },
            Self::Ld {
                size,
                signed,
                base,
                offset,
                dest,
            } => dint::Uop::Lda {
                base,
                offset,
                dest: add_ld(size, signed, dest)?,
            },
            Self::St {
                size,
                src,
                base,
                offset,
            } => dint::Uop::Sta {
                base,
                offset,
                dest: add_st(size, src)?,
            },
            Self::Csr { op, csr, src, dest } => dint::Uop::Csr { op, csr, src, dest },
        }))
    }
}
