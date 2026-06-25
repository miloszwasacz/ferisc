use super::{Execute, Latency, WriteReg};
use crate::core::{
    csr::CsrFile,
    ee::rob::{EntryHandle, locks::ExecuteLock},
    reg::int::{IntReg, IntVal, Sint, Uint},
    uop::{int::*, issue::int::Uop},
};
use static_assertions::const_assert_eq;

impl Execute for Uop {
    type Reg = IntReg;

    fn latency(&self) -> Latency {
        match self {
            Uop::Alu { .. }
            | Uop::Jalr { .. }
            | Uop::Br { .. }
            | Uop::Lda { .. }
            | Uop::Sta { .. }
            | Uop::Csr { .. } => 0,
            //TODO Data-dependent latency
            Uop::Mul { op, .. } => match op {
                MulOp::Mul | MulOp::Mulh | MulOp::Mulhsu | MulOp::Mulhu | MulOp::Mulw => 2,
                MulOp::Div
                | MulOp::Divu
                | MulOp::Divw
                | MulOp::Divuw
                | MulOp::Rem
                | MulOp::Remu
                | MulOp::Remw
                | MulOp::Remuw => 3,
            },
        }
    }

    fn execute(
        self,
        mut rob_entry: EntryHandle,
        lock: &mut ExecuteLock<'_, '_, '_>,
        write_reg: &mut impl WriteReg<Self::Reg>,
        csrf: &mut CsrFile,
    ) {
        type Sword = i32;
        type Uword = u32;
        type Sqword = i128;
        type Uqword = u128;
        const_assert_eq!(size_of::<Sword>() * 2, size_of::<Sint>());
        const_assert_eq!(size_of::<Uword>() * 2, size_of::<Uint>());
        const_assert_eq!(size_of::<Sqword>(), size_of::<Sint>() * 2);
        const_assert_eq!(size_of::<Uqword>(), size_of::<Uint>() * 2);

        match self {
            Uop::Alu {
                op,
                src1,
                src2,
                dest,
            } => {
                const SHIFT_MASK: u64 = 0b111111;
                const WORD_SHIFT_MASK: u32 = 0b11111;

                write_reg.write(
                    *dest,
                    match op {
                        AluOp::Add => IntVal::signed(src1.s().wrapping_add(src2.s())),
                        AluOp::Sub => IntVal::signed(src1.s().wrapping_sub(src2.s())),
                        AluOp::And => IntVal::unsigned(src1.u() & src2.u()),
                        AluOp::Or => IntVal::unsigned(src1.u() | src2.u()),
                        AluOp::Xor => IntVal::unsigned(src1.u() ^ src2.u()),
                        AluOp::Sll => {
                            let sh = src2.u() & SHIFT_MASK;
                            IntVal::unsigned(src1.u() << sh)
                        }
                        AluOp::Srl => {
                            let sh = src2.u() & SHIFT_MASK;
                            IntVal::unsigned(src1.u() >> sh)
                        }
                        AluOp::Sra => {
                            let sh = src2.u() & SHIFT_MASK;
                            IntVal::signed(src1.s() >> sh)
                        }
                        AluOp::Slt => IntVal::unsigned(if src1.s() < src2.s() { 1 } else { 0 }),
                        AluOp::Sltu => IntVal::unsigned(if src1.u() < src2.u() { 1 } else { 0 }),
                        AluOp::Addw => {
                            let src1 = src1.s() as Sword;
                            let src2 = src2.s() as Sword;
                            IntVal::signed(src1.wrapping_add(src2) as _)
                        }
                        AluOp::Subw => {
                            let src1 = src1.s() as Sword;
                            let src2 = src2.s() as Sword;
                            IntVal::signed(src1.wrapping_sub(src2) as _)
                        }
                        AluOp::Sllw => {
                            let src1 = src1.u() as Uword;
                            let sh = src2.u() as Uword & WORD_SHIFT_MASK;
                            IntVal::unsigned((src1 << sh) as _)
                        }
                        AluOp::Srlw => {
                            let src1 = src1.u() as Uword;
                            let sh = src2.u() as Uword & WORD_SHIFT_MASK;
                            IntVal::unsigned((src1 >> sh) as _)
                        }
                        AluOp::Sraw => {
                            let src1 = src1.s() as Sword;
                            let sh = src2.u() as Uword & WORD_SHIFT_MASK;
                            IntVal::signed((src1 >> sh) as _)
                        }
                    },
                );
            }
            Uop::Mul {
                op,
                src1,
                src2,
                dest,
            } => {
                const MULH_SHIFT: u32 = Uint::BITS;

                write_reg.write(
                    *dest,
                    match op {
                        MulOp::Mul => IntVal::signed(src1.s().wrapping_mul(src2.s())),
                        MulOp::Mulh => {
                            let src1 = src1.s() as Sqword;
                            let src2 = src2.s() as Sqword;
                            let out = (src1 * src2) >> MULH_SHIFT;
                            IntVal::signed(out as _)
                        }
                        MulOp::Mulhsu => {
                            let src1 = src1.s() as Sqword;
                            let src2 = src2.u() as Uqword as Sqword;
                            let out = (src1 * src2) >> MULH_SHIFT;
                            IntVal::signed(out as _)
                        }
                        MulOp::Mulhu => {
                            let src1 = src1.u() as Uqword;
                            let src2 = src2.u() as Uqword;
                            let out = (src1 * src2) >> MULH_SHIFT;
                            IntVal::unsigned(out as _)
                        }
                        MulOp::Div => IntVal::signed(match src2.s() {
                            0 => -1,
                            src2 => src1.s().overflowing_div(src2).0,
                        }),
                        MulOp::Divu => {
                            IntVal::unsigned(src1.u().checked_div(src2.u()).unwrap_or(Uint::MAX))
                        }
                        MulOp::Rem => match src2.s() {
                            0 => src1,
                            src2 => IntVal::signed(src1.s().overflowing_rem(src2).0),
                        },
                        MulOp::Remu => src1
                            .u()
                            .checked_rem(src2.u())
                            .map(IntVal::unsigned)
                            .unwrap_or(src1),
                        MulOp::Mulw => {
                            let src1 = src1.s() as Sword;
                            let src2 = src2.s() as Sword;
                            IntVal::signed(src1.wrapping_mul(src2) as _)
                        }
                        MulOp::Divw => {
                            let src1 = src1.s() as Sword;
                            let src2 = src2.s() as Sword;
                            IntVal::signed(match src2 {
                                0 => -1,
                                src2 => src1.overflowing_div(src2).0,
                            } as _)
                        }
                        MulOp::Divuw => {
                            let src1 = src1.u() as Uword;
                            let src2 = src2.u() as Uword;
                            IntVal::unsigned(src1.checked_div(src2).unwrap_or(Uword::MAX) as _)
                        }
                        MulOp::Remw => {
                            let src1 = src1.s() as Sword;
                            let src2 = src2.s() as Sword;
                            IntVal::signed(match src2 {
                                0 => src1,
                                src2 => src1.overflowing_rem(src2).0,
                            } as _)
                        }
                        MulOp::Remuw => {
                            let src1 = src1.u() as Uword;
                            let src2 = src2.u() as Uword;
                            IntVal::unsigned(src1.checked_rem(src2).unwrap_or(src1) as _)
                        }
                    },
                )
            }
            Uop::Jalr { base, offset, pred } => {
                let target = base.addr().wrapping_add_signed(offset.s());
                if pred.is_none_or(|addr| addr != target) {
                    lock.jump(target, &rob_entry);
                    //TODO Notify BTB
                    //TODO Notify BP?
                }
            }
            Uop::Br {
                cmp,
                src1,
                src2,
                next,
                target,
                pred,
            } => {
                let out = match cmp {
                    BrCmp::Eq => src1.s() == src2.s(),
                    BrCmp::Ne => src1.s() != src2.s(),
                    BrCmp::Lt => src1.s() < src2.s(),
                    BrCmp::Ltu => src1.u() < src2.u(),
                    BrCmp::Ge => src1.s() >= src2.s(),
                    BrCmp::Geu => src1.u() >= src2.u(),
                };
                if out != pred {
                    let next = if out { target } else { next };
                    lock.jump(next, &rob_entry);
                    //TODO Notify BP
                    //TODO Notify BTB if taken
                }
            }
            Uop::Lda {
                base,
                offset,
                mut dest,
            }
            | Uop::Sta {
                base,
                offset,
                #[allow(unused_mut)]
                mut dest,
            } => {
                dest.write(base.addr().wrapping_add_signed(offset.s()));
            }
            Uop::Csr { op, csr, src, dest } => match csrf.access(csr, op, src) {
                Ok(out) => {
                    write_reg.write(*dest, out);
                }
                Err(ex) => {
                    rob_entry.throw(ex);
                    return;
                }
            },
        }
        rob_entry.mark_done();
    }
}
