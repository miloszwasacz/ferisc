pub use self::lock::PredecodeLock;
use crate::seq::{Buf, Clearable, Sequential};

type CrossBlock = Option<[u8; 2]>;

#[derive(Debug, Default)]
pub struct Predecoder(Buf<CrossBlock>);

impl Predecoder {
    pub fn lock(&mut self) -> PredecodeLock<'_> {
        let cross = self.0.read_copied();
        PredecodeLock(self, cross)
    }
}

impl Sequential for Predecoder {
    fn finish_cycle(&mut self) {
        self.0.finish_cycle();
    }
}

impl Clearable for Predecoder {
    fn clear(&mut self) {
        self.0.clear();
    }
}

mod lock {
    use super::*;
    use crate::core::uop::{
        fetch::FetchPacket,
        predecode::{Meta, PdcdBytes, Predecoded},
    };
    use crate::mem::Addr;
    use std::collections::VecDeque;

    pub struct PredecodeLock<'a>(pub(super) &'a mut Predecoder, pub(super) CrossBlock);

    impl PredecodeLock<'_> {
        pub fn transform(&mut self, packet: FetchPacket) -> VecDeque<Predecoded> {
            let FetchPacket {
                mut addr,
                bytes,
                //TODO BTB pred
                btb_pred: _,
            } = packet;

            let mut out = VecDeque::new();
            let bytes_owned = match bytes {
                Ok(bytes) => bytes,
                Err(err) => {
                    return [Predecoded {
                        meta: Meta {
                            addr,
                            //TODO BP
                            pred: false,
                        },
                        bytes: Err(err),
                    }]
                    .into();
                }
            };
            let mut bytes = bytes_owned.as_ref();
            debug_assert!(bytes.len().is_multiple_of(2));

            if let Some(half) = self.1.take() {
                debug_assert!(bytes.len() >= 2);
                let raw = i32::from_le_bytes([half[0], half[1], bytes[0], bytes[1]]);
                out.push_back(Predecoded {
                    meta: Meta {
                        addr: addr - 2,
                        //TODO BP
                        pred: false,
                    },
                    bytes: Ok(PdcdBytes::Normal(raw)),
                });
                addr += 2;
                bytes = &bytes[2..];
            }

            while bytes.len() >= 2 {
                let (pdcd, len) = if bytes[0] & 0b11 == 0b11 {
                    let len = 4;
                    if bytes.len() < len {
                        self.1 = Some([bytes[0], bytes[1]]);
                        break;
                    }

                    let bytes = i32::from_le_bytes(bytes[..len].try_into().unwrap());
                    (PdcdBytes::Normal(bytes), len)
                } else {
                    let len = 2;
                    let bytes = i16::from_le_bytes(bytes[..len].try_into().unwrap());
                    (PdcdBytes::Compressed(bytes), len)
                };

                out.push_back(Predecoded {
                    meta: Meta {
                        addr,
                        //TODO BP
                        pred: false,
                    },
                    bytes: Ok(pdcd),
                });
                addr += len as Addr;
                bytes = &bytes[len..];
            }
            out
        }

        pub fn forget(self) {
            let Self(_, _) = self;
        }
    }

    impl Drop for PredecodeLock<'_> {
        fn drop(&mut self) {
            self.0.0.write(self.1);
        }
    }
}
