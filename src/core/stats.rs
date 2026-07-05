use super::csr;
use super::ee::{pipeline, rob};
use super::fe::{addr_gen, decode, fetch, predecode};
use super::mem;
use super::{Core, Decoder, MemHierarchy, Mode};

#[derive(Debug, Clone)]
pub struct Stats {
    pub addr_gen: addr_gen::Stats,
    pub fetch: fetch::Stats,
    pub predecode: predecode::Stats,
    pub decode: decode::Stats,
    pub rob: rob::Stats,
    pub pipelines: pipeline::AllStats,
    pub mem: mem::Stats,
    pub csrf: csr::Stats,
    pub mode: Mode,
}

impl<H: MemHierarchy, D: Decoder> Core<H, D> {
    pub fn stats(&self) -> Stats {
        Stats {
            addr_gen: self.addr_gen.stats(),
            fetch: self.fetch.stats(self.hier.imem()),
            predecode: self.predecode.stats(&self.fetch),
            decode: self.decode.stats(),
            rob: self.rob.stats(),
            pipelines: self.pipelines.stats(),
            mem: self.mem.stats(),
            csrf: self.csrf.stats(),
            mode: self.mode.read_copied(),
        }
    }
}
