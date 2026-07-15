pub use self::config::{Config, Configs};
pub use self::stats::{AllStats, LineStats, Stats};
use crate::core::{
    csr::CsrFile,
    ee::{
        eu::Eu,
        rob::locks::{AccessLock as RobLock, ExecuteLock},
        rs::{self, Rs},
    },
    fe::addr_gen,
    mem::AddrBypassBuilder,
    reg::{
        ArchReg,
        int::IntReg,
        phys::{Prf, Rat, RegOper},
    },
    uop::{
        Infer, InferUop, decode::Uop as DecUop, dispatch::Uop as DUop, execute::Execute,
        issue::Issue, rename::Uop as RUop,
    },
};
use crate::seq::{Clearable, IntoPipelineStage, Sequential};
use std::cell::RefCell;

mod config;
pub mod locks;
mod stats;

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
    pub fn new(config: &Config) -> Self {
        let (prf, rat) = Prf::new(config.prf_size.get());
        let lines = config
            .lines
            .iter()
            .map(|(rs, eu)| (Rs::new(rs), Eu::new(eu)))
            .collect();

        Self { rat, prf, lines }
    }

    pub fn tick<'a>(
        &'a mut self,
        rob_lock: &mut ExecuteLock<'_, '_, '_>,
        addr_bypass: &mut AddrBypassBuilder,
        csrf: &mut CsrFile,
    ) -> Bypass<'a, T>
    where
        T::Repr: Clone,
        <IUop<T> as Issue>::Issued: Execute<Reg = T>,
    {
        let mut bypass = rs::Bypass::empty();
        for (_, eu) in &mut self.lines {
            bypass.append(eu.tick(rob_lock, addr_bypass, &mut self.prf, csrf));
        }
        for (rs, eu) in &mut self.lines {
            let prf = RefCell::new(&mut self.prf);
            rs.issue(|reg| bypass.try_read(&**prf.borrow(), reg).cloned())
                .into_pipeline_stage()
                .substage(eu.issue(
                    rob_lock,
                    |reg, val| prf.borrow_mut()[reg].write(val),
                    addr_bypass,
                    csrf,
                ))
                .execute();
        }
        Bypass(bypass, &mut self.prf)
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
    DUop: InferUop<T>,
    IUop<T>: Issue,
{
    fn clear(&mut self) {
        self.rat.flush_all(&mut self.prf);
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
    pub fn new(configs: &Configs) -> Self {
        let Configs { int } = configs;

        Self {
            int: Pipeline::new(int),
        }
    }

    pub fn tick<'a>(
        &'a mut self,
        rob: &mut RobLock<'_>,
        addr_bypass: &mut AddrBypassBuilder,
        csrf: &mut CsrFile,
        addr_gen: addr_gen::Lock<'_>,
    ) -> BypassNetwork<'a> {
        let Self { int } = self;
        let mut rob_lock = rob.execute(addr_gen);

        BypassNetwork {
            int: int.tick(&mut rob_lock, addr_bypass, csrf),
        }
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

//#region Bypass

pub struct Bypass<'a, T: ArchReg>(rs::Bypass<T>, &'a mut Prf<T>);

impl<'a, T: ArchReg> Bypass<'a, T> {
    pub fn try_read(&self, reg: RegOper<T>) -> Option<&T::Repr> {
        self.0.try_read(self.1, reg)
    }

    pub fn try_read_copied(&self, reg: RegOper<T>) -> Option<T::Repr>
    where
        T::Repr: Copy,
    {
        self.0.try_read_copied(self.1, reg)
    }

    pub fn write(&mut self, reg: RegOper<T>, val: T::Repr) {
        self.1[reg].write(val);
    }
}

pub struct BypassNetwork<'a> {
    pub int: Bypass<'a, IntReg>,
}

//#endregion
