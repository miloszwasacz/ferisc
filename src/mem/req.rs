pub use self::buf::ReqBuf;
use super::{Addr, Bytes, Size};

mod buf;

macro_rules! impl_req {
    (
        impl<E> $req:ident<E> {
            pub fn new($( $n_args:tt )*) -> Self {
                $( $n_body:tt )*
            }

            pub fn complete($( $c_args:tt )*) -> Self {
                $( $c_body:tt )*
            }

            $( $rest:tt )*
        }
    ) => {
        impl<E> $req<E> {
            pub fn new( $( $n_args )* ) -> Self {
                $( $n_body )*
            }

            pub fn addr(&self) -> Addr {
                self.0.addr
            }

            pub fn size(&self) -> Size {
                self.0.size
            }

            pub fn complete($( $c_args )*) -> Self {
                $( $c_body )*
            }

            $( $rest )*
        }
    };
}

pub type ReqData = Bytes;

#[derive(Debug, Clone)]
pub struct LdReq<E>(Req<ReqState<(), ReqData, E>>);

#[derive(Debug, Clone)]
pub struct StReq<E>(Req<ReqState<ReqData, (), E>>);

#[derive(Debug, Clone, Copy)]
struct Req<S> {
    addr: Addr,
    size: Size,
    state: S,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReqState<I, C, E> {
    Incomplete(I),
    Complete(C),
    Error(E),
}

impl_req! {
    impl<E> LdReq<E> {
        pub fn new(addr: u64, size: Size) -> Self {
            Self(Req {
                addr,
                size,
                state: ReqState::Incomplete(()),
            })
        }

        pub fn complete(&self, data: ReqData) -> Self {
            Self(Req {
                state: ReqState::Complete(data),
                ..self.0
            })
        }

        pub fn throw(&self, err: E) -> Self {
            Self(Req {
                state: ReqState::Error(err),
                ..self.0
            })
        }

        pub fn state(&self) -> ReqState<(), &ReqData, &E> {
            match &self.0.state {
                ReqState::Incomplete(()) => ReqState::Incomplete(()),
                ReqState::Complete(data) => ReqState::Complete(data),
                ReqState::Error(err) => ReqState::Error(err),
            }
        }
    }
}

impl_req! {
    impl<E> StReq<E> {
        pub fn new(addr: u64, data: ReqData) -> Self {
            Self(Req {
                addr,
                size: data.size(),
                state: ReqState::Incomplete(data),
            })
        }

        pub fn complete(&self) -> Self {
            Self(Req {
                state: ReqState::Complete(()),
                ..self.0
            })
        }

        pub fn throw(&self, err: E) -> Self {
            Self(Req {
                state: ReqState::Error(err),
                ..self.0
            })
        }

        pub fn state(&self) -> ReqState<&ReqData, (), &E> {
            match &self.0.state {
                ReqState::Incomplete(data) => ReqState::Incomplete(data),
                ReqState::Complete(()) => ReqState::Complete(()),
                ReqState::Error(err) => ReqState::Error(err),
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MemConn(pub(super) usize);

pub struct ReqHandle {
    #[cfg(debug_assertions)]
    pub(super) id: *const u8,
    #[cfg(debug_assertions)]
    pub(super) ty: ReqType,
    pub(super) conn: MemConn,
}

#[cfg(debug_assertions)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum ReqType {
    Ld,
    St,
}
