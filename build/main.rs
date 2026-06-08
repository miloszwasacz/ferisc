use std::env;
use std::path::PathBuf;

mod decode;

const PROJECT_DIR: &str = "CARGO_MANIFEST_DIR";
const OUT_DIR: &str = "OUT_DIR";
const SPEC_DIR: &str = "spec";

pub fn main() {
    let out_dir = PathBuf::from(env::var_os(OUT_DIR).unwrap());
    let proj_dir = PathBuf::from(env::var_os(PROJECT_DIR).unwrap());
    let spec_dir = proj_dir.join(SPEC_DIR);

    decode::gen_decode_fns(&spec_dir, &out_dir);

    println!("cargo::rerun-if-changed=build");
    println!("cargo::rerun-if-changed=crates/macros");
    println!("cargo::rerun-if-changed={}", SPEC_DIR);
}
