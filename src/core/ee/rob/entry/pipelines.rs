use super::{Dest, UopKind};
use crate::core::{
    ee::pipeline,
    uop::{SpecialUopKind, rename},
};

impl From<rename::UopKind> for UopKind {
    fn from(value: rename::UopKind) -> Self {
        match value {
            rename::UopKind::Pipeline(kind) => match kind {
                pipeline::Kind::Int(int) => int.into(),
            },
            rename::UopKind::Special(kind) => match kind {
                SpecialUopKind::MemOrd(fence) => UopKind::MemOrd(fence),
                SpecialUopKind::Except(ex) => UopKind::Except(ex),
            },
        }
    }
}

mod int {
    use super::*;
    use rename::int::Uop;

    impl From<Uop> for UopKind {
        fn from(value: Uop) -> Self {
            match value {
                Uop::Alu { dest, .. }
                | Uop::Mul { dest, .. }
                | Uop::Jal { link: dest, .. }
                | Uop::Jalr { link: dest, .. } => UopKind::Reg(Dest::Int(dest)),
                Uop::Br { .. } => UopKind::Br,
                Uop::Ld { dest, .. } => UopKind::Ld(Dest::Int(dest)),
                Uop::St { .. } => UopKind::St,
                Uop::Csr { dest, csr, .. } => UopKind::Csr(Dest::Int(dest), csr),
            }
        }
    }
}
