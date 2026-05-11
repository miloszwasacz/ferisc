use super::{CircuitLock, lock::LockCompose};

/// A trait defining a transformation of a single element that is processed within a [`PipelineStage`].
pub trait Transformer<'a> {
    /// The input type that will be transformed into [`Self::Out`].
    type In;
    /// The output type that is created from [`Self::In`]
    type Out;
    /// The [`CircuitLock`] that creates instances of this transformer for each individual element.
    type Lock: CircuitLock<'a, Trans = Self>;

    /// Fallibly transforms `input` from [`Self::In`] into [`Self::Out`].
    ///
    /// It is important to note that no changes will be [`committed`](Self::commit),
    /// unless all subsequent transformations within the pipeline stage succeed.
    ///
    /// This method is guaranteed to be called at most once for a given instance of the transformer.
    /// It will not be called if a previous transformation in the pipeline stage failed.
    fn transform(&mut self, lock: &Self::Lock, input: Self::In) -> Option<Self::Out>;

    /// Commits the transformation to the `lock`.
    ///
    /// This method is guaranteed to be called at most once, and only after the
    /// [`transformation`](Self::transform) has taken place and all transformations
    /// defined by the pipeline stage have succeeded for a given element. It is
    /// also guaranteed to run after all subsequent transformations' commits,
    /// and before any preceding transformations' commits.
    fn commit(self, lock: &mut Self::Lock);
}

/// A [`Transformer`] that sequentially combines two other transformers.
pub struct TransCompose<T1, T2> {
    pub(super) t1: T1,
    pub(super) t2: T2,
}

impl<'a, T1, T2> Transformer<'a> for TransCompose<T1, T2>
where
    T1: Transformer<'a>,
    T2: Transformer<'a, In = T1::Out>,
    T1::Lock: CircuitLock<'a, Trans = T1>,
    T2::Lock: CircuitLock<'a, Trans = T2>,
{
    type In = T1::In;
    type Out = T2::Out;
    type Lock = LockCompose<T1::Lock, T2::Lock>;

    fn transform(&mut self, lock: &Self::Lock, input: Self::In) -> Option<Self::Out> {
        let input = self.t1.transform(&lock.l1, input)?;
        self.t2.transform(&lock.l2, input)
    }

    fn commit(self, lock: &mut Self::Lock) {
        self.t2.commit(&mut lock.l2);
        self.t1.commit(&mut lock.l1);
    }
}
