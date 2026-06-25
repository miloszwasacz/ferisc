use crate::core::{
    csr::CsrFile,
    ee::{
        eu::Eu,
        rob::locks::{AccessLock as RobLock, ExecuteLock},
        rs::Rs,
    },
    fe::AddrGenLock,
    reg::{
        ArchReg,
        int::IntReg,
        phys::{Prf, Rat},
    },
    uop::{
        Infer, InferUop, decode::Uop as DecUop, dispatch::Uop as DUop, execute::Execute,
        issue::Issue, rename::Uop as RUop,
    },
};
use crate::seq::{Clearable, IntoPipelineStage, Sequential};
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
    pub fn tick(&mut self, rob_lock: &mut ExecuteLock<'_, '_, '_>, csrf: &mut CsrFile)
    where
        T::Repr: Clone,
        <IUop<T> as Issue>::Issued: Execute<Reg = T>,
    {
        for (rs, eu) in &mut self.lines {
            let bypass = eu.tick(rob_lock, &mut self.prf, csrf);
            let prf = RefCell::new(&mut self.prf);

            rs.issue(|reg| bypass.try_read(&**prf.borrow(), reg).cloned())
                .into_pipeline_stage()
                .substage(eu.issue(rob_lock, |reg, val| prf.borrow_mut()[reg].write(val), csrf))
                .execute();
        }
    }
}

impl<T> Sequential for Pipeline<T>
where
    T: ArchReg,
    DUop: InferUop<T>,
    IUop<T>: Issue,
{
    fn finish_cycle(&mut self) {
        self.rat.finish_cycle();
        for (rs, eu) in &mut self.lines {
            rs.finish_cycle();
            eu.finish_cycle();
        }
    }
}

impl<T> Clearable for Pipeline<T>
where
    T: ArchReg,
    DUop: InferUop<T>,
    IUop<T>: Issue,
{
    fn clear(&mut self) {
        self.rat.flush_all(self.prf.size());
        for (rs, eu) in &mut self.lines {
            rs.clear();
            eu.clear();
        }
    }
}

#[derive(Debug)]
pub struct Pipelines {
    int: Pipeline<IntReg>,
}

impl Pipelines {
    pub fn tick(&mut self, rob: &mut RobLock<'_>, csrf: &mut CsrFile, addr_gen: AddrGenLock<'_>) {
        let Self { int } = self;
        let mut rob_lock = rob.execute(addr_gen);

        int.tick(&mut rob_lock, csrf);
    }
}

impl Sequential for Pipelines {
    fn finish_cycle(&mut self) {
        let Self { int } = self;

        int.finish_cycle();
    }
}

impl Clearable for Pipelines {
    fn clear(&mut self) {
        let Self { int } = self;

        int.clear();
    }
}
