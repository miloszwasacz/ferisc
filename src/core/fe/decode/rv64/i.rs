pub(in super::super) use self::generated::*;
use super::fmt::*;
use super::rv32::macros::*;
use crate::core::{
    Mode,
    reg::int::IntVal,
    uop::{
        Exception, SpecialUopKind,
        decode::{Kind, UopKind, int},
        int::*,
        predecode::Meta,
    },
};
use crate::mem::{ScalarSize, Size};

load! {
    pub(in super::super) lwu(Word, false)
    pub(in super::super) ld(DoubleWord, true)
}

store! {
    pub(in super::super) sd(DoubleWord)
}

shift_imm! {
    pub(in super::super) slli(Sll, 0b111111)
    pub(in super::super) srli(Srl, 0b111111)
    pub(in super::super) srai(Sra, 0b111111)
}

int_inst! {
    pub(in super::super) addiw(fmt: I, _, _): Alu {
        op: AluOp::Addw,
        src1: Src::Reg(fmt.rs1),
        src2: Src::Imm(fmt.imm),
        dest: fmt.rd,
    }
}

shift_imm! {
    pub(in super::super) slliw(Sllw, 0b11111)
    pub(in super::super) srliw(Srlw, 0b11111)
    pub(in super::super) sraiw(Sraw, 0b11111)
}

int_reg! {
    pub(in super::super) addw(Addw)
    pub(in super::super) subw(Subw)
    pub(in super::super) sllw(Sllw)
    pub(in super::super) srlw(Srlw)
    pub(in super::super) sraw(Sraw)
}

#[allow(unused_imports)]
mod generated {
    pub use super::super::rv32::i::*;
    use super::*;
    use super::{slli, srai, srli};

    include_generated!("decode/rv64i.rs");
}
