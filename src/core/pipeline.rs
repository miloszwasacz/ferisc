use crate::core::{
    csr::CsrFile,
    eu::Eu,
    fe::AddrGenLock,
    reg::{
        ArchReg,
        int::IntReg,
        phys::{Prf, Rat},
    },
    rob::{Rob, locks::ExecuteLock},
    rs::Rs,
    uop::{
        Infer, InferUop, decode::Uop as DecUop, dispatch::Uop as DUop, execute::Execute,
        issue::Issue, rename::Uop as RUop,
    },
};
use crate::seq::IntoPipelineStage;
use std::cell::RefCell;

pub mod locks;

type IUop<T> = Infer<DUop, T>;
type EUop<T> = <IUop<T> as Issue>::Issued;
type Line<T> = (Rs<IUop<T>>, Eu<EUop<T>>);

#[derive(derive_more::Debug)]
pub struct Pipeline<T>
where
    T: ArchReg,
    DUop: InferUop<T>,
    IUop<T>: Issue,
{
    rat: Rat<T>,
    prf: Prf<T>,
    lines: Box<[Line<T>]>,
}

impl<T> Pipeline<T>
where
    T: ArchReg,
    DUop: InferUop<T>,
    IUop<T>: Issue<Reg = T> + Clone,
{
    pub fn tick(&mut self, mut rob_lock: ExecuteLock<'_, '_>, csrf: &mut CsrFile)
    where
        T::Repr: Clone,
        <IUop<T> as Issue>::Issued: Execute<Reg = T>,
    {
        for (rs, eu) in &mut self.lines {
            let bypass = eu.tick(&mut rob_lock, &mut self.prf, csrf);
            let prf = RefCell::new(&mut self.prf);

            rs.issue(|reg| bypass.try_read(&**prf.borrow(), reg).cloned())
                .into_pipeline_stage()
                .substage(eu.issue(
                    &mut rob_lock,
                    |reg, val| prf.borrow_mut()[reg].write(val),
                    csrf,
                ))
                .execute();
        }
    }
}

#[derive(Debug)]
pub struct Pipelines {
    int: Pipeline<IntReg>,
}

impl Pipelines {
    pub fn tick(&mut self, rob: &mut Rob, csrf: &mut CsrFile, addr_gen: AddrGenLock<'_>) {
        let Self { int } = self;
        macro_rules! tick {
            ($pipeline:expr) => {
                $pipeline.tick(rob.execute(addr_gen.clone()), csrf)
            };
        }

        tick!(int);
    }
}
