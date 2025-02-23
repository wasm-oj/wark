use getrandom::Error;

pub fn deterministic_random(buf: &mut [u8]) -> Result<(), Error> {
    let mut state: u8 = 0;
    for byte in buf.iter_mut() {
        *byte = state;
        state = state.wrapping_add(1);
    }
    Ok(())
}
