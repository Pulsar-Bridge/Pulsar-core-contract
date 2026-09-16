//! Input validation helpers. Every new entry point that accepts a string or
//! address must route it through here before it is stored or acted on
//! (see CLAUDE.md's security review checklist item 2).

use soroban_sdk::String;

use crate::errors::Error;
use crate::types::MAX_STRING_LEN;

const STRKEY_LEN: usize = 56;
const STRKEY_DECODED_LEN: usize = 35;
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

fn base32_value(c: u8) -> Option<u8> {
    match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'2'..=b'7' => Some(c - b'2' + 26),
        _ => None,
    }
}

/// Decodes 56 base32 characters (280 bits, no padding) into 35 raw bytes.
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
