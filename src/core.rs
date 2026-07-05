pub use self::config::Config;
use self::csr::CsrFile;
use self::ee::{
    pipeline::Pipelines,
    rob::{Rob, RobAccess},
};
use self::fe::{
    AddrGen,
    decode::{DecodeUnit, Decoder, DefaultDecoder},
    fetch::FetchUnit,
    predecode::Predecoder,
};
use self::mem::{AddrBypassBuilder, MemHierarchy, MemSubsystem};
use crate::seq::{Buf, Clearable, IntoPipelineStage, PipelineStage, Sequential};
use std::ops::DerefMut;

mod config;
pub mod csr;
pub mod ee;
pub mod fe;
pub mod mem;
pub mod reg;
pub mod uop;

pub struct Core<H: MemHierarchy, D = DefaultDecoder> {
    addr_gen: AddrGen,
    fetch: FetchUnit<H::IMem>,
    predecode: Predecoder,
    decode: DecodeUnit<D>,
    rob: Rob,
    pipelines: Pipelines,
    mem: MemSubsystem<H::DMem>,
    hier: H,
    csrf: CsrFile,
    mode: Buf<Mode>,
}

impl<H: MemHierarchy, D: Decoder> Core<H, D> {
    pub fn new(config: Config, make_mem_hier: impl FnOnce() -> H) -> Self {
        let mut hier = make_mem_hier();
        Self {
            addr_gen: AddrGen::new(config.entrypoint),
            fetch: FetchUnit::new(&config.fetch, hier.imem()),
            predecode: Predecoder::default(),
            decode: DecodeUnit::new(&config.decode),
            rob: Rob::new(&config.rob),
            pipelines: Pipelines::new(&config.pipelines),
            mem: MemSubsystem::new(&config.mem, hier.dmem()),
            hier,
            csrf: CsrFile::default(),
            mode: Buf::new(Mode::M),
        }
    }

    pub fn tick(&mut self) {
        let addr_gen = self.addr_gen.lock();
        let mode = self.mode.read_copied();

        //TODO Process tick
        {
            // Fetch
            let fetch = self
                .fetch
                .fetch(self.hier.imem(), addr_gen.clone(), self.predecode.lock());

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
    }
}

impl<H: MemHierarchy, D> Sequential for Core<H, D> {
    fn finish_cycle(&mut self) {
        self.addr_gen.finish_cycle();
        self.fetch.finish_cycle();
        self.predecode.finish_cycle();
        self.decode.finish_cycle();
        self.pipelines.finish_cycle();
        self.rob.finish_cycle();
        self.mem.finish_cycle();
        self.hier.finish_cycle();
        self.csrf.finish_cycle();
        self.mode.finish_cycle();
    }
}

//TODO More modes than machine mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Mode {
    // /// User mode
    // U,
    // /// Supervisor mode
    // S,
    /// Machine mode
    #[default]
    M,
}
