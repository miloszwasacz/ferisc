use crate::core::{
    ee::rob,
    reg::{ArchReg, phys::RegOper},
};

macro_rules! try_read {
    ($read:expr, $reg:expr) => {
        $read.try_read($reg)?
    };
}
macro_rules! unwrap_src {
    ($read:expr, $src:expr) => {{
        match $src {
            Src::Reg(reg) => try_read!($read, reg),
            Src::Imm(imm) => imm,
        }
    }};
}

pub mod int;

pub type Uop<K> = rob::Uop<K>;

pub trait Issue {
    type Reg: ArchReg;
    type Issued;

    fn issue(self, read: impl TryReadReg<Self::Reg>) -> Option<Self::Issued>;
}

//TODO Replace with `type TryReadReg<..> = impl Fn(..) -> ..` once it's stabilized
pub trait TryReadReg<R: ArchReg> {
    fn try_read(&self, reg: RegOper<R>) -> Option<R::Repr>;
}
impl<R, F> TryReadReg<R> for F
where
    R: ArchReg,
    F: Fn(RegOper<R>) -> Option<R::Repr>,
{
    fn try_read(&self, reg: RegOper<R>) -> Option<R::Repr> {
        self(reg)
    }
}
