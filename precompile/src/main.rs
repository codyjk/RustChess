use std::{fs::File, io::BufWriter, path::PathBuf};

use precompile::{magic::find_magics::find_and_write_all_magics, zobrist::write_zobrist_tables};

fn file_exists_in_build_cache(file_name: &str) -> bool {
    let mut out: PathBuf = std::env::var("OUT_DIR").unwrap().into();
    out.push(file_name);
    out.exists()
}

fn build_zobrist_tables(filename: &str) {
    let mut out: PathBuf = std::env::var("OUT_DIR").unwrap().into();
    out.push(filename);
    let mut out = BufWriter::new(File::create(out).unwrap());
    write_zobrist_tables(&mut out).unwrap();
}

fn build_magics_tables(filename: &str) {
    let mut out: PathBuf = std::env::var("OUT_DIR").unwrap().into();
    out.push(filename);
    let mut out = BufWriter::new(File::create(out).unwrap());
    find_and_write_all_magics(&mut out).unwrap();
}

fn main() {
    if !file_exists_in_build_cache("zobrist_table.rs") {
        println!("cargo:warning=Building zobrist tables...");
        build_zobrist_tables("zobrist_table.rs");
        println!("cargo:warning=Finished building zobrist tables.");
    }

    if !file_exists_in_build_cache("magic_table.rs") {
        println!("cargo:warning=Building magic tables...");
        build_magics_tables("magic_table.rs");
        println!("cargo:warning=Finished building magic tables.");
    }
}
