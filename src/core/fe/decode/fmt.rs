//! Encoding formats

pub(crate) use self::fields::*;
use crate::core::reg::{
    RegRepr,
    int::{IntReg, IntVal},
};
use derive_more::{Deref, DerefMut};

mod fields;

pub trait EncFmt: Sized {
    type Enc: Copy;

    fn decode(enc: Self::Enc) -> Option<Self>;
}

//#region RVI

#[derive(Debug, Clone, Copy)]
pub struct R<R_ = IntReg> {
    pub rd: R_,
    pub rs1: R_,
    pub rs2: R_,
}

impl<R_: RegRepr> EncFmt for R<R_> {
    type Enc = i32;

    fn decode(mut enc: Self::Enc) -> Option<Self> {
        opcode!(enc);
        let rd = reg!(R_, 5, enc);
        funct3!(enc);
        let rs1 = reg!(R_, 5, enc);
        let rs2 = reg!(R_, 5, enc);

        Some(Self { rd, rs1, rs2 })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct I<R = IntReg> {
    pub rd: R,
    pub rs1: R,
    pub imm: IntVal,
}

impl<R: RegRepr> EncFmt for I<R> {
    type Enc = i32;

    fn decode(mut enc: Self::Enc) -> Option<Self> {
        opcode!(enc);
        let rd = reg!(R, 5, enc);
        funct3!(enc);
        let rs1 = reg!(R, 5, enc);
        let imm = IntVal::signed(enc as _);

        Some(Self { rd, rs1, imm })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct S<R = IntReg> {
    pub rs1: R,
    pub rs2: R,
    pub imm: IntVal,
}

impl<R: RegRepr> EncFmt for S<R> {
    type Enc = i32;

    fn decode(mut enc: Self::Enc) -> Option<Self> {
        opcode!(enc);
        let imm0 = field!(5, enc);
        funct3!(enc);
        let rs1 = reg!(R, 5, enc);
        let rs2 = reg!(R, 5, enc);
        let imm5 = enc << 5;
        let imm = IntVal::signed((imm5 | imm0) as _);

        Some(Self { rs1, rs2, imm })
    }
}

#[derive(Debug, Clone, Copy, Deref, DerefMut)]
pub struct B<R = IntReg>(S<R>);

impl<R: RegRepr> EncFmt for B<R> {
    type Enc = i32;

    fn decode(mut enc: Self::Enc) -> Option<Self> {
        opcode!(enc);
        let imm11 = field!(1, enc) << 11;
        let imm1 = field!(4, enc) << 1;
        funct3!(enc);
        let rs1 = reg!(R, 5, enc);
        let rs2 = reg!(R, 5, enc);
        let imm5 = field!(6, enc) << 5;
        let imm12 = enc << 12;
        let imm = IntVal::signed((imm12 | imm11 | imm5 | imm1) as _);

        Some(Self(S { rs1, rs2, imm }))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct U<R = IntReg> {
    pub rd: R,
    pub imm: IntVal,
}

impl<R: RegRepr> EncFmt for U<R> {
    type Enc = i32;

    fn decode(mut enc: Self::Enc) -> Option<Self> {
        opcode!(enc);
        let rd = reg!(R, 5, enc);
        let imm12 = enc << 12;
        let imm = IntVal::signed(imm12 as _);

        Some(Self { rd, imm })
    }
}

#[derive(Debug, Clone, Copy, Deref, DerefMut)]
pub struct J<R = IntReg>(U<R>);

impl<R: RegRepr> EncFmt for J<R> {
    type Enc = i32;

    fn decode(mut enc: Self::Enc) -> Option<Self> {
        opcode!(enc);
        let rd = reg!(R, 5, enc);
        let imm12 = field!(8, enc) << 12;
        let imm11 = field!(1, enc) << 11;
        let imm1 = field!(10, enc) << 1;
        let imm20 = enc << 20;
        let imm = IntVal::signed((imm20 | imm12 | imm11 | imm1) as _);

        Some(Self(U { rd, imm }))
    }
}

//#endregion

//#region RVC

#[derive(Debug, Clone, Copy)]
pub struct CR<R = IntReg> {
    pub rd_rs1: R,
    pub rs2: R,
}

impl<R: RegRepr> EncFmt for CR<R> {
    type Enc = i16;

    fn decode(mut enc: Self::Enc) -> Option<Self> {
        rvc_opcode!(enc);
        let rs2 = reg!(R, 5, enc);
        let rd_rs1 = reg!(R, 5, enc);

        Some(Self { rd_rs1, rs2 })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CI<R = IntReg> {
    pub rd_rs1: R,
    pub imm2: u16,
    pub imm12: u16,
}

impl<R: RegRepr> EncFmt for CI<R> {
    type Enc = i16;

    fn decode(mut enc: Self::Enc) -> Option<Self> {
        rvc_opcode!(enc);
        let imm2 = field!(5, enc) as _;
        let rd_rs1 = reg!(R, 5, enc);
        let imm12 = field!(1, enc) as _;

        Some(Self {
            rd_rs1,
            imm2,
            imm12,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CSS<R = IntReg> {
    pub rs2: R,
    pub imm7: u16,
}

impl<R: RegRepr> EncFmt for CSS<R> {
    type Enc = i16;

    fn decode(mut enc: Self::Enc) -> Option<Self> {
        rvc_opcode!(enc);
        let rs2 = reg!(R, 5, enc);
        let imm7 = field!(6, enc) as _;

        Some(Self { rs2, imm7 })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CIW<R = IntReg> {
    pub rd: R,
    pub imm5: u16,
}

impl<R: RegRepr> EncFmt for CIW<R> {
    type Enc = i16;

    fn decode(mut enc: Self::Enc) -> Option<Self> {
        rvc_opcode!(enc);
        let rd = rvc_reg!(R, enc);
        let imm5 = field!(8, enc) as _;

        Some(Self { rd, imm5 })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CL<R = IntReg> {
    pub rd: R,
    pub rs1: IntReg,
    pub imm5: u16,
    pub imm10: u16,
}

impl<R: RegRepr> EncFmt for CL<R> {
    type Enc = i16;

    fn decode(mut enc: Self::Enc) -> Option<Self> {
        rvc_opcode!(enc);
        let rd = rvc_reg!(R, enc);
        let imm5 = field!(2, enc) as _;
        let rs1 = rvc_reg!(IntReg, enc);
        let imm10 = field!(3, enc) as _;

        Some(Self {
            rd,
            rs1,
            imm5,
            imm10,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CS<R = IntReg> {
    pub rs1: IntReg,
    pub rs2: R,
    pub imm5: u16,
    pub imm10: u16,
}

impl<R: RegRepr> EncFmt for CS<R> {
    type Enc = i16;

    fn decode(mut enc: Self::Enc) -> Option<Self> {
        rvc_opcode!(enc);
        let rs2 = rvc_reg!(R, enc);
        let imm5 = field!(2, enc) as _;
        let rs1 = rvc_reg!(IntReg, enc);
        let imm10 = field!(3, enc) as _;

        Some(Self {
            rs1,
            rs2,
            imm5,
            imm10,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CA<R = IntReg> {
    pub rd_rs1: R,
    pub rs2: R,
}

impl<R: RegRepr> EncFmt for CA<R> {
    type Enc = i16;

    fn decode(mut enc: Self::Enc) -> Option<Self> {
        rvc_opcode!(enc);
        let rs2 = rvc_reg!(R, enc);
        funct2!(enc);
        let rd_rs1 = rvc_reg!(R, enc);

        Some(Self { rd_rs1, rs2 })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CB<R = IntReg> {
    pub rd_rs1: R,
    pub offset2: u16,
    pub offset10: u16,
}

impl<R: RegRepr> EncFmt for CB<R> {
    type Enc = i16;

    fn decode(mut enc: Self::Enc) -> Option<Self> {
        rvc_opcode!(enc);
        let offset2 = field!(5, enc) as _;
        let rd_rs1 = rvc_reg!(R, enc);
        let offset10 = field!(3, enc) as _;

        Some(Self {
            rd_rs1,
            offset2,
            offset10,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CJ {
    pub target2: u16,
}

impl EncFmt for CJ {
    type Enc = i16;

    fn decode(mut enc: Self::Enc) -> Option<Self> {
        rvc_opcode!(enc);
        let target2 = field!(11, enc) as _;

        Some(Self { target2 })
    }
}

//#endregion
