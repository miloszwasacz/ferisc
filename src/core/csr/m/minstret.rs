use crate::core::{
    csr::Csr,
    reg::int::{IntVal, Uint},
    uop::Exception,
};
use std::ops::Add;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Minstret(Uint);

impl Add<Uint> for &Minstret {
    type Output = Minstret;

    fn add(self, rhs: Uint) -> Self::Output {
        Minstret(self.0.wrapping_add(rhs))
    }
}

impl Csr for Minstret {
    fn new_value(&self, int: IntVal) -> Result<Self, Exception> {
        Ok(Self(int.u()))
    }

    fn as_int(&self) -> IntVal {
        (*self).into()
    }
}

impl From<Minstret> for IntVal {
    fn from(value: Minstret) -> Self {
        IntVal::unsigned(value.0)
    }
}
