pub use self::config::Config;
use self::csr::CsrFile;
use self::ee::{
    pipeline::Pipelines,
    rob::{Rob, lock::Access as RobAccess},
};
use self::fe::{
    addr_gen::AddrGen,
    decode::{DecodeUnit, Decoder, DefaultDecoder},
    fetch::FetchUnit,
    predecode::PredecodeUnit,
};
use self::mem::{MemSubsystem, hierarchy::MemHierarchy};
pub use self::stats::Stats;
use crate::seq::{Buf, Clearable, IntoPipelineStage, PipelineStage, Sequential};
use std::ops::DerefMut;

mod config;
mod stats;

pub mod csr;
pub mod ee;
pub mod fe;
pub mod mem;
pub mod reg;
pub mod uop;

pub struct Core<H: MemHierarchy, D = DefaultDecoder> {
    addr_gen: AddrGen,
    fetch: FetchUnit<H::IMem>,
    predecode: PredecodeUnit,
    decode: DecodeUnit<D>,
    rob: Rob,
    pipelines: Pipelines,
    mem: MemSubsystem<H::DMem>,
    hierarchy: H,
    csrf: CsrFile,
    mode: Buf<Mode>,
}

impl<H: MemHierarchy, D: Decoder> Core<H, D> {
    pub fn new(config: &Config, make_mem_hier: impl FnOnce() -> H) -> Self {
        let mut hierarchy = make_mem_hier();
        let fetch = FetchUnit::new(&config.fetch, hierarchy.imem_mut().deref_mut());
        let mem = MemSubsystem::new(&config.mem, hierarchy.dmem_mut().deref_mut());

        Self {
            addr_gen: AddrGen::new(config.entrypoint),
            fetch,
            predecode: PredecodeUnit::new(&config.predecode),
            decode: DecodeUnit::new(&config.decode),
            rob: Rob::new(&config.rob),
            pipelines: Pipelines::new(&config.pipelines),
            mem,
            hierarchy,
            csrf: CsrFile::default(),
            mode: Buf::new(Mode::M),
        }
    }

    pub fn tick(&mut self) {
        let addr_gen = self.addr_gen.lock();
        let mode = self.mode.read_copied();

        ferisc_macros::pipeline! {
            // Fetch & Predecode
            stage[mut imem = self.hierarchy.imem_mut()] {
                self.fetch
                    .out(addr_gen.clone(), imem.deref_mut())
                    .into_pipeline_stage()
                    .substage(self.predecode.r#in())
            },

            // Decode
            stage {
                self.predecode
                    .out()
                    .into_pipeline_stage()
                    .copied()
                    .substage(self.decode.r#in(addr_gen.clone(), &self.csrf, mode))
            },

            select self.rob.access(&mut self.csrf) {
                RobAccess::Tick(mut rob) => {

                    // Rename/Dispatch
                    stage {
                        let (rename, dispatch) = self.pipelines.rename_and_dispatch();
                        self.decode
                            .out()
                            .into_pipeline_stage()
                            .copied()
                            .substage(rename)
                            .substage(rob.dispatch(self.mem.dispatch()))
                            .substage(dispatch)
                    },

                    with[mut rob_execute = rob.execute(addr_gen.clone())] {
                        // Issue
                        custom {
                            self.pipelines.issue(&mut rob_execute, &mut self.csrf);
                        },

                        // Memory Access
                        custom {
                            let mut dmem = self.hierarchy.dmem_mut();
                            self.pipelines.mem_access(&mut self.mem, dmem.deref_mut());
                        },

                        // Execute
                        custom {
                           self.pipelines.execute(&mut rob_execute, &mut self.csrf);
                        },
                    },

                    // Commit
                    stage {
                        rob.commit(
                            addr_gen.clone(),
                            self.mem.commit(),
                            &mut self.csrf,
                            |mode| self.mode.write(mode),
                        )
                        .into_pipeline_stage()
                        .substage(self.pipelines.commit())
                        .map(|_| ())
                    },

                },

                RobAccess::FlushSome(flush) => custom {
                    let mut dmem = self.hierarchy.dmem_mut();
                    self.pipelines.flush(&flush);
                    self.mem.flush(dmem.deref_mut(), &flush);
                },

                RobAccess::FlushAll(_) => custom {
                    let mut dmem = self.hierarchy.dmem_mut();
                    self.pipelines.clear();
                    self.mem.flush_all(dmem.deref_mut());
                },
            },

            // Implicit CSR updates
            custom {
                let cycle = self.csrf.mcycle.read();
                self.csrf.mcycle.implicit_write(cycle + 1);
            },
        }

        // Front-End flushing
        {
            //TODO How to update BTB and BP when flushing?
            let flush = addr_gen.apply();
            if flush.flush_fetch() {
                let mut imem = self.hierarchy.imem_mut();
                self.fetch.flush(imem.deref_mut());
                self.predecode.clear();
            }
            if flush.flush_decode() {
                self.decode.clear();
            }
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
        self.hierarchy.finish_cycle();
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
