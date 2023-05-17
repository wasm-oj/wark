use std::io::Read;
use std::{borrow::Cow, fs::File, path::PathBuf};

/// Read a wasm module from a file
pub fn read_wasm(path: PathBuf) -> Result<Cow<'static, [u8]>, ()> {
    let mut file = File::open(path).expect("wasm module not found");
    let mut wasm_bytes = Vec::new();
    file.read_to_end(&mut wasm_bytes)
        .expect("failed to read wasm module");
    Ok(Cow::Owned(wasm_bytes))
}
