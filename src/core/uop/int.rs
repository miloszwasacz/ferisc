use super::INST_NAME_WIDTH;
use crate::core::{
    csr::CsrAddr,
    mem::AddrHandle,
    reg::{
        int::{IntReg, IntVal},
        phys::RegDest,
    },
    uop::fetch::BtbPred,
};
use crate::mem::{Addr, AddrView, Size};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeUop<R, S, D> {
    Alu {
        op: AluOp,
        src1: S,
        src2: S,
        dest: D,
    },
    Mul {
        op: MulOp,
        src1: R,
        src2: R,
        dest: D,
    },
    Jal {
        target: Addr,
        link: D,
        link_addr: Addr,
        pred: BtbPred,
    },
    Jalr {
        base: R,
        offset: IntVal,
        link: D,
        link_addr: Addr,
        pred: BtbPred,
    },
    Br {
        cmp: BrCmp,
        src1: R,
        src2: R,
        next: Addr,
        target: Addr,
        pred: bool,
        btb_pred: BtbPred,
    },
    Ld {
        size: Size,
        signed: bool,
        base: R,
        offset: IntVal,
        dest: D,
    },
    St {
        size: Size,
        src: R,
        base: R,
        offset: IntVal,
    },
    Csr {
        op: CsrOp,
        csr: CsrAddr,
        src: S,
        dest: D,
    },
}

#[derive(Debug, Clone)]
pub enum EeUop<S, R> {
    Alu {
        op: AluOp,
        src1: S,
        src2: S,
        dest: RegDest<IntReg>,
    },
    Mul {
        op: MulOp,
        src1: R,
        src2: R,
        dest: RegDest<IntReg>,
    },
    Jalr {
        base: R,
        offset: IntVal,
        pred: BtbPred,
    },
    Br {
        cmp: BrCmp,
        src1: R,
        src2: R,
        next: Addr,
        target: Addr,
        pred: bool,
    },
    Lda {
        base: R,
        offset: IntVal,
        dest: AddrHandle,
    },
    Sta {
        base: R,
        offset: IntVal,
        dest: AddrHandle,
    },
    Csr {
        op: CsrOp,
        csr: CsrAddr,
        src: S,
        dest: RegDest<IntReg>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Src<R> {
    Reg(R),
    Imm(IntVal),
}

impl<R> Src<R> {
    pub fn map_reg<U>(self, f: impl Fn(R) -> U) -> Src<U> {
        match self {
            Self::Reg(r) => Src::Reg(f(r)),
            Self::Imm(imm) => Src::Imm(imm),
        }
    }
}

impl<R: fmt::Display> fmt::Display for Src<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Reg(reg) => fmt::Display::fmt(reg, f),
            Self::Imm(imm) => fmt::Display::fmt(&imm.s(), f),
        }
    }
}

#[derive(Debug, derive_more::Display, Clone, Copy, PartialEq, Eq, Hash)]
#[display(rename_all = "lowercase")]
pub enum AluOp {
    Add,
    Sub,
    And,
    Or,
    Xor,
    Sll,
    Srl,
    Sra,
    Slt,
    Sltu,
    Addw,
    Subw,
    Sllw,
    Srlw,
    Sraw,
}

#[derive(Debug, derive_more::Display, Clone, Copy, PartialEq, Eq, Hash)]
#[display(rename_all = "lowercase")]
pub enum MulOp {
    Mul,
    Mulh,
    Mulhsu,
    Mulhu,
    Div,
    Divu,
    Rem,
    Remu,
    Mulw,
    Divw,
    Divuw,
    Remw,
    Remuw,
}

#[derive(Debug, derive_more::Display, Clone, Copy, PartialEq, Eq, Hash)]
#[display(rename_all = "lowercase")]
pub enum BrCmp {
    Eq,
    Ne,
    Lt,
    Ltu,
    Ge,
    Geu,
}

#[derive(Debug, derive_more::Display, Clone, Copy, PartialEq, Eq, Hash)]
#[display(rename_all = "lowercase")]
pub enum CsrOp {
    /// Atomic Read/Write
    Rw,
    /// Atomic Read and Set Bits
    Rs,
    /// Atomic Read and Clear Bits
    Rc,
}

impl<R: fmt::Display, S: fmt::Display, D: fmt::Display> fmt::Display for FeUop<R, S, D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut width = if f.alternate() { INST_NAME_WIDTH } else { 0 };
        match self {
            Self::Alu {
                op,
                src1,
                src2,
                dest,
            } => write!(f, "{:<width$} {}, {}, {}", op.to_string(), dest, src1, src2),
            Self::Mul {
                op,
                src1,
                src2,
                dest,
            } => write!(f, "{:<width$} {}, {}, {}", op.to_string(), dest, src1, src2),
            Self::Jal { target, link, .. } => {
                write!(f, "{:<width$} {}, {}", "jal", link, AddrView(*target))
            }
            Self::Jalr {
                base, offset, link, ..
            } => {
                write!(f, "{:<width$} {}, {}({})", "jalr", link, offset.s(), base)
            }
            Self::Br {
                cmp,
                src1,
                src2,
                target,
                ..
            } => {
                width = width.saturating_sub(1);
                write!(
                    f,
                    "b{:<width$} {}, {}, {}",
                    cmp.to_string(),
                    src1,
                    src2,
                    AddrView(*target)
                )
            }
            Self::Ld {
                size,
                signed,
                base,
                offset,
                dest,
            } => {
                width = width.saturating_sub(2);
                let signed = if *signed {
                    ""
                } else {
                    width = width.saturating_sub(1);
                    "u"
                };
                write!(
                    f,
                    "l{}{:<width$} {}, {}({})",
                    size,
                    signed,
                    dest,
                    offset.s(),
                    base
                )
            }
            Self::St {
                size,
                src,
                base,
                offset,
            } => {
                width = width.saturating_sub(1);
                write!(f, "s{:<width$} {}, {}({})", size, src, offset.s(), base)
            }
            Self::Csr { op, csr, src, dest } => {
                width = width.saturating_sub(3);
                write!(f, "csr{:<width$} {}{}{}", op.to_string(), csr, src, dest)
            }
        }
    }
}
