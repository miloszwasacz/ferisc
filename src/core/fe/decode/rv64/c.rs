pub(in super::super) use super::rv32::c::*;
use crate::core::{
    csr::m::misa::Extensions,
    fe::decode::fmt::*,
    reg::int::{IntReg, IntVal},
    uop::{
        Exception, Mode,
        decode::{UopKind, int},
        int::{AluOp, Src},
        predecode::Meta,
    },
};

fn c_ld(fmt: CL, _: Meta, _: Mode) -> UopKind {
    let imm6 = (fmt.imm5 & 0b11) << 6;
    let imm3 = (fmt.imm10 & 0b111) << 3;
    let offset = IntVal::unsigned((imm6 | imm3) as _);

    UopKind::Int(int::Uop::Ld {
        size: 8,
        signed: true,
        base: fmt.rs1,
        offset,
        dest: fmt.rd,
    })
}

fn c_sd(fmt: CS, _: Meta, _: Mode) -> UopKind {
    let imm6 = (fmt.imm5 & 0b11) << 6;
    let imm3 = (fmt.imm10 & 0b111) << 3;
    let offset = IntVal::unsigned((imm6 | imm3) as _);

    UopKind::Int(int::Uop::St {
        size: 8,
        src: fmt.rs2,
        base: fmt.rs1,
        offset,
    })
}

fn c_addiw(fmt: CI, _: Meta, _: Mode) -> UopKind {
    if fmt.rd_rs1 == IntReg::ZERO {
        return RESERVED;
    }

    let imm0 = fmt.imm2 as i16 & 0b11111;
    let mut imm_enc = fmt.imm12 as i16;
    let imm5 = imm_part!(1, imm_enc, sign extend) << 5;
    let imm = IntVal::signed((imm5 | imm0) as _);

    UopKind::Int(int::Uop::Alu {
        op: AluOp::Addw,
        src1: Src::Reg(fmt.rd_rs1),
        src2: Src::Imm(imm),
        dest: fmt.rd_rs1,
    })
}

fn c_subw(fmt: CA, _: Meta, _: Mode) -> UopKind {
    UopKind::Int(int::Uop::Alu {
        op: AluOp::Subw,
        src1: Src::Reg(fmt.rs2),
        src2: Src::Reg(fmt.rd_rs1),
        dest: fmt.rd_rs1,
    })
}

fn c_addw(fmt: CA, _: Meta, _: Mode) -> UopKind {
    UopKind::Int(int::Uop::Alu {
        op: AluOp::Addw,
        src1: Src::Reg(fmt.rs2),
        src2: Src::Reg(fmt.rd_rs1),
        dest: fmt.rd_rs1,
    })
}

fn c_ldsp(fmt: CI, _: Meta, _: Mode) -> UopKind {
    if fmt.rd_rs1 == IntReg::ZERO {
        return RESERVED;
    }

    let mut imm_enc = fmt.imm2;
    let imm6 = imm_part!(3, imm_enc) << 6;
    let imm3 = imm_part!(2, imm_enc) << 3;
    let imm5 = fmt.imm12 & 0b1;
    let offset = IntVal::unsigned((imm6 | imm5 | imm3) as _);

    UopKind::Int(int::Uop::Ld {
        size: 8,
        signed: true,
        base: IntReg::SP,
        offset,
        dest: fmt.rd_rs1,
    })
}

fn c_sdsp(fmt: CSS, _: Meta, _: Mode) -> UopKind {
    let mut imm_enc = fmt.imm7;
    let imm6 = imm_part!(3, imm_enc) << 6;
    let imm3 = imm_part!(3, imm_enc) << 3;
    let offset = IntVal::unsigned((imm6 | imm3) as _);

    UopKind::Int(int::Uop::St {
        size: 8,
        src: fmt.rs2,
        base: IntReg::SP,
        offset,
    })
}

pub fn decode(bytes: i16, meta: Meta, mode: Mode, ext: Extensions) -> Option<UopKind> {
    debug_assert!(
        ext.c(),
        "C extension decoder should not be called when C is not enabled"
    );

    zca::decode(bytes, meta, mode).or_else(|| {
        if ext.d() {
            zcd::decode(bytes, meta, mode)
        } else {
            None
        }
    })
}

#[allow(unused_imports)]
mod zca {
    use super::*;

    include_generated!("decode/rv64_zca.rs");
}

#[allow(unused_imports)]
mod zcd {
    use super::*;

    include_generated!("decode/zcd.rs");
}
