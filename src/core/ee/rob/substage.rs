pub use self::commit::Commit;
pub use self::dispatch::Dispatch;
use crate::core::ee::rob::{Rob, entry::Entry};
use crate::seq::{Outcome, Substage};
use std::convert::Infallible;

mod dispatch {
    use super::*;
    use crate::core::{
        ee::{pipeline::Kind, rob::Uop},
        mem,
        uop::{
            SpecialUopKind, UopKind,
            commit::Dest,
            dispatch::{self, Handled},
            rename,
        },
    };
    use crate::seq::queue::Push;

    pub struct Dispatch<'a>(Push<'a, Entry>, mem::Dispatch<'a>);
    pub struct Meta(Entry, Option<mem::substage::DispatchMeta>);

    impl<'a> Dispatch<'a> {
        pub(in crate::core::ee::rob) fn new(rob: &'a mut Rob, mem: mem::Dispatch<'a>) -> Self {
            Self(rob.queue.push(), mem)
        }
    }

    impl<'a> Substage for Dispatch<'a> {
        type In = rename::Uop;
        type Out = Uop<dispatch::Uop>;
        type Extra = Meta;
        type Err = Infallible;

        fn start(&mut self, input: Self::In) -> Outcome<Self::Out, Self::Extra, Self::Err> {
            let entry = Entry::new(input);
            let handle = entry.handle();
            let mut mem_meta = None;
            match self.0.start(entry) {
                Outcome::Ok((), entry) => {
                    let kind = match input.kind {
                        UopKind::Pipeline(kind) => UopKind::Pipeline(match kind {
                            Kind::Int(uop) => Kind::Int(match uop {
                                rename::int::Uop::Alu {
                                    op,
                                    src1,
                                    src2,
                                    dest,
                                } => dispatch::int::Uop::Alu {
                                    op,
                                    src1,
                                    src2,
                                    dest,
                                },
                                rename::int::Uop::Mul {
                                    op,
                                    src1,
                                    src2,
                                    dest,
                                } => dispatch::int::Uop::Mul {
                                    op,
                                    src1,
                                    src2,
                                    dest,
                                },
                                rename::int::Uop::Jal { .. } => {
                                    entry.handle().mark_done();
                                    return Outcome::Ok(
                                        Uop {
                                            uop: dispatch::Uop {
                                                kind: UopKind::Special(Handled),
                                            },
                                            handle,
                                        },
                                        Meta(entry, None),
                                    );
                                }
                                rename::int::Uop::Jalr {
                                    base, offset, pred, ..
                                } => dispatch::int::Uop::Jalr { base, offset, pred },
                                rename::int::Uop::Br {
                                    cmp,
                                    src1,
                                    src2,
                                    next,
                                    target,
                                    pred,
                                    ..
                                } => dispatch::int::Uop::Br {
                                    cmp,
                                    src1,
                                    src2,
                                    next,
                                    target,
                                    pred,
                                },
                                rename::int::Uop::Ld {
                                    size,
                                    signed,
                                    base,
                                    offset,
                                    dest,
                                } => match self.1.start(Uop {
                                    uop: mem::substage::DUop::Ld {
                                        size,
                                        signed,
                                        dest: Dest::Int(dest),
                                    },
                                    handle: handle.clone(),
                                }) {
                                    Outcome::Ok(Some(dest), meta) => {
                                        let prev = mem_meta.replace(meta);
                                        debug_assert!(prev.is_none());
                                        dispatch::int::Uop::Lda { base, offset, dest }
                                    }
                                    Outcome::Stop => return Outcome::Stop,
                                    Outcome::Ok(None, _) => unreachable!(
                                        "memory subsystem should return a valid address handle"
                                    ),
                                },
                                rename::int::Uop::St {
                                    size,
                                    src,
                                    base,
                                    offset,
                                } => match self.1.start(Uop {
                                    uop: mem::substage::DUop::St {
                                        size,
                                        src: mem::Src::Int(src),
                                    },
                                    handle: handle.clone(),
                                }) {
                                    Outcome::Ok(Some(dest), meta) => {
                                        let prev = mem_meta.replace(meta);
                                        debug_assert!(prev.is_none());
                                        dispatch::int::Uop::Sta { base, offset, dest }
                                    }
                                    Outcome::Stop => return Outcome::Stop,
                                    Outcome::Ok(None, _) => unreachable!(
                                        "memory subsystem should return a valid address handle"
                                    ),
                                },
                                rename::int::Uop::Csr { op, csr, src, dest } => {
                                    dispatch::int::Uop::Csr { op, csr, src, dest }
                                }
                            }),
                        }),
                        UopKind::Special(kind) => UopKind::Special(match kind {
                            SpecialUopKind::MemOrd(fence) => match self.1.start(Uop {
                                uop: mem::substage::DUop::Fence(fence),
                                handle: handle.clone(),
                            }) {
                                Outcome::Ok(None, meta) => {
                                    let prev = mem_meta.replace(meta);
                                    debug_assert!(prev.is_none());
                                    Handled
                                }
                                Outcome::Stop => return Outcome::Stop,
                                Outcome::Ok(Some(_), _) => unreachable!(
                                    "memory subsystem should not return any address handles for a fence"
                                ),
                            },
                            SpecialUopKind::Except(_) => Handled,
                        }),
                    };
                    Outcome::Ok(
                        Uop {
                            uop: dispatch::Uop { kind },
                            handle,
                        },
                        Meta(entry, mem_meta),
                    )
                }
                Outcome::Stop => Outcome::Stop,
            }
        }

        fn finish(&mut self, extra: Self::Extra) {
            self.0.finish(extra.0);
            if let Some(extra) = extra.1 {
                self.1.finish(extra);
            }
        }
    }
}

mod commit {
    use super::*;
    use crate::core::{
        Mode,
        csr::{CsrAddr, CsrFile},
        ee::rob::{
            Committed,
            entry::UopKind,
            flush::{Flush, FlushPriority},
        },
        fe::addr_gen,
        mem,
        reg::int::Uint,
        uop::Exception,
    };
    use crate::mem::Addr;
    use crate::seq::{Pbuf, queue::Pop};

    pub struct Commit<'a, 'g, F> {
        inner: Option<Inner<'a, 'g, F>>,
        mem: mem::Commit<'a>,
        csrf: &'a mut CsrFile,
        flush: &'a mut Pbuf<Flush, FlushPriority>,
        count: Uint,
    }

    struct Inner<'a, 'g, F> {
        pop: Pop<'a, Entry>,
        addr_gen: addr_gen::Lock<'g>,
        write_mode: F,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub enum Meta {
        #[default]
        None,
        Csr(CsrAddr),
        MemOrd,
        Exception {
            cause: Exception,
            epc: Addr,
        },
    }

    impl<'a, 'g, F: FnOnce(Mode)> Commit<'a, 'g, F> {
        pub(in crate::core::ee::rob) fn new(
            rob: &'a mut Rob,
            addr_gen: addr_gen::Lock<'g>,
            mem: mem::Commit<'a>,
            csrf: &'a mut CsrFile,
            write_mode: F,
        ) -> Self {
            Self {
                inner: Some(Inner {
                    pop: rob.queue.pop(rob.commit_width.into()),
                    addr_gen,
                    write_mode,
                }),
                mem,
                csrf,
                flush: &mut rob.flush,
                count: 0,
            }
        }
    }

    impl<'a, 'g, F: FnOnce(Mode)> Substage for Commit<'a, 'g, F> {
        type In = ();
        type Out = Committed;
        type Extra = Meta;
        type Err = Infallible;

        fn start(&mut self, input: Self::In) -> Outcome<Self::Out, Self::Extra, Self::Err> {
            let Some(pop) = self.inner.as_mut().map(|inner| &mut inner.pop) else {
                return Outcome::Stop;
            };
            match pop.start(input) {
                Outcome::Ok(entry, ()) if !entry.is_done() => Outcome::Stop,
                Outcome::Ok(entry, ()) => match entry.kind() {
                    UopKind::Reg(dest) | UopKind::Ld(dest) => {
                        Outcome::Ok(Committed::Reg(dest), Meta::None)
                    }
                    UopKind::Br | UopKind::St => Outcome::Ok(Committed::Handled, Meta::None),
                    UopKind::Csr(dest, csr) => Outcome::Ok(Committed::Reg(dest), Meta::Csr(csr)),
                    UopKind::MemOrd(fence) => self
                        .mem
                        .start(fence)
                        .map(|()| Committed::Handled)
                        .map_meta(|()| Meta::MemOrd),
                    UopKind::Except(ex) => Outcome::Ok(
                        Committed::Handled,
                        Meta::Exception {
                            cause: ex,
                            epc: entry.addr(),
                        },
                    ),
                },
                Outcome::Stop => Outcome::Stop,
                Outcome::Err(err) => Outcome::Err(err),
            }
        }

        fn finish(&mut self, extra: Self::Extra) {
            let Inner {
                mut pop,
                addr_gen,
                write_mode,
            } = self
                .inner
                .take()
                .expect("started operation should be completable");

            pop.finish(());
            match extra {
                Meta::None | Meta::MemOrd => {
                    // Continue committing
                    self.inner = Some(Inner {
                        pop,
                        addr_gen,
                        write_mode,
                    });

                    // Finish commiting the fence
                    if matches!(extra, Meta::MemOrd) {
                        self.mem.finish(());
                    }

                    // Increase retired instruction count
                    self.count += 1;
                }
                Meta::Csr(csr) => {
                    // Stop committing
                    debug_assert!(self.inner.is_none());

                    // Mark CSR as committed
                    self.csrf.commit(csr);

                    // Increase retired instruction count
                    self.count += 1;
                }
                Meta::Exception { cause, epc } => {
                    // Stop committing
                    debug_assert!(self.inner.is_none());

                    // Write CSRs
                    self.csrf.mcause.implicit_write(cause.into());
                    self.csrf.mepc.implicit_write(epc.into());
                    todo!("trap handling (cause: {})", cause);

                    //TODO Check ISA if this privilege elevation is correct
                    write_mode(Mode::M);
                    self.flush.write(Flush::All, FlushPriority::Commit);
                    // addr_gen.commit(todo!());

                    // Exceptions don't count as retired
                }
            }
        }
    }

    impl<F> Drop for Commit<'_, '_, F> {
        fn drop(&mut self) {
            let minstret = self.csrf.minstret.read() + self.count;
            self.csrf.minstret.implicit_write(minstret);
        }
    }
}
