use crate::mem::Addr;
use crate::seq::{Buf, Sequential};
use bitflags::bitflags;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct AddrGen {
    next_pc: Buf<Addr>,
}

impl AddrGen {
    pub fn new(entrypoint: Addr) -> Self {
        Self {
            next_pc: Buf::new(entrypoint),
        }
    }

    pub fn lock(&mut self) -> AddrGenLock<'_> {
        AddrGenLock(Rc::new(RefCell::new(AddrGenLockImpl {
            addr_gen: self,
            mux: Mux::None,
            target: Default::default(),
            #[cfg(debug_assertions)]
            applied: false,
        })))
    }
}

impl Sequential for AddrGen {
    fn finish_cycle(&mut self) {
        self.next_pc.finish_cycle();
    }
}

// Addr gen needs info from:
// - interrupts
// - commit -> exceptions
// - execute -> mispredictions
// - decode -> wrong target from BTB or BP vs BTB mismatch
// - BP     -> branch
// - BTB    -> jump target if known _taken_ branch
// - fetch  -> last fully processed PC (including stalling because of memory)

//#region Lock

#[derive(Debug, Clone)]
pub struct AddrGenLock<'a>(Rc<RefCell<AddrGenLockImpl<'a>>>);

#[derive(Debug)]
struct AddrGenLockImpl<'a> {
    addr_gen: &'a mut AddrGen,
    mux: Mux,
    target: Addr,
    #[cfg(debug_assertions)]
    applied: bool,
}

macro_rules! add_input {
    ($addr_gen:expr, $mux:ident, $pc:expr) => {{
        let mut addr_gen = $addr_gen.0.borrow_mut();
        let AddrGenLockImpl { mux, target, .. } = &mut *addr_gen;
        mux.add_input(Mux::$mux, target, $pc);
    }};
}

impl<'a> AddrGenLock<'a> {
    pub fn next_pc(&self) -> Addr {
        self.0.borrow().addr_gen.next_pc.read_copied()
    }

    pub fn fetch(&mut self, processed_pc: Addr) {
        add_input!(self, Fetch, processed_pc);
    }

    pub fn btb(&mut self, target: Addr) {
        add_input!(self, Btb, target);
    }

    pub fn bp(&mut self, new_pc: Addr) {
        add_input!(self, Bp, new_pc);
    }

    pub fn decode(&mut self, jump_target: Addr) {
        add_input!(self, Decode, jump_target);
    }

    pub fn execute(&mut self, jump_target: Addr) {
        add_input!(self, Execute, jump_target);
    }

    pub fn commit(&mut self, next_pc: Addr) {
        add_input!(self, Commit, next_pc);
    }

    pub fn apply(self) -> FeFlush {
        let mut lock = self.0.borrow_mut();

        assert!(
            lock.mux > Mux::None,
            "at least one input must be supplied to the multiplexer"
        );
        let target = lock.target;
        lock.addr_gen.next_pc.write(target);
        #[cfg(debug_assertions)]
        {
            lock.applied = true;
        }

        let mut flush = FeFlush(FlushInfo::empty());
        if lock.mux > Mux::Fetch {
            flush.0.insert(FlushInfo::FETCH);
        }
        if lock.mux > Mux::Decode {
            flush.0.insert(FlushInfo::DECODE);
        }
        flush
    }
}

#[cfg(debug_assertions)]
impl Drop for AddrGenLockImpl<'_> {
    fn drop(&mut self) {
        assert!(self.applied, "address gen lock was not applied");
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
enum Mux {
    #[default]
    None,
    Fetch,
    Btb,
    Bp,
    Decode,
    Execute,
    Commit,
    //TODO Interrupts
}

impl Mux {
    fn add_input(&mut self, input: Self, target: &mut Addr, pc: Addr) {
        debug_assert_ne!(
            *self, input,
            "cannot add input from the same stage more than once"
        );
        if *self < input {
            *self = input;
            *target = pc;
        }
    }
}

//#endregion

//#region Flush info

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FeFlush(FlushInfo);

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    struct FlushInfo: u8 {
        const FETCH = 1 << 0;
        const DECODE = 1 << 1;
    }
}

impl FeFlush {
    pub fn flush_fetch(&self) -> bool {
        self.0.contains(FlushInfo::FETCH)
    }

    pub fn flush_decode(&self) -> bool {
        self.0.contains(FlushInfo::DECODE)
    }
}

//#endregion
