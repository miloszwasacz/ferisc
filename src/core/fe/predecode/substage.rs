pub use self::input::In;
pub use self::output::Out;
use super::PredecodeUnit;
use crate::core::uop::predecode::Predecoded;
use std::collections::VecDeque;

mod input {
    use super::*;
    use crate::core::{
        fe::predecode::CrossBlock,
        uop::{
            fetch::FetchPacket,
            predecode::{Meta, PdcdBytes},
        },
    };
    use crate::mem::Addr;
    use crate::seq::{Outcome, Substage};
    use std::convert::Infallible;

    pub struct In<'a> {
        predecode: &'a mut PredecodeUnit,
        cb: CrossBlock,
    }

    pub struct Extra {
        new: VecDeque<Predecoded>,
        cb: CrossBlock,
    }

    impl<'a> In<'a> {
        pub(in crate::core::fe::predecode) fn new(predecode: &'a mut PredecodeUnit) -> Self {
            debug_assert!(
                predecode.out_queue.len == predecode.out_queue.len(),
                "cannot predecode more than once per clock cycle"
            );

            Self {
                cb: predecode.cross_block.read_copied(),
                predecode,
            }
        }
    }

    impl<'a> Substage for In<'a> {
        type In = FetchPacket;
        type Out = ();
        type Extra = Extra;
        type Err = Infallible;

        fn start(&mut self, input: Self::In) -> Outcome<Self::Out, Self::Extra, Self::Err> {
            let FetchPacket {
                mut addr,
                bytes,
                pred,
            } = input;
            let used_space = || self.predecode.out_queue.buf.len() - self.predecode.out_queue.len;

            if used_space() >= self.predecode.width.get() {
                return Outcome::Stop;
            }

            let mut cb = self.cb;
            let bytes = match bytes {
                Ok(bytes) => bytes,
                Err(ex) => {
                    let extra = Extra {
                        new: [Predecoded {
                            meta: Meta { addr, pred },
                            bytes: Err(ex),
                        }]
                        .into(),
                        cb,
                    };
                    return Outcome::Ok((), extra);
                }
            };

            let mut out = VecDeque::new();
            let mut bytes = bytes.as_slice();
            debug_assert!(bytes.len().is_multiple_of(2));

            if let Some(half) = cb.take() {
                debug_assert!(bytes.len() >= 2);
                let raw = i32::from_le_bytes([half[0], half[1], bytes[0], bytes[1]]);
                out.push_back(Predecoded {
                    meta: Meta {
                        addr: addr - 2,
                        pred,
                    },
                    bytes: Ok(PdcdBytes::Normal(raw)),
                });
                addr += 2;
                bytes = &bytes[2..];
            }

            while bytes.len() >= 2 {
                let (pdcd, len) = if (bytes[0] & 0b11) == 0b11 {
                    let len = 4;
                    if bytes.len() < len {
                        cb = Some([bytes[0], bytes[1]]);
                        break;
                    }

                    let bytes = i32::from_le_bytes(bytes[..len].try_into().unwrap());
                    (PdcdBytes::Normal(bytes), len)
                } else {
                    let len = 2;
                    let bytes = i16::from_le_bytes(bytes[..len].try_into().unwrap());
                    (PdcdBytes::Compressed(bytes), len)
                };

                if used_space() + out.len() > self.predecode.width.get() {
                    return Outcome::Stop;
                }
                out.push_back(Predecoded {
                    meta: Meta { addr, pred },
                    bytes: Ok(pdcd),
                });
                addr += len as Addr;
                bytes = &bytes[len..];
            }
            Outcome::Ok((), Extra { new: out, cb })
        }

        fn finish(&mut self, mut extra: Self::Extra) {
            self.predecode.out_queue.append(&mut extra.new);
            self.cb = extra.cb;
        }
    }

    impl Drop for In<'_> {
        fn drop(&mut self) {
            self.predecode.cross_block.write(self.cb);
        }
    }
}

mod output {
    use super::*;
    use crate::seq::SimpleSubStage;
    use std::collections::vec_deque::Iter;
    use std::iter::Take;

    pub struct Out<'a> {
        queue: Take<Iter<'a, Predecoded>>,
        popped: &'a mut usize,
    }

    impl<'a> Out<'a> {
        pub(in crate::core::fe::predecode) fn new(predecode: &'a mut PredecodeUnit) -> Self {
            debug_assert_eq!(
                predecode.state.popped, 0,
                "cannot pop more than once per clock cycle"
            );

            let max_count = predecode.out_queue.len.min(predecode.width.get());
            Self {
                queue: predecode.out_queue.iter().take(max_count),
                popped: &mut predecode.state.popped,
            }
        }
    }

    impl<'a> SimpleSubStage for Out<'a> {
        type In = ();
        type Out = &'a Predecoded;

        fn start(&mut self, (): Self::In) -> Option<Self::Out> {
            self.queue.next()
        }

        fn finish(&mut self) {
            *self.popped += 1;
        }
    }
}
