use crate::core::{
    csr::CsrFile,
    ee::rob::{self, locks::ExecuteLock},
    reg::{ArchReg, phys::RegOper},
};

pub mod int;

pub type Uop<K> = super::issue::Uop<K>;

pub type Latency = u8;

pub trait Execute {
    type Reg: ArchReg<Repr: Clone>;

    fn latency(&self) -> Latency;

    fn execute(
        self,
        rob_entry: rob::EntryHandle,
        lock: &mut ExecuteLock<'_, '_, '_>,
        write_reg: &mut impl WriteReg<Self::Reg>,
        csrf: &mut CsrFile,
    );
}

pub trait WriteReg<R: ArchReg> {
    fn write(&mut self, reg: RegOper<R>, val: R::Repr);
}
impl<R: ArchReg, F: FnMut(RegOper<R>, R::Repr)> WriteReg<R> for F {
    fn write(&mut self, reg: RegOper<R>, val: R::Repr) {
        self(reg, val);
    }
}
