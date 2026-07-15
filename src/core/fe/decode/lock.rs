use super::Decoder;
use crate::core::{
    Mode,
    csr::CsrFile,
    fe::addr_gen,
    uop::{
        decode::{Uop, UopKind, int},
        predecode::Predecoded,
    },
};
use crate::mem::Addr;
use crate::seq::locks::{CircuitLock, Transformer};
use std::marker::PhantomData;
use std::slice::Iter;

pub struct DecodeLock<'a, 'g, D> {
    decoders: Option<Iter<'a, D>>,
    csrf: &'a CsrFile,
    addr_gen: addr_gen::Lock<'g>,
    mode: Mode,
}

impl<'a, 'g, D> DecodeLock<'a, 'g, D> {
    pub(super) fn new(
        decoders: &'a mut [D],
        csrf: &'a CsrFile,
        addr_gen: addr_gen::Lock<'g>,
        mode: Mode,
    ) -> Self {
        Self {
            decoders: Some(decoders.iter()),
            csrf,
            addr_gen,
            mode,
        }
    }
}

impl<'a, 'g, D: Decoder + 'a> CircuitLock<'a> for DecodeLock<'a, 'g, D> {
    type Trans = DecodeTrans<'a, 'g, D>;

    fn make_trans(&mut self) -> Self::Trans {
        DecodeTrans {
            decoder: self.decoders.as_mut().and_then(Iterator::next),
            jump: None,
            _addr_gen: PhantomData,
        }
    }
}

pub struct DecodeTrans<'a, 'g, D> {
    decoder: Option<&'a D>,
    jump: Option<Addr>,
    _addr_gen: PhantomData<addr_gen::Lock<'g>>,
}

impl<'a, 'g, D: Decoder + 'a> Transformer<'a> for DecodeTrans<'a, 'g, D> {
    type In = &'a Predecoded;
    type Out = Uop;
    type Lock = DecodeLock<'a, 'g, D>;

    fn transform(&mut self, lock: &Self::Lock, input: Self::In) -> Option<Self::Out> {
        self.decoder.map(|decoder| {
            let mut uop = decoder.decode(*input, lock.csrf, lock.mode);
            match &mut uop.kind {
                UopKind::Int(int::Uop::Jal { target, pred, .. })
                    if pred.is_none_or(|addr| addr != *target) =>
                {
                    let old = self.jump.replace(*target);
                    debug_assert!(old.is_none());
                    //TODO Notify BTB
                }
                UopKind::Int(int::Uop::Br {
                    next,
                    target,
                    pred,
                    btb_pred,
                    ..
                }) => {
                    *pred = false; // TODO BP
                    let old = match *btb_pred {
                        // Taken, but no BTB prediction
                        None if *pred => self.jump.replace(*target),
                        // Not-taken, but BTB predicted taken
                        Some(_) if !*pred => {
                            //TODO Notify BTB
                            self.jump.replace(*next)
                        }
                        // Taken, but BTB predicted the wrong target
                        Some(addr) if addr != *target => {
                            //TODO Notify BTB
                            self.jump.replace(*target)
                        }
                        _ => None,
                    };
                    debug_assert!(old.is_none());
                }
                _ => {}
            }
            uop
        })
    }

    fn commit(self, lock: &mut Self::Lock) {
        if let Some(target) = self.jump {
            lock.addr_gen.decode(target);
            lock.decoders = None;
        }
    }
}
