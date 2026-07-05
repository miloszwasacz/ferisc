use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CsrAddr(u16);

impl CsrAddr {
    pub const fn new(addr: u16) -> Option<Self> {
        if addr <= 0xFFF {
            Some(Self(addr))
        } else {
            None
        }
    }
}

impl fmt::Display for CsrAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#05X}", self.0)
    }
}
