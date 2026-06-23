use crate::core::{
    csr::CsrFile,
    reg::{ArchReg, phys::Prf},
    rob::{self, locks::ExecuteLock},
};

pub mod int;

pub type Latency = u8;

pub trait Execute {
    type Reg: ArchReg;

    fn latency(&self) -> Latency;
    fn execute(
        self,
        rob_entry: rob::EntryHandle,
        lock: &mut ExecuteLock<'_, '_>,
        prf: &mut Prf<Self::Reg>,
        csrf: &mut CsrFile,
    );
}
