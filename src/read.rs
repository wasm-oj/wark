use std::fs::File;
use std::io::{Error, Read};
use std::path::PathBuf;

/// Read a wasm module from a file
pub fn read_wasm(path: PathBuf) -> Result<Box<[u8]>, Error> {
    let mut file = File::open(path).expect("wasm module not found");
    let mut wasm_bytes = Vec::new();
    file.read_to_end(&mut wasm_bytes)?;
    Ok(wasm_bytes.into_boxed_slice())
}
