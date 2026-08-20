pub use self::commit::Commit;
pub use self::rename::Rename;
use super::{Priority, Rat};
use crate::core::reg::{
    ArchReg,
    phys::{Prf, RegDest},
};
use crate::seq::{Outcome, Substage};
use std::convert::Infallible;

struct RatSubstage<'a, T: ArchReg> {
    rat: &'a mut Rat<T>,
    prf: &'a mut Prf<T>,
    priority: usize,
}

impl<'a, T: ArchReg> RatSubstage<'a, T> {
    fn new(rat: &'a mut Rat<T>, prf: &'a mut Prf<T>) -> Self {
        Self {
            rat,
            prf,
            priority: Default::default(),
        }
    }
}

macro_rules! impl_new {
    ($name:ident) => {
        impl<'a, T: ArchReg> $name<'a, T> {
            pub(in crate::core::reg::phys::rat) fn new(
                rat: &'a mut Rat<T>,
                prf: &'a mut Prf<T>,
            ) -> Self {
                Self(RatSubstage::new(rat, prf))
            }
        }
    };
}

mod rename {
    use super::*;
    use crate::core::{
        reg::phys::{HARDWIRED_IDX, RegOper, rat::free_list::AllocLock},
        uop::{
            Infer, InferUop,
            decode::Uop as DUop,
            rename::{Rename as UopRename, Repr, Uop as RUop},
        },
    };

    pub struct Rename<'a, T: ArchReg>(RatSubstage<'a, T>);

    pub struct RenameMeta<T: ArchReg> {
        arch: T,
        lock: AllocLock,
        value: Option<Repr<T>>,
    }

    impl_new!(Rename);

    impl<'a, T: ArchReg> Substage for Rename<'a, T>
    where
        DUop: InferUop<T>,
        RUop: InferUop<T>,
        Infer<DUop, T>: UopRename<Reg = T, Renamed = Infer<RUop, T>>,
    {
        type In = Infer<DUop, T>;
        type Out = Infer<RUop, T>;
        type Extra = Option<RenameMeta<T>>;
        type Err = Infallible;

        fn start(&mut self, input: Self::In) -> Outcome<Self::Out, Self::Extra, Self::Err> {
            let rat = &mut self.0.rat;
            let mut extra = None;

            let read_mapping = |reg| *rat.mapping[reg].read_bypassed();
            let get_oper = |reg| RegOper {
                #[cfg(debug_assertions)]
                id: rat.id,
                idx: read_mapping(reg),
                arch: reg,
            };
            let alloc_dest = |reg: T, value| {
                if reg.is_hardwired() {
                    return Some(RegDest {
                        oper: RegOper {
                            #[cfg(debug_assertions)]
                            id: rat.id,
                            idx: HARDWIRED_IDX,
                            arch: reg,
                        },
                        prev: HARDWIRED_IDX,
                    });
                }

                rat.free_list.try_alloc().map(|lock| {
                    let dest = RegDest {
                        oper: RegOper {
                            #[cfg(debug_assertions)]
                            id: rat.id,
                            idx: lock.idx(),
                            arch: reg,
                        },
                        prev: read_mapping(reg),
                    };
                    let old = extra.replace(RenameMeta {
                        arch: dest.arch,
                        lock,
                        value,
                    });
                    debug_assert!(old.is_none());
                    dest
                })
            };

            match input.rename(get_oper, alloc_dest) {
                Some(uop) => Outcome::Ok(uop, extra),
                None => Outcome::Stop,
            }
        }

        fn finish(&mut self, extra: Option<RenameMeta<T>>) {
            if let Some(dest) = extra {
                let phys = dest.lock.idx();

                // Update mapping
                self.0.rat.mapping[dest.arch].write(phys, Priority::Rename(self.0.priority));

                // Update free list
                dest.lock.commit(&mut self.0.rat.free_list);

                // Update PRF
                let reg = &mut self.0.prf.regs[phys];
                match dest.value {
                    None => reg.mark_busy(),
                    Some(value) => reg.write(value),
                }
            }

            self.0.priority += 1;
        }
    }
}

mod commit {
    use super::*;

    pub struct Commit<'a, T: ArchReg>(RatSubstage<'a, T>);

    impl_new!(Commit);

    impl<'a, T: ArchReg> Substage for Commit<'a, T> {
        type In = RegDest<T>;
        type Out = ();
        type Extra = RegDest<T>;
        type Err = Infallible;

        fn start(&mut self, input: Self::In) -> Outcome<Self::Out, Self::Extra, Self::Err> {
            match self.0.prf[*input].try_read() {
                Some(_) => Outcome::Ok((), input),
                None => Outcome::Stop,
            }
        }

        fn finish(&mut self, dest: Self::Extra) {
            if !dest.arch.is_hardwired() {
                // Update mapping
                self.0.rat.committed[dest.arch].write(dest.idx, Priority::Commit(self.0.priority));

                // Update free list
                self.0.rat.free_list.dealloc(dest.prev);
            }

            self.0.priority += 1;
        }
    }
}
