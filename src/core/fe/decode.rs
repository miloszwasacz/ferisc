use self::substage::{In, Out};
use crate::core::{
    Mode,
    csr::{CsrFile, m::misa::Mxl},
    fe::addr_gen,
    uop::{
        Exception, SpecialUopKind,
        decode::{Uop, UopKind},
        predecode::{PdcdBytes, Predecoded},
    },
};
use crate::seq::{Clearable, Queue, Sequential};
use std::num::NonZeroUsize;

pub mod rv32;
pub mod rv64;

pub mod fmt;
pub mod substage;

#[derive(Debug)]
pub struct DecodeUnit<D = DefaultDecoder> {
    decoders: Box<[D]>,
    out_queue: Queue<Uop>,
    //TODO Different IN and OUT widths
    width: NonZeroUsize,
    flushed: bool,
}

impl<D: Decoder> DecodeUnit<D> {
    pub fn new(config: &Config) -> Self {
        let mut dec = Vec::new();
        dec.resize_with(config.width.get() as _, Default::default);

        Self {
            decoders: dec.into_boxed_slice(),
            out_queue: Queue::new(config.queue_size.unwrap_or(config.width.into())),
            width: config.width,
            flushed: false,
        }
    }

    pub fn r#in<'a, 'g>(
        &'a mut self,
        addr_gen: addr_gen::Lock<'g>,
        csrf: &'a CsrFile,
        mode: Mode,
    ) -> In<'a, 'g, D> {
        In::new(self, addr_gen, csrf, mode)
    }

    pub fn out(&mut self) -> Out<'_> {
        Out::new(self)
    }
}

impl<D> Sequential for DecodeUnit<D> {
    fn finish_cycle(&mut self) {
        self.out_queue.finish_cycle();
        if std::mem::take(&mut self.flushed) {
            self.out_queue.clear();
            self.out_queue.finish_cycle();
        }
    }
}

impl<D> Clearable for DecodeUnit<D> {
    fn clear(&mut self) {
        debug_assert!(!self.flushed, "cannot flush more than once per clock cycle");
        self.flushed = true;
    }
}

pub trait Decoder: Default {
    fn decode(&self, predecoded: Predecoded, csrs: &CsrFile, mode: Mode) -> Uop;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultDecoder(());

impl Decoder for DefaultDecoder {
    fn decode(&self, Predecoded { meta, bytes }: Predecoded, csrs: &CsrFile, mode: Mode) -> Uop {
        macro_rules! decode {
            ($module:ident, $bytes:expr, $meta:expr, $mode:expr, $ext:expr) => {{
                match $bytes {
                    PdcdBytes::Normal(bytes) => $module::i::decode(bytes, $meta, $mode),
                    PdcdBytes::Compressed(bytes) => {
                        if $ext.c() {
                            $module::c::decode(bytes, $meta, $mode, $ext)
                        } else {
                            Some(exception(Exception::Illegal))
                        }
                    }
                }
            }};
        }

        let addr = meta.addr;
        let bytes = match bytes {
            Ok(bytes) => bytes,
            Err(ex) => {
                return Uop {
                    kind: exception(ex),
                    addr,
                    comp: false,
                };
            }
        };
        let comp = match bytes {
            PdcdBytes::Normal(_) => false,
            PdcdBytes::Compressed(_) => true,
        };

        let misa = csrs.misa.read();
        debug_assert!(misa.ext.i());
        let kind = match misa.mxl {
            Mxl::RV32 => decode!(rv32, bytes, meta, mode, misa.ext),
            Mxl::RV64 => decode!(rv64, bytes, meta, mode, misa.ext),
        }
        .unwrap_or(exception(Exception::Illegal));

        Uop { kind, addr, comp }
    }
}

const fn exception(ex: Exception) -> UopKind {
    UopKind::Special(SpecialUopKind::Except(ex))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    pub width: NonZeroUsize,
    pub queue_size: Option<NonZeroUsize>,
}

#[derive(Debug, Clone, derive_more::Deref)]
#[deref(forward)]
pub struct Stats(Box<[Uop]>);

impl<D> DecodeUnit<D> {
    pub fn stats(&self) -> Stats {
        Stats(self.out_queue.iter().copied().collect())
    }
}
