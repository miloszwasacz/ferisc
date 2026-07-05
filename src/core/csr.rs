pub use self::addr::CsrAddr;
pub use self::buf::CsrBuf;
pub use self::file::{CsrFile, Stats};
use crate::core::{reg::int::IntVal, uop::Exception};

macro_rules! csr_mod {
    ($name:ident) => {
        ::paste::paste! {
            pub mod [<$name:snake>];
            pub use [<$name:snake>]::[<$name>];
        }
    };
}

mod addr;
mod buf;
mod file;

pub mod f;
pub mod m;

pub trait CsrMeta {
    const ADDR: CsrAddr;
    const NAME: &str;
}

pub trait Csr: CsrMeta + Copy + Default {
    fn new_value(&self, int: IntVal) -> Result<Self, Exception>;
    fn as_int(&self) -> IntVal;
}

pub trait CsrAlias: CsrMeta {
    type Target: Csr;
    type Alias: TryFrom<IntVal, Error = Exception> + Into<IntVal>;

    fn alias(target: &Self::Target) -> &Self::Alias;
    fn dealias(alias: Self::Alias, current: &Self::Target) -> Self::Target;
    fn alias_as_int(alias: &Self::Alias) -> IntVal;
    fn dealias_int(aliased: IntVal) -> IntVal;
}
