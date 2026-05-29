use super::fcsr::Fcsr;
use crate::core::{csr::CsrAlias, reg::int::IntVal, uop::Exception};
use ferisc_macros::bitfields;

#[derive(Debug, Clone, Copy)]
pub struct Fflags;

bitfields! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(u8)]
    pub enum Flags {
        NX,
        UF,
        OF,
        DZ,
        NV,
    }
}

impl Default for Flags {
    fn default() -> Self {
        Self::empty()
    }
}

impl CsrAlias for Fflags {
    type Target = Fcsr;
    type Alias = Flags;

    fn alias(target: &Self::Target) -> &Self::Alias {
        &target.flags
    }

    fn dealias(alias: Self::Alias, current: &Self::Target) -> Self::Target {
        Self::Target {
            flags: alias,
            ..*current
        }
    }

    fn alias_as_int(alias: &Self::Alias) -> IntVal {
        (*alias).into()
    }

    fn dealias_int(aliased: IntVal) -> IntVal {
        aliased
    }
}

impl TryFrom<IntVal> for Flags {
    type Error = Exception;

    fn try_from(value: IntVal) -> Result<Self, Self::Error> {
        Self::from_bits(value.u() as _).ok_or(Exception::Illegal)
    }
}

impl From<Flags> for IntVal {
    fn from(value: Flags) -> Self {
        IntVal::unsigned(value.bits() as _)
    }
}
