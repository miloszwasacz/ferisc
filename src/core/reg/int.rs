use super::{ArchReg, ArchRegCount, RegRepr};
use crate::mem::Addr;
use ferisc_macros::ArchReg;
use strum::{Display, FromRepr};

//#region Arch

/// Architectural integer register
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash, ArchReg, FromRepr)]
#[strum(serialize_all = "lowercase")]
#[repr(u8)]
pub enum IntReg {
    /// Always zero
    #[hardwired]
    ZERO,

    /// Return address
    RA,

    /// Stack pointer
    SP,

    /// Global pointer
    GP,

    /// Thread pointer
    TP,

    /// Temporary / alternate return address
    T0,

    /// Temporary
    T1,
    /// Temporary
    T2,

    /// Frame pointer
    FP,
    /// Saved register
    S1,

    /// Function argument / return value
    A0,
    /// Function argument / return value
    A1,

    /// Function argument
    A2,
    /// Function argument
    A3,
    /// Function argument
    A4,
    /// Function argument
    A5,
    /// Function argument
    A6,
    /// Function argument
    A7,

    /// Saved register
    S2,
    /// Saved register
    S3,
    /// Saved register
    S4,
    /// Saved register
    S5,
    /// Saved register
    S6,
    /// Saved register
    S7,
    /// Saved register
    S8,
    /// Saved register
    S9,
    /// Saved register
    S10,
    /// Saved register
    S11,

    /// Temporary
    T3,
    /// Temporary
    T4,
    /// Temporary
    T5,
    /// Temporary
    T6,
}

impl IntReg {
    /// Saved register
    pub const S0: Self = Self::FP;
}

impl RegRepr for IntReg {
    type Repr = IntVal;
}

//#endregion

//#region Val

/// Integer register value
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct IntVal(Uint);

/// Unsigned integer register value
pub type Uint = u64;

/// Signed integer register value
pub type Sint = i64;

const _: () = {
    assert!(size_of::<Uint>() == size_of::<Sint>() && size_of::<Uint>() == size_of::<Addr>());
};

impl IntVal {
    /// Crates a new unsigned integer register value.
    pub fn unsigned(val: Uint) -> Self {
        Self(val as _)
    }

    /// Crates a new signed integer register value.
    pub fn signed(val: Sint) -> Self {
        Self(val as _)
    }

    /// Crates a new memory address register value.
    pub fn address(val: Addr) -> Self {
        Self(val as _)
    }

    /// Returns the register value as an unsigned integer.
    pub fn u(&self) -> Uint {
        self.0 as _
    }

    /// Returns the register value as a signed integer.
    pub fn s(&self) -> Sint {
        self.0 as _
    }

    /// Returns the register value as a memory address.
    pub fn addr(&self) -> Addr {
        self.0 as _
    }
}

//#endregion
