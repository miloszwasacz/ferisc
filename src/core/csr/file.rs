use super::{Csr, CsrAddr, CsrAlias, CsrMeta, buf::CsrBuf, f::*, m::*};
use crate::core::{
    reg::int::IntVal,
    uop::{Exception, int::CsrOp},
};
use ferisc_macros::csr_file;

csr_file! {
    #[derive(Debug, Clone)]
    #[spec_file = "spec/csr.spec"]
    pub struct CsrFile;
}
