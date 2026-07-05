use self::lock::Lock;
use crate::core::{
    csr::CsrFile,
    ee::{
        rob::locks::ExecuteLock,
        rs::{Bypass, BypassBuilder},
    },
    mem::AddrBypassBuilder,
    reg::phys::Prf,
    uop::execute::{Execute, Latency, Uop, WriteReg},
};
use crate::seq::{Buf, Clearable, Sequential, locks::StageLock};

mod lock;

//TODO Pipelined EU
#[derive(Debug, Clone)]
pub struct Eu<U> {
    uop: Option<Uop<U>>,
    new: Option<Uop<U>>,
    cycles: Buf<Latency>,
    #[cfg(debug_assertions)]
    state: State,
}

impl<U: Execute> Eu<U> {
    pub fn new(config: &Config) -> Self {
        let _ = config;
        Self {
            uop: None,
            new: None,
            cycles: Default::default(),
            #[cfg(debug_assertions)]
            state: State::empty(),
        }
    }

    pub fn issue(
        &mut self,
        rob_lock: &mut ExecuteLock<'_, '_, '_>,
        write_reg: impl WriteReg<U::Reg>,
        addr_bypass: &mut AddrBypassBuilder,
        csrf: &mut CsrFile,
    ) -> impl StageLock<'_, Uop<U>, ()> {
        #[cfg(debug_assertions)]
        assert!(
            !self.state.contains(State::ISSUED),
            "cannot issue more than once in a single clock cycle"
        );

        Lock::new(self, rob_lock, write_reg, addr_bypass, csrf)
    }

    pub fn tick(
        &mut self,
        rob_lock: &mut ExecuteLock<'_, '_, '_>,
        addr_bypass: &mut AddrBypassBuilder,
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
                        addr_bypass,
                        csrf,
                    );
                }
            }
            Some(cycles) => self.cycles.write(cycles),
        }
        bypass.build()
    }

    //TODO Document how `flush` is different from `clear` (i.e. flush vs flush all)
    pub fn flush(&mut self) {
        #[cfg(debug_assertions)]
        {
            assert!(
                self.state.intersects(State::CLEARED),
                "cannot flush and issue or execute in the same clock cycle"
            );
            self.state.insert(State::CLEARED);
        }
        debug_assert!(self.new.is_none());

        match self.uop.as_ref() {
            None => debug_assert_eq!(self.cycles.read_copied(), Default::default()),
            Some(uop) => {
                if uop.rob_entry.is_flushed() {
                    self.uop = None;
                    self.cycles.clear();
                }
            }
        }
    }
}

impl<U> Default for Eu<U> {
    fn default() -> Self {
        Self {
            uop: None,
            new: None,
            cycles: Default::default(),
            #[cfg(debug_assertions)]
            state: Default::default(),
        }
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

impl<U> Clearable for Eu<U> {
    fn clear(&mut self) {
        #[cfg(debug_assertions)]
        {
            assert!(
                self.state.intersects(State::CLEARED),
                "cannot flush and issue or execute in the same clock cycle"
            );
            self.state.insert(State::CLEARED);
        }
        debug_assert!(self.new.is_none());

        *self = Self {
            #[cfg(debug_assertions)]
            state: self.state,
            ..Default::default()
        };
    }
}

#[cfg(debug_assertions)]
bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, Default)]
    struct State: u8 {
        const ISSUED = 1 << 0;
        const EXECUTED = 1 << 1;
        const CLEARED = 1 << 2 | Self::ISSUED.bits() | Self::EXECUTED.bits();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {}

#[derive(Debug, Clone)]
pub struct Stats<U>(Option<Uop<U>>);

impl<U> Stats<U> {
    pub fn executing(&self) -> Option<&Uop<U>> {
        self.0.as_ref()
    }
}

impl<U: Clone> Eu<U> {
    pub fn stats(&self) -> Stats<U> {
        #[cfg(debug_assertions)]
        assert!(
            self.state.is_empty(),
            "cannot create statistics in the middle of a clock cycle"
        );

        Stats(self.uop.clone())
    }
}
