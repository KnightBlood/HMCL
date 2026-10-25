//! The ChaCha20-Poly1305 cipher the launcher protects private data with.
//!
//! The launcher of Java stores the private data of the accounts in an
//! envelope that is encrypted with ChaCha20-Poly1305, as RFC 8439 defines it.
//! The key is built into both launchers rather than kept on the machine, so
//! the protection keeps the data from being read at a glance; it is not a
//! secret the machine holds. The launcher has to speak the same cipher to read
//! a file the launcher of Java wrote, so this module implements it.

/// The number of the bytes of the key of the cipher.
pub const KEY_LENGTH: usize = 32;

/// The number of the bytes of the nonce of the cipher.
pub const NONCE_LENGTH: usize = 12;

/// The number of the bytes the authentication tag adds to the ciphertext.
pub const TAG_LENGTH: usize = 16;

/// The constants that start the state of ChaCha20.
const CONSTANTS: [u32; 4] = [0x6170_7865, 0x3320_646e, 0x7962_2d32, 0x6b20_6574];

/// The number of the bytes of a block of the cipher.
const BLOCK_LENGTH: usize = 64;

/// The number of the bytes of a block of the authenticator, which is the size
/// of the number the authenticator works with.
const AUTHENTICATOR_BLOCK_LENGTH: usize = 16;

/// Encrypts data and appends the authentication tag.
///
/// The data is authenticated but not associated with any other data, which is
/// the way the launcher protects its private data. The result is the
/// ciphertext followed by the tag.
pub fn seal(key: &[u8; KEY_LENGTH], nonce: &[u8; NONCE_LENGTH], data: &[u8]) -> Vec<u8> {
    let (authentication_key, ciphertext) = encrypt(key, nonce, data);

    let mut sealed = ciphertext;
    let tag = tag_of(&authentication_key, &sealed);
    sealed.extend_from_slice(&tag);
    sealed
}

/// Decrypts data that ends with an authentication tag.
///
/// The data is refused when the tag does not match, so a file that was
/// changed is not read as another payload.
pub fn open(key: &[u8; KEY_LENGTH], nonce: &[u8; NONCE_LENGTH], sealed: &[u8]) -> Option<Vec<u8>> {
    if sealed.len() < TAG_LENGTH {
        return None;
    }

    let (ciphertext, tag) = sealed.split_at(sealed.len() - TAG_LENGTH);
    let (authentication_key, _) = encrypt(key, nonce, &[]);

    // The tag covers the ciphertext, which is the data of the caller without
    // the tag; it is checked before anything is decrypted.
    let expected = tag_of(&authentication_key, ciphertext);
    if !constant_time_eq(&expected, tag) {
        return None;
    }

    let (_, plaintext) = encrypt(key, nonce, ciphertext);
    Some(plaintext)
}

/// Returns the authentication key and the encrypted data.
///
/// The authentication key is the first half of the first block of the key
/// stream; the data is encrypted from the second block on.
fn encrypt(key: &[u8; KEY_LENGTH], nonce: &[u8; NONCE_LENGTH], data: &[u8]) -> ([u8; 32], Vec<u8>) {
    let mut stream = Stream::new(key, nonce);
    let mut authentication_key = [0u8; 32];
    authentication_key.copy_from_slice(&stream.next_block()[..32]);
    (authentication_key, stream.apply(data))
}

/// Returns the authentication tag of data under an authentication key.
fn tag_of(authentication_key: &[u8; 32], data: &[u8]) -> [u8; TAG_LENGTH] {
    let mut poly1305 = Poly1305::new(authentication_key);

    // The tag covers the ciphertext and the lengths of the data and of the
    // associated data, which is empty here. Both are padded with zeroes to a
    // whole number of blocks, unlike the one-shot authenticator of RFC 8439
    // section 2.5.
    poly1305.update_zero_padded(data);
    let mut lengths = [0u8; 16];
    lengths[8..].copy_from_slice(&(data.len() as u64).to_le_bytes());
    poly1305.update_block(&lengths, true);

    poly1305.finish()
}

/// Returns whether two tags are the same.
///
/// The comparison takes the same time whatever the difference is, so a wrong
/// tag tells nothing about how much of it was right.
fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }

    let mut difference = 0u8;
    for (left, right) in left.iter().zip(right) {
        difference |= left ^ right;
    }
    difference == 0
}

/// The key stream of ChaCha20.
struct Stream {
    /// The state the blocks are made from, which holds the key and the nonce.
    state: [u32; 16],
    /// The number of the next block.
    counter: u32,
    /// The bytes of the current block that were not used yet.
    block: [u8; BLOCK_LENGTH],
    /// The position of the next unused byte of the current block.
    position: usize,
}

impl Stream {
    /// Creates a stream that starts at the first block.
    fn new(key: &[u8; KEY_LENGTH], nonce: &[u8; NONCE_LENGTH]) -> Self {
        let mut state = [0u32; 16];
        state[..4].copy_from_slice(&CONSTANTS);
        for (index, word) in key.as_chunks::<4>().0.iter().enumerate() {
            state[4 + index] = u32::from_le_bytes(*word);
        }
        for (index, word) in nonce.as_chunks::<4>().0.iter().enumerate() {
            state[13 + index] = u32::from_le_bytes(*word);
        }

        Self {
            state,
            counter: 0,
            block: [0u8; BLOCK_LENGTH],
            position: BLOCK_LENGTH,
        }
    }

    /// Returns the next block of the key stream.
    fn next_block(&mut self) -> [u8; BLOCK_LENGTH] {
        let mut state = self.state;
        state[12] = self.counter;
        self.counter = self.counter.wrapping_add(1);

        let working = block_of(&state);
        let mut block = [0u8; BLOCK_LENGTH];
        for (index, word) in working.iter().enumerate() {
            block[index * 4..index * 4 + 4].copy_from_slice(&word.to_le_bytes());
        }
        block
    }

    /// Returns data encrypted with the key stream.
    fn apply(&mut self, data: &[u8]) -> Vec<u8> {
        let mut result = Vec::with_capacity(data.len());
        for &byte in data {
            if self.position == BLOCK_LENGTH {
                self.block = self.next_block();
                self.position = 0;
            }
            result.push(byte ^ self.block[self.position]);
            self.position += 1;
        }
        result
    }
}

/// Returns the block of ChaCha20 made from a state.
fn block_of(state: &[u32; 16]) -> [u32; 16] {
    let mut working = *state;
    for _ in 0..10 {
        quarter_round(&mut working, 0, 4, 8, 12);
        quarter_round(&mut working, 1, 5, 9, 13);
        quarter_round(&mut working, 2, 6, 10, 14);
        quarter_round(&mut working, 3, 7, 11, 15);
        quarter_round(&mut working, 0, 5, 10, 15);
        quarter_round(&mut working, 1, 6, 11, 12);
        quarter_round(&mut working, 2, 7, 8, 13);
        quarter_round(&mut working, 3, 4, 9, 14);
    }

    for (index, word) in working.iter_mut().enumerate() {
        *word = word.wrapping_add(state[index]);
    }
    working
}

/// Mixes four words of a state.
fn quarter_round(state: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize) {
    state[a] = state[a].wrapping_add(state[b]);
    state[d] = (state[d] ^ state[a]).rotate_left(16);
    state[c] = state[c].wrapping_add(state[d]);
    state[b] = (state[b] ^ state[c]).rotate_left(12);
    state[a] = state[a].wrapping_add(state[b]);
    state[d] = (state[d] ^ state[a]).rotate_left(8);
    state[c] = state[c].wrapping_add(state[d]);
    state[b] = (state[b] ^ state[c]).rotate_left(7);
}

/// The Poly1305 one-time authenticator.
///
/// The accumulator holds five limbs of 26 bits rather than four words, which
/// leaves room for the carry of the multiplication.
struct Poly1305 {
    /// The clamped key of the authenticator.
    r: [u32; 5],
    /// The accumulator.
    h: [u32; 5],
    /// The value added to the accumulator at the end.
    pad: [u32; 4],
}

impl Poly1305 {
    /// Creates an authenticator from its one-time key.
    fn new(key: &[u8; 32]) -> Self {
        let t0 = u32::from_le_bytes([key[0], key[1], key[2], key[3]]);
        let t1 = u32::from_le_bytes([key[4], key[5], key[6], key[7]]);
        let t2 = u32::from_le_bytes([key[8], key[9], key[10], key[11]]);
        let t3 = u32::from_le_bytes([key[12], key[13], key[14], key[15]]);

        let r = [
            t0 & 0x03ff_ffff,
            ((t0 >> 26) | (t1 << 6)) & 0x03ff_ff03,
            ((t1 >> 20) | (t2 << 12)) & 0x03ff_c0ff,
            ((t2 >> 14) | (t3 << 18)) & 0x03f0_3fff,
            (t3 >> 8) & 0x000f_ffff,
        ];

        let pad = [
            u32::from_le_bytes([key[16], key[17], key[18], key[19]]),
            u32::from_le_bytes([key[20], key[21], key[22], key[23]]),
            u32::from_le_bytes([key[24], key[25], key[26], key[27]]),
            u32::from_le_bytes([key[28], key[29], key[30], key[31]]),
        ];

        Self {
            r,
            h: [0u32; 5],
            pad,
        }
    }

    /// Authenticates data of any length, the way the one-shot authenticator
    /// of RFC 8439 section 2.5 does.
    ///
    /// The cipher does not use this form, for it pads its data with zeroes
    /// instead; it is kept so the authenticator can be checked against the
    /// vectors of the standard on its own.
    #[cfg(test)]
    fn update(&mut self, data: &[u8]) {
        let blocks = data.as_chunks::<AUTHENTICATOR_BLOCK_LENGTH>();
        for block in blocks.0.iter() {
            self.update_block(block, true);
        }

        // The last block is read as the number its bytes stand for, with a
        // marker byte after the bytes of the data; the number is then smaller
        // than the modulus by the marker alone.
        let remainder = blocks.1;
        if !remainder.is_empty() {
            let mut block = [0u8; AUTHENTICATOR_BLOCK_LENGTH];
            block[..remainder.len()].copy_from_slice(remainder);
            block[remainder.len()] = 1;
            self.update_block(&block, false);
        }
    }

    /// Authenticates data that is padded with zeroes to a whole number of
    /// blocks.
    ///
    /// The authenticated data of the cipher is padded this way, so the last
    /// block of a ciphertext is read as the number its bytes stand for with
    /// the bytes after it zero.
    fn update_zero_padded(&mut self, data: &[u8]) {
        let blocks = data.as_chunks::<AUTHENTICATOR_BLOCK_LENGTH>();
        for block in blocks.0.iter() {
            self.update_block(block, true);
        }

        let remainder = blocks.1;
        if !remainder.is_empty() {
            let mut block = [0u8; AUTHENTICATOR_BLOCK_LENGTH];
            block[..remainder.len()].copy_from_slice(remainder);
            self.update_block(&block, true);
        }
    }

    /// Authenticates one block.
    ///
    /// A whole block is marked as a block of the message by the bit above its
    /// highest byte, so a block of zeroes at the end of the data is not the
    /// same as no block at all. A block the caller padded already carries the
    /// marker inside its bytes.
    fn update_block(&mut self, block: &[u8], whole: bool) {
        let mut words = [0u32; 4];
        for (index, word) in words.iter_mut().enumerate() {
            let start = index * 4;
            *word = u32::from_le_bytes([
                block[start],
                block[start + 1],
                block[start + 2],
                block[start + 3],
            ]);
        }

        self.h[0] = self.h[0].wrapping_add(words[0] & 0x03ff_ffff);
        self.h[1] = self.h[1].wrapping_add(((words[0] >> 26) | (words[1] << 6)) & 0x03ff_ffff);
        self.h[2] = self.h[2].wrapping_add(((words[1] >> 20) | (words[2] << 12)) & 0x03ff_ffff);
        self.h[3] = self.h[3].wrapping_add(((words[2] >> 14) | (words[3] << 18)) & 0x03ff_ffff);
        self.h[4] = self.h[4].wrapping_add((words[3] >> 8) | if whole { 1 << 24 } else { 0 });

        self.multiply();
    }

    /// Multiplies the accumulator by the key and keeps it small.
    fn multiply(&mut self) {
        let [h0, h1, h2, h3, h4] = self.h;
        let [r0, r1, r2, r3, r4] = self.r;

        // The limbs that are multiplied by five stand for the parts of the
        // product that wrap around the modulus.
        let s1 = r1 * 5;
        let s2 = r2 * 5;
        let s3 = r3 * 5;
        let s4 = r4 * 5;

        let d0 = u64::from(h0) * u64::from(r0)
            + u64::from(h1) * u64::from(s4)
            + u64::from(h2) * u64::from(s3)
            + u64::from(h3) * u64::from(s2)
            + u64::from(h4) * u64::from(s1);
        let d1 = u64::from(h0) * u64::from(r1)
            + u64::from(h1) * u64::from(r0)
            + u64::from(h2) * u64::from(s4)
            + u64::from(h3) * u64::from(s3)
            + u64::from(h4) * u64::from(s2);
        let d2 = u64::from(h0) * u64::from(r2)
            + u64::from(h1) * u64::from(r1)
            + u64::from(h2) * u64::from(r0)
            + u64::from(h3) * u64::from(s4)
            + u64::from(h4) * u64::from(s3);
        let d3 = u64::from(h0) * u64::from(r3)
            + u64::from(h1) * u64::from(r2)
            + u64::from(h2) * u64::from(r1)
            + u64::from(h3) * u64::from(r0)
            + u64::from(h4) * u64::from(s4);
        let d4 = u64::from(h0) * u64::from(r4)
            + u64::from(h1) * u64::from(r3)
            + u64::from(h2) * u64::from(r2)
            + u64::from(h3) * u64::from(r1)
            + u64::from(h4) * u64::from(r0);

        let mut carry = d0 >> 26;
        self.h[0] = (d0 as u32) & 0x03ff_ffff;
        let d1 = d1 + carry;
        carry = d1 >> 26;
        self.h[1] = (d1 as u32) & 0x03ff_ffff;
        let d2 = d2 + carry;
        carry = d2 >> 26;
        self.h[2] = (d2 as u32) & 0x03ff_ffff;
        let d3 = d3 + carry;
        carry = d3 >> 26;
        self.h[3] = (d3 as u32) & 0x03ff_ffff;
        let d4 = d4 + carry;
        carry = d4 >> 26;
        self.h[4] = (d4 as u32) & 0x03ff_ffff;

        // The carry of the highest limb wraps around and is added back at the
        // bottom, scaled by five.
        self.h[0] = self.h[0].wrapping_add((carry as u32).wrapping_mul(5));
        let carry = self.h[0] >> 26;
        self.h[0] &= 0x03ff_ffff;
        self.h[1] = self.h[1].wrapping_add(carry);
    }

    /// Returns the tag of the data.
    fn finish(mut self) -> [u8; TAG_LENGTH] {
        // The accumulator is brought below the modulus by subtracting it once
        // when it is not smaller.
        let mut carry = self.h[1] >> 26;
        self.h[1] &= 0x03ff_ffff;
        self.h[2] = self.h[2].wrapping_add(carry);
        carry = self.h[2] >> 26;
        self.h[2] &= 0x03ff_ffff;
        self.h[3] = self.h[3].wrapping_add(carry);
        carry = self.h[3] >> 26;
        self.h[3] &= 0x03ff_ffff;
        self.h[4] = self.h[4].wrapping_add(carry);
        carry = self.h[4] >> 26;
        self.h[4] &= 0x03ff_ffff;
        self.h[0] = self.h[0].wrapping_add(carry * 5);
        carry = self.h[0] >> 26;
        self.h[0] &= 0x03ff_ffff;
        self.h[1] = self.h[1].wrapping_add(carry);

        let [h0, h1, h2, h3, h4] = self.h;

        // The value of the modulus is subtracted once and the result is taken
        // when it did not go below zero, which is the comparison with the
        // modulus without a branch.
        let mut g0 = h0.wrapping_add(5);
        let mut carry = g0 >> 26;
        g0 &= 0x03ff_ffff;
        let mut g1 = h1.wrapping_add(carry);
        carry = g1 >> 26;
        g1 &= 0x03ff_ffff;
        let mut g2 = h2.wrapping_add(carry);
        carry = g2 >> 26;
        g2 &= 0x03ff_ffff;
        let mut g3 = h3.wrapping_add(carry);
        carry = g3 >> 26;
        g3 &= 0x03ff_ffff;
        let g4 = h4.wrapping_add(carry).wrapping_sub(1 << 26);

        // A negative highest limb means the accumulator was already smaller
        // than the modulus, so the subtraction is undone.
        let mask = (g4 >> 31).wrapping_sub(1);
        g0 &= mask;
        g1 &= mask;
        g2 &= mask;
        g3 &= mask;
        let g4 = g4 & mask;

        let h0 = (h0 & !mask) | g0;
        let h1 = (h1 & !mask) | g1;
        let h2 = (h2 & !mask) | g2;
        let h3 = (h3 & !mask) | g3;
        let h4 = (h4 & !mask) | g4;

        // The limbs are packed back into four words.
        let word0 = h0 | (h1 << 26);
        let word1 = (h1 >> 6) | (h2 << 20);
        let word2 = (h2 >> 12) | (h3 << 14);
        let word3 = (h3 >> 18) | (h4 << 8);

        let mut sum = u64::from(word0) + u64::from(self.pad[0]);
        let mut tag = [0u8; TAG_LENGTH];
        tag[0..4].copy_from_slice(&(sum as u32).to_le_bytes());
        sum = u64::from(word1) + u64::from(self.pad[1]) + (sum >> 32);
        tag[4..8].copy_from_slice(&(sum as u32).to_le_bytes());
        sum = u64::from(word2) + u64::from(self.pad[2]) + (sum >> 32);
        tag[8..12].copy_from_slice(&(sum as u32).to_le_bytes());
        sum = u64::from(word3) + u64::from(self.pad[3]) + (sum >> 32);
        tag[12..16].copy_from_slice(&(sum as u32).to_le_bytes());

        tag
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Returns the bytes of a hexadecimal text.
    fn hex(text: &str) -> Vec<u8> {
        let text: String = text.chars().filter(|c| !c.is_whitespace()).collect();
        (0..text.len() / 2)
            .map(|index| u8::from_str_radix(&text[index * 2..index * 2 + 2], 16).expect("hex"))
            .collect()
    }

    #[test]
    fn the_authenticator_matches_the_vector_of_rfc_8439() {
        // The vector of section 2.5.2 of RFC 8439.
        let key = hex("85d6be7857556d337f4452fe42d506a80103808afb0db2fd4abff6af4149f51b");
        let key: [u8; 32] = key.try_into().expect("the key has thirty-two bytes");
        let message = b"Cryptographic Forum Research Group";

        let mut poly1305 = Poly1305::new(&key);
        poly1305.update(message);
        assert_eq!(
            poly1305.finish(),
            hex("a8061dc1305136c6c22b8baf0c0127a9")[..]
        );
    }

    #[test]
    fn the_cipher_matches_the_vector_of_rfc_8439() {
        // The vector of section 2.8.2 of RFC 8439, which is also the vector
        // Java's ChaCha20-Poly1305 produces.
        let mut key = [0u8; 32];
        for (index, byte) in key.iter_mut().enumerate() {
            *byte = 0x80 + index as u8;
        }
        let nonce = hex("070000004041424344454647");
        let nonce: [u8; 12] = nonce.try_into().expect("the nonce has twelve bytes");
        let plaintext = b"Ladies and Gentlemen of the class of '99: If I could offer you only one tip for the future, sunscreen would be it.";

        let sealed = seal(&key, &nonce, plaintext);
        let expected_ciphertext = hex(
            "d31a8d34648e60db7b86afbc53ef7ec2a4aded51296e08fea9e2b5a736ee62d6\
             3dbea45e8ca9671282fafb69da92728b1a71de0a9e060b2905d6a5b67ecd3b36\
             92ddbd7f2d778b8c9803aee328091b58fab324e4fad675945585808b4831d7bc\
             3ff4def08e4b7a9de576d26586cec64b6116",
        );
        assert_eq!(
            &sealed[..sealed.len() - TAG_LENGTH],
            &expected_ciphertext[..]
        );
        assert_eq!(
            &sealed[sealed.len() - TAG_LENGTH..],
            &hex("6a23a4681fd59456aea1d29f82477216")[..]
        );

        assert_eq!(open(&key, &nonce, &sealed), Some(plaintext.to_vec()));
    }

    #[test]
    fn the_payloads_of_the_launcher_of_java_are_read() {
        // The key the launcher of Java protects the private data of the
        // accounts with; the payloads were encrypted by Java itself.
        let key: [u8; 32] = [
            0x3c, 0xd8, 0xa2, 0x22, 0x11, 0xd2, 0x8d, 0x89, 0xb4, 0xf7, 0xd9, 0xb0, 0x65, 0xbc,
            0x14, 0x8a, 0x6e, 0xb0, 0xa9, 0x4d, 0xeb, 0x93, 0x99, 0x6f, 0x84, 0x07, 0x5a, 0x9e,
            0xbd, 0xc8, 0xd1, 0xeb,
        ];
        let nonce: [u8; 12] = [
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c,
        ];

        let vectors: [(&str, &str); 3] = [
            (
                "[]",
                "2f3dfebb0c2a7f9d58004b05291e15dbd56b",
            ),
            (
                r#"[{"accountID":"account-0199a3b4-1111-7222-8333-444444444444","privateData":{"accessToken":"secret-token"}}]"#,
                "2f1b4b2bf61c6d13123f08c98726fac462741bb0c91ef55fc11d33ca2ef6005f990281530885430148a4bf79d1467b34134f3bb136d0d7475c4ba2da5925f594a21abe47791a11a70c0832960b976d3544af9cbdbc47d01983a90ed79ff5465ebcf1f2952476c7ea2f99d3f867d4b9778997f0d1a1618cf0874980",
            ),
            (
                r#"{"profileName":"Steve","accessToken":"eyJhbGciOiJub25lIn0.eyJ4IjoxfQ."}"#,
                "0f421938fa196b0a190520e0c03ee287526311b3c248f44d914769ce6ee7601dc356de401f90144a30e1e50d811c19696d0e6db73788aa1d5851f3813f33cc8ca414b962327c0d511a38207d73976d1024cf7fb1933508",
            ),
        ];

        for (payload, sealed) in vectors {
            let sealed = hex(sealed);
            assert_eq!(
                seal(&key, &nonce, payload.as_bytes()),
                sealed,
                "{payload} should be protected the way Java protects it"
            );
            assert_eq!(
                open(&key, &nonce, &sealed),
                Some(payload.as_bytes().to_vec())
            );
        }
    }

    #[test]
    fn data_the_cipher_did_not_produce_is_refused() {
        let key = [7u8; 32];
        let nonce = [3u8; 12];
        let sealed = seal(&key, &nonce, b"private data");

        // A changed byte anywhere is refused.
        let mut changed = sealed.clone();
        changed[0] ^= 1;
        assert_eq!(open(&key, &nonce, &changed), None);

        let mut changed = sealed.clone();
        let last = changed.len() - 1;
        changed[last] ^= 1;
        assert_eq!(open(&key, &nonce, &changed), None);

        // Data that is too short to hold a tag is refused as well.
        assert_eq!(open(&key, &nonce, &sealed[..TAG_LENGTH - 1]), None);
    }

    #[test]
    fn data_of_any_length_is_protected() {
        let key = [0u8; 32];
        let nonce = [0u8; 12];
        for length in [0, 1, 15, 16, 17, 63, 64, 65, 100, 255, 1000] {
            let data: Vec<u8> = (0..length).map(|index| (index % 251) as u8).collect();
            let sealed = seal(&key, &nonce, &data);
            assert_eq!(sealed.len(), data.len() + TAG_LENGTH);
            assert_eq!(open(&key, &nonce, &sealed), Some(data));
        }
    }
}
