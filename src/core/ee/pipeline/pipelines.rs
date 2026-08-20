pub use self::commit::Commit;
pub use self::rename_dispatch::{Dispatch, Rename};
use super::stats::Stats;
use super::*;
use crate::core::{ee::rob::Flushed, uop::commit::Dest};
use crate::seq::{Outcome, Substage};
use std::convert::Infallible;

macro_rules! pipelines {
    (($d:tt) $( $kind:ident ),*) => {
        ::paste::paste! {
            #[derive(Debug)]
            pub struct Pipelines(pub(in $crate::core::ee::pipeline) PipelineSplit<
                $( Pipeline<[< $kind Reg >]> ),*
            >);

            #[derive(Debug)]
            pub(in $crate::core::ee::pipeline) struct PipelineSplit<$( $kind ),*> {
                $( pub [< $kind:snake >]: $kind ),*
            }

            #[derive(Debug, Clone, PartialEq, Eq)]
            pub struct Configs {
                $( pub [< $kind:snake >]: Config ),*
            }

            #[derive(Debug, Clone)]
            pub struct AllStats {
                $( pub [< $kind:snake >]: Stats<[< $kind Reg >]> ),*
            }
        }

        #[derive(Debug, derive_more::Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum Kind< $( $kind ),* > {
            $( $kind($kind) ),*
        }

        ::paste::paste! {
            impl Pipelines {
                #[deny(unused_variables)]
                pub fn new(configs: &Configs) -> Self {
                    let Configs {
                        $( [< $kind:snake >] ),*
                    } = configs;

                    Self(PipelineSplit {
                        $( [< $kind:snake >]: Pipeline::new([< $kind:snake >]) ),*
                    })
                }

                #[deny(unused_variables)]
                pub fn stats(&self) -> AllStats {
                    let PipelineSplit {
                        $( [< $kind:snake >] ),*
                    } = &self.0;

                    AllStats {
                        $( [< $kind:snake >]: [< $kind:snake >].stats() ),*
                    }
                }

                #[deny(unused_variables)]
                pub fn flush(&mut self, flush: &rob::lock::Flush) {
                    $(
                    let mut [< $kind:snake _flushed >] = Vec::new();
                    )*
                    for flushed in flush.flushed() {
                        let (Flushed::Reg(dest) | Flushed::Ld(dest) | Flushed::Csr(dest, _)) = flushed else {
                            continue;
                        };
                        match *dest {
                            $(
                            Dest::$kind(dest) => [< $kind:snake _flushed >].push(dest),
                            )*
                        }
                    }

                    let PipelineSplit {
                        $( [< $kind:snake >] ),*
                    } = &mut self.0;
                    $(
                    [< $kind:snake >].flush(flush, [< $kind:snake _flushed >]);
                    )*
                }
            }

            impl<$( [< $kind P >] ),*> PipelineSplit<$( [< $kind P >] ),*> {
                pub fn map_mut<'a, $( $kind ),*>(
                    &'a mut self,
                    $( [< f_ $kind:snake >]: impl FnOnce(&'a mut [< $kind P >]) -> $kind ),*
                ) -> PipelineSplit<$( $kind ),*> {
                    let Self {
                        $( [< $kind:snake >] ),*
                    } = self;

                    PipelineSplit {
                        $( [< $kind:snake >]: [< f_ $kind:snake >]([< $kind:snake >]) ),*
                    }
                }

                #[inline(always)]
                pub fn split_mut<'a, T, $( $kind ),*>(
                    &'a mut self,
                    kind: Kind< $( $kind ),* >,
                    $( [< f_ $kind:snake >]: impl FnOnce($kind, &'a mut [< $kind P >]) -> T ),*
                ) -> T {
                    let Self {
                        $( [< $kind:snake >] ),*
                    } = self;

                    match kind {
                        $( Kind::$kind(kind) => [< f_ $kind:snake >](kind, [< $kind:snake >]) ),*
                    }
                }
            }

            macro_rules! all {
                ( $d pipelines:expr, $d func:expr $d(, $d args:expr )*) => {{
                    ::paste::paste! {
                        let Pipelines(PipelineSplit {
                            $( [< $kind:snake >] ),*
                        }) = $d pipelines;

                        $(
                        $d func([< $kind: snake >], $d ( $d args ),*);
                        )*
                    }
                }}
            }
            macro_rules! split_mut {
                ($d pipelines:expr, $d kind:expr, |$d k:ident, $d p:ident| $d body:expr) => {
                    $d pipelines.0.split_mut($d kind, $( |$d k, [< $kind:snake >]| {
                        let $d p = [< $kind:snake >];
                        $d body
                    }),*)
                };
                ($d pipelines:expr, $d kind:expr, |$d k:ident, $d p:ident| $d body:expr => Outcome) => {
                    $d pipelines.0.split_mut($d kind, $( |$d k, [< $kind:snake >]| {
                        let $d p = [< $kind:snake >];
                        Outcome::map($d body, Kind::$kind).map_meta(Kind::$kind)
                    }),*)
                };
            }
            macro_rules! reg_split {
                ($d vis:vis struct $d name:ident <$d ( $d param:tt ),*>($d inner_vis:vis $d inner:ident);) => {
                    $d vis struct $d name < $d ( $d param ),* >(
                        $d inner_vis PipelineSplit<$( $d inner < $d ( $d param, )* [< $kind Reg >]> ),*>,
                    );
                }
            }

            impl<$( $kind ),*> Kind< $( $kind ),* > {
                pub const fn as_ref(&self) -> Kind< $( &$kind),* > {
                    match self {
                        $( Kind::$kind(kind) => Kind::$kind(kind) ),*
                    }
                }

                pub const fn as_mut(&mut self) -> Kind< $( &mut $kind),* > {
                    match self {
                        $( Kind::$kind(kind) => Kind::$kind(kind) ),*
                    }
                }
            }

            $(
            ::static_assertions::assert_impl_one!(
                <[< $kind Reg >] as $crate::core::reg::RegRepr>::Repr:
                    $crate::mem::FromBytes,
                    $crate::mem::FromBytesSigned
            );
            ::static_assertions::assert_impl_all!(
                <[< $kind Reg >] as $crate::core::reg::RegRepr>::Repr:
                    $crate::mem::ToBytes,
            );
            )*
        }
    };
    ($( $kind:ident ),*) => {
        pipelines!(($) $( $kind ),*);
    }
}
pipelines!(Int);

impl Sequential for Pipelines {
    fn finish_cycle(&mut self) {
        all!(self, Sequential::finish_cycle);
    }
}

impl Clearable for Pipelines {
    fn clear(&mut self) {
        all!(self, Clearable::clear);
    }
}

mod rename_dispatch {
    pub use self::dispatch::Dispatch;
    pub use self::rename::Rename;
    use super::*;

    macro_rules! extras {
        ($substage:ident $( , $kind:ident )+) => {
            ::paste::paste! {
                Kind<$( <$substage<'static, [< $kind Reg >]> as Substage>::Extra ),*>
            }
        };
        ($substage:ident) => {
            extras!($substage, Int)
        }
    }

    mod rename {
        use super::substage::Rename as PR;
        use super::*;
        use crate::core::uop::{
            decode::{Uop as DUop, UopKind as DKind},
            rename::{Uop as RUop, UopKind as RKind},
        };

        reg_split! {
            pub struct Rename<'a>(pub(super) PR);
        }
        pub type Meta = extras!(PR);

        impl<'a> Substage for Rename<'a> {
            type In = DUop;
            type Out = RUop;
            type Extra = Option<Meta>;
            type Err = Infallible;

            fn start(&mut self, input: Self::In) -> Outcome<Self::Out, Self::Extra, Self::Err> {
                match input.kind {
                    DKind::Pipeline(kind) => {
                        split_mut!(self, kind, |uop, r| r.start(uop) => Outcome)
                            .map(RKind::Pipeline)
                            .map_meta(Some)
                    }
                    DKind::Special(kind) => Outcome::Ok(RKind::Special(kind), None),
                }
                .map(|kind| RUop {
                    kind,
                    addr: input.addr,
                    comp: input.comp,
                    #[cfg(feature = "tui")]
                    display: input.kind,
                })
            }

            fn finish(&mut self, extra: Self::Extra) {
                if let Some(meta) = extra {
                    split_mut!(self, meta, |extra, r| r.finish(extra));
                }
            }
        }
    }

    mod dispatch {
        use super::substage::Dispatch as PD;
        use super::*;
        use crate::core::uop::dispatch::UopKind;

        reg_split! {
            pub struct Dispatch<'a>(pub(super) PD);
        }
        pub type Meta = Option<extras!(PD)>;

        impl<'a> Substage for Dispatch<'a> {
            type In = rob::Uop<DUop>;
            type Out = ();
            type Extra = Meta;
            type Err = Infallible;

            fn start(&mut self, input: Self::In) -> Outcome<Self::Out, Self::Extra, Self::Err> {
                let Self::In { uop, handle } = input;
                match uop.kind {
                    UopKind::Pipeline(kind) => split_mut!(
                        self,
                        kind,
                        |uop, d| d.start(IUop { uop, handle }) => Outcome
                    )
                    .map(|_: Kind<()>| ())
                    .map_meta(Some),
                    UopKind::Special(_) => Outcome::Ok((), None),
                }
            }

            fn finish(&mut self, extra: Self::Extra) {
                if let Some(meta) = extra {
                    split_mut!(self, meta, |extra, d| d.finish(extra))
                }
            }
        }
    }

    impl Pipelines {
        pub fn rename_and_dispatch(&mut self) -> (Rename<'_>, Dispatch<'_>) {
            let Self(PipelineSplit { int }) = self;
            let int = int.rename_and_dispatch();

            let rename = Rename(PipelineSplit { int: int.0 });
            let dispatch = Dispatch(PipelineSplit { int: int.1 });
            (rename, dispatch)
        }
    }
}

mod issue_execute {
    use super::*;

    impl Pipelines {
        pub fn issue(&mut self, rob: &mut rob::lock::Execute<'_, '_>, csrf: &mut CsrFile) {
            all!(self, Pipeline::issue, rob, csrf);
        }

        pub fn execute(&mut self, rob: &mut rob::lock::Execute<'_, '_>, csrf: &mut CsrFile) {
            all!(self, Pipeline::execute, rob, csrf);
        }
    }
}

mod mem_access {
    use super::*;
    use crate::core::mem::{LdResult, MemSubsystem, hierarchy::DMem};
    use crate::mem::{FromBytesSigned, ToBytes};
    use std::cell::RefCell;

    impl Pipelines {
        pub fn mem_access<M: DMem>(&mut self, mem: &mut MemSubsystem<M>, dmem: &mut M) {
            let pipelines = RefCell::new(self);
            mem.tick(
                dmem,
                |src, size| {
                    macro_rules! try_read_bypassed {
                        ($src:expr, $pipeline:expr) => {
                            $pipeline.prf[$src]
                                .try_read_bypassed()
                                .map(|v| v.to_bytes(size))
                        };
                    }

                    split_mut!(pipelines.borrow_mut(), src, |src, p| {
                        try_read_bypassed!(src, p)
                    })
                },
                |LdResult {
                     dest,
                     signed,
                     bytes,
                 }| {
                    macro_rules! write {
                        ($dest:expr, $pipeline:expr) => {
                            $pipeline.prf[*dest].write(FromBytes::from_bytes(bytes))
                        };
                        ($dest:expr, $pipeline:expr, signed) => {
                            $pipeline.prf[*$dest].write(FromBytesSigned::from_bytes(bytes, signed))
                        };
                    }

                    split_mut!(pipelines.borrow_mut(), dest, |dest, p| {
                        write!(dest, p, signed)
                    })
                },
            )
        }
    }
}

mod commit {
    use super::substage::Commit as PC;
    use super::*;
    use crate::core::{ee::rob::Committed, uop::commit::Dest};

    reg_split! {
        pub struct Commit<'a>(PC);
    }

    impl Pipelines {
        pub fn commit(&mut self) -> Commit<'_> {
            Commit(self.0.map_mut(Pipeline::commit))
        }
    }

    impl<'a> Substage for Commit<'a> {
        type In = Committed;
        type Out = Committed;
        type Extra = Option<Dest>;
        type Err = Infallible;

        fn start(&mut self, input: Self::In) -> Outcome<Self::Out, Self::Extra, Self::Err> {
            match input {
                Committed::Handled => Outcome::Ok(input, None),
                Committed::Reg(dest) => {
                    split_mut!(self, dest, |d, p| p.start(d).map_meta(|_| dest))
                        .map(|()| input)
                        .map_meta(Some)
                }
            }
        }

        fn finish(&mut self, extra: Self::Extra) {
            if let Some(dest) = extra {
                split_mut!(self, dest, |d, p| p.finish(d));
            }
        }
    }
}
