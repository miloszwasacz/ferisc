use super::{Eu, Priority, State, Uop};
use crate::core::{
    csr::CsrFile,
    uop::execute::{Execute, Jump, WriteReg},
};
use crate::seq::{Outcome, Substage};
use std::convert::Infallible;

pub struct Issue<'a, U: Uop, J, W> {
    eu: &'a mut Eu<U>,
    fns: Option<Fns<J, W>>,
    csrf: &'a mut CsrFile,
}

pub struct Fns<J, W> {
    jump: J,
    write: W,
}

impl<'a, U: Uop, J: Jump, W: WriteReg<U::Reg>> Issue<'a, U, J, W> {
    pub(super) fn new(eu: &'a mut Eu<U>, jump: J, write: W, csrf: &'a mut CsrFile) -> Self {
        Self {
            eu,
            fns: Some(Fns { jump, write }),
            csrf,
        }
    }
}

impl<'a, U: Uop, J: Jump, W: WriteReg<U::Reg>> Substage for Issue<'a, U, J, W> {
    type In = U;
    type Out = ();
    type Extra = (U, Fns<J, W>);
    type Err = Infallible;

    fn start(&mut self, input: Self::In) -> Outcome<Self::Out, Self::Extra, Self::Err> {
        match self.fns.take() {
            Some(fns) => match self.eu.0.read_bypassed() {
                State::None => Outcome::Ok((), (input, fns)),
                State::Executing { .. } => {
                    let prev = self.fns.replace(fns);
                    debug_assert!(prev.is_none());
                    Outcome::Stop
                }
            },
            None => Outcome::Stop,
        }
    }

    fn finish(&mut self, extra: Self::Extra) {
        let (uop, Fns { jump, write }) = extra;
        let latency = uop.latency();
        self.eu
            .perform_tick(uop, latency, Priority::Issue, jump, write, self.csrf);
    }
}
