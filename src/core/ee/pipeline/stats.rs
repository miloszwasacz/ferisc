use super::*;
use crate::core::{
    ee::{eu, rs},
    reg::phys::{Stats as PrfStats, rat::Stats as RatStats},
};

#[derive(derive_more::Debug)]
pub struct Stats<T>
where
    T: ArchReg,
    DUop: InferUop<T>,
    IUop<T>: Issue,
{
    pub rat: RatStats<T>,
    pub prf: PrfStats<T>,
    lines: Box<[LineStats<T>]>,
}

pub type LineStats<T> = (rs::Stats<IUop<T>>, eu::Stats<EUop<T>>);

impl<T> Clone for Stats<T>
where
    T: ArchReg,
    T::Repr: Clone,
    DUop: InferUop<T>,
    IUop<T>: Issue + Clone,
    EUop<T>: Clone,
{
    fn clone(&self) -> Self {
        Self {
            rat: self.rat.clone(),
            prf: self.prf.clone(),
            lines: self.lines.clone(),
        }
    }
}

impl<T> Stats<T>
where
    T: ArchReg,
    DUop: InferUop<T>,
    IUop<T>: Issue,
{
    pub fn lines(&self) -> &[LineStats<T>] {
        &self.lines
    }
}

impl<T> Pipeline<T>
where
    T: ArchReg,
    T::Repr: Clone,
    DUop: InferUop<T>,
    IUop<T>: Issue + Clone,
    EUop<T>: Clone,
{
    pub fn stats(&self) -> Stats<T> {
        Stats {
            rat: self.rat.stats(&self.prf),
            prf: self.prf.stats(),
            lines: self
                .lines
                .iter()
                .map(|(rs, eu)| (rs.stats(), eu.stats()))
                .collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AllStats {
    pub int: Stats<IntReg>,
}

impl Pipelines {
    pub fn stats(&self) -> AllStats {
        let Self { int } = self;
        AllStats { int: int.stats() }
    }
}
