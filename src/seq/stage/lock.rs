use super::{Transformer, trans::TransCompose};

/// A lock used for transaction-based modifications of a circuit's state that is
/// placed for the duration of the execution of a [`PipelineStage`].
///
/// It is recommended that the lock first gathers all changes that should be
/// applied to a given circuit within a single clock cycle (e.g. new elements
/// that should be pushed to a queue), and only commits them when it is [dropped](Drop).
pub trait CircuitLock<'a> {
    /// The type [`Transformer`] that is used to convert a given element,
    /// based on the locked state of the circuit.
    type Trans: Transformer<'a, Lock = Self>;

    /// Creates a new instance of a [transformer](Self::Trans) dependent on this
    /// lock, which processes a single element.
    fn make_trans(&mut self) -> Self::Trans;
}

/// A [`CircuitLock`] that combines two other locks.
pub struct LockCompose<L1, L2> {
    pub(super) l1: L1,
    pub(super) l2: L2,
}

impl<'a, L1, L2> CircuitLock<'a> for LockCompose<L1, L2>
where
    L1: CircuitLock<'a>,
    L2: CircuitLock<'a>,
    L1::Trans: Transformer<'a, Lock = L1>,
    L2::Trans: Transformer<'a, In = <L1::Trans as Transformer<'a>>::Out, Lock = L2>,
{
    type Trans = TransCompose<L1::Trans, L2::Trans>;

    fn make_trans(&mut self) -> Self::Trans {
        Self::Trans {
            t1: self.l1.make_trans(),
            t2: self.l2.make_trans(),
        }
    }
}
