use crate::core::{csr::CsrAddr, reg::int::IntVal};
use crate::mem::Addr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Uop<R, S, D> {
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
    },
    Jalr {
        base: R,
        offset: IntVal,
        link: D,
        link_addr: Addr,
    },
    Br {
        cmp: BrCmp,
        src1: R,
        src2: R,
        target: Addr,
        pred: bool,
    },
    Ld {
        size: usize,
        signed: bool,
        base: R,
        offset: IntVal,
        dest: D,
    },
    St {
        size: usize,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BrCmp {
    Eq,
    Ne,
    Lt,
    Ltu,
    Ge,
    Geu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CsrOp {
    /// Atomic Read/Write
    Rw,
    /// Atomic Read and Set Bits
    Rs,
    /// Atomic Read and Clear Bits
    Rc,
}
