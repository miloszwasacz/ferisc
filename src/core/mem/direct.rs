use super::MemHierarchy;
use crate::mem::Dram;
use crate::seq::Sequential;
use std::ops::DerefMut;
use std::sync::{Arc, RwLock};

#[derive(Debug)]
pub struct DirectHierarchy {
    dram: Arc<RwLock<Dram>>,
}

impl DirectHierarchy {
    pub fn new(dram: Arc<RwLock<Dram>>) -> Self {
        Self { dram }
    }
}

impl MemHierarchy for DirectHierarchy {
    type IMem = Dram;
    type DMem = Dram;

    fn imem(&mut self) -> impl DerefMut<Target = Self::IMem> {
        self.dram.write().unwrap()
    }

    fn dmem(&mut self) -> impl DerefMut<Target = Self::DMem> {
        self.dram.write().unwrap()
    }
}

impl Sequential for DirectHierarchy {
    fn finish_cycle(&mut self) {
        // Do nothing - DRAM's clock should be advanced separately
    }
}
