use crate::core::{csr::Csr, reg::int::IntVal, uop::Exception};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mcause(pub Exception);

impl Default for Mcause {
    fn default() -> Self {
        Self(Exception::Illegal)
    }
}

impl From<Exception> for Mcause {
    fn from(value: Exception) -> Self {
        Self(value)
    }
}

impl Csr for Mcause {
    fn new_value(&self, int: IntVal) -> Result<Self, Exception> {
        Exception::from_repr(int.u())
            .map(From::from)
            .ok_or(Exception::Illegal)
    }

    fn as_int(&self) -> IntVal {
        (*self).into()
    }
}

impl From<Mcause> for IntVal {
    fn from(value: Mcause) -> Self {
        IntVal::unsigned(value.0 as _)
    }
}
