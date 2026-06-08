use super::MemHierarchy;
use crate::core::uop::Exception;
use crate::mem::Dram;
use std::sync::{Arc, RwLock, RwLockWriteGuard};

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
    type Err = Exception;
    type IMem<'a> = RwLockWriteGuard<'a, Dram>;
    type DMem<'a> = RwLockWriteGuard<'a, Dram>;

    fn imem(&mut self) -> Self::IMem<'_> {
        self.dram.write().unwrap()
    }

    fn dmem(&mut self) -> Self::DMem<'_> {
        self.dram.write().unwrap()
    }
}
