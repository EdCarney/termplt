use std::io;

const B64_CHARS: [u8; 64] = [
    b'A', b'B', b'C', b'D', b'E', b'F', b'G', b'H', b'I', b'J', b'K', b'L', b'M', b'N', b'O', b'P',
    b'Q', b'R', b'S', b'T', b'U', b'V', b'W', b'X', b'Y', b'Z', b'a', b'b', b'c', b'd', b'e', b'f',
    b'g', b'h', b'i', b'j', b'k', b'l', b'm', b'n', b'o', b'p', b'q', b'r', b's', b't', b'u', b'v',
    b'w', b'x', b'y', b'z', b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'+', b'/',
];

pub fn read_bytes_to_b64(bytes: &[u8]) -> Result<Vec<u8>, io::Error> {
    // every 3 bytes produces 4 base64 characters; per RFC 4648, the output length is always a
    // multiple of 4 (partial groups are padded with '=')
    let num_raw = bytes.len();
    let rem = num_raw % 3;
    let num_b64 = if rem > 0 {
        ((num_raw / 3) + 1) * 4
    } else {
        (num_raw / 3) * 4
    };
    let mut b64_data = vec![0; num_b64];

    // iterate first through all full byte sets to allow us to avoid checking the length of the
    // chunk during each encoding; that is, all chunks of length 3
    for (ind, byte_chunk) in bytes.chunks(3).take(num_raw / 3).enumerate() {
        let buf_start = ind * 4;
        let buf_end = buf_start + 4;
        convert_full_bytes_to_b64(byte_chunk, &mut b64_data[buf_start..buf_end])
    }

    // encode the last 1 or 2 bytes if present, with '=' padding per RFC 4648
    if rem > 0 {
        let raw_start = num_raw - rem;
        let b64_start = num_b64 - 4;
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
            buf[2] = b'=';
            buf[3] = b'=';
        }
        2 => {
            buf[0] = B64_CHARS[usize::try_from(bytes[0] >> 2).unwrap()];
            buf[1] = B64_CHARS[usize::try_from(bytes[0] << 6 >> 2 | bytes[1] >> 4).unwrap()];
            buf[2] = B64_CHARS[usize::try_from(bytes[1] << 4 >> 2).unwrap()];
            buf[3] = b'=';
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
        let enc_text = std::str::from_utf8(&enc_bytes).expect("Encoded text is invalid UTF-8");
        assert_eq!(enc_text.len(), 4, "1 byte should be 4 base64 chars (2 data + 2 padding)");
        assert_eq!(enc_text, "TQ==");
    }

    #[test]
    fn encode_2_bytes() {
        let text = b"Ma";
        let enc_bytes = read_bytes_to_b64(text).expect("Failed to encode text");
        let enc_text = std::str::from_utf8(&enc_bytes).expect("Encoded text is invalid UTF-8");
        assert_eq!(enc_text.len(), 4, "2 bytes should be 4 base64 chars (3 data + 1 padding)");
        assert_eq!(enc_text, "TWE=");
    }

    #[test]
    fn encode_3_bytes() {
        let text = b"Man";
        let enc_bytes = read_bytes_to_b64(text).expect("Failed to encode text");
        let enc_text = std::str::from_utf8(&enc_bytes).expect("Encoded text is invalid UTF-8");
        assert_eq!(enc_text.len(), 4, "3 bytes should be 4 base64 chars (no padding)");
        assert_eq!(enc_text, "TWFu");
    }

    #[test]
    fn encode_empty_input() {
        let text = b"";
        let enc_bytes = read_bytes_to_b64(text).expect("Failed to encode text");
        assert_eq!(enc_bytes.len(), 0, "empty input should produce empty output");
    }

    #[test]
    fn encode_output_length_is_always_multiple_of_4() {
        for len in 1..=20 {
            let input: Vec<u8> = (0..len).map(|i| i as u8).collect();
            let enc_bytes = read_bytes_to_b64(&input).expect("Failed to encode");
            assert_eq!(
                enc_bytes.len() % 4,
                0,
                "base64 output length should be a multiple of 4 for input length {len}"
            );
        }
    }

    #[test]
    fn encode_longer_string() {
        // "Many hands make light work." is a well-known base64 test vector
        let text = b"Many hands make light work.";
        let enc_bytes = read_bytes_to_b64(text).expect("Failed to encode text");
        let enc_text = std::str::from_utf8(&enc_bytes).expect("Encoded text is invalid UTF-8");
        assert_eq!(enc_text, "TWFueSBoYW5kcyBtYWtlIGxpZ2h0IHdvcmsu");
    }
}
