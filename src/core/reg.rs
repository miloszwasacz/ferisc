use generic_array::ArrayLength;

pub mod int;

/// Architectural registers as defined by the ISA.
pub trait ArchReg: RegRepr + ArchRegCount + Into<usize> + Copy {
    /// Returns an iterator of all available architectural registers.
    fn iter() -> impl Iterator<Item = Self>;

    /// Returns whether an architectural register is hardwired.
    fn is_hardwired(&self) -> bool;
}

/// Physical representation of the data stored by registers.
pub trait RegRepr {
    /// The data type stored by physical registers associated with
    /// architectural registers of a given kind.
    type Repr: Default;
}

pub trait ArchRegCount {
    /// The total number of architectural registers of a given kind.
    const COUNT: usize;

    /// The total number of architectural registers of a given kind as an
    /// associated type to be used by [`generic_array::GenericArray`].
    type COUNT: ArrayLength<ArrayType<usize>: Copy> + core::fmt::Debug + Copy;
}
