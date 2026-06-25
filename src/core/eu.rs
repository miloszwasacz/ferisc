use self::lock::Lock;
use crate::core::{
    csr::CsrFile,
    reg::phys::Prf,
    rob::locks::ExecuteLock,
    rs::{Bypass, BypassBuilder},
    uop::execute::{Execute, Latency, Uop, WriteReg},
};
use crate::seq::{Buf, Sequential, locks::StageLock};

mod lock;

//TODO Pipelined EU
#[derive(Debug, Clone, Default)]
pub struct Eu<U> {
    uop: Option<Uop<U>>,
    new: Option<Uop<U>>,
    cycles: Buf<Latency>,
    #[cfg(debug_assertions)]
    state: State,
}

impl<U: Execute> Eu<U> {
    pub fn issue(
        &mut self,
        rob_lock: &mut ExecuteLock<'_, '_>,
        write_reg: impl WriteReg<U::Reg>,
        csrf: &mut CsrFile,
    ) -> impl StageLock<'_, Uop<U>, ()> {
        #[cfg(debug_assertions)]
        assert!(
            !self.state.contains(State::ISSUED),
            "cannot issue more than once in a single clock cycle"
        );

        Lock::new(self, rob_lock, write_reg, csrf)
    }

    pub fn tick(
        &mut self,
        rob_lock: &mut ExecuteLock<'_, '_>,
        prf: &mut Prf<U::Reg>,
        csrf: &mut CsrFile,
    ) -> Bypass<U::Reg> {
        #[cfg(debug_assertions)]
        {
            assert!(
                !self.state.contains(State::EXECUTED),
                "cannot execute more than once in a single clock cycle"
            );
            self.state.insert(State::EXECUTED);
        }

        let mut bypass = BypassBuilder::new(prf);
        match self.cycles.read_copied().checked_sub(1) {
            None => {
                if let Some(Uop {
                    kind: uop,
                    rob_entry,
                }) = self.uop.take()
                {
                    uop.execute(
                        rob_entry,
                        rob_lock,
                        &mut |reg, val| bypass.write(reg, val),
                        csrf,
                    );
                }
            }
            Some(cycles) => self.cycles.write(cycles),
        }
        bypass.build()
    }
}

impl<U> Sequential for Eu<U> {
    fn finish_cycle(&mut self) {
        if let Some(new) = self.new.take() {
            let old = self.uop.replace(new);
            debug_assert!(old.is_none());
        }
        self.cycles.finish_cycle();

        #[cfg(debug_assertions)]
        {
            use bitflags::Flags;
            self.state.clear();
        }
    }
}

#[cfg(debug_assertions)]
bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, Default)]
    struct State: u8 {
        const ISSUED = 1 << 0;
        const EXECUTED = 1 << 1;
    }
}
