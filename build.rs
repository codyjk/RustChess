use std::{fs::File, io::BufWriter, path::PathBuf};

use build::zobrist::write_zobrist_tables;

fn build_zobrist_tables() {
    let mut out: PathBuf = std::env::var("OUT_DIR").unwrap().into();
    out.push("zobrist_tables.rs");
    let mut out = BufWriter::new(File::create(out).unwrap());
    write_zobrist_tables(&mut out).unwrap();
}

fn main() {
    println!("cargo:warning=Building zobrist tables...");
    build_zobrist_tables();
    println!("cargo:warning=Finished building zobrist tables.");
}
