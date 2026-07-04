//TODO Better docs with examples
pub use self::lock::{CircuitLock, LockCompose};
use self::map::{CopiedLock, TransLock};
pub use self::trans::{TransCompose, Transformer};

mod lock;
mod map;
mod trans;

/// A transaction-based pipeline stage definition.
///
/// This most useful for paths between two queues with some kind of transformation
/// applied, such as decoding or register renaming.
pub struct PipelineStage<T>(T);

impl<'a, T: CircuitLock<'a>> PipelineStage<T> {
    /// Starts a new pipeline stage definition.
    ///
    /// The definition has to start with a transformation that generates elements,
    /// such as popping them out of a queue.
    ///
    /// Note that a pipeline stage definition is lazy and the processing is only
    /// started when [`execute`](Self::execute) is called.
    pub fn new(lock: T) -> Self
    where
        T::Trans: Transformer<'a, In = ()>,
    {
        Self(lock)
    }

    /// Adds a new fallible substage within the pipeline stage.
    ///
    /// The substages are applied sequentially to the elements processed by the
    /// pipeline stage in the order of their definition. For more information on
    /// the execution guarantees, see the [`execute`](Self::execute) method.
    pub fn substage<U: CircuitLock<'a>>(self, lock: U) -> PipelineStage<LockCompose<T, U>> {
        PipelineStage(LockCompose::new(self.0, lock))
    }

    /// Adds a new infallible substage which applies a simple transformation
    /// to the elements without modifying any external state, similar to [`Iterator::map`].
    pub fn transform<I, O, F>(self, f: F) -> PipelineStage<LockCompose<T, TransLock<I, O, F>>>
    where
        T::Trans: Transformer<'a, Out = I>,
        F: Fn(I) -> O,
    {
        PipelineStage(LockCompose::new(self.0, TransLock::new(f)))
    }

    pub fn copied<U>(self) -> PipelineStage<LockCompose<T, CopiedLock<'a, U>>>
    where
        T::Trans: Transformer<'a, Out = &'a U>,
        U: Copy,
    {
        PipelineStage(LockCompose::new(self.0, CopiedLock::copied()))
    }

    /// Executes the pipeline stage.
    ///
    /// It is guaranteed that the transformations defined by individual
    /// [`substages`](Self::substage) are executed in order of definition and
    /// committed in reverse order. The execution proceeds until a transformation
    /// fails (i.e. [`Transformer::transform`] returns [`None`]). When that happens,
    /// the failing transformer is discarded, and its state is _not_ committed
    /// to its associated [lock](CircuitLock). It is also guaranteed that if all
    /// substages succeed for a given element, the changes will be committed to
    /// the lock before a new element starts being processed.
    pub fn execute(mut self)
    where
        T::Trans: Transformer<'a, In = (), Out = ()>,
    {
        loop {
            let mut trans = self.0.make_trans();
            if trans.transform(&self.0, ()).is_some() {
                trans.commit(&mut self.0);
            } else {
                break;
            }
        }
    }
}

pub trait IntoPipelineStage {
    fn into_pipeline_stage(self) -> PipelineStage<Self>
    where
        Self: Sized;
}

impl<'a, T> IntoPipelineStage for T
where
    T: CircuitLock<'a, Trans: Transformer<'a, In = ()>>,
{
    fn into_pipeline_stage(self) -> PipelineStage<Self> {
        PipelineStage::new(self)
    }
}
