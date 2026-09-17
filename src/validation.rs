//! Input validation helpers. Every new entry point that accepts a string or
//! address must route it through here before it is stored or acted on
//! (see CLAUDE.md's security review checklist item 2).

use soroban_sdk::String;

use crate::errors::Error;
use crate::types::MAX_STRING_LEN;

// No current entry point takes a Stellar address as a free-form string (the
// only free-form string fields are foreign-chain values that can't be
// strkeys — see `validate_string_len` below), so this validator has no
// caller yet. It's kept, not deleted, because CLAUDE.md names SEP-23 strkey
// validation as the established pattern any future entry point that *does*
// take one must use.
#[allow(dead_code)]
const STRKEY_LEN: usize = 56;
#[allow(dead_code)]
const STRKEY_DECODED_LEN: usize = 35;
#[allow(dead_code)]
const STRKEY_VERSION_ED25519_PUBLIC_KEY: u8 = 6 << 3;

/// Rejects empty strings and strings over `MAX_STRING_LEN` bytes. Applies to
/// every free-form field (chain ids, foreign-chain addresses, failure reasons)
/// that can't be validated as a Stellar strkey because the source chain isn't
/// necessarily Stellar.
pub fn validate_string_len(s: &String) -> Result<(), Error> {
    if s.is_empty() || s.len() > MAX_STRING_LEN {
        return Err(Error::InvalidInput);
    }
    Ok(())
}

pub fn validate_amount(amount: i128) -> Result<(), Error> {
    if amount <= 0 {
        return Err(Error::InvalidInput);
    }
    Ok(())
}

/// Validates a Stellar ED25519 public key strkey (`G...`) per SEP-23: correct
/// length, valid base32 alphabet, correct version byte, and a matching
/// CRC16/XMODEM checksum. Used wherever a caller supplies a Stellar address as
/// a free-form string rather than the SDK's native `Address` (which the host
/// already validates during XDR decoding, making a second check redundant).
#[allow(dead_code)]
pub fn validate_strkey_ed25519_public_key(s: &String) -> Result<(), Error> {
    if s.len() as usize != STRKEY_LEN {
        return Err(Error::InvalidStrkey);
    }

    let mut buf = [0u8; STRKEY_LEN];
    s.copy_into_slice(&mut buf);

    let decoded = decode_base32(&buf).ok_or(Error::InvalidStrkey)?;

    let version = decoded[0];
    if version != STRKEY_VERSION_ED25519_PUBLIC_KEY {
        return Err(Error::InvalidStrkey);
    }

    let payload = &decoded[..STRKEY_DECODED_LEN - 2];
    let expected_checksum = u16::from_le_bytes([
        decoded[STRKEY_DECODED_LEN - 2],
        decoded[STRKEY_DECODED_LEN - 1],
    ]);
    if crc16_xmodem(payload) != expected_checksum {
        return Err(Error::InvalidStrkey);
    }

    Ok(())
}

#[allow(dead_code)]
fn base32_value(c: u8) -> Option<u8> {
    match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'2'..=b'7' => Some(c - b'2' + 26),
        _ => None,
    }
}

/// Decodes 56 base32 characters (280 bits, no padding) into 35 raw bytes.
#[allow(dead_code)]
fn decode_base32(input: &[u8; STRKEY_LEN]) -> Option<[u8; STRKEY_DECODED_LEN]> {
    let mut out = [0u8; STRKEY_DECODED_LEN];
    let mut bit_buffer: u64 = 0;
    let mut bits_in_buffer: u32 = 0;
    let mut out_idx = 0usize;

    for &c in input.iter() {
        let val = base32_value(c)?;
        bit_buffer = (bit_buffer << 5) | (val as u64);
        bits_in_buffer += 5;
        if bits_in_buffer >= 8 {
            bits_in_buffer -= 8;
            if out_idx >= STRKEY_DECODED_LEN {
                return None;
            }
            out[out_idx] = ((bit_buffer >> bits_in_buffer) & 0xFF) as u8;
            out_idx += 1;
        }
    }

    if out_idx != STRKEY_DECODED_LEN {
        return None;
    }
    Some(out)
}

/// CRC16/XMODEM: poly 0x1021, init 0x0000, no reflect, no final xor — the
/// checksum algorithm SEP-23 strkeys use.
#[allow(dead_code)]
fn crc16_xmodem(data: &[u8]) -> u16 {
    let mut crc: u16 = 0x0000;
    for &byte in data {
        crc ^= (byte as u16) << 8;
        for _ in 0..8 {
            if crc & 0x8000 != 0 {
                crc = (crc << 1) ^ 0x1021;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}

#[cfg(test)]
mod tests {
    //! `validate_strkey_ed25519_public_key` has no caller anywhere in the
    //! contract yet (see the `#[allow(dead_code)]` rationale above), so
    //! nothing else in the test suite exercises this hand-rolled base32 +
    //! CRC16 implementation. Test it directly against vectors from
    //! `stellar-strkey`'s own test suite so it's known-correct before any
    //! future entry point comes to depend on it.
    extern crate std;

    use proptest::prelude::*;
    use soroban_sdk::{Env, String as SorobanString};
    use std::{format, vec};

    use super::{
        validate_amount, validate_string_len, validate_strkey_ed25519_public_key, STRKEY_LEN,
    };
    use crate::types::MAX_STRING_LEN;

    // From stellar-strkey's tests/tests.rs::test_valid_public_keys /
    // test_invalid_public_keys.
    const VALID_1: &str = "GA3D5KRYM6CB7OWQ6TWYRR3Z4T7GNZLKERYNZGGA5SOAOPIFY6YQHES5";
    const VALID_2: &str = "GA7QYNF7SOWQ3GLR2BGMZEHXAVIRZA4KVWLTJJFC7MGXUA74P7UJVSGZ";
    // VALID_2 with its version byte's low 3 bits corrupted (encoded
    // algorithm changes from 0/ed25519 to 7/invalid).
    const INVALID_VERSION: &str = "G47QYNF7SOWQ3GLR2BGMZEHXAVIRZA4KVWLTJJFC7MGXUA74P7UJVP2I";

    #[test]
    fn validate_string_len_accepts_exactly_max_length() {
        let env = Env::default();
        let buf = [b'a'; MAX_STRING_LEN as usize];
        let s = core::str::from_utf8(&buf).unwrap();
        assert!(validate_string_len(&SorobanString::from_str(&env, s)).is_ok());
    }

    #[test]
    fn validate_string_len_rejects_over_max_length() {
        let env = Env::default();
        let buf = [b'a'; MAX_STRING_LEN as usize + 1];
        let s = core::str::from_utf8(&buf).unwrap();
        assert!(validate_string_len(&SorobanString::from_str(&env, s)).is_err());
    }

    #[test]
    fn validate_amount_rejects_negative() {
        assert!(validate_amount(-1).is_err());
    }

    #[test]
    fn validate_amount_accepts_positive() {
        assert!(validate_amount(1).is_ok());
    }

    proptest! {
        #[test]
        fn rejects_any_correct_length_strkey_with_an_invalid_alphabet_char(
            prefix in "[A-Z2-7]{0,55}",
            bad_char in prop::sample::select(vec!['0', '1', '8', '9', 'a', '!', '_']),
        ) {
            // A correct-length (56) string containing even one byte outside
            // the base32 alphabet (A-Z, 2-7) must always be rejected by
            // decode_base32, regardless of where the bad byte falls or what
            // the rest of the string looks like.
            let suffix_len = STRKEY_LEN - prefix.len() - 1;
            let suffix = "A".repeat(suffix_len);
            let s = format!("{prefix}{bad_char}{suffix}");
            prop_assert_eq!(s.len(), STRKEY_LEN);

            let env = Env::default();
            prop_assert!(
                validate_strkey_ed25519_public_key(&SorobanString::from_str(&env, &s)).is_err()
            );
        }

        #[test]
        fn rejects_any_wrong_length_strkey(s in "[A-Z2-7]{0,120}") {
            // STRKEY_LEN is always exactly 56; any other length must be
            // rejected regardless of content, before base32/checksum logic
            // ever runs.
            prop_assume!(s.len() != STRKEY_LEN);
            let env = Env::default();
            prop_assert!(
                validate_strkey_ed25519_public_key(&SorobanString::from_str(&env, &s)).is_err()
            );
        }
    }

    #[test]
    fn accepts_known_valid_strkeys() {
        let env = Env::default();
        assert!(
            validate_strkey_ed25519_public_key(&SorobanString::from_str(&env, VALID_1)).is_ok()
        );
        assert!(
            validate_strkey_ed25519_public_key(&SorobanString::from_str(&env, VALID_2)).is_ok()
        );
    }

    #[test]
    fn rejects_wrong_length() {
        let env = Env::default();
        let short = SorobanString::from_str(&env, "GAAAAAAAACGC6");
        assert!(validate_strkey_ed25519_public_key(&short).is_err());
    }

    #[test]
    fn rejects_bad_version_byte() {
        let env = Env::default();
        let bad = SorobanString::from_str(&env, INVALID_VERSION);
        assert!(validate_strkey_ed25519_public_key(&bad).is_err());
    }

    #[test]
    fn rejects_bad_checksum() {
        let env = Env::default();
        // Flip the last character of a valid strkey: same length and
        // version byte, but the payload/checksum no longer agree.
        let mut buf = [0u8; VALID_1.len()];
        buf.copy_from_slice(VALID_1.as_bytes());
        let last = buf.len() - 1;
        buf[last] = if buf[last] == b'5' { b'6' } else { b'5' };
        let corrupted = core::str::from_utf8(&buf).unwrap();

        let bad = SorobanString::from_str(&env, corrupted);
        assert!(validate_strkey_ed25519_public_key(&bad).is_err());
    }
}
