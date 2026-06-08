macro_rules! field {
    ($width:literal, $enc:expr) => {{
        let mask = ((1 as Self::Enc) << $width) - 1;
        let field = $enc & mask;
        #[allow(unused)]
        {
            $enc >>= $width;
        }
        field
    }};
}
pub(crate) use field;

macro_rules! opcode {
    ($enc:expr) => {{
        let _ = field!(7, $enc);
    }};
}
pub(crate) use opcode;

macro_rules! funct3 {
    ($enc:expr) => {{
        let _ = field!(3, $enc);
    }};
}
pub(crate) use funct3;

macro_rules! reg {
    ($r:ty, $width:literal, $enc:expr) => {{ <$r>::decode(field!($width, $enc) as _)? }};
}
pub(crate) use reg;

//#region RVC

macro_rules! rvc_opcode {
    ($enc:expr) => {{
        let _ = field!(7, $enc);
    }};
}
pub(crate) use rvc_opcode;

macro_rules! funct2 {
    ($enc:expr) => {{
        let _ = field!(2, $enc);
    }};
}
pub(crate) use funct2;

macro_rules! rvc_reg {
    ($r:ty, $enc:expr) => {{ <$r>::decode((field!(3, $enc) + 8) as _)? }};
}
pub(crate) use rvc_reg;

//#endregion
