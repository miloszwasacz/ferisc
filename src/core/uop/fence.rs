use bitflags::bitflags;

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

    pub(crate) fn sld(&self) -> bool {
        self.0.intersects(Fields::SI | Fields::SR)
    }
}
