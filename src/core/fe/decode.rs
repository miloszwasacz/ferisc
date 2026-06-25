use self::lock::DecodeLock;
use crate::core::{
    Mode,
    csr::{CsrFile, m::misa::Mxl},
    fe::AddrGenLock,
    uop::{
        Exception, UopKind,
        decode::Uop,
        predecode::{PdcdBytes, Predecoded},
    },
};
use crate::seq::{
    Clearable, Queue, Sequential,
    locks::{LockCompose, StageLock},
};

pub mod rv32;
pub mod rv64;

pub mod fmt;
mod lock;

#[derive(Debug)]
pub struct DecodeUnit<D = DefaultDecoder> {
    decoders: Box<[D]>,
    out_queue: Queue<Uop>,
    //TODO Different IN and OUT widths
    width: usize,
    flushed: bool,
}

impl<D: Decoder> DecodeUnit<D> {
    pub fn new(width: usize) -> Self {
        Self::with_queue_size(width, width)
    }

    pub fn with_queue_size(width: usize, queue_size: usize) -> Self {
        let mut dec = Vec::new();
        dec.resize_with(width, Default::default);

        Self {
            decoders: dec.into_boxed_slice(),
            out_queue: Queue::new(queue_size),
            width,
            flushed: false,
        }
    }

    pub fn decode<'a, 'g>(
        &'a mut self,
        csrf: &'a CsrFile,
        addr_gen: AddrGenLock<'g>,
        mode: Mode,
    ) -> impl StageLock<'a, &'a Predecoded, ()> {
        LockCompose::new(
            DecodeLock::new(&mut self.decoders, csrf, addr_gen, mode),
            self.out_queue.push(),
        )
    }

    pub fn lock(&mut self) -> impl StageLock<'_, (), &Uop> {
        self.out_queue.pop(self.width)
    }
}

impl<D> Sequential for DecodeUnit<D> {
    fn finish_cycle(&mut self) {
        self.out_queue.finish_cycle();
        if self.flushed {
            self.flushed = false;
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
                            Some(UopKind::Except(Exception::Illegal))
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
                    kind: UopKind::Except(ex),
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
        .unwrap_or(UopKind::Except(Exception::Illegal));

        Uop { kind, addr, comp }
    }
}
