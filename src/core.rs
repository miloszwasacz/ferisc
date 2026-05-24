use self::fe::{AddrGen, fetch::FetchUnit, predecode::Predecoder};
use self::mem::{DirectHierarchy, MemHierarchy};
use crate::seq::Clearable;
use std::ops::DerefMut;

pub mod fe;

mod flush;
pub mod mem;
pub mod reg;
pub mod uop;

pub struct Core {
    addr_gen: AddrGen,
    fetch: FetchUnit,
    predecode: Predecoder,

    mem: DirectHierarchy,
}

impl Core {
    pub fn tick(&mut self) {
        let mut addr_gen = self.addr_gen.lock();

        //TODO Process tick
        self.fetch.fetch(
            self.mem.imem().deref_mut(),
            &mut addr_gen,
            self.predecode.lock(),
        );

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
