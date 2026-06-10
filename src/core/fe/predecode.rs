pub use self::lock::PredecodeLock;
use crate::seq::{Buf, Clearable, Sequential};

type CrossBlock = Option<[u8; 2]>;

#[derive(Debug, Default)]
pub struct Predecoder(Buf<CrossBlock>);

impl Predecoder {
    pub fn lock(&mut self) -> PredecodeLock<'_> {
        let cross = self.0.read_copied();
        PredecodeLock::new(self, cross)
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

    pub struct PredecodeLock<'a> {
        predecoder: &'a mut Predecoder,
        cross_block: CrossBlock,
        forget: bool,
    }

    impl<'a> PredecodeLock<'a> {
        pub(super) fn new(predecoder: &'a mut Predecoder, cross_block: CrossBlock) -> Self {
            Self {
                predecoder,
                cross_block,
                forget: false,
            }
        }

        pub fn transform(&mut self, packet: FetchPacket) -> VecDeque<Predecoded> {
            let FetchPacket {
                mut addr,
                bytes,
                pred,
            } = packet;

            let mut out = VecDeque::new();
            let bytes_owned = match bytes {
                Ok(bytes) => bytes,
                Err(err) => {
                    return [Predecoded {
                        meta: Meta { addr, pred },
                        bytes: Err(err),
                    }]
                    .into();
                }
            };
            let mut bytes = bytes_owned.as_ref();
            debug_assert!(bytes.len().is_multiple_of(2));

            if let Some(half) = self.cross_block.take() {
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
                let (pdcd, len) = if bytes[0] & 0b11 == 0b11 {
                    let len = 4;
                    if bytes.len() < len {
                        self.cross_block = Some([bytes[0], bytes[1]]);
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
                    meta: Meta { addr, pred },
                    bytes: Ok(pdcd),
                });
                addr += len as Addr;
                bytes = &bytes[len..];
            }
            out
        }

        pub fn forget(mut self) {
            self.forget = true;
        }
    }

    impl Drop for PredecodeLock<'_> {
        fn drop(&mut self) {
            if !self.forget {
                self.predecoder.0.write(self.cross_block);
            }
        }
    }
}
