use super::Decoder;
use crate::core::{
    csr::CsrFile,
    uop::{Mode, decode::Uop, predecode::Predecoded},
};
use crate::seq::locks::{CircuitLock, Transformer};
use std::slice::Iter;

pub struct DecodeLock<'a, D> {
    decoders: Iter<'a, D>,
    csrf: &'a CsrFile,
    mode: Mode,
}

impl<'a, D> DecodeLock<'a, D> {
    pub(super) fn new(decoders: &'a mut [D], csrf: &'a CsrFile, mode: Mode) -> Self {
        Self {
            decoders: decoders.iter(),
            csrf,
            mode,
        }
    }
}

impl<'a, D: Decoder + 'a> CircuitLock<'a> for DecodeLock<'a, D> {
    type Trans = DecodeTrans<'a, D>;

    fn make_trans(&mut self) -> Self::Trans {
        DecodeTrans(self.decoders.next())
    }
}

pub struct DecodeTrans<'a, D>(Option<&'a D>);

impl<'a, D: Decoder + 'a> Transformer<'a> for DecodeTrans<'a, D> {
    type In = &'a Predecoded;
    type Out = Uop;
    type Lock = DecodeLock<'a, D>;

    fn transform(&mut self, lock: &Self::Lock, input: Self::In) -> Option<Self::Out> {
        self.0
            .map(|decoder| decoder.decode(*input, lock.csrf, lock.mode))
    }

    fn commit(self, _: &mut Self::Lock) {}
}
