use self::csr::CsrFile;
use self::ee::{
    pipeline::Pipelines,
    rob::{Rob, RobAccess},
};
use self::fe::{AddrGen, decode::DecodeUnit, fetch::FetchUnit, predecode::Predecoder};
use self::mem::{AddrBypassBuilder, MemHierarchy, MemSubsystem};
use crate::seq::{Buf, Clearable, IntoPipelineStage, PipelineStage, Sequential};
use std::ops::DerefMut;

pub mod csr;
pub mod ee;
pub mod fe;
pub mod mem;
pub mod reg;
pub mod uop;

pub struct Core<H: MemHierarchy> {
    addr_gen: AddrGen,
    fetch: FetchUnit<H::IMem>,
    predecode: Predecoder,
    decode: DecodeUnit,
    pipelines: Pipelines,
    rob: Rob,

    mem: MemSubsystem<H::DMem>,
    hier: H,

    csrf: CsrFile,
    mode: Buf<Mode>,
}

impl<H: MemHierarchy> Core<H> {
    pub fn tick(&mut self) {
        let addr_gen = self.addr_gen.lock();
        let mode = self.mode.read_copied();

        //TODO Process tick
        {
            let mut imem = self.hier.imem();

            // Fetch
            let fetch = self
                .fetch
                .fetch(imem.deref_mut(), addr_gen.clone(), self.predecode.lock());

            // Decode
            PipelineStage::new(fetch)
                .substage(self.decode.decode(&self.csrf, addr_gen.clone(), mode))
                .execute();
        }

        match self.rob.access() {
            RobAccess::Ok(mut rob) => {
                // Rename/Dispatch
                let (rename, dispatch) = self.pipelines.input_locks();
                self.decode
                    .lock()
                    .into_pipeline_stage()
                    .copied()
                    .substage(rename)
                    .substage(rob.dispatch())
                    .substage(self.mem.dispatch())
                    .substage(dispatch)
                    .execute();

                // Issue & Execute
                let mut addr_bypass = AddrBypassBuilder::default();
                let mut bypass = self.pipelines.tick(
                    &mut rob,
                    &mut addr_bypass,
                    &mut self.csrf,
                    addr_gen.clone(),
                );

                // Memory Access
                self.mem
                    .tick(self.hier.dmem(), addr_bypass.build(), &mut bypass);

                // Commit
                rob.commit(&mut self.csrf, addr_gen.clone(), &mut self.mode)
                    .into_pipeline_stage()
                    .substage(self.pipelines.commit())
                    .execute();
            }
            RobAccess::FlushSome(flush) => {
                flush
                    .into_pipeline_stage()
                    .substage(self.pipelines.flush())
                    .substage(self.mem.flush(self.hier.dmem()))
                    .transform(|_| ())
                    .execute();
            }
            RobAccess::FlushAll(_) => {
                self.pipelines.clear();
                self.mem.flush_all(self.hier.dmem());
            }
        }

        let cycle = self.csrf.mcycle.read();
        self.csrf.mcycle.implicit_write(cycle + 1);

        // Flush
        //TODO How to update BTB and BP when flushing?
        let flush = addr_gen.apply();
        if flush.flush_fetch() {
            self.fetch.clear(self.hier.imem().deref_mut());
            self.predecode.clear();
        }
        if flush.flush_decode() {
            self.decode.clear();
        }

        self.finish_cycle();
    }
}

impl<H: MemHierarchy> Sequential for Core<H> {
    fn finish_cycle(&mut self) {
        self.fetch.finish_cycle();
        self.predecode.finish_cycle();
        todo!("finish cycle");
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Mode {
    //TODO More modes than machine mode

    // /// User mode
    // U,
    // /// Supervisor mode
    // S,
    /// Machine mode
    #[default]
    M,
}
