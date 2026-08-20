use self::substage::Issue;
pub use self::uop::Uop;
use crate::core::{
    csr::CsrFile,
    ee::rob,
    uop::execute::{Jump, Latency, WriteReg},
};
use crate::seq::{Clearable, Pbuf, Sequential};
use std::num::NonZero;

pub mod substage;
mod uop;

//TODO Pipelined EU
#[derive(Debug, Clone)]
pub struct Eu<U>(Pbuf<State<U>, Priority>);

#[derive(Debug, Clone, Default)]
enum State<U> {
    #[default]
    None,
    Executing {
        uop: U,
        cycles_left: NonZero<Latency>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Priority {
    Tick,
    Issue,
    Flush,
}

impl<U: Uop> Eu<U> {
    pub fn new(config: &Config) -> Self {
        let Config {} = config;
        Self(Default::default())
    }

    pub fn issue<'a, J, W>(
        &'a mut self,
        jump: J,
        write: W,
        csrf: &'a mut CsrFile,
    ) -> Issue<'a, U, J, W>
    where
        J: Jump,
        W: WriteReg<U::Reg>,
    {
        Issue::new(self, jump, write, csrf)
    }

    pub fn execute(&mut self, jump: impl Jump, write: impl WriteReg<U::Reg>, csrf: &mut CsrFile)
    where
        U: Clone,
    {
        let State::Executing { uop, cycles_left } = self.0.read_bypassed() else {
            return;
        };
        self.perform_tick(
            uop.clone(),
            cycles_left.get() - 1,
            Priority::Tick,
            jump,
            write,
            csrf,
        );
    }

    fn perform_tick(
        &mut self,
        uop: U,
        latency: Latency,
        priority: Priority,
        jump: impl Jump,
        write: impl WriteReg<U::Reg>,
        csrf: &mut CsrFile,
    ) {
        let new = match NonZero::new(latency) {
            Some(cycles_left) => State::Executing { uop, cycles_left },
            None => {
                uop.execute(jump, write, csrf);
                State::None
            }
        };
        self.0.write(new, priority);
    }

    //TODO Document how `flush` is different from `clear` (i.e. flush vs flush all)
    pub fn flush(&mut self, flush: &rob::lock::Flush<'_>) {
        let State::Executing { uop, .. } = self.0.read() else {
            return;
        };
        if uop.is_flushed(flush) {
            self.0.write(State::None, Priority::Flush);
        }
    }
}

impl<U> Sequential for Eu<U> {
    /// Marks the end of the clock cycle, finalizing any outstanding operations.
    ///
    /// <div class="warning">
    ///
    /// This method updates the state of _ONLY_ the execution unit, not the contained uOP.
    ///
    /// </div>
    fn finish_cycle(&mut self) {
        self.0.finish_cycle();
    }
}

impl<U> Clearable for Eu<U> {
    fn clear(&mut self) {
        self.0.clear();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {}

#[derive(Debug, Clone)]
pub struct Stats<U>(Option<U>);

impl<U> Stats<U> {
    pub fn executing(&self) -> Option<&U> {
        self.0.as_ref()
    }
}

impl<U: Clone> Eu<U> {
    pub fn stats(&self) -> Stats<U> {
        Stats(match self.0.read() {
            State::None => None,
            State::Executing { uop, .. } => Some(uop.clone()),
        })
    }
}
