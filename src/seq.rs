pub use self::buf::Buf;

mod buf;

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
