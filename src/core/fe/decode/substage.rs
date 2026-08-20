pub use self::input::In;
pub use self::output::Out;
use super::DecodeUnit;
use crate::core::uop::decode::Uop;

mod input {
    use super::*;
    use crate::core::{
        Mode,
        csr::CsrFile,
        fe::{addr_gen, decode::Decoder},
        uop::{
            decode::{Kind, UopKind, int},
            predecode::Predecoded,
        },
    };
    use crate::mem::Addr;
    use crate::seq::{Outcome, Substage, queue::Push};
    use std::convert::Infallible;
    use std::slice::Iter;

    pub struct In<'a, 'g, D> {
        decoders: Option<Iter<'a, D>>,
        queue: Push<'a, Uop>,
        addr_gen: addr_gen::Lock<'g>,
        csrf: &'a CsrFile,
        mode: Mode,
    }

    pub struct Meta {
        pushed: Uop,
        jump: Option<Addr>,
    }

    impl<'a, 'g, D: Decoder> In<'a, 'g, D> {
        pub(in crate::core::fe::decode) fn new(
            decode: &'a mut DecodeUnit<D>,
            addr_gen: addr_gen::Lock<'g>,
            csrf: &'a CsrFile,
            mode: Mode,
        ) -> Self {
            Self {
                decoders: Some(decode.decoders.iter()),
                queue: decode.out_queue.push(),
                addr_gen,
                csrf,
                mode,
            }
        }
    }

    impl<'a, 'g, D: Decoder> Substage for In<'a, 'g, D> {
        type In = Predecoded;
        type Out = ();
        type Extra = Meta;
        type Err = Infallible;

        fn start(&mut self, input: Self::In) -> Outcome<Self::Out, Self::Extra, Self::Err> {
            let Some(decoder) = self.decoders.as_mut().and_then(Iter::next) else {
                return Outcome::Stop;
            };
            let mut uop = decoder.decode(input, self.csrf, self.mode);
            let jump = match &mut uop.kind {
                UopKind::Pipeline(Kind::Int(int::Uop::Jal { target, pred, .. }))
                    if pred.is_none_or(|addr| addr != *target) =>
                {
                    //TODO Notify BTB
                    Some(*target)
                }
                UopKind::Pipeline(Kind::Int(int::Uop::Br {
                    next,
                    target,
                    pred,
                    btb_pred,
                    ..
                })) => {
                    *pred = false; //TODO BP
                    match *btb_pred {
                        // Taken, but no BTB prediction
                        None if *pred => Some(*target),
                        // Not-taken, but BTB prediction taken
                        Some(_) if !*pred => {
                            //TODO Notify BTB
                            Some(*next)
                        }
                        // Taken, but BTB predicted the wrong target
                        Some(addr) if addr != *target => {
                            //TODO Notify BTB
                            Some(*target)
                        }
                        _ => None,
                    }
                }
                _ => None,
            };
            self.queue
                .start(uop)
                .map_meta(|pushed| Meta { pushed, jump })
        }

        fn finish(&mut self, extra: Self::Extra) {
            self.queue.finish(extra.pushed);
            if let Some(target) = extra.jump {
                self.decoders = None;
                self.addr_gen.decode(target);
            }
        }
    }
}

mod output {
    use super::*;
    use crate::seq::{SimpleSubStage, queue::Pop};

    pub struct Out<'a>(Pop<'a, Uop>);

    impl<'a> Out<'a> {
        pub(in crate::core::fe::decode) fn new<D>(decode: &'a mut DecodeUnit<D>) -> Self {
            Self(decode.out_queue.pop(decode.width))
        }
    }

    impl<'a> SimpleSubStage for Out<'a> {
        type In = <Pop<'a, Uop> as SimpleSubStage>::In;
        type Out = <Pop<'a, Uop> as SimpleSubStage>::Out;

        fn start(&mut self, input: Self::In) -> Option<Self::Out> {
            self.0.start(input)
        }

        fn finish(&mut self) {
            self.0.finish()
        }
    }
}
