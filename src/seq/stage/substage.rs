use std::convert::Infallible;
use std::marker::PhantomData;

pub trait Substage {
    type In;
    type Out;
    type Extra;
    type Err;

    fn start(&mut self, input: Self::In) -> Outcome<Self::Out, Self::Extra, Self::Err>;
    fn finish(&mut self, extra: Self::Extra);
}

pub trait SimpleSubStage {
    type In;
    type Out;

    fn start(&mut self, input: Self::In) -> Option<Self::Out>;
    fn finish(&mut self);
}

impl<S: SimpleSubStage> Substage for S {
    type In = S::In;
    type Out = S::Out;
    type Extra = ();
    type Err = Infallible;

    #[inline(always)]
    fn start(&mut self, input: Self::In) -> Outcome<Self::Out, Self::Extra, Self::Err> {
        self.start(input).into()
    }

    #[inline(always)]
    fn finish(&mut self, (): Self::Extra) {
        self.finish();
    }
}

#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Outcome<O, M, E = Infallible> {
    Ok(O, M),
    Stop,
    Err(E),
}

impl<O, M, E> Outcome<O, M, E> {
    pub fn map<T>(self, f: impl FnOnce(O) -> T) -> Outcome<T, M, E> {
        match self {
            Outcome::Ok(ok, meta) => Outcome::Ok(f(ok), meta),
            Outcome::Stop => Outcome::Stop,
            Outcome::Err(err) => Outcome::Err(err),
        }
    }

    pub fn map_meta<T>(self, f: impl FnOnce(M) -> T) -> Outcome<O, T, E> {
        match self {
            Outcome::Ok(ok, meta) => Outcome::Ok(ok, f(meta)),
            Outcome::Stop => Outcome::Stop,
            Outcome::Err(err) => Outcome::Err(err),
        }
    }

    pub fn map_err<T>(self, f: impl FnOnce(E) -> T) -> Outcome<O, M, T> {
        match self {
            Outcome::Ok(ok, meta) => Outcome::Ok(ok, meta),
            Outcome::Stop => Outcome::Stop,
            Outcome::Err(err) => Outcome::Err(f(err)),
        }
    }
}

impl From<()> for Outcome<(), ()> {
    fn from((): ()) -> Self {
        Outcome::Ok((), ())
    }
}

impl<T, A: From<()>> From<Option<T>> for Outcome<T, A> {
    fn from(value: Option<T>) -> Self {
        match value {
            Some(out) => Outcome::Ok(out, From::from(())),
            None => Outcome::Stop,
        }
    }
}

impl<T, E> From<Result<T, E>> for Outcome<T, (), E> {
    fn from(value: Result<T, E>) -> Self {
        match value {
            Ok(out) => Self::Ok(out, ()),
            Err(err) => Self::Err(err),
        }
    }
}

pub struct Compose<L, R, E>(L, R, PhantomData<E>);

impl<L, R, E> Compose<L, R, E> {
    pub fn new<T>(left: L, right: R) -> Self
    where
        L: Substage<Out = T, Err: Into<E>>,
        R: Substage<In = T, Err: Into<E>>,
    {
        Self(left, right, PhantomData)
    }
}

impl<L, R, T, E> Substage for Compose<L, R, E>
where
    L: Substage<Out = T, Err: Into<E>>,
    R: Substage<In = T, Err: Into<E>>,
{
    type In = L::In;
    type Out = R::Out;
    type Extra = (L::Extra, R::Extra);
    type Err = E;

    fn start(&mut self, input: Self::In) -> Outcome<Self::Out, Self::Extra, Self::Err> {
        match self.0.start(input) {
            Outcome::Ok(value, left) => match self.1.start(value) {
                Outcome::Ok(out, right) => Outcome::Ok(out, (left, right)),
                Outcome::Stop => Outcome::Stop,
                Outcome::Err(err) => Outcome::Err(err.into()),
            },
            Outcome::Stop => Outcome::Stop,
            Outcome::Err(err) => Outcome::Err(err.into()),
        }
    }

    fn finish(&mut self, extra: Self::Extra) {
        self.1.finish(extra.1);
        self.0.finish(extra.0);
    }
}

pub struct Map<S, F>(S, F);

impl<S, F, T, U> Map<S, F>
where
    S: Substage<Out = T>,
    F: Fn(T) -> U,
{
    pub fn new(substage: S, f: F) -> Self {
        Self(substage, f)
    }
}

impl<S, F, T, U> Substage for Map<S, F>
where
    S: Substage<Out = T>,
    F: Fn(T) -> U,
{
    type In = S::In;
    type Out = U;
    type Extra = S::Extra;
    type Err = S::Err;

    fn start(&mut self, input: Self::In) -> Outcome<Self::Out, Self::Extra, Self::Err> {
        self.0.start(input).map(&self.1)
    }

    fn finish(&mut self, extra: Self::Extra) {
        self.0.finish(extra)
    }
}

pub type Cloned<S, T> = Map<S, fn(&T) -> T>;
pub type Copied<S, T> = Cloned<S, T>;
