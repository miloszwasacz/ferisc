use super::Eu;
use crate::core::{
    csr::CsrFile,
    ee::rob,
    uop::execute::{Execute, Uop, WriteReg},
};
use crate::seq::locks::{CircuitLock, Transformer};
use std::marker::PhantomData;

pub struct Lock<'e, 'l, 'a, 'r, 'g, 'c, U: Execute, W> {
    eu: &'e mut Eu<U>,
    rob_lock: &'l mut rob::locks::ExecuteLock<'a, 'r, 'g>,
    write_reg: W,
    csrf: &'c mut CsrFile,
}

impl<'e, 'l, 'a, 'r, 'g, 'c, U, W> Lock<'e, 'l, 'a, 'r, 'g, 'c, U, W>
where
    U: Execute,
    W: WriteReg<U::Reg>,
{
    pub(super) fn new(
        eu: &'e mut Eu<U>,
        rob_lock: &'l mut rob::locks::ExecuteLock<'a, 'r, 'g>,
        write_reg: W,
        csrf: &'c mut CsrFile,
    ) -> Self {
        Self {
            eu,
            rob_lock,
            write_reg,
            csrf,
        }
    }
}

impl<'e, 'l, 'a, 'r, 'g, 'c, U, W> CircuitLock<'e> for Lock<'e, 'l, 'a, 'r, 'g, 'c, U, W>
where
    U: Execute + 'e,
    W: WriteReg<U::Reg>,
{
    type Trans = Trans<'e, 'l, 'a, 'r, 'g, 'c, U, W>;

    fn make_trans(&mut self) -> Self::Trans {
        Trans(None, PhantomData)
    }
}

pub struct Trans<'e, 'l, 'a, 'r, 'g, 'c, U: Execute, W>(
    Option<Uop<U>>,
    PhantomData<Lock<'e, 'l, 'a, 'r, 'g, 'c, U, W>>,
);

impl<'e, 'l, 'a, 'r, 'g, 'c, U, W> Transformer<'e> for Trans<'e, 'l, 'a, 'r, 'g, 'c, U, W>
where
    U: Execute + 'e,
    W: WriteReg<U::Reg>,
{
    type In = Uop<U>;
    type Out = ();
    type Lock = Lock<'e, 'l, 'a, 'r, 'g, 'c, U, W>;

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
        let uop = self.0.unwrap();

        match uop.kind.latency().checked_sub(1) {
            None => uop
                .kind
                .execute(uop.rob_entry, lock.rob_lock, &mut lock.write_reg, lock.csrf),
            Some(cycles) => {
                lock.eu.cycles.write(cycles);
                let old = lock.eu.new.replace(uop);
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
