use super::INST_NAME_WIDTH;
use bitflags::bitflags;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fence(Fields);

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct Fields: u8 {
        const SW = 1 << 0;
        const SR = 1 << 1;
        const SO = 1 << 2;
        const SI = 1 << 3;
        const PW = 1 << 4;
        const PR = 1 << 5;
        const PO = 1 << 6;
        const PI = 1 << 7;
    }
}

impl Fence {
    pub const NONE: Self = Self(Fields::empty());

    pub fn decode(fields: u8) -> Self {
        Self(Fields::from_bits_truncate(fields))
    }
}

macro_rules! fields {
    ($( $field:ident ),*) => {
        ::paste::paste! {
            impl Fence {
                $(
                pub const fn [<$field:lower>](&self) -> bool {
                    self.0.contains(Fields::[<$field:upper>])
                }
                )*
            }
        }
    };
}
fields!(pi, po, pr, pw, si, so, sr, sw);

impl Fence {
    pub(crate) fn pld(&self) -> bool {
        self.0.intersects(Fields::PI | Fields::PR)
    }

    pub(crate) fn pst(&self) -> bool {
        self.0.intersects(Fields::PO | Fields::PW)
    }

    pub(crate) fn sld(&self) -> bool {
        self.0.intersects(Fields::SI | Fields::SR)
    }

    pub(crate) fn sst(&self) -> bool {
        self.0.intersects(Fields::SO | Fields::SW)
    }
}

impl fmt::Display for Fence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let width = if f.alternate() { INST_NAME_WIDTH } else { 0 };
        write!(f, "{:<width$}", "fence")?;
        let fields = |f: &mut fmt::Formatter<'_>, r, w, i, o| {
            if r {
                write!(f, " {}", "R")?;
            }
            if w {
                write!(f, " {}", "W")?;
            }
            if i {
                write!(f, " {}", "I")?;
            }
            if o {
                write!(f, " {}", "O")?;
            }
            Ok(())
        };
        fields(f, self.pr(), self.pw(), self.pi(), self.po())?;
        write!(f, ", ")?;
        fields(f, self.sr(), self.sw(), self.si(), self.so())
    }
}
