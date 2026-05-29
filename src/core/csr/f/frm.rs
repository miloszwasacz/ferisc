use super::fcsr::Fcsr;
use crate::core::{csr::CsrAlias, reg::int::IntVal, uop::Exception};
use strum::FromRepr;

#[derive(Debug, Clone, Copy, Default)]
pub struct Frm;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, FromRepr)]
#[repr(u8)]
pub enum Rm {
    /// Round to Nearest, ties to Even
    #[default]
    RNE = 0b000,
    /// Round towards Zero
    RTZ = 0b001,
    /// Round Down (towards -∞)
    RDN = 0b010,
    /// Round Up (towards +∞)
    RUP = 0b011,
    /// Round to Nearest, ties to Max Magnitude
    RMM = 0b100,
    /// In instruction’s `rm` field, selects dynamic rounding mode;
    /// In Rounding Mode register, _reserved_.
    DYN = 0b111,
}

impl CsrAlias for Frm {
    type Target = Fcsr;
    type Alias = Rm;

    fn alias(target: &Self::Target) -> &Self::Alias {
        &target.rm
    }

    fn dealias(alias: Self::Alias, current: &Self::Target) -> Self::Target {
        Self::Target {
            rm: alias,
            ..*current
        }
    }

    fn alias_as_int(alias: &Self::Alias) -> IntVal {
        (*alias).into()
    }

    fn dealias_int(aliased: IntVal) -> IntVal {
        aliased << 5
    }
}

impl TryFrom<IntVal> for Rm {
    type Error = Exception;

    fn try_from(value: IntVal) -> Result<Self, Self::Error> {
        Self::from_repr(value.u() as u8 & 0b111)
            .filter(|rm| *rm != Rm::DYN)
            .ok_or(Exception::Illegal)
    }
}

impl From<Rm> for IntVal {
    fn from(value: Rm) -> Self {
        IntVal::unsigned(value as u8 as _)
    }
}
