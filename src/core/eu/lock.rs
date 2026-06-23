use super::Eu;
use crate::core::{csr::CsrFile, reg::phys::Prf, rob, uop::execute::Execute};
use crate::seq::locks::{CircuitLock, Transformer};
use std::marker::PhantomData;

pub struct Lock<'e, 'l, 'r, 'g, 'p, 'c, U: Execute> {
    eu: &'e mut Eu<U>,
    rob_lock: &'l mut rob::locks::ExecuteLock<'r, 'g>,
    prf: &'p mut Prf<U::Reg>,
    csrf: &'c mut CsrFile,
}

impl<'e, 'l, 'r, 'g, 'p, 'c, U: Execute> Lock<'e, 'l, 'r, 'g, 'p, 'c, U> {
    pub(super) fn new(
        eu: &'e mut Eu<U>,
        rob_lock: &'l mut rob::locks::ExecuteLock<'r, 'g>,
        prf: &'p mut Prf<U::Reg>,
        csrf: &'c mut CsrFile,
    ) -> Self {
        Self {
            eu,
            rob_lock,
            prf,
            csrf,
        }
    }
}

impl<'e, 'l, 'r, 'g, 'p, 'c, U: Execute + 'e> CircuitLock<'e> for Lock<'e, 'l, 'r, 'g, 'p, 'c, U> {
    type Trans = Trans<'e, 'l, 'r, 'g, 'p, 'c, U>;

    fn make_trans(&mut self) -> Self::Trans {
        Trans(None, PhantomData)
    }
}

pub struct Trans<'e, 'l, 'r, 'g, 'p, 'c, U: Execute>(
    Option<(U, rob::EntryHandle)>,
    PhantomData<Lock<'e, 'l, 'r, 'g, 'p, 'c, U>>,
);

impl<'e, 'l, 'r, 'g, 'p, 'c, U: Execute + 'e> Transformer<'e> for Trans<'e, 'l, 'r, 'g, 'p, 'c, U> {
    type In = (U, rob::EntryHandle);
    type Out = ();
    type Lock = Lock<'e, 'l, 'r, 'g, 'p, 'c, U>;

    fn transform(&mut self, lock: &Self::Lock, input: Self::In) -> Option<Self::Out> {
        if lock.eu.cycles.read_copied() > 0 || lock.eu.new.is_some() {
            return None;
        }

        let old = self.0.replace(input);
        debug_assert!(old.is_none());
        Some(())
    }

    fn commit(self, lock: &mut Self::Lock) {
        debug_assert_eq!(lock.eu.cycles.read_copied(), 0);

        let (uop, rob_entry) = self.0.unwrap();
        match uop.latency().checked_sub(1) {
            None => uop.execute(rob_entry, lock.rob_lock, lock.prf, lock.csrf),
            Some(cycles) => {
                lock.eu.cycles.write(cycles);
                let old = lock.eu.new.replace((uop, rob_entry));
                debug_assert!(
                    old.is_none(),
                    "cannot execute more than one instruction in a single clock cycle"
                );
            }
        }

        #[cfg(debug_assertions)]
        lock.eu.state.insert(super::State::ISSUED);
    }
}
