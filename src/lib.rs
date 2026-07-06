/// Includes a file generated during build.
macro_rules! include_generated {
    ($file:literal) => {
        include!(concat!(env!("OUT_DIR"), "/", $file));
    };
    ($file:literal as expr) => {
        include!(concat!(env!("OUT_DIR"), "/", $file))
    };
}

pub mod core;
pub mod mem;
pub mod seq;

#[cfg(feature = "tui")]
pub mod tui;
