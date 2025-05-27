use std::io;

const B64_CHARS: [u8; 64] = [
    b'A', b'B', b'C', b'D', b'E', b'F', b'G', b'H', b'I', b'J', b'K', b'L', b'M', b'N', b'O', b'P',
    b'Q', b'R', b'S', b'T', b'U', b'V', b'W', b'X', b'Y', b'Z', b'a', b'b', b'c', b'd', b'e', b'f',
    b'g', b'h', b'i', b'j', b'k', b'l', b'm', b'n', b'o', b'p', b'q', b'r', b's', b't', b'u', b'v',
    b'w', b'x', b'y', b'z', b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'+', b'/',
];

pub fn read_bytes_to_b64(bytes: &[u8]) -> Result<Vec<u8>, io::Error> {
    // every 3 bytes will be 4 6-bit base64 encoded values; the number of base64 values will
    // [number of bytes] * (4/3), with an extra value if the division is not clean; use this to
    // preallocate the encoded vector
    let num_raw = bytes.len();
    let rem = num_raw % 3;
    let num_b64 = ((num_raw * 4) / 3) + if rem > 0 { 1 } else { 0 };
    let mut b64_data = vec![0; num_b64];

    // iterate first through all full byte sets to allow us to avoid checking the length of the
    // chunk during each encoding; that is, all chunks of length 3
    for (ind, byte_chunk) in bytes.chunks(3).take(num_raw / 3).enumerate() {
        let buf_start = ind * 4;
        let buf_end = buf_start + 4;
        convert_full_bytes_to_b64(byte_chunk, &mut b64_data[buf_start..buf_end])
    }

    // encode the last 1 or 2 bytes if present
    if rem > 0 {
        let raw_start = num_raw - rem;
        let b64_start = num_b64 - (rem + 1);
        convert_partial_bytes_to_b64(&bytes[raw_start..], &mut b64_data[b64_start..]);
    }

    Ok(b64_data)
}

/// Converts a 'full set' (3 bytes) to four 6-bit base64 encoded values. Populates these values
/// in the buffer.
fn convert_full_bytes_to_b64(bytes: &[u8], buf: &mut [u8]) {
    buf[0] = B64_CHARS[usize::try_from(bytes[0] >> 2).unwrap()];
    buf[1] = B64_CHARS[usize::try_from(bytes[0] << 6 >> 2 | bytes[1] >> 4).unwrap()];
    buf[2] = B64_CHARS[usize::try_from(bytes[1] << 4 >> 2 | bytes[2] >> 6).unwrap()];
    buf[3] = B64_CHARS[usize::try_from(bytes[2] << 2 >> 2).unwrap()];
}

/// Converts a potentially 'partial set' (less than 3 bytes) into the appropriate number of 6-bit
/// base64 encoded values. Populates these values into the buffer.
fn convert_partial_bytes_to_b64(bytes: &[u8], buf: &mut [u8]) {
    match bytes.len() {
        0 => ( /* no-op */ ),
        1 => {
            buf[0] = B64_CHARS[usize::try_from(bytes[0] >> 2).unwrap()];
            buf[1] = B64_CHARS[usize::try_from(bytes[0] << 6 >> 2).unwrap()];
        }
        2 => {
            buf[0] = B64_CHARS[usize::try_from(bytes[0] >> 2).unwrap()];
            buf[1] = B64_CHARS[usize::try_from(bytes[0] << 6 >> 2 | bytes[1] >> 4).unwrap()];
            buf[2] = B64_CHARS[usize::try_from(bytes[1] << 4 >> 2).unwrap()];
        }
        3 => convert_full_bytes_to_b64(bytes, buf),
        _ => panic!("Number of bytes cannot exceed 3"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_1_byte() {
        let text = b"M";
        let enc_bytes = read_bytes_to_b64(text).expect("Failed to encode text");
        let enc_text = str::from_utf8(&enc_bytes).expect("Encoded text is invalid UTF-8");
        assert_eq!(enc_text.len(), 2, "1 byte should be 2 base64 values");
        assert_eq!(enc_text, "TQ");
    }

    #[test]
    fn encode_2_bytes() {
        let text = b"Ma";
        let enc_bytes = read_bytes_to_b64(text).expect("Failed to encode text");
        let enc_text = str::from_utf8(&enc_bytes).expect("Encoded text is invalid UTF-8");
        assert_eq!(enc_text.len(), 3, "2 bytes should be 3 base64 values");
        assert_eq!(enc_text, "TWE");
    }

    #[test]
    fn encode_3_bytes() {
        let text = b"Man";
        let enc_bytes = read_bytes_to_b64(text).expect("Failed to encode text");
        let enc_text = str::from_utf8(&enc_bytes).expect("Encoded text is invalid UTF-8");
        assert_eq!(enc_text.len(), 4, "3 bytes should be 4 base64 values");
        assert_eq!(enc_text, "TWFu");
    }
}
