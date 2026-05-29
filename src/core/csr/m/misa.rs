use crate::core::{
    csr::Csr,
    reg::int::{IntVal, Uint},
    uop::Exception,
};
use ferisc_macros::bitfields;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Misa {
    pub mxl: Mxl,
    pub ext: Extensions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum Mxl {
    RV32 = 1,
    #[default]
    RV64 = 2,
}

bitfields! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(u32)]
    pub enum Extensions {
        /// Atomic extension
        A,
        /// B extension
        B,
        /// Compressed extension
        C,
        /// Double-precision floating-point extension
        D,
        /// RV32/64E base ISA
        E,
        /// Single-precision floating-point extension
        F,
        /// Reserved
        G,
        /// Hypervisor extension
        H,
        /// RV32I/64I base ISA
        I,
        /// Reserved
        J,
        /// Reserved
        K,
        /// Reserved
        L,
        /// Integer Multiply/Divide extension
        M,
        /// Reserved (User-level interrupts extension)
        N,
        /// Reserved
        O,
        /// Reserved (Packed-SIMD extension)
        P,
        /// Quad-precision floating-point extension
        Q,
        /// Reserved
        R,
        /// Supervisor mode implemented
        S,
        /// Reserved
        T,
        /// User mode implemented
        U,
        /// Vector extension
        V,
        /// Reserved
        W,
        /// Non-standard extension present
        X,
        /// Reserved
        Y,
        /// Reserved
        Z,
    }
}

impl Default for Extensions {
    fn default() -> Self {
        Self::I | Self::C
    }
}

impl Csr for Misa {
    fn new_value(&self, _: IntVal) -> Result<Self, Exception> {
        //TODO Allow modifying the `misa` CSR
        Ok(*self)
    }

    fn as_int(&self) -> IntVal {
        (*self).into()
    }
}

impl From<Misa> for IntVal {
    fn from(value: Misa) -> Self {
        let mxl = IntVal::unsigned((value.mxl as Uint).rotate_right(2));
        let ext = IntVal::unsigned(value.ext.bits() as _);
        mxl | ext
    }
}
