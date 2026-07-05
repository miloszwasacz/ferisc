use crate::core::{csr::Csr, reg::int::IntVal, uop::Exception};
use crate::mem::Addr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Mepc(Addr);

//TODO If IALIGN changes to 32 (by writing into the MISA and disabling the C extension),
//     the mask should clear `mepc[1:0]`, not just `mepc[0]`.

impl Mepc {
    pub fn pc(&self) -> Addr {
        self.0
    }
}

impl From<Addr> for Mepc {
    fn from(value: Addr) -> Self {
        Self(value & !0b1)
    }
}

impl Csr for Mepc {
    fn new_value(&self, int: IntVal) -> Result<Self, Exception> {
        Ok(int.addr().into())
    }

    fn as_int(&self) -> IntVal {
        (*self).into()
    }
}

impl From<Mepc> for IntVal {
    fn from(value: Mepc) -> Self {
        Self::address(value.0)
    }
}
