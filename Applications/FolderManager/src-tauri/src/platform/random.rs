// Secret bytes from the OS generator, since std has no public CSPRNG and no new crate is wanted for one.
use std::io;

#[cfg(windows)]
pub fn fill(buf: &mut [u8]) -> io::Result<()> {
    use windows_sys::Win32::Security::Cryptography::{BCryptGenRandom, BCRYPT_USE_SYSTEM_PREFERRED_RNG};
    let len = u32::try_from(buf.len()).map_err(|_| io::Error::other("random buffer too large"))?;
    // SAFETY: the pointer and length describe `buf`, and a null handle is valid with the system-preferred flag.
    let status = unsafe { BCryptGenRandom(std::ptr::null_mut(), buf.as_mut_ptr(), len, BCRYPT_USE_SYSTEM_PREFERRED_RNG) };
    if status == 0 {
        Ok(())
    } else {
        Err(io::Error::other(format!("BCryptGenRandom failed with status {status:#x}")))
    }
}

#[cfg(not(windows))]
pub fn fill(buf: &mut [u8]) -> io::Result<()> {
    use std::io::Read;
    std::fs::File::open("/dev/urandom")?.read_exact(buf)
}

pub fn hex_token(bytes: usize) -> io::Result<String> {
    let mut buf = vec![0u8; bytes];
    fill(&mut buf)?;
    Ok(buf.iter().map(|b| format!("{b:02x}")).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_are_hex_of_the_asked_length_and_differ() {
        let (a, b) = (hex_token(32).unwrap(), hex_token(32).unwrap());
        assert!(a.len() == 64 && a.chars().all(|c| c.is_ascii_hexdigit()), "{a}");
        assert_ne!(a, b);
    }
}
