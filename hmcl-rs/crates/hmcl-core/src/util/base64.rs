//! The Base64 encoding the launcher uses.
//!
//! The launcher stores the protected private data of the accounts as Base64,
//! like the launcher of Java does, so a file written by one launcher can be
//! read by the other. The alphabet is the standard one of RFC 4648, with the
//! padding Java writes.

/// The alphabet of the standard Base64 encoding of RFC 4648.
const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// The alphabet of the Base64 encoding of RFC 4648 that is safe in an address
/// and in the value of a form.
///
/// The two characters that are not safe in an address, `+` and `/`, are
/// replaced by `-` and `_`, which is the alphabet the proof key of an OAuth
/// login is written with.
const URL_SAFE_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// The character that pads the encoding when the data does not fill the last
/// group of four characters.
const PADDING: u8 = b'=';

/// Returns the data of a text encoded with the standard Base64 encoding.
///
/// The padding at the end of the text may be left out, which Java accepts and
/// some writers do; characters that are not part of the alphabet make the
/// whole text invalid rather than being skipped.
pub fn decode(text: &str) -> Option<Vec<u8>> {
    let bytes = text.as_bytes();

    // The padding only pads the last group, so it is counted from the end.
    let unpadded = match bytes {
        [] => 0,
        _ if bytes.ends_with(b"==") => bytes.len() - 2,
        _ if bytes.ends_with(b"=") => bytes.len() - 1,
        _ => bytes.len(),
    };

    // A group of four characters holds three bytes; a group that is not
    // complete holds two or three characters only at the end of the text.
    let remainder = unpadded % 4;
    if remainder == 1 {
        return None;
    }
    if unpadded != bytes.len() && !bytes.len().is_multiple_of(4) {
        return None;
    }

    let mut data = Vec::with_capacity(unpadded / 4 * 3 + 2);
    let mut accumulator: u32 = 0;
    let mut bits = 0;
    for &byte in &bytes[..unpadded] {
        let value = value_of(byte)?;
        accumulator = (accumulator << 6) | u32::from(value);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            data.push((accumulator >> bits) as u8);
            accumulator &= (1 << bits) - 1;
        }
    }

    // The characters that remain after the last whole byte must be zero,
    // since Java refuses an encoding that carries bits no byte holds.
    if accumulator != 0 {
        return None;
    }

    Some(data)
}

/// Returns the text of the standard Base64 encoding of the data.
///
/// The text is padded the way Java pads it, so the same data is written as the
/// same text by both launchers.
pub fn encode(data: &[u8]) -> String {
    encode_with(data, ALPHABET, Some(PADDING))
}

/// Returns the text of the Base64 encoding of the data that is safe in an
/// address, without the padding.
///
/// The proof key of an OAuth login (RFC 7636) is written with this encoding,
/// which is the one Java writes with `Base64.getUrlEncoder().withoutPadding()`.
/// The padding is left out because the addresses of an authorization page
/// carry the value as it is, and a `=` would be encoded again.
pub fn encode_url_safe_unpadded(data: &[u8]) -> String {
    encode_with(data, URL_SAFE_ALPHABET, None)
}

/// Returns the text of the Base64 encoding of the data with an alphabet.
///
/// A padding that is absent leaves the characters of the last group shorter,
/// which is the form of the encoding of an address.
fn encode_with(data: &[u8], alphabet: &[u8; 64], padding: Option<u8>) -> String {
    let mut text = String::with_capacity(data.len().div_ceil(3) * 4);
    for group in data.chunks(3) {
        let first = group[0];
        let second = group.get(1).copied().unwrap_or(0);
        let third = group.get(2).copied().unwrap_or(0);

        text.push(char::from(alphabet[usize::from(first >> 2)]));
        text.push(char::from(
            alphabet[usize::from(((first & 0x03) << 4) | (second >> 4))],
        ));

        if group.len() > 1 {
            text.push(char::from(
                alphabet[usize::from(((second & 0x0f) << 2) | (third >> 6))],
            ));
        } else if let Some(padding) = padding {
            text.push(char::from(padding));
        }

        if group.len() > 2 {
            text.push(char::from(alphabet[usize::from(third & 0x3f)]));
        } else if let Some(padding) = padding {
            text.push(char::from(padding));
        }
    }
    text
}

/// Returns the value of a character of the alphabet.
fn value_of(byte: u8) -> Option<u8> {
    match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a' + 26),
        b'0'..=b'9' => Some(byte - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The vectors of the examples of RFC 4648, which Java encodes the same
    /// way.
    #[test]
    fn the_encoding_matches_the_one_of_java() {
        let vectors: [(&[u8], &str); 7] = [
            (b"", ""),
            (b"f", "Zg=="),
            (b"fo", "Zm8="),
            (b"foo", "Zm9v"),
            (b"foob", "Zm9vYg=="),
            (b"fooba", "Zm9vYmE="),
            (b"foobar", "Zm9vYmFy"),
        ];

        for (data, expected) in vectors {
            assert_eq!(encode(data), expected);
            assert_eq!(decode(expected), Some(data.to_vec()));
        }
    }

    #[test]
    fn every_byte_is_encoded() {
        let data: Vec<u8> = (0..=255).collect();
        let text = encode(&data);
        assert_eq!(
            text,
            "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8gISIjJCUmJygpKissLS4vMDEyMzQ1Njc4OTo7PD0+P0BBQkNERUZHSElKS0xNTk9QUVJTVFVWV1hZWltcXV5fYGFiY2RlZmdoaWprbG1ub3BxcnN0dXZ3eHl6e3x9fn+AgYKDhIWGh4iJiouMjY6PkJGSk5SVlpeYmZqbnJ2en6ChoqOkpaanqKmqq6ytrq+wsbKztLW2t7i5uru8vb6/wMHCw8TFxsfIycrLzM3Oz9DR0tPU1dbX2Nna29zd3t/g4eLj5OXm5+jp6uvs7e7v8PHy8/T19vf4+fr7/P3+/w=="
        );
        assert_eq!(decode(&text), Some(data));
    }

    #[test]
    fn the_padding_may_be_left_out() {
        assert_eq!(decode("Zg"), Some(b"f".to_vec()));
        assert_eq!(decode("Zm8"), Some(b"fo".to_vec()));
        assert_eq!(decode("Zm9vYg"), Some(b"foob".to_vec()));
    }

    #[test]
    fn a_text_outside_the_alphabet_is_refused() {
        // Java refuses the same texts, which keeps a broken file from being
        // read as another payload.
        assert_eq!(decode("Zg-="), None);
        assert_eq!(decode("Zg ="), None);
        assert_eq!(decode("Z"), None);
        assert_eq!(decode("Zg==="), None);
        assert_eq!(decode("Zg=="), Some(b"f".to_vec()));
    }

    #[test]
    fn the_encoding_of_an_address_is_safe_and_unpadded() {
        // The bytes that are written with `+` and `/` in the standard encoding
        // are written with `-` and `_` here, and the padding is left out, which
        // is the text Java writes with `getUrlEncoder().withoutPadding()`.
        assert_eq!(encode_url_safe_unpadded(b""), "");
        assert_eq!(encode_url_safe_unpadded(b"f"), "Zg");
        assert_eq!(encode_url_safe_unpadded(b"fo"), "Zm8");
        assert_eq!(encode_url_safe_unpadded(b"foob"), "Zm9vYg");
        assert_eq!(encode_url_safe_unpadded(&[0xfb, 0xff]), "-_8");
        assert_eq!(encode_url_safe_unpadded(&[0xff, 0xef]), "_-8");
        assert_eq!(encode(&[0xff, 0xef]), "/+8=");
    }
}
