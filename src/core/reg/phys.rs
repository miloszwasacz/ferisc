pub use self::preg::PhysReg;
pub use self::rat::Rat;
use super::ArchReg;
use crate::seq::Sequential;
use std::iter::repeat_with;
use std::ops::{Deref, DerefMut, Index, IndexMut};

mod preg;
pub mod rat;

//#region PRF

/// Index of the zero-hardwired register, if one exists.
const HARDWIRED_IDX: usize = 0;

#[macro_export]
macro_rules! prf {
    ($arch_reg:ty) => {
        $crate::core::reg::phys::Prf<
            <$arch_reg as $crate::core::reg::ArchReg>::Repr,
            { <$arch_reg as $crate::core::reg::ArchReg>::COUNT }
        >
    };
    ($arch_reg:ty, $size:expr) => {
        <$crate::prf!($arch_reg)>::new::<$arch_reg>($size)
    };
}

/// Physical Register File
#[derive(Debug)]
pub struct Prf<T: ArchReg> {
    #[cfg(debug_assertions)]
    id: id::Id,
    regs: Box<[PhysReg<T::Repr>]>,
}

impl<T: ArchReg> Prf<T> {
    pub fn new(size: usize) -> (Self, Rat<T>) {
        #[cfg(debug_assertions)]
        let id = id::new_id();
        let regs = {
            let mut regs = repeat_with(PhysReg::new).take(size).collect::<Box<[_]>>();
            if T::VARIANTS.iter().any(|reg| reg.is_hardwired()) {
                regs[HARDWIRED_IDX] = PhysReg::hardwired();
            }
            regs
        };
        let rat = Rat::new(
            #[cfg(debug_assertions)]
            id,
            size,
        );
        let prf = Self {
            #[cfg(debug_assertions)]
            id,
            regs,
        };
        (prf, rat)
    }

    pub fn size(&self) -> usize {
        self.regs.len()
    }
}

impl<T: ArchReg> Clone for Prf<T>
where
    T::Repr: Clone,
{
    fn clone(&self) -> Self {
        Self {
            #[cfg(debug_assertions)]
            id: self.id,
            regs: self.regs.clone(),
        }
    }
}

impl<T: ArchReg> Sequential for Prf<T> {
    fn finish_cycle(&mut self) {
        self.regs.iter_mut().for_each(Sequential::finish_cycle);
    }
}

//#endregion

//#region Operands

type RegIdx = usize;

/// Physical register operand
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RegOper<T> {
    #[cfg(debug_assertions)]
    id: usize,
    idx: RegIdx,
    arch: T,
}

/// Physical register destination
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegDest<T> {
    oper: RegOper<T>,
    prev: RegIdx,
}

impl<T: ArchReg> Index<RegOper<T>> for Prf<T> {
    type Output = PhysReg<T::Repr>;

    fn index(&self, index: RegOper<T>) -> &Self::Output {
        #[cfg(debug_assertions)]
        assert_eq!(self.id, index.id, "invalid register file");
        self.regs.index(index.idx)
    }
}

impl<T: ArchReg> IndexMut<RegOper<T>> for Prf<T> {
    fn index_mut(&mut self, index: RegOper<T>) -> &mut Self::Output {
        #[cfg(debug_assertions)]
        assert_eq!(self.id, index.id, "invalid register file");
        self.regs.index_mut(index.idx)
    }
}

impl<T> Deref for RegDest<T> {
    type Target = RegOper<T>;

    fn deref(&self) -> &Self::Target {
        &self.oper
    }
}

impl<T> DerefMut for RegDest<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.oper
    }
}

//#endregion

//#region Stats

#[derive(Debug, Clone, PartialEq, Eq, derive_more::Deref)]
#[deref(forward)]
pub struct Stats<T: ArchReg>(Box<[Option<T::Repr>]>);

impl<T: ArchReg> Prf<T> {
    pub fn stats(&self) -> Stats<T>
    where
        T::Repr: Clone,
    {
        Stats(
            self.regs
                .iter()
                .map(|reg| reg.try_read().cloned())
                .collect(),
        )
    }
}

//#endregion

#[cfg(debug_assertions)]
/// Unique IDs for register files used for correctness checks in debug builds.
mod id {
    use std::sync::atomic::{AtomicUsize, Ordering};

    static FILE_ID: AtomicUsize = AtomicUsize::new(0);
    pub(super) type Id = usize;

    /// Creates a new unique ID for a register file.
    ///
    /// # Panics
    ///
    /// Only at most [`Id::MAX`] IDs can be created - calling this function more
    /// times causes a panic.
    pub(super) fn new_id() -> Id {
        let id = FILE_ID.fetch_add(1, Ordering::SeqCst);
        assert!(id < Id::MAX, "maximum ID exceeded");
        id
    }
}
