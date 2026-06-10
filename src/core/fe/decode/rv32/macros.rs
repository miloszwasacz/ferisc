macro_rules! int_inst {
    ($(
        $(#[$attr:meta])*
        $vis:vis $name:ident($fmt:ident: $fmt_ty:ty, $meta:pat, $mode:pat): $uop:ident { $( $fields:tt )* }
    )*) => {
        $(
        $( #[$attr] )*
        $vis fn $name($fmt: $fmt_ty, $meta: Meta, $mode: Mode) -> UopKind {
            UopKind::Int(int::Uop::$uop { $( $fields )* })
        }
        )*
    };
}
pub(in crate::core::fe::decode) use int_inst;

macro_rules! branch {
    ($( $vis:vis $name:ident($cmp:ident) )*) => {
        int_inst! {
            $(
            $vis $name(fmt: B, Meta { addr: pc, pred: btb_pred }, _): Br {
                cmp: BrCmp::$cmp,
                src1: fmt.rs1,
                src2: fmt.rs2,
                target: pc.wrapping_add_signed(fmt.imm.s()),
                // The prediction needs to be updated later, once the decoding is done,
                // since some BPs need the target address.
                pred: false,
                btb_pred,
            }
            )*
        }
    };
}
pub(in crate::core::fe::decode) use branch;

macro_rules! load {
    ($( $vis:vis $name:ident($size:literal, $signed:literal) )*) => {
        int_inst! {
            $(
            $vis $name(fmt: I, _, _): Ld {
                size: $size,
                signed: $signed,
                base: fmt.rs1,
                offset: fmt.imm,
                dest: fmt.rd,
            }
            )*
        }
    };
}
pub(in crate::core::fe::decode) use load;

macro_rules! store {
    ($( $vis:vis $name:ident($size:literal) )*) => {
        int_inst! {
            $(
            $vis $name(fmt: S, _, _): St {
                size: $size,
                src: fmt.rs2,
                base: fmt.rs1,
                offset: fmt.imm,
            }
            )*
        }
    };
}
pub(in crate::core::fe::decode) use store;

macro_rules! shift_imm {
    ($( $vis:vis $name:ident($op:ident, $mask:literal) )*) => {
        int_inst! {
            $(
            $vis $name(fmt: I, _, _): Alu {
                op: AluOp::$op,
                src1: Src::Reg(fmt.rs1),
                src2: Src::Imm(IntVal::unsigned(fmt.imm.u() & $mask)),
                dest: fmt.rd,
            }
            )*
        }
    };
}
pub(in crate::core::fe::decode) use shift_imm;

macro_rules! int_reg {
    ($( $vis:vis $name:ident($op:ident) )*) => {
        int_inst! {
            $(
            $vis $name(fmt: R, _, _): Alu {
                op: AluOp::$op,
                src1: Src::Reg(fmt.rs1),
                src2: Src::Reg(fmt.rs2),
                dest: fmt.rd,
            }
            )*
        }
    };
}
pub(in crate::core::fe::decode) use int_reg;
