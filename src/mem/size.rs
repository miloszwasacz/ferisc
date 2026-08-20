use super::Addr;
use std::fmt;
use std::num::NonZeroU8;
use std::ops::Range;

#[derive(
    Debug,
    derive_more::Display,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    derive_more::From,
)]
pub enum Size {
    Scalar(ScalarSize),
    Vector(VectorSize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ScalarSize {
    Byte,
    HalfWord,
    Word,
    DoubleWord,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VectorSize {
    pub unit: ScalarSize,
    pub count: NonZeroU8,
}

impl Size {
    pub const fn as_usize(self) -> usize {
        match self {
            Self::Scalar(s) => s.as_usize(),
            Self::Vector(s) => s.as_usize(),
        }
    }

    pub const fn addr_range(&self, addr: Addr) -> Range<Addr> {
        addr..addr + self.as_usize() as Addr
    }
}

impl ScalarSize {
    pub const fn as_usize(self) -> usize {
        match self {
            ScalarSize::Byte => size_of::<u8>(),
            ScalarSize::HalfWord => size_of::<u16>(),
            ScalarSize::Word => size_of::<u32>(),
            ScalarSize::DoubleWord => size_of::<u64>(),
        }
    }
}

impl VectorSize {
    pub const fn as_usize(self) -> usize {
        self.unit.as_usize() * self.count.get() as usize
    }
}

impl fmt::Display for ScalarSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let size = match self {
            Self::Byte => "b",
            Self::HalfWord => "h",
            Self::Word => "w",
            Self::DoubleWord => "d",
        };
        fmt::Display::fmt(size, f)
    }
}

impl fmt::Display for VectorSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let size = format!("{}x{}", self.count.get(), self.unit.as_usize());
        fmt::Display::fmt(size.as_str(), f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Bytes {
    Byte([u8; ScalarSize::Byte.as_usize()]),
    HalfWord([u8; ScalarSize::HalfWord.as_usize()]),
    Word([u8; ScalarSize::Word.as_usize()]),
    DoubleWord([u8; ScalarSize::DoubleWord.as_usize()]),
    Vector(Box<[u8]>, VectorSize),
}

impl Bytes {
    pub fn as_slice(&self) -> &[u8] {
        match self {
            Self::Byte(b) => b.as_slice(),
            Self::HalfWord(b) => b.as_slice(),
            Self::Word(b) => b.as_slice(),
            Self::DoubleWord(b) => b.as_slice(),
            Self::Vector(b, _) => b.as_ref(),
        }
    }

    pub fn size(&self) -> Size {
        Size::Scalar(match self {
            Bytes::Byte(_) => ScalarSize::Byte,
            Bytes::HalfWord(_) => ScalarSize::HalfWord,
            Bytes::Word(_) => ScalarSize::Word,
            Bytes::DoubleWord(_) => ScalarSize::DoubleWord,
            Bytes::Vector(_, size) => return Size::Vector(*size),
        })
    }

    pub fn take_from(slice: &[u8], size: Size) -> Option<Self> {
        slice.get(..size.as_usize()).map(|slice| match size {
            Size::Scalar(size) => match size {
                ScalarSize::Byte => Self::Byte(slice.try_into().unwrap()),
                ScalarSize::HalfWord => Self::HalfWord(slice.try_into().unwrap()),
                ScalarSize::Word => Self::Word(slice.try_into().unwrap()),
                ScalarSize::DoubleWord => Self::DoubleWord(slice.try_into().unwrap()),
            },
            Size::Vector(size) => {
                debug_assert_eq!(slice.len(), size.as_usize());
                Self::Vector(slice.into(), size)
            }
        })
    }

    pub fn write_into(&self, slice: &mut [u8]) -> Option<()> {
        let src = self.as_slice();
        slice
            .get_mut(..src.len())
            .map(|slice| slice.copy_from_slice(src))
    }
}
