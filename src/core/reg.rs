use generic_array::ArrayLength;
use std::hash::Hash;
use strum::VariantArray;

pub mod int;
pub mod phys;

/// Architectural registers as defined by the ISA.
pub trait ArchReg: RegRepr + ArchRegCount + Copy + Eq + Hash + Into<usize> + VariantArray {
    /// Returns whether an architectural register is hardwired.
    fn is_hardwired(&self) -> bool;
}

/// Representations associated with architectural registers
pub trait RegRepr: Sized {
    /// The data type stored by physical registers associated with
    /// architectural registers of a given kind.
    type Repr: Default;

    /// Decodes a binary representation into an architectural register, if possible.
    fn decode(enc: i32) -> Option<Self>;
}

pub trait ArchRegCount {
    /// The total number of architectural registers of a given kind.
    const COUNT: usize;

    /// The total number of architectural registers of a given kind as an
    /// associated type to be used by [`generic_array::GenericArray`].
    type COUNT: ArrayLength<ArrayType<usize>: Copy> + core::fmt::Debug + Copy;
}
