//! The bytes the launcher uses for the values that must differ every time.
//!
//! The launcher needs values nobody can guess for the login of an account: the
//! `state` of an OAuth login and the verifier of a proof key (RFC 7636). It
//! also needs bytes that differ for the identifiers it creates. The crate has
//! no random number generator of its own, so the bytes come from a counter, the
//! clock and the process: the value differs between two calls of the launcher
//! and between two calls of one launcher, which is what those values need, but
//! the source is not a cryptographic one. A launcher that has to protect a
//! long-lived secret must not use these bytes for it.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// The number of the calls, so two calls of one moment differ.
static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Returns bytes that differ from the bytes of every other call.
///
/// The length is the length of the array the caller asks for, so the function
/// is used as `bytes::<32>()`. The bytes are mixed from the number of the call,
/// the clock and the process, so a call in the same nanosecond of another
/// process still differs.
pub fn bytes<const N: usize>() -> [u8; N] {
    let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.subsec_nanos() as u64)
        .unwrap_or(0);

    let mut state = counter
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(nanos ^ (u64::from(std::process::id()) << 32));

    let mut bytes = [0u8; N];
    for chunk in bytes.chunks_mut(8) {
        state = splitmix64(state);
        chunk.copy_from_slice(&state.to_be_bytes()[..chunk.len()]);
    }
    bytes
}

/// Advances the state of the bytes.
fn splitmix64(state: u64) -> u64 {
    let mut value = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bytes_differ_between_the_calls() {
        let first = bytes::<32>();
        let second = bytes::<32>();

        assert_eq!(first.len(), 32);
        assert_ne!(first, second);
    }

    #[test]
    fn the_bytes_of_one_call_are_not_all_the_same() {
        // A stream that repeats one value would still differ between the
        // calls, so the bytes of a call are checked as well.
        let bytes = bytes::<64>();
        assert!(bytes.windows(8).any(|window| window != &bytes[..8]));
    }
}
