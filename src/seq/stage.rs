//TODO Better docs with examples
use self::substage::{Cloned, Compose, Copied, Map};
pub use self::substage::{Outcome, SimpleSubStage, Substage};
use std::convert::Infallible;
use unwrap_infallible::UnwrapInfallible;

pub mod substage;

/// A transaction-based pipeline stage definition.
///
/// This most useful for paths between two queues with some kind of transformation
/// applied, such as decoding or register renaming.
pub struct PipelineStage<S>(S);

impl<S> PipelineStage<S> {
    /// Starts a new pipeline stage definition.
    ///
    /// The definition has to start with a transformation that generates elements,
    /// such as popping them out of a queue.
    ///
    /// Note that a pipeline stage definition is lazy and the processing is only
    /// started when [`PipelineStage::try_execute`] is called.
    pub fn new(producer: S) -> Self
    where
        S: Substage<In = ()>,
    {
        Self(producer)
    }

    /// Adds a new [`Substage`] within the pipeline stage.
    ///
    /// The substages are applied sequentially to the elements processed by the
    /// pipeline stage in the order of their definition. For more information on
    /// the execution guarantees, see the [`PipelineStage::try_execute`] method.
    pub fn substage<R, T, E>(self, substage: R) -> PipelineStage<Compose<S, R, E>>
    where
        S: Substage<Out = T, Err = E>,
        R: Substage<In = T, Err = E>,
    {
        //TODO Add facilities to convert errors when necessary
        PipelineStage(Compose::new(self.0, substage))
    }

    /// Adds a new infallible [`Substage`] which transforms the elements from
    /// one type to another, similar to [`Iterator::map`].
    pub fn map<T, U, F>(self, f: F) -> PipelineStage<Map<S, F>>
    where
        S: Substage<Out = T>,
        F: Fn(T) -> U,
    {
        PipelineStage(Map::new(self.0, f))
    }

    /// Adds a new infallible [`Substage`] which clones the elements,
    /// similar to [`Iterator::cloned`].
    pub fn cloned<'a, T>(self) -> PipelineStage<Cloned<S, T>>
    where
        S: Substage<Out = &'a T>,
        T: Clone + 'a,
    {
        self.map(Clone::clone)
    }

    /// Adds a new infallible [`Substage`] which copies the elements,
    /// similar to [`Iterator::copied`].
    pub fn copied<'a, T>(self) -> PipelineStage<Copied<S, T>>
    where
        S: Substage<Out = &'a T>,
        T: Copy + 'a,
    {
        self.map(|t| *t)
    }

    //TODO Example with a queue
    /// Executes the pipeline stage.
    ///
    /// The execution proceeds by repeatedly trying to execute all substages -
    /// first all substages apply their transformations in order of definition,
    /// and if all of them [succeed](substage::Result::Ok), they are [applied](Apply::apply)
    /// in reverse order; if at any point a substage [stops](substage::Result::Stopped)
    /// or [fails](substage::Result::Error), the state from the current iteration
    /// is discarded without getting applied, including all substages preceding
    /// the stopping/failing substage. This means that a new iteration is only
    /// started if all substages succeed and are applied.
    pub fn try_execute<E>(mut self) -> Result<(), E>
    where
        S: Substage<In = (), Out = (), Err = E>,
    {
        loop {
            match self.0.start(()) {
                Outcome::Ok((), extra) => self.0.finish(extra),
                Outcome::Stop => break Ok(()),
                Outcome::Err(err) => break Err(err),
            }
        }
    }

    /// Executes the pipeline stage.
    ///
    /// This is a more restricted version of [`PipelineStage::try_execute`],
    /// which cannot produce an error.
    #[inline(always)]
    pub fn execute(self)
    where
        S: Substage<In = (), Out = (), Err = Infallible>,
    {
        self.try_execute().unwrap_infallible()
    }
}

/// A converter which can start a new [`PipelineStage`](PipelineStage::new)
/// definition with `Self` as the producer.
///
/// This trait is automatically implemented for all types [`S: SubStage<In = ()>`](Substage).
pub trait IntoPipelineStage: Substage<In = ()> + Sized {
    /// Starts a new [`PipelineStage`](PipelineStage::new) definition with `self`
    /// as the producer.
    fn into_pipeline_stage(self) -> PipelineStage<Self>;
}

impl<S: Substage<In = ()>> IntoPipelineStage for S {
    fn into_pipeline_stage(self) -> PipelineStage<Self> {
        PipelineStage::new(self)
    }
}
