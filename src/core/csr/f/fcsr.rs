use super::{
    fflags::{Fflags, Flags},
    frm::{Frm, Rm},
};
use crate::core::{
    csr::{Csr, CsrAlias},
    reg::int::IntVal,
    uop::Exception,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Fcsr {
    pub flags: Flags,
    pub rm: Rm,
}

impl Csr for Fcsr {
    fn new_value(&self, int: IntVal) -> Result<Self, Exception> {
        let flags = int.try_into()?;
        let rm = (int >> 5).try_into()?;
        Ok(Self { flags, rm })
    }

    fn as_int(&self) -> IntVal {
        (*self).into()
    }
}

impl From<Fcsr> for IntVal {
    fn from(value: Fcsr) -> Self {
        let flags = Fflags::dealias_int(value.flags.into());
        let rm = Frm::dealias_int(value.rm.into());
        flags | rm
    }
}
