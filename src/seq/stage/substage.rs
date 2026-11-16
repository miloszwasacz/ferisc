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
    type Err = !;

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
pub enum Outcome<O, M, E = !> {
    Ok(O, M),
    Stop,
    Err(E),
}

impl<O, M, E> Outcome<O, M, E> {
    #[inline]
    pub fn map<T>(self, f: impl FnOnce(O) -> T) -> Outcome<T, M, E> {
        match self {
            Outcome::Ok(ok, meta) => Outcome::Ok(f(ok), meta),
            Outcome::Stop => Outcome::Stop,
            Outcome::Err(err) => Outcome::Err(err),
        }
    }

    #[inline]
    pub fn map_meta<T>(self, f: impl FnOnce(M) -> T) -> Outcome<O, T, E> {
        match self {
            Outcome::Ok(ok, meta) => Outcome::Ok(ok, f(meta)),
            Outcome::Stop => Outcome::Stop,
            Outcome::Err(err) => Outcome::Err(err),
        }
    }

    #[inline]
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

impl<T, M: From<()>> From<Option<T>> for Outcome<T, M> {
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

pub struct Compose<L, R>(L, R);

impl<L, R> Compose<L, R> {
    pub fn new<T, E>(left: L, right: R) -> Self
    where
        L: Substage<Out = T, Err: Into<E>>,
        R: Substage<In = T, Err = E>,
    {
        Self(left, right)
    }
}

impl<L, R, T, E> Substage for Compose<L, R>
where
    L: Substage<Out = T, Err: Into<E>>,
    R: Substage<In = T, Err = E>,
{
    type In = L::In;
    type Out = R::Out;
    type Extra = (L::Extra, R::Extra);
    type Err = E;

    fn start(&mut self, input: Self::In) -> Outcome<Self::Out, Self::Extra, Self::Err> {
        match self.0.start(input) {
            Outcome::Ok(value, left) => self.1.start(value).map_meta(|right| (left, right)),
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

impl<S, F> Map<S, F> {
    pub fn new<T, U>(substage: S, f: F) -> Self
    where
        S: Substage<Out = T>,
        F: Fn(T) -> U,
    {
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

    #[inline(always)]
    fn start(&mut self, input: Self::In) -> Outcome<Self::Out, Self::Extra, Self::Err> {
        self.0.start(input).map(&self.1)
    }

    #[inline(always)]
    fn finish(&mut self, extra: Self::Extra) {
        self.0.finish(extra)
    }
}

pub struct MapErr<S, F>(S, F);

impl<S, F> MapErr<S, F> {
    pub fn new<T, U>(substage: S, f: F) -> Self
    where
        S: Substage<Err = T>,
        F: Fn(T) -> U,
    {
        Self(substage, f)
    }
}

impl<S, F, T, U> Substage for MapErr<S, F>
where
    S: Substage<Err = T>,
    F: Fn(T) -> U,
{
    type In = S::In;
    type Out = S::Out;
    type Extra = S::Extra;
    type Err = U;

    #[inline(always)]
    fn start(&mut self, input: Self::In) -> Outcome<Self::Out, Self::Extra, Self::Err> {
        self.0.start(input).map_err(&self.1)
    }

    #[inline(always)]
    fn finish(&mut self, extra: Self::Extra) {
        self.0.finish(extra)
    }
}

pub type Cloned<S, T> = Map<S, fn(&T) -> T>;
pub type Copied<S, T> = Cloned<S, T>;
