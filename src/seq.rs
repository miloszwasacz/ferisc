pub use self::buf::Buf;
pub use self::pbuf::Pbuf;
pub use self::queue::Queue;
pub use self::stage::{IntoPipelineStage, PipelineStage};

mod buf;
mod pbuf;
mod queue;
mod stage;

/// A trait for a clock-based circuit.
pub trait Sequential {
    /// Marks the end of the clock cycle, finalizing any outstanding operations.
    fn finish_cycle(&mut self);
}

/// A trait for circuits that have an ENABLE port.
pub trait Stallable: Sequential {
    /// Disables all operations modifying the state of the circuit
    /// in the current clock cycle.
    fn stall(&mut self);
}

/// A trait for circuits that have a CLEAR port.
///
/// Usually implemented for circuits that need to support flushing.
pub trait Clearable: Sequential {
    /// Restores the circuit to a default state at the end of the current
    /// clock cycle.
    ///
    /// Since this operation only modifies the state at the end of a clock
    /// cycle, the circuit can still be read after calling this method.
    ///
    /// Clearing takes precedence over [stalling](Stallable::stall).
    fn clear(&mut self);
}

pub mod locks {
    pub use super::stage::{CircuitLock, LockCompose, TransCompose, Transformer};

    /// An auto-trait for the [`CircuitLock`] trait that encapsulates all type
    /// information required for proper usage within a [`PipelineStage`].
    ///
    /// This trait is useful for hiding the concrete type of the lock in the
    /// form of `impl StageLock<...>`
    pub trait StageLock<'a, I, O>:
        CircuitLock<'a, Trans: Transformer<'a, In = I, Out = O, Lock = Self>>
    {
    }
    impl<'a, I, O, L> StageLock<'a, I, O> for L where
        L: CircuitLock<'a, Trans: Transformer<'a, In = I, Out = O, Lock = Self>>
    {
    }

    pub mod queue {
        pub use crate::seq::queue::{PopLock, PopTrans, PushLock, PushTrans};
    }
}
