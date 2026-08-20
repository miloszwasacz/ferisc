use crate::core::{
    csr::CsrFile,
    ee::rob::lock::Flush,
    reg::ArchReg,
    uop::execute::{self, Execute, Jump, WriteReg},
};
use std::ops::Deref;

pub trait Uop: Deref<Target: Execute<Reg = Self::Reg>> {
    type Reg: ArchReg;

    fn is_flushed(&self, flush: &Flush<'_>) -> bool;
    fn execute(self, jump: impl Jump, write: impl WriteReg<Self::Reg>, csrf: &mut CsrFile);
}

impl<T: Execute> Uop for execute::Uop<T> {
    type Reg = T::Reg;

    fn is_flushed(&self, flush: &Flush<'_>) -> bool {
        self.handle.is_flushed(flush)
    }

    fn execute(self, jump: impl Jump, write: impl WriteReg<Self::Reg>, csrf: &mut CsrFile) {
        self.uop.execute(self.handle, jump, write, csrf)
    }
}
