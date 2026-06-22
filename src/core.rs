use self::csr::CsrFile;
use self::fe::{AddrGen, decode::DecodeUnit, fetch::FetchUnit, predecode::Predecoder};
use self::mem::{DirectHierarchy, MemHierarchy};
use self::pipeline::Pipelines;
use self::rob::Rob;
use crate::seq::{Buf, Clearable, PipelineStage, Sequential};
use std::ops::DerefMut;

pub mod fe;

pub mod csr;
pub mod mem;
pub mod pipeline;
pub mod reg;
pub mod rob;
pub mod uop;

pub struct Core {
    addr_gen: AddrGen,
    fetch: FetchUnit,
    predecode: Predecoder,
    decode: DecodeUnit,
    pipelines: Pipelines,
    rob: Rob,

    mem: DirectHierarchy,
    csrf: CsrFile,
    mode: Buf<Mode>,
}

impl Core {
    pub fn tick(&mut self) {
        let addr_gen = self.addr_gen.lock();
        let mode = self.mode.read_copied();

        //TODO Process tick
        {
            let mut imem = self.mem.imem();

            // Fetch
            let fetch = self
                .fetch
                .fetch(imem.deref_mut(), addr_gen.clone(), self.predecode.lock());

            // Decode
            PipelineStage::new(fetch)
                .substage(self.decode.decode(&self.csrf, addr_gen.clone(), mode))
                .execute();
        }

        // // Rename/Dispatch
        // let (rename, dispatch) = self.pipelines.locks();
        // PipelineStage::new(self.decode.lock())
        //     .substage(rename)
        //     .substage(self.rob.dispatch())
        //     .substage(dispatch)
        //     .execute();

        let cycle = self.csrf.mcycle.read();
        self.csrf.mcycle.implicit_write(cycle + 1);

        // Flush
        //TODO How to update BTB and BP when flushing?
        let mut flush_fetch = |mem: &mut DirectHierarchy| {
            self.fetch.clear(mem.imem().deref_mut());
            self.predecode.clear();
        };
        let flush_decode = || {
            todo!("flush decode");
        };
        match addr_gen.apply() {
            FlushInfo::None => {}
            FlushInfo::Fetch => {
                flush_fetch(&mut self.mem);
            }
            FlushInfo::Decode => {
                flush_fetch(&mut self.mem);
                flush_decode();
            }
            FlushInfo::Execute => {
                flush_fetch(&mut self.mem);
                flush_decode();
                todo!("flush the rest of the pipeline (only up to causing branch)");
            }
            FlushInfo::Exception => {
                flush_fetch(&mut self.mem);
                flush_decode();
                todo!("flush the rest of the pipeline on exception");
            }
        }

        self.finish_cycle();
    }
}

impl Sequential for Core {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FlushInfo {
    #[default]
    None,
    Fetch,
    Decode,
    Execute,
    Exception,
}
