use crate::core::{
    csr::CsrFile,
    ee::rob,
    reg::{ArchReg, phys::RegOper},
};
use crate::mem::Addr;

pub mod int;

pub type Uop<K> = super::issue::Uop<K>;

pub type Latency = u8;

pub trait Execute {
    type Reg: ArchReg;

    fn latency(&self) -> Latency;
    fn execute(
        self,
        handle: rob::Handle,
        jump: impl Jump,
        write: impl WriteReg<Self::Reg>,
        csrf: &mut CsrFile,
    );
}

//TODO Replace with `type Jump = impl FnOnce(..)` once it's stabilized
pub trait Jump {
    fn jump(self, target: Addr, cause: &rob::Handle);
}
impl<F: FnOnce(Addr, &rob::Handle)> Jump for F {
    fn jump(self, target: Addr, cause: &rob::Handle) {
        self(target, cause)
    }
}

//TODO Replace with `type WriteReg<..> = impl FnMut(..)` once it's stabilized
pub trait WriteReg<R: ArchReg> {
    fn write(&mut self, reg: RegOper<R>, val: R::Repr);
}
impl<T: ArchReg, F: FnMut(RegOper<T>, T::Repr)> WriteReg<T> for F {
    fn write(&mut self, reg: RegOper<T>, val: T::Repr) {
        self(reg, val)
    }
}
