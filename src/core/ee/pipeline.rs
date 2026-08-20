pub use self::pipelines::{Commit, Configs, Dispatch, Kind, Pipelines, Rename};
pub use self::stats::{AllStats, LineStats, Stats};
use crate::core::{
    csr::CsrFile,
    ee::{
        eu::{self, Eu},
        rob,
        rs::{self, Rs},
    },
    reg::{
        ArchReg,
        int::IntReg,
        phys::{Prf, RegDest, rat::Rat},
    },
    uop::{
        Infer, InferUop, dispatch,
        issue::{self, Issue},
    },
};
use crate::seq::{Clearable, Sequential};
use std::num::NonZeroUsize;

mod pipelines;
mod stats;
pub mod substage;

type DUop = dispatch::Uop;
type IUop<T> = issue::Uop<Infer<DUop, T>>;
type EUop<T> = <IUop<T> as rs::Uop>::Issued;
type Line<T> = (Rs<IUop<T>>, Eu<EUop<T>>);

#[derive(derive_more::Debug)]
pub struct Pipeline<T>
where
    T: ArchReg,
    DUop: InferUop<T>,
    IUop<T>: rs::Uop<Reg = T>,
    EUop<T>: eu::Uop<Reg = T>,
{
    rat: Rat<T>,
    prf: Prf<T>,
    lines: Box<[Line<T>]>,
}

impl<T> Pipeline<T>
where
    T: ArchReg,
    DUop: InferUop<T>,
    IUop<T>: rs::Uop<Reg = T>,
    EUop<T>: eu::Uop<Reg = T>,
{
    pub fn new(config: &Config) -> Self {
        let (prf, rat) = Prf::new(config.prf_size.get());
        let lines = config
            .lines
            .iter()
            .map(|(rs, eu)| (Rs::new(rs), Eu::new(eu)))
            .collect();

        Self { rat, prf, lines }
    }

    pub fn flush(
        &mut self,
        flush: &rob::lock::Flush<'_>,
        flushed: impl IntoIterator<Item = RegDest<T>>,
    ) {
        self.rat.flush(&mut self.prf, flushed);
        for (rs, eu) in &mut self.lines {
            rs.flush(flush);
            eu.flush(flush);
        }
    }
}

impl<T> Sequential for Pipeline<T>
where
    T: ArchReg,
    DUop: InferUop<T, Kind: Issue<Reg = T>>,
    IUop<T>: rs::Uop<Reg = T>,
    EUop<T>: eu::Uop<Reg = T>,
{
    fn finish_cycle(&mut self) {
        self.rat.finish_cycle();
        self.prf.finish_cycle();
        for (rs, eu) in &mut self.lines {
            rs.finish_cycle();
            eu.finish_cycle();
        }
    }
}

impl<T> Clearable for Pipeline<T>
where
    T: ArchReg,
    DUop: InferUop<T, Kind: Issue<Reg = T>>,
    IUop<T>: rs::Uop<Reg = T>,
    EUop<T>: eu::Uop<Reg = T>,
{
    fn clear(&mut self) {
        self.rat.flush_all(&mut self.prf);
        for (rs, eu) in &mut self.lines {
            rs.clear();
            eu.clear();
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub prf_size: NonZeroUsize,
    pub lines: Box<[(rs::Config, eu::Config)]>,
}
