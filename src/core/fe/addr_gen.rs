use crate::core::flush::FlushInfo;
use crate::mem::Addr;
use crate::seq::{Buf, Sequential};
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
// - commit -> mispredictions
// - decode -> wrong target from BTB or BP vs BTB mismatch
// - BP     -> branch
// - BTB    -> jump target if known _taken_ branch
// - fetch  -> last fully processed PC (including stalling because of memory)

#[derive(Debug, Clone)]
pub struct AddrGenLock<'a>(Rc<RefCell<AddrGenLockImpl<'a>>>);

#[derive(Debug)]
struct AddrGenLockImpl<'a> {
    addr_gen: &'a mut AddrGen,
    mux: Mux,
    #[cfg(debug_assertions)]
    applied: bool,
}

impl<'a> AddrGenLock<'a> {
    pub fn next_pc(&self) -> Addr {
        self.0.borrow().addr_gen.next_pc.read_copied()
    }

    pub fn fetch(&mut self, processed_pc: Addr) {
        self.0.borrow_mut().mux.add_input(Mux::Fetch(processed_pc));
    }

    pub fn btb(&mut self, target: Addr) {
        self.0.borrow_mut().mux.add_input(Mux::Btb(target));
    }

    pub fn bp(&mut self, new_pc: Addr) {
        self.0.borrow_mut().mux.add_input(Mux::Bp(new_pc));
    }

    pub fn decode(&mut self, jump_target: Addr) {
        self.0.borrow_mut().mux.add_input(Mux::Decode(jump_target));
    }

    pub fn commit(&mut self, next_pc: Addr) {
        self.0.borrow_mut().mux.add_input(Mux::Commit(next_pc));
    }

    pub fn apply(self) -> FlushInfo {
        let mut lock = self.0.borrow_mut();
        let next_pc = match lock.mux {
            Mux::Fetch(pc) | Mux::Btb(pc) | Mux::Bp(pc) | Mux::Decode(pc) | Mux::Commit(pc) => pc,
            Mux::None => panic!("at least one input must be supplied to the multiplexer"),
        };
        lock.addr_gen.next_pc.write(next_pc);
        #[cfg(debug_assertions)]
        {
            lock.applied = true;
        }

        match lock.mux {
            Mux::Fetch(_) | Mux::Btb(_) => FlushInfo::none(),
            Mux::Bp(_) => FlushInfo::fetch(),
            Mux::Decode(_) => FlushInfo::decode(),
            Mux::Commit(_) => FlushInfo::commit(),
            Mux::None => unreachable!(),
        }
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
    Fetch(Addr),
    Btb(Addr),
    Bp(Addr),
    Decode(Addr),
    Commit(Addr),
    //TODO Interrupts
}

impl Mux {
    fn add_input(&mut self, input: Self) {
        debug_assert_ne!(
            *self, input,
            "cannot add input from the same stage more than once"
        );
        if *self < input {
            *self = input
        }
    }
}
