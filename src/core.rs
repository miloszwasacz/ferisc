use self::fe::{AddrGen, decode::DecodeUnit, fetch::FetchUnit, predecode::Predecoder};
use self::mem::{DirectHierarchy, MemHierarchy};
use self::pipeline::Pipelines;
use crate::{
    core::{csr::CsrFile, uop::Mode},
    seq::{Buf, Clearable, PipelineStage},
};
use std::ops::DerefMut;

pub mod fe;

pub mod csr;
mod flush;
pub mod mem;
pub mod pipeline;
pub mod reg;
pub mod uop;

pub struct Core {
    addr_gen: AddrGen,
    fetch: FetchUnit,
    predecode: Predecoder,
    decode: DecodeUnit,
    pipelines: Pipelines,

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

        //TODO How to update BTB and BP when flushing?
        addr_gen.apply().flush_stages(
            &mut self.mem,
            |mem| {
                self.fetch.clear(mem.imem().deref_mut());
                self.predecode.clear();
            },
            || todo!("flush decode"),
            |_mem| todo!("flush the rest of the pipeline"),
        );
    }
}
