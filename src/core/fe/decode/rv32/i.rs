use super::fmt::*;
use super::macros::*;
use crate::core::{
    Mode,
    reg::int::IntVal,
    uop::{
        Exception,
        decode::{UopKind, int},
        fence::Fence,
        int::*,
        predecode::Meta,
    },
};

int_inst! {
    pub(in super::super) lui(fmt: U, _, _): Alu {
        op: AluOp::Add,
        src1: Src::Imm(IntVal::unsigned(0)),
        src2: Src::Imm(fmt.imm),
        dest: fmt.rd,
    }

    #[allow(unused_variables)]
    pub(in super::super) auipc(fmt: U, Meta { addr: pc, .. }, _): Alu {
        op: AluOp::Add,
        src1: Src::Imm(IntVal::address(pc)),
        src2: Src::Imm(fmt.imm),
        dest: fmt.rd,
    }

    #[allow(unused_variables)]
    pub(in super::super) jal(fmt: J, Meta { addr: pc, .. }, _): Jal {
        target: pc.wrapping_add_signed(fmt.imm.s()),
        link: fmt.rd,
        link_addr: pc.wrapping_add(4),
    }

    #[allow(unused_variables)]
    pub(in super::super) jalr(fmt: I, Meta { addr: pc, pred }, _): Jalr {
        base: fmt.rs1,
        offset: fmt.imm,
        link: fmt.rd,
        link_addr: pc.wrapping_add(4),
        pred,
    }
}

// The branch predictions need to be updated later, once the decoding is done,
// since some BPs need the target address.
branch! {
    pub(in super::super) beq(Eq)
    pub(in super::super) bne(Ne)
    pub(in super::super) blt(Lt)
    pub(in super::super) bge(Ge)
    pub(in super::super) bltu(Ltu)
    pub(in super::super) bgeu(Geu)
}

load! {
    pub(in super::super) lb(1, true)
    pub(in super::super) lh(2, true)
    pub(in super::super) lw(4, true)
    pub(in super::super) lbu(1, false)
    pub(in super::super) lhu(2, false)
}

store! {
    pub(in super::super) sb(1)
    pub(in super::super) sh(2)
    pub(in super::super) sw(4)
}

int_inst! {
    pub(in super::super) addi(fmt: I, _, _): Alu {
        op: AluOp::Add,
        src1: Src::Reg(fmt.rs1),
        src2: Src::Imm(fmt.imm),
        dest: fmt.rd,
    }

    pub(in super::super) slti(fmt: I, _, _): Alu {
        op: AluOp::Slt,
        src1: Src::Reg(fmt.rs1),
        src2: Src::Imm(fmt.imm),
        dest: fmt.rd,
    }

    pub(in super::super) sltiu(fmt: I, _, _): Alu {
        op: AluOp::Sltu,
        src1: Src::Reg(fmt.rs1),
        src2: Src::Imm(fmt.imm),
        dest: fmt.rd,
    }

    pub(in super::super) xori(fmt: I, _, _): Alu {
        op: AluOp::Xor,
        src1: Src::Reg(fmt.rs1),
        src2: Src::Imm(fmt.imm),
        dest: fmt.rd,
    }

    pub(in super::super) ori(fmt: I, _, _): Alu {
        op: AluOp::Or,
        src1: Src::Reg(fmt.rs1),
        src2: Src::Imm(fmt.imm),
        dest: fmt.rd,
    }

    pub(in super::super) andi(fmt: I, _, _): Alu {
        op: AluOp::And,
        src1: Src::Reg(fmt.rs1),
        src2: Src::Imm(fmt.imm),
        dest: fmt.rd,
    }
}

shift_imm! {
    pub(in super::super) slli(Sll, 0b11111)
    pub(in super::super) srli(Srl, 0b11111)
    pub(in super::super) srai(Sra, 0b11111)
}

int_reg! {
    pub(in super::super) add(Add)
    pub(in super::super) sub(Sub)
    pub(in super::super) sll(Sll)
    pub(in super::super) slt(Slt)
    pub(in super::super) sltu(Sltu)
    pub(in super::super) xor(Xor)
    pub(in super::super) srl(Srl)
    pub(in super::super) sra(Sra)
    pub(in super::super) or(Or)
    pub(in super::super) and(And)
}

pub(in super::super) fn fence(fmt: I, _: Meta, _: Mode) -> UopKind {
    UopKind::MemOrd(Fence::decode(fmt.imm.u() as u8))
}

pub(in super::super) fn fence_tso(fmt: I, meta: Meta, mode: Mode) -> UopKind {
    //TODO Better FENCE.TSO
    fence(fmt, meta, mode)
}

pub(in super::super) fn pause(_: I, _: Meta, _: Mode) -> UopKind {
    //TODO Pause
    UopKind::Except(Exception::Illegal)
}

pub(in super::super) fn ecall(_: I, _: Meta, mode: Mode) -> UopKind {
    UopKind::Except(match mode {
        Mode::M => Exception::EcallM,
    })
}

pub(in super::super) fn ebreak(_: I, _: Meta, _: Mode) -> UopKind {
    UopKind::Except(Exception::Breakpoint)
}

pub(in super::super) fn mret(_: I, _: Meta, _: Mode) -> UopKind {
    todo!("mret")
}

pub(in super::super) fn wfi(_: I, _: Meta, _: Mode) -> UopKind {
    todo!("wfi")
}

include_generated!("decode/rv32i.rs");
