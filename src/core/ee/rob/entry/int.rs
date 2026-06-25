use super::Dest;
use crate::core::uop::rename::int::Uop as RUop;

impl From<RUop> for super::UopKind {
    fn from(value: RUop) -> Self {
        match value {
            RUop::Alu { dest, .. }
            | RUop::Mul { dest, .. }
            | RUop::Jal { link: dest, .. }
            | RUop::Jalr { link: dest, .. } => super::UopKind::Reg(Dest::Int(dest)),
            RUop::Br { .. } => super::UopKind::Br { done: false },
            RUop::Ld { dest, .. } => super::UopKind::Ld {
                dest: Dest::Int(dest),
                done: false,
            },
            RUop::St { .. } => super::UopKind::St { done: false },
            RUop::Csr { dest, csr, .. } => super::UopKind::Csr(Dest::Int(dest), csr),
        }
    }
}
