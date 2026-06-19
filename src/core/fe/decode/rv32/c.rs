#![allow(unused_variables, unused_qualifications)] //TODO Remove

use crate::core::{
    Mode,
    csr::m::misa::Extensions,
    fe::decode::fmt::*,
    reg::int::{IntReg, IntVal, Sint},
    uop::{
        Exception,
        decode::{UopKind, int},
        int::*,
        predecode::Meta,
    },
};

//TODO Replace with the actual FpReg
pub(in super::super) type FpReg = IntReg;

macro_rules! imm_part {
    ($width:literal, $imm:expr) => {{
        let mask = (1 << $width) - 1;
        let part = $imm & mask;
        #[allow(unused)]
        {
            $imm >>= $width;
        }
        part
    }};
    ($width:literal, $imm:expr, sign extend) => {{
        let part = imm_part!($width, $imm);
        part.rotate_right($width) >> (16 - $width)
    }};
}
pub(in super::super) use imm_part;

pub(in super::super) const RESERVED: UopKind = UopKind::Except(Exception::Illegal);

pub(in super::super) fn c_addi4spn(fmt: CIW, _: Meta, _: Mode) -> UopKind {
    let mut imm_enc = fmt.imm5;
    let imm3 = imm_part!(1, imm_enc) << 3;
    let imm2 = imm_part!(1, imm_enc) << 2;
    let imm6 = imm_part!(4, imm_enc) << 6;
    let imm4 = imm_part!(2, imm_enc) << 4;
    let imm = IntVal::unsigned((imm6 | imm4 | imm3 | imm2) as _);
    if imm.u() == 0 {
        return RESERVED;
    }

    UopKind::Int(int::Uop::Alu {
        op: AluOp::Add,
        src1: Src::Reg(IntReg::SP),
        src2: Src::Imm(imm),
        dest: fmt.rd,
    })
}

pub(in super::super) fn c_fld(fmt: CL<FpReg>, _: Meta, _: Mode) -> UopKind {
    let imm6 = (fmt.imm5 & 0b11) << 6;
    let imm3 = (fmt.imm10 & 0b111) << 3;
    let offset = IntVal::unsigned((imm6 | imm3) as _);

    todo!("c.fld decode")
    // UopKind::Fp(fp::Uop::Ld {
    //     size: 8,
    //     signed: true,
    //     base: fmt.rs1,
    //     offset,
    //     dest: fmt.rd,
    // })
}

pub(in super::super) fn c_lw(fmt: CL, _: Meta, _: Mode) -> UopKind {
    let mut imm_enc = fmt.imm5;
    let imm6 = imm_part!(1, imm_enc) << 6;
    let imm2 = imm_part!(1, imm_enc) << 2;
    let imm3 = (fmt.imm10 & 0b111) << 3;
    let offset = IntVal::unsigned((imm6 | imm3 | imm2) as _);

    UopKind::Int(int::Uop::Ld {
        size: 4,
        signed: true,
        base: fmt.rs1,
        offset,
        dest: fmt.rd,
    })
}

fn c_flw(fmt: CL<FpReg>, _: Meta, _: Mode) -> UopKind {
    let mut imm_enc = fmt.imm5;
    let imm6 = imm_part!(1, imm_enc) << 6;
    let imm2 = imm_part!(1, imm_enc) << 2;
    let imm3 = (fmt.imm10 & 0b111) << 3;
    let offset = IntVal::unsigned((imm6 | imm3 | imm2) as _);

    todo!("c.flw decode")
    // UopKind::Fp(fp::Uop::Ld {
    //     size: 4,
    //     signed: true,
    //     base: fmt.rs1,
    //     offset,
    //     dest: fmt.rd,
    // })
}

pub(in super::super) fn c_fsd(fmt: CS<FpReg>, _: Meta, _: Mode) -> UopKind {
    let imm6 = (fmt.imm5 & 0b11) << 6;
    let imm3 = (fmt.imm10 & 0b111) << 3;
    let offset = IntVal::unsigned((imm6 | imm3) as _);

    todo!("c.fsd decode")
    // UopKind::Fp(fp::Uop::St {
    //     size: 8,
    //     src: fmt.rs2,
    //     base: fmt.rs1,
    //     offset,
    // })
}

pub(in super::super) fn c_sw(fmt: CS, _: Meta, _: Mode) -> UopKind {
    let mut imm_enc = fmt.imm5;
    let imm6 = imm_part!(1, imm_enc) << 6;
    let imm2 = imm_part!(1, imm_enc) << 2;
    let imm3 = (fmt.imm10 & 0b111) << 3;
    let offset = IntVal::unsigned((imm6 | imm3 | imm2) as _);

    UopKind::Int(int::Uop::St {
        size: 4,
        src: fmt.rs2,
        base: fmt.rs1,
        offset,
    })
}

fn c_fsw(fmt: CS<FpReg>, _: Meta, _: Mode) -> UopKind {
    let mut imm_enc = fmt.imm5;
    let imm6 = imm_part!(1, imm_enc) << 6;
    let imm2 = imm_part!(1, imm_enc) << 2;
    let imm3 = (fmt.imm10 & 0b111) << 3;
    let offset = IntVal::unsigned((imm6 | imm3 | imm2) as _);

    todo!("c.fsw decode")
    // UopKind::Fp(fp::Uop::St {
    //     size: 4,
    //     src: fmt.rs2,
    //     base: fmt.rs1,
    //     offset,
    // })
}

pub(in super::super) fn c_nop(fmt: CI, meta: Meta, mode: Mode) -> UopKind {
    //TODO Better NOP
    //TODO Hints if imm == 0
    c_addi(fmt, meta, mode)
}

pub(in super::super) fn c_addi(fmt: CI, _: Meta, _: Mode) -> UopKind {
    let imm0 = fmt.imm2 as i16 & 0b11111;
    let mut imm_enc = fmt.imm12 as i16;
    let imm5 = imm_part!(1, imm_enc, sign extend) << 5;
    let imm = IntVal::signed((imm5 | imm0) as _);
    // debug_assert_ne!(fmt.rd_rs1, IntReg::ZERO, "should be c.nop"); //TODO Better NOP
    //TODO Hints if imm == 0

    UopKind::Int(int::Uop::Alu {
        op: AluOp::Add,
        src1: Src::Reg(fmt.rd_rs1),
        src2: Src::Imm(imm),
        dest: fmt.rd_rs1,
    })
}

fn c_jal(fmt: CJ, meta: Meta, _: Mode) -> UopKind {
    let mut imm_enc = fmt.target2 as i16;
    let imm5 = imm_part!(1, imm_enc) << 5;
    let imm1 = imm_part!(3, imm_enc) << 1;
    let imm7 = imm_part!(1, imm_enc) << 7;
    let imm6 = imm_part!(1, imm_enc) << 6;
    let imm10 = imm_part!(1, imm_enc) << 10;
    let imm8 = imm_part!(2, imm_enc) << 8;
    let imm4 = imm_part!(1, imm_enc) << 4;
    let imm11 = imm_part!(1, imm_enc, sign extend) << 11;
    let offset = imm11 | imm10 | imm8 | imm7 | imm6 | imm5 | imm4 | imm1;
    let target = meta.addr.wrapping_add_signed(offset as _);

    UopKind::Int(int::Uop::Jal {
        target,
        link: IntReg::RA,
        link_addr: meta.addr.wrapping_add(2),
    })
}

pub(in super::super) fn c_li(fmt: CI, _: Meta, _: Mode) -> UopKind {
    //TODO Hint if rd == x0

    let imm0 = fmt.imm2 as i16 & 0b11111;
    let mut imm_enc = fmt.imm12 as i16;
    let imm5 = imm_part!(1, imm_enc, sign extend) << 5;
    let imm = IntVal::signed((imm5 | imm0) as _);

    UopKind::Int(int::Uop::Alu {
        op: AluOp::Add,
        src1: Src::Reg(IntReg::ZERO),
        src2: Src::Imm(imm),
        dest: fmt.rd_rs1,
    })
}

pub(in super::super) fn c_addi16sp(fmt: CI, _: Meta, _: Mode) -> UopKind {
    debug_assert_eq!(fmt.rd_rs1, IntReg::SP);

    let mut imm_enc = fmt.imm2 as i16;
    let imm5 = imm_part!(1, imm_enc, sign extend) << 5;
    let imm7 = imm_part!(2, imm_enc) << 7;
    let imm6 = imm_part!(1, imm_enc) << 6;
    let imm4 = imm_part!(1, imm_enc) << 4;
    let mut imm_enc = fmt.imm12 as i16;
    let imm9 = imm_part!(1, imm_enc, sign extend) << 9;
    let imm = IntVal::signed((imm9 | imm7 | imm6 | imm5 | imm4) as _);
    if imm.u() == 0 {
        return RESERVED;
    }

    UopKind::Int(int::Uop::Alu {
        op: AluOp::Add,
        src1: Src::Reg(IntReg::SP),
        src2: Src::Imm(imm),
        dest: IntReg::SP,
    })
}

pub(in super::super) fn c_lui(fmt: CI, _: Meta, _: Mode) -> UopKind {
    let imm12 = {
        let imm12 = fmt.imm2 as i16 & 0b11111;
        let mut imm_enc = fmt.imm12 as i16;
        let imm17 = imm_part!(1, imm_enc, sign extend) << 5;
        (imm17 | imm12) as Sint
    } << 12;
    let imm = IntVal::signed(imm12);
    if imm.u() == 0 {
        return RESERVED;
    }
    debug_assert_ne!(fmt.rd_rs1, IntReg::SP, "should be c.addi16sp");
    //TODO Hint if rd == x0

    UopKind::Int(int::Uop::Alu {
        op: AluOp::Add,
        src1: Src::Imm(IntVal::unsigned(0)),
        src2: Src::Imm(imm),
        dest: fmt.rd_rs1,
    })
}

pub(in super::super) fn c_srli(fmt: CB, _: Meta, _: Mode) -> UopKind {
    let imm0 = fmt.offset2 & 0b11111;
    let imm5 = ((fmt.offset10 >> 2) & 0b1) << 5;
    let imm = IntVal::unsigned((imm5 | imm0) as _);
    //TODO Hints if imm == 0

    UopKind::Int(int::Uop::Alu {
        op: AluOp::Srl,
        src1: Src::Reg(fmt.rd_rs1),
        src2: Src::Imm(imm),
        dest: fmt.rd_rs1,
    })
}

pub(in super::super) fn c_srai(fmt: CB, _: Meta, _: Mode) -> UopKind {
    let imm0 = fmt.offset2 & 0b11111;
    let imm5 = ((fmt.offset10 >> 2) & 0b1) << 5;
    let imm = IntVal::unsigned((imm5 | imm0) as _);
    //TODO Hints if imm == 0

    UopKind::Int(int::Uop::Alu {
        op: AluOp::Sra,
        src1: Src::Reg(fmt.rd_rs1),
        src2: Src::Imm(imm),
        dest: fmt.rd_rs1,
    })
}

pub(in super::super) fn c_andi(fmt: CB, _: Meta, _: Mode) -> UopKind {
    let imm0 = fmt.offset2 as i16 & 0b11111;
    let mut imm_enc = fmt.offset10 as i16 >> 2;
    let imm5 = imm_part!(1, imm_enc, sign extend) << 5;
    let imm = IntVal::signed((imm5 | imm0) as _);
    //TODO Hints if imm == 0

    UopKind::Int(int::Uop::Alu {
        op: AluOp::Srl,
        src1: Src::Reg(fmt.rd_rs1),
        src2: Src::Imm(imm),
        dest: fmt.rd_rs1,
    })
}

pub(in super::super) fn c_sub(fmt: CA, _: Meta, _: Mode) -> UopKind {
    UopKind::Int(int::Uop::Alu {
        op: AluOp::Sub,
        src1: Src::Reg(fmt.rs2),
        src2: Src::Reg(fmt.rd_rs1),
        dest: fmt.rd_rs1,
    })
}

pub(in super::super) fn c_xor(fmt: CA, _: Meta, _: Mode) -> UopKind {
    UopKind::Int(int::Uop::Alu {
        op: AluOp::Xor,
        src1: Src::Reg(fmt.rs2),
        src2: Src::Reg(fmt.rd_rs1),
        dest: fmt.rd_rs1,
    })
}

pub(in super::super) fn c_or(fmt: CA, _: Meta, _: Mode) -> UopKind {
    UopKind::Int(int::Uop::Alu {
        op: AluOp::Or,
        src1: Src::Reg(fmt.rs2),
        src2: Src::Reg(fmt.rd_rs1),
        dest: fmt.rd_rs1,
    })
}

pub(in super::super) fn c_and(fmt: CA, _: Meta, _: Mode) -> UopKind {
    UopKind::Int(int::Uop::Alu {
        op: AluOp::And,
        src1: Src::Reg(fmt.rs2),
        src2: Src::Reg(fmt.rd_rs1),
        dest: fmt.rd_rs1,
    })
}

pub(in super::super) fn c_subw(_: CA, _: Meta, _: Mode) -> UopKind {
    RESERVED
}

pub(in super::super) fn c_addw(_: CA, _: Meta, _: Mode) -> UopKind {
    RESERVED
}

pub(in super::super) fn c_j(fmt: CJ, meta: Meta, _: Mode) -> UopKind {
    let mut imm_enc = fmt.target2 as i16;
    let imm5 = imm_part!(1, imm_enc) << 5;
    let imm1 = imm_part!(3, imm_enc) << 1;
    let imm7 = imm_part!(1, imm_enc) << 7;
    let imm6 = imm_part!(1, imm_enc) << 6;
    let imm10 = imm_part!(1, imm_enc) << 10;
    let imm8 = imm_part!(2, imm_enc) << 8;
    let imm4 = imm_part!(1, imm_enc) << 4;
    let imm11 = imm_part!(1, imm_enc, sign extend) << 11;
    let offset = imm11 | imm10 | imm8 | imm7 | imm6 | imm5 | imm4 | imm1;
    let target = meta.addr.wrapping_add_signed(offset as _);

    UopKind::Int(int::Uop::Jal {
        target,
        link: IntReg::ZERO,
        link_addr: meta.addr.wrapping_add(2),
    })
}

pub(in super::super) fn c_beqz(
    fmt: CB,
    Meta {
        addr: pc,
        pred: btb_pred,
    }: Meta,
    _: Mode,
) -> UopKind {
    let mut imm_enc = fmt.offset2 as i16;
    let imm5 = imm_part!(1, imm_enc) << 5;
    let imm1 = imm_part!(2, imm_enc) << 1;
    let imm6 = imm_part!(2, imm_enc) << 6;
    let mut imm_enc = fmt.offset10 as i16;
    let imm3 = imm_part!(2, imm_enc) << 3;
    let imm8 = imm_part!(1, imm_enc, sign extend) << 8;
    let offset = imm8 | imm6 | imm5 | imm3 | imm1;
    let target = pc.wrapping_add_signed(offset as _);

    UopKind::Int(int::Uop::Br {
        cmp: BrCmp::Eq,
        src1: fmt.rd_rs1,
        src2: IntReg::ZERO,
        target,
        // The prediction needs to be updated later, once the decoding is done,
        // since some BPs need the target address.
        pred: false,
        btb_pred,
    })
}

pub(in super::super) fn c_bnez(
    fmt: CB,
    Meta {
        addr: pc,
        pred: btb_pred,
    }: Meta,
    _: Mode,
) -> UopKind {
    let mut imm_enc = fmt.offset2 as i16;
    let imm5 = imm_part!(1, imm_enc) << 5;
    let imm1 = imm_part!(2, imm_enc) << 1;
    let imm6 = imm_part!(2, imm_enc) << 6;
    let mut imm_enc = fmt.offset10 as i16;
    let imm3 = imm_part!(2, imm_enc) << 3;
    let imm8 = imm_part!(1, imm_enc, sign extend) << 8;
    let offset = imm8 | imm6 | imm5 | imm3 | imm1;
    let target = pc.wrapping_add_signed(offset as _);

    UopKind::Int(int::Uop::Br {
        cmp: BrCmp::Ne,
        src1: fmt.rd_rs1,
        src2: IntReg::ZERO,
        target,
        // The prediction needs to be updated later, once the decoding is done,
        // since some BPs need the target address.
        pred: false,
        btb_pred,
    })
}

pub(in super::super) fn c_slli(fmt: CI, _: Meta, _: Mode) -> UopKind {
    let imm0 = fmt.imm2 & 0b11111;
    let imm5 = (fmt.imm12 & 0b1) << 5;
    let imm = IntVal::unsigned((imm5 | imm0) as _);
    //TODO Hints if imm == 0 or rd == x0

    UopKind::Int(int::Uop::Alu {
        op: AluOp::Sll,
        src1: Src::Reg(fmt.rd_rs1),
        src2: Src::Imm(imm),
        dest: fmt.rd_rs1,
    })
}

pub(in super::super) fn c_fldsp(fmt: CI<FpReg>, _: Meta, _: Mode) -> UopKind {
    let mut imm_enc = fmt.imm2;
    let imm6 = imm_part!(3, imm_enc) << 6;
    let imm3 = imm_part!(2, imm_enc) << 3;
    let imm5 = fmt.imm12 & 0b1;
    let offset = IntVal::unsigned((imm6 | imm5 | imm3) as _);

    todo!("c.fldsp decode")
    // UopKind::Fp(fp::Uop::Ld {
    //     size: 8,
    //     base: IntReg::SP,
    //     offset,
    //     dest: fmt.rd_rs1,
    // })
}

pub(in super::super) fn c_lwsp(fmt: CI, _: Meta, _: Mode) -> UopKind {
    if fmt.rd_rs1 == IntReg::ZERO {
        return RESERVED;
    }

    let mut imm_enc = fmt.imm2;
    let imm6 = imm_part!(2, imm_enc) << 6;
    let imm2 = imm_part!(3, imm_enc) << 2;
    let imm5 = fmt.imm12 & 0b1;
    let offset = IntVal::unsigned((imm6 | imm5 | imm2) as _);

    UopKind::Int(int::Uop::Ld {
        size: 4,
        signed: true,
        base: IntReg::SP,
        offset,
        dest: fmt.rd_rs1,
    })
}

fn c_flwsp(fmt: CI<FpReg>, _: Meta, _: Mode) -> UopKind {
    let mut imm_enc = fmt.imm2;
    let imm6 = imm_part!(2, imm_enc) << 6;
    let imm2 = imm_part!(3, imm_enc) << 2;
    let imm5 = fmt.imm12 & 0b1;
    let offset = IntVal::unsigned((imm6 | imm5 | imm2) as _);

    todo!("c.flwsp decode")
    // UopKind::Fp(fp::Uop::Ld {
    //     size: 4,
    //     base: IntReg::SP,
    //     offset,
    //     dest: fmt.rd_rs1,
    // })
}

pub(in super::super) fn c_jr(fmt: CR, Meta { addr: pc, pred }: Meta, _: Mode) -> UopKind {
    debug_assert_eq!(fmt.rs2, IntReg::ZERO);
    if fmt.rd_rs1 == IntReg::ZERO {
        return RESERVED;
    }

    UopKind::Int(int::Uop::Jalr {
        base: fmt.rd_rs1,
        offset: IntVal::signed(0),
        link: IntReg::ZERO,
        link_addr: pc.wrapping_add(2),
        pred,
    })
}

pub(in super::super) fn c_mv(fmt: CR, _: Meta, _: Mode) -> UopKind {
    debug_assert_ne!(fmt.rs2, IntReg::ZERO, "should be c.jr");
    //TODO Hints if rd == x0

    // We expand to `mv rd, rs2` instead of `add rd, x0, rs2` to make register
    // renaming easier, as mentioned in the Unprivileged ISA documentation.
    UopKind::Int(int::Uop::Alu {
        op: AluOp::Add,
        src1: Src::Reg(fmt.rs2),
        src2: Src::Imm(IntVal::unsigned(0)),
        dest: fmt.rd_rs1,
    })
}

pub(in super::super) fn c_ebreak(fmt: CR, _: Meta, _: Mode) -> UopKind {
    debug_assert_eq!(fmt.rs2, IntReg::ZERO);
    debug_assert_eq!(fmt.rd_rs1, IntReg::ZERO);

    UopKind::Except(Exception::Breakpoint)
}

pub(in super::super) fn c_jalr(fmt: CR, Meta { addr: pc, pred }: Meta, _: Mode) -> UopKind {
    debug_assert_eq!(fmt.rs2, IntReg::ZERO);
    debug_assert_ne!(fmt.rd_rs1, IntReg::ZERO, "should be c.ebreak");

    UopKind::Int(int::Uop::Jalr {
        base: fmt.rd_rs1,
        offset: IntVal::signed(0),
        link: IntReg::RA,
        link_addr: pc.wrapping_add(2),
        pred,
    })
}

pub(in super::super) fn c_add(fmt: CR, _: Meta, _: Mode) -> UopKind {
    debug_assert_ne!(fmt.rs2, IntReg::ZERO, "should be c.jalr or c.ebreak");
    //TODO Hints if rd == x0

    UopKind::Int(int::Uop::Alu {
        op: AluOp::Add,
        src1: Src::Reg(fmt.rs2),
        src2: Src::Reg(fmt.rd_rs1),
        dest: fmt.rd_rs1,
    })
}

pub(in super::super) fn c_fsdsp(fmt: CSS<FpReg>, _: Meta, _: Mode) -> UopKind {
    let mut imm_enc = fmt.imm7;
    let imm6 = imm_part!(3, imm_enc) << 6;
    let imm3 = imm_part!(3, imm_enc) << 3;
    let offset = IntVal::unsigned((imm6 | imm3) as _);

    todo!("c.fsdsp decode")
    // UopKind::Int(fp::Uop::St {
    //     size: 8,
    //     src: fmt.rs2,
    //     base: IntReg::SP,
    //     offset,
    // })
}

pub(in super::super) fn c_swsp(fmt: CSS, _: Meta, _: Mode) -> UopKind {
    let mut imm_enc = fmt.imm7;
    let imm6 = imm_part!(2, imm_enc) << 6;
    let imm2 = imm_part!(4, imm_enc) << 2;
    let offset = IntVal::unsigned((imm6 | imm2) as _);

    UopKind::Int(int::Uop::St {
        size: 4,
        src: fmt.rs2,
        base: IntReg::SP,
        offset,
    })
}

fn c_fswsp(fmt: CSS<FpReg>, _: Meta, _: Mode) -> UopKind {
    let mut imm_enc = fmt.imm7;
    let imm6 = imm_part!(2, imm_enc) << 6;
    let imm2 = imm_part!(4, imm_enc) << 2;
    let offset = IntVal::unsigned((imm6 | imm2) as _);

    todo!("c.fswsp decode")
    // UopKind::Int(fp::Uop::St {
    //     size: 4,
    //     src: fmt.rs2,
    //     base: IntReg::SP,
    //     offset,
    // })
}

pub fn decode(bytes: i16, meta: Meta, mode: Mode, ext: Extensions) -> Option<UopKind> {
    debug_assert!(
        ext.c(),
        "C extension decoder should not be called when C is not enabled"
    );

    zca::decode(bytes, meta, mode)
        .or_else(|| {
            if ext.f() {
                zcf::decode(bytes, meta, mode)
            } else {
                None
            }
        })
        .or_else(|| {
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

    include_generated!("decode/rv32_zca.rs");
}

#[allow(unused_imports)]
mod zcf {
    use super::*;

    include_generated!("decode/zcf.rs");
}

#[allow(unused_imports)]
mod zcd {
    use super::*;

    include_generated!("decode/zcd.rs");
}
