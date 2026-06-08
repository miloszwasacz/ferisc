use self::fence::Fence;
use crate::core::reg::ArchReg;

pub mod fence;
pub mod int;

pub mod decode;
pub mod fetch;
pub mod predecode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UopKind<I> {
    Int(I),
    MemOrd(Fence),
    Except(Exception),
}

pub trait InferUop<T: ArchReg> {
    type Kind;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exception {
    /// Instruction address misaligned
    InstrAddrMisaligned = 0,
    /// Instruction access fault
    InstrAccessFault = 1,
    /// Illegal instruction
    Illegal = 2,
    /// Breakpoint
    Breakpoint = 3,
    /// Load address misaligned
    LdAddrMisaligned = 4,
    /// Load access fault
    LdAccessFault = 5,
    /// Store/AMO address misaligned
    StAddrMisaligned = 6,
    /// Store/AMO access fault
    StAccessFault = 7,
    /// Environment call from U-Mode
    EcallU = 8,
    /// Environment call from S-Mode
    EcallS = 9,
    // Reserved = 10,
    /// Environment call from M-Mode
    EcallM = 11,
    /// Instruction page fault
    InstrPageFault = 12,
    /// Load page fault
    LdPageFault = 13,
    // Reserved = 14,
    /// Store page fault
    StPageFault = 15,
    /// Double trap
    DoubleTrap = 16,
    // Reserved = 17,
    /// Software check
    SwCheck = 18,
    /// Hardware error
    HwError = 19,
    // Reserved = 20-23,
    // Custom = 24-31,
    // Reserved = 32-47,
    // Custom = 48-63,
    // Reserved = >=64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Mode {
    //TODO More modes than machine mode

    // /// User mode
    // U,
    // /// Supervisor mode
    // S,
    /// Machine mode
    #[default]
    M,
}
