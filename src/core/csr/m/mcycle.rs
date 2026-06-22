use crate::core::{
    csr::Csr,
    reg::int::{IntVal, Uint},
    uop::Exception,
};
use std::ops::Add;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Mcycle(Uint);

impl Add<Uint> for &Mcycle {
    type Output = Mcycle;

    fn add(self, rhs: Uint) -> Self::Output {
        Mcycle(self.0.wrapping_add(rhs))
    }
}

impl Csr for Mcycle {
    fn new_value(&self, int: IntVal) -> Result<Self, Exception> {
        Ok(Self(int.u()))
    }

    fn as_int(&self) -> IntVal {
        (*self).into()
    }
}

impl From<Mcycle> for IntVal {
    fn from(value: Mcycle) -> Self {
        IntVal::unsigned(value.0)
    }
}
