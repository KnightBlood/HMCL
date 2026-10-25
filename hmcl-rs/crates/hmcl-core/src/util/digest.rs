//! The digests the launcher computes.
//!
//! The launcher checks the files it downloads against the digests the
//! servers give, and it checks the update of the launcher itself. The
//! digests are the ones of Java, so the same files have the same digests in
//! both launchers.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use crate::error::Error;

/// The algorithms of the digests the launcher computes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Algorithm {
    /// The digest of the files of the game, of 160 bits.
    Sha1,
    /// The digest of the update of the launcher, of 256 bits.
    Sha256,
    /// The digest of 512 bits.
    Sha512,
    /// The digest of 128 bits, which Java still uses for a few things.
    Md5,
}

impl Algorithm {
    /// Returns the name of the algorithm, as Java names it.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Sha1 => "SHA-1",
            Self::Sha256 => "SHA-256",
            Self::Sha512 => "SHA-512",
            Self::Md5 => "MD5",
        }
    }

    /// Returns the number of the bytes of the digest.
    pub const fn length(self) -> usize {
        match self {
            Self::Sha1 => 20,
            Self::Sha256 => 32,
            Self::Sha512 => 64,
            Self::Md5 => 16,
        }
    }

    /// Computes the digest of the data.
    pub fn digest(self, data: &[u8]) -> Vec<u8> {
        match self {
            Self::Sha1 => {
                let mut digest = Sha1::new();
                digest.update(data);
                digest.finish().to_vec()
            }
            Self::Sha256 => {
                let mut digest = Sha256::new();
                digest.update(data);
                digest.finish().to_vec()
            }
            Self::Sha512 => {
                let mut digest = Sha512::new();
                digest.update(data);
                digest.finish().to_vec()
            }
            Self::Md5 => {
                let mut digest = Md5::new();
                digest.update(data);
                digest.finish().to_vec()
            }
        }
    }

    /// Computes the digest of the data, written as a text of hexadecimal
    /// digits.
    pub fn digest_to_string(self, data: &[u8]) -> String {
        to_hex(&self.digest(data))
    }

    /// Computes the digest of a file.
    pub fn digest_file(self, path: &Path) -> Result<Vec<u8>, Error> {
        let mut file = File::open(path).map_err(|error| Error::io(path, error))?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)
            .map_err(|error| Error::io(path, error))?;

        Ok(self.digest(&data))
    }

    /// Computes the digest of a file, written as a text of hexadecimal
    /// digits.
    pub fn digest_file_to_string(self, path: &Path) -> Result<String, Error> {
        Ok(to_hex(&self.digest_file(path)?))
    }
}

/// Returns the text of the hexadecimal digits of the bytes.
pub fn to_hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";

    let mut result = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        result.push(DIGITS[(byte >> 4) as usize] as char);
        result.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    result
}

/// Returns whether the text is a digest of 160 bits.
pub fn is_sha1_digest(digest: &str) -> bool {
    is_hex_digest(digest, 40)
}

/// Returns whether the text is a digest of 256 bits.
pub fn is_sha256_digest(digest: &str) -> bool {
    is_hex_digest(digest, 64)
}

/// Returns whether the text is a digest of 512 bits.
pub fn is_sha512_digest(digest: &str) -> bool {
    is_hex_digest(digest, 128)
}

/// Returns whether the text is a text of hexadecimal digits of the given
/// length.
fn is_hex_digest(digest: &str, length: usize) -> bool {
    digest.len() == length
        && digest
            .chars()
            .all(|character| character.is_ascii_hexdigit())
}

/// The digest of the files of the game, of 160 bits.
#[derive(Debug, Clone)]
pub struct Sha1 {
    /// The state of the digest.
    state: [u32; 5],
    /// The data that is not hashed yet.
    buffer: [u8; 64],
    /// The number of the bytes in the buffer.
    length: usize,
    /// The number of the bytes that were hashed.
    total: u64,
}

impl Default for Sha1 {
    fn default() -> Self {
        Self::new()
    }
}

impl Sha1 {
    /// Creates a digest of no data.
    pub fn new() -> Self {
        Self {
            state: [
                0x6745_2301,
                0xefcd_ab89,
                0x98ba_dcfe,
                0x1032_5476,
                0xc3d2_e1f0,
            ],
            buffer: [0; 64],
            length: 0,
            total: 0,
        }
    }

    /// Adds the data to the digest.
    pub fn update(&mut self, data: &[u8]) {
        self.total += data.len() as u64;

        let mut offset = 0;
        if self.length > 0 {
            let count = (64 - self.length).min(data.len());
            self.buffer[self.length..self.length + count].copy_from_slice(&data[..count]);
            self.length += count;
            offset = count;

            if self.length == 64 {
                let block = self.buffer;
                self.hash_block(&block);
                self.length = 0;
            }
        }

        while offset + 64 <= data.len() {
            let mut block = [0u8; 64];
            block.copy_from_slice(&data[offset..offset + 64]);
            self.hash_block(&block);
            offset += 64;
        }

        if offset < data.len() {
            let count = data.len() - offset;
            self.buffer[..count].copy_from_slice(&data[offset..]);
            self.length = count;
        }
    }

    /// Returns the digest of the data.
    pub fn finish(mut self) -> [u8; 20] {
        let bits = self.total * 8;

        self.update(&[0x80]);
        while self.length != 56 {
            self.update(&[0]);
        }
        self.update(&bits.to_be_bytes());

        let mut result = [0u8; 20];
        for (index, word) in self.state.iter().enumerate() {
            result[index * 4..index * 4 + 4].copy_from_slice(&word.to_be_bytes());
        }
        result
    }

    /// Hashes a block of 64 bytes.
    fn hash_block(&mut self, block: &[u8; 64]) {
        let mut words = [0u32; 80];
        for (index, word) in words.iter_mut().enumerate().take(16) {
            let start = index * 4;
            *word = u32::from_be_bytes([
                block[start],
                block[start + 1],
                block[start + 2],
                block[start + 3],
            ]);
        }

        for index in 16..80 {
            words[index] =
                (words[index - 3] ^ words[index - 8] ^ words[index - 14] ^ words[index - 16])
                    .rotate_left(1);
        }

        let [mut a, mut b, mut c, mut d, mut e] = self.state;

        for (index, word) in words.iter().enumerate() {
            let (f, k) = match index {
                0..=19 => ((b & c) | ((!b) & d), 0x5a82_7999u32),
                20..=39 => (b ^ c ^ d, 0x6ed9_eba1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8f1b_bcdc),
                _ => (b ^ c ^ d, 0xca62_c1d6),
            };

            let temp = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(*word);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }

        self.state[0] = self.state[0].wrapping_add(a);
        self.state[1] = self.state[1].wrapping_add(b);
        self.state[2] = self.state[2].wrapping_add(c);
        self.state[3] = self.state[3].wrapping_add(d);
        self.state[4] = self.state[4].wrapping_add(e);
    }
}

/// The digest of 128 bits, which Java still uses for a few things.
#[derive(Debug, Clone)]
pub struct Md5 {
    /// The state of the digest.
    state: [u32; 4],
    /// The data that is not hashed yet.
    buffer: [u8; 64],
    /// The number of the bytes in the buffer.
    length: usize,
    /// The number of the bytes that were hashed.
    total: u64,
}

impl Default for Md5 {
    fn default() -> Self {
        Self::new()
    }
}

impl Md5 {
    /// Creates a digest of no data.
    pub fn new() -> Self {
        Self {
            state: [0x6745_2301, 0xefcd_ab89, 0x98ba_dcfe, 0x1032_5476],
            buffer: [0; 64],
            length: 0,
            total: 0,
        }
    }

    /// Adds the data to the digest.
    pub fn update(&mut self, data: &[u8]) {
        self.total += data.len() as u64;

        let mut offset = 0;
        if self.length > 0 {
            let count = (64 - self.length).min(data.len());
            self.buffer[self.length..self.length + count].copy_from_slice(&data[..count]);
            self.length += count;
            offset = count;

            if self.length == 64 {
                let block = self.buffer;
                self.hash_block(&block);
                self.length = 0;
            }
        }

        while offset + 64 <= data.len() {
            let mut block = [0u8; 64];
            block.copy_from_slice(&data[offset..offset + 64]);
            self.hash_block(&block);
            offset += 64;
        }

        if offset < data.len() {
            let count = data.len() - offset;
            self.buffer[..count].copy_from_slice(&data[offset..]);
            self.length = count;
        }
    }

    /// Returns the digest of the data.
    pub fn finish(mut self) -> [u8; 16] {
        let bits = self.total * 8;

        self.update(&[0x80]);
        while self.length != 56 {
            self.update(&[0]);
        }
        self.update(&bits.to_le_bytes());

        let mut result = [0u8; 16];
        for (index, word) in self.state.iter().enumerate() {
            result[index * 4..index * 4 + 4].copy_from_slice(&word.to_le_bytes());
        }
        result
    }

    /// Hashes a block of 64 bytes.
    fn hash_block(&mut self, block: &[u8; 64]) {
        const S: [u32; 64] = [
            7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20,
            5, 9, 14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23,
            6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
        ];

        const K: [u32; 64] = [
            0xd76a_a478,
            0xe8c7_b756,
            0x2420_70db,
            0xc1bd_ceee,
            0xf57c_0faf,
            0x4787_c62a,
            0xa830_4613,
            0xfd46_9501,
            0x6980_98d8,
            0x8b44_f7af,
            0xffff_5bb1,
            0x895c_d7be,
            0x6b90_1122,
            0xfd98_7193,
            0xa679_438e,
            0x49b4_0821,
            0xf61e_2562,
            0xc040_b340,
            0x265e_5a51,
            0xe9b6_c7aa,
            0xd62f_105d,
            0x0244_1453,
            0xd8a1_e681,
            0xe7d3_fbc8,
            0x21e1_cde6,
            0xc337_07d6,
            0xf4d5_0d87,
            0x455a_14ed,
            0xa9e3_e905,
            0xfcef_a3f8,
            0x676f_02d9,
            0x8d2a_4c8a,
            0xfffa_3942,
            0x8771_f681,
            0x6d9d_6122,
            0xfde5_380c,
            0xa4be_ea44,
            0x4bde_cfa9,
            0xf6bb_4b60,
            0xbebf_bc70,
            0x289b_7ec6,
            0xeaa1_27fa,
            0xd4ef_3085,
            0x0488_1d05,
            0xd9d4_d039,
            0xe6db_99e5,
            0x1fa2_7cf8,
            0xc4ac_5665,
            0xf429_2244,
            0x432a_ff97,
            0xab94_23a7,
            0xfc93_a039,
            0x655b_59c3,
            0x8f0c_cc92,
            0xffef_f47d,
            0x8584_5dd1,
            0x6fa8_7e4f,
            0xfe2c_e6e0,
            0xa301_4314,
            0x4e08_11a1,
            0xf753_7e82,
            0xbd3a_f235,
            0x2ad7_d2bb,
            0xeb86_d391,
        ];

        let mut words = [0u32; 16];
        for (index, word) in words.iter_mut().enumerate() {
            let start = index * 4;
            *word = u32::from_le_bytes([
                block[start],
                block[start + 1],
                block[start + 2],
                block[start + 3],
            ]);
        }

        let [mut a, mut b, mut c, mut d] = self.state;

        for index in 0..64 {
            let (f, g) = match index {
                0..=15 => ((b & c) | ((!b) & d), index),
                16..=31 => ((d & b) | ((!d) & c), (5 * index + 1) % 16),
                32..=47 => (b ^ c ^ d, (3 * index + 5) % 16),
                _ => (c ^ (b | (!d)), (7 * index) % 16),
            };

            let temp = d;
            d = c;
            c = b;
            let sum = a
                .wrapping_add(f)
                .wrapping_add(K[index])
                .wrapping_add(words[g]);
            b = b.wrapping_add(sum.rotate_left(S[index]));
            a = temp;
        }

        self.state[0] = self.state[0].wrapping_add(a);
        self.state[1] = self.state[1].wrapping_add(b);
        self.state[2] = self.state[2].wrapping_add(c);
        self.state[3] = self.state[3].wrapping_add(d);
    }
}

/// The digest of 256 bits.
#[derive(Debug, Clone)]
pub struct Sha256 {
    /// The state of the digest.
    state: [u32; 8],
    /// The data that is not hashed yet.
    buffer: [u8; 64],
    /// The number of the bytes in the buffer.
    length: usize,
    /// The number of the bytes that were hashed.
    total: u64,
}

impl Default for Sha256 {
    fn default() -> Self {
        Self::new()
    }
}

impl Sha256 {
    /// Creates a digest of no data.
    pub fn new() -> Self {
        Self {
            state: [
                0x6a09_e667,
                0xbb67_ae85,
                0x3c6e_f372,
                0xa54f_f53a,
                0x510e_527f,
                0x9b05_688c,
                0x1f83_d9ab,
                0x5be0_cd19,
            ],
            buffer: [0; 64],
            length: 0,
            total: 0,
        }
    }

    /// Adds the data to the digest.
    pub fn update(&mut self, data: &[u8]) {
        self.total += data.len() as u64;

        let mut offset = 0;
        if self.length > 0 {
            let count = (64 - self.length).min(data.len());
            self.buffer[self.length..self.length + count].copy_from_slice(&data[..count]);
            self.length += count;
            offset = count;

            if self.length == 64 {
                let block = self.buffer;
                self.hash_block(&block);
                self.length = 0;
            }
        }

        while offset + 64 <= data.len() {
            let mut block = [0u8; 64];
            block.copy_from_slice(&data[offset..offset + 64]);
            self.hash_block(&block);
            offset += 64;
        }

        if offset < data.len() {
            let count = data.len() - offset;
            self.buffer[..count].copy_from_slice(&data[offset..]);
            self.length = count;
        }
    }

    /// Returns the digest of the data.
    pub fn finish(mut self) -> [u8; 32] {
        let bits = self.total * 8;

        self.update(&[0x80]);
        while self.length != 56 {
            self.update(&[0]);
        }
        self.update(&bits.to_be_bytes());

        let mut result = [0u8; 32];
        for (index, word) in self.state.iter().enumerate() {
            result[index * 4..index * 4 + 4].copy_from_slice(&word.to_be_bytes());
        }
        result
    }

    /// Hashes a block of 64 bytes.
    fn hash_block(&mut self, block: &[u8; 64]) {
        const K: [u32; 64] = [
            0x428a_2f98,
            0x7137_4491,
            0xb5c0_fbcf,
            0xe9b5_dba5,
            0x3956_c25b,
            0x59f1_11f1,
            0x923f_82a4,
            0xab1c_5ed5,
            0xd807_aa98,
            0x1283_5b01,
            0x2431_85be,
            0x550c_7dc3,
            0x72be_5d74,
            0x80de_b1fe,
            0x9bdc_06a7,
            0xc19b_f174,
            0xe49b_69c1,
            0xefbe_4786,
            0x0fc1_9dc6,
            0x240c_a1cc,
            0x2de9_2c6f,
            0x4a74_84aa,
            0x5cb0_a9dc,
            0x76f9_88da,
            0x983e_5152,
            0xa831_c66d,
            0xb003_27c8,
            0xbf59_7fc7,
            0xc6e0_0bf3,
            0xd5a7_9147,
            0x06ca_6351,
            0x1429_2967,
            0x27b7_0a85,
            0x2e1b_2138,
            0x4d2c_6dfc,
            0x5338_0d13,
            0x650a_7354,
            0x766a_0abb,
            0x81c2_c92e,
            0x9272_2c85,
            0xa2bf_e8a1,
            0xa81a_664b,
            0xc24b_8b70,
            0xc76c_51a3,
            0xd192_e819,
            0xd699_0624,
            0xf40e_3585,
            0x106a_a070,
            0x19a4_c116,
            0x1e37_6c08,
            0x2748_774c,
            0x34b0_bcb5,
            0x391c_0cb3,
            0x4ed8_aa4a,
            0x5b9c_ca4f,
            0x682e_6ff3,
            0x748f_82ee,
            0x78a5_636f,
            0x84c8_7814,
            0x8cc7_0208,
            0x90be_fffa,
            0xa450_6ceb,
            0xbef9_a3f7,
            0xc671_78f2,
        ];

        let mut words = [0u32; 64];
        for (index, word) in words.iter_mut().enumerate().take(16) {
            let start = index * 4;
            *word = u32::from_be_bytes([
                block[start],
                block[start + 1],
                block[start + 2],
                block[start + 3],
            ]);
        }

        for index in 16..64 {
            let s0 = words[index - 15].rotate_right(7)
                ^ words[index - 15].rotate_right(18)
                ^ (words[index - 15] >> 3);
            let s1 = words[index - 2].rotate_right(17)
                ^ words[index - 2].rotate_right(19)
                ^ (words[index - 2] >> 10);
            words[index] = words[index - 16]
                .wrapping_add(s0)
                .wrapping_add(words[index - 7])
                .wrapping_add(s1);
        }

        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = self.state;

        for index in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let choose = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(s1)
                .wrapping_add(choose)
                .wrapping_add(K[index])
                .wrapping_add(words[index]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let majority = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(majority);

            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        self.state[0] = self.state[0].wrapping_add(a);
        self.state[1] = self.state[1].wrapping_add(b);
        self.state[2] = self.state[2].wrapping_add(c);
        self.state[3] = self.state[3].wrapping_add(d);
        self.state[4] = self.state[4].wrapping_add(e);
        self.state[5] = self.state[5].wrapping_add(f);
        self.state[6] = self.state[6].wrapping_add(g);
        self.state[7] = self.state[7].wrapping_add(h);
    }
}

/// The digest of 512 bits.
#[derive(Debug, Clone)]
pub struct Sha512 {
    /// The state of the digest.
    state: [u64; 8],
    /// The data that is not hashed yet.
    buffer: [u8; 128],
    /// The number of the bytes in the buffer.
    length: usize,
    /// The number of the bytes that were hashed.
    total: u128,
}

impl Default for Sha512 {
    fn default() -> Self {
        Self::new()
    }
}

impl Sha512 {
    /// Creates a digest of no data.
    pub fn new() -> Self {
        Self {
            state: [
                0x6a09_e667_f3bc_c908,
                0xbb67_ae85_84ca_a73b,
                0x3c6e_f372_fe94_f82b,
                0xa54f_f53a_5f1d_36f1,
                0x510e_527f_ade6_82d1,
                0x9b05_688c_2b3e_6c1f,
                0x1f83_d9ab_fb41_bd6b,
                0x5be0_cd19_137e_2179,
            ],
            buffer: [0; 128],
            length: 0,
            total: 0,
        }
    }

    /// Adds the data to the digest.
    pub fn update(&mut self, data: &[u8]) {
        self.total += data.len() as u128;

        let mut offset = 0;
        if self.length > 0 {
            let count = (128 - self.length).min(data.len());
            self.buffer[self.length..self.length + count].copy_from_slice(&data[..count]);
            self.length += count;
            offset = count;

            if self.length == 128 {
                let block = self.buffer;
                self.hash_block(&block);
                self.length = 0;
            }
        }

        while offset + 128 <= data.len() {
            let mut block = [0u8; 128];
            block.copy_from_slice(&data[offset..offset + 128]);
            self.hash_block(&block);
            offset += 128;
        }

        if offset < data.len() {
            let count = data.len() - offset;
            self.buffer[..count].copy_from_slice(&data[offset..]);
            self.length = count;
        }
    }

    /// Returns the digest of the data.
    pub fn finish(mut self) -> [u8; 64] {
        let bits = self.total * 8;

        self.update(&[0x80]);
        while self.length != 112 {
            self.update(&[0]);
        }
        self.update(&bits.to_be_bytes());

        let mut result = [0u8; 64];
        for (index, word) in self.state.iter().enumerate() {
            result[index * 8..index * 8 + 8].copy_from_slice(&word.to_be_bytes());
        }
        result
    }

    /// Hashes a block of 128 bytes.
    fn hash_block(&mut self, block: &[u8; 128]) {
        const K: [u64; 80] = [
            0x428a2f98d728ae22,
            0x7137449123ef65cd,
            0xb5c0fbcfec4d3b2f,
            0xe9b5dba58189dbbc,
            0x3956c25bf348b538,
            0x59f111f1b605d019,
            0x923f82a4af194f9b,
            0xab1c5ed5da6d8118,
            0xd807aa98a3030242,
            0x12835b0145706fbe,
            0x243185be4ee4b28c,
            0x550c7dc3d5ffb4e2,
            0x72be5d74f27b896f,
            0x80deb1fe3b1696b1,
            0x9bdc06a725c71235,
            0xc19bf174cf692694,
            0xe49b69c19ef14ad2,
            0xefbe4786384f25e3,
            0x0fc19dc68b8cd5b5,
            0x240ca1cc77ac9c65,
            0x2de92c6f592b0275,
            0x4a7484aa6ea6e483,
            0x5cb0a9dcbd41fbd4,
            0x76f988da831153b5,
            0x983e5152ee66dfab,
            0xa831c66d2db43210,
            0xb00327c898fb213f,
            0xbf597fc7beef0ee4,
            0xc6e00bf33da88fc2,
            0xd5a79147930aa725,
            0x06ca6351e003826f,
            0x142929670a0e6e70,
            0x27b70a8546d22ffc,
            0x2e1b21385c26c926,
            0x4d2c6dfc5ac42aed,
            0x53380d139d95b3df,
            0x650a73548baf63de,
            0x766a0abb3c77b2a8,
            0x81c2c92e47edaee6,
            0x92722c851482353b,
            0xa2bfe8a14cf10364,
            0xa81a664bbc423001,
            0xc24b8b70d0f89791,
            0xc76c51a30654be30,
            0xd192e819d6ef5218,
            0xd69906245565a910,
            0xf40e35855771202a,
            0x106aa07032bbd1b8,
            0x19a4c116b8d2d0c8,
            0x1e376c085141ab53,
            0x2748774cdf8eeb99,
            0x34b0bcb5e19b48a8,
            0x391c0cb3c5c95a63,
            0x4ed8aa4ae3418acb,
            0x5b9cca4f7763e373,
            0x682e6ff3d6b2b8a3,
            0x748f82ee5defb2fc,
            0x78a5636f43172f60,
            0x84c87814a1f0ab72,
            0x8cc702081a6439ec,
            0x90befffa23631e28,
            0xa4506cebde82bde9,
            0xbef9a3f7b2c67915,
            0xc67178f2e372532b,
            0xca273eceea26619c,
            0xd186b8c721c0c207,
            0xeada7dd6cde0eb1e,
            0xf57d4f7fee6ed178,
            0x06f067aa72176fba,
            0x0a637dc5a2c898a6,
            0x113f9804bef90dae,
            0x1b710b35131c471b,
            0x28db77f523047d84,
            0x32caab7b40c72493,
            0x3c9ebe0a15c9bebc,
            0x431d67c49c100d4c,
            0x4cc5d4becb3e42b6,
            0x597f299cfc657e2a,
            0x5fcb6fab3ad6faec,
            0x6c44198c4a475817,
        ];

        let mut words = [0u64; 80];
        for (index, word) in words.iter_mut().enumerate().take(16) {
            let start = index * 8;
            let mut bytes = [0u8; 8];
            bytes.copy_from_slice(&block[start..start + 8]);
            *word = u64::from_be_bytes(bytes);
        }

        for index in 16..80 {
            let s0 = words[index - 15].rotate_right(1)
                ^ words[index - 15].rotate_right(8)
                ^ (words[index - 15] >> 7);
            let s1 = words[index - 2].rotate_right(19)
                ^ words[index - 2].rotate_right(61)
                ^ (words[index - 2] >> 6);
            words[index] = words[index - 16]
                .wrapping_add(s0)
                .wrapping_add(words[index - 7])
                .wrapping_add(s1);
        }

        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = self.state;

        for index in 0..80 {
            let s1 = e.rotate_right(14) ^ e.rotate_right(18) ^ e.rotate_right(41);
            let choose = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(s1)
                .wrapping_add(choose)
                .wrapping_add(K[index])
                .wrapping_add(words[index]);
            let s0 = a.rotate_right(28) ^ a.rotate_right(34) ^ a.rotate_right(39);
            let majority = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(majority);

            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        self.state[0] = self.state[0].wrapping_add(a);
        self.state[1] = self.state[1].wrapping_add(b);
        self.state[2] = self.state[2].wrapping_add(c);
        self.state[3] = self.state[3].wrapping_add(d);
        self.state[4] = self.state[4].wrapping_add(e);
        self.state[5] = self.state[5].wrapping_add(f);
        self.state[6] = self.state[6].wrapping_add(g);
        self.state[7] = self.state[7].wrapping_add(h);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_digests_of_the_empty_data_are_the_ones_of_java() {
        assert_eq!(
            Algorithm::Sha1.digest_to_string(b""),
            "da39a3ee5e6b4b0d3255bfef95601890afd80709"
        );
        assert_eq!(
            Algorithm::Sha256.digest_to_string(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            Algorithm::Sha512.digest_to_string(b""),
            "cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce\
             47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e"
        );
        assert_eq!(
            Algorithm::Md5.digest_to_string(b""),
            "d41d8cd98f00b204e9800998ecf8427e"
        );
    }

    #[test]
    fn the_digests_of_the_text_are_the_ones_of_java() {
        assert_eq!(
            Algorithm::Sha1.digest_to_string(b"abc"),
            "a9993e364706816aba3e25717850c26c9cd0d89d"
        );
        assert_eq!(
            Algorithm::Sha256.digest_to_string(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            Algorithm::Sha512.digest_to_string(b"abc"),
            "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a\
             2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f"
        );
        assert_eq!(
            Algorithm::Md5.digest_to_string(b"abc"),
            "900150983cd24fb0d6963f7d28e17f72"
        );
    }

    #[test]
    fn the_digests_do_not_depend_on_the_parts_of_the_data() {
        let data = b"The quick brown fox jumps over the lazy dog";

        let whole = Algorithm::Sha1.digest_to_string(data);
        let mut digest = Sha1::new();
        digest.update(&data[..10]);
        digest.update(&data[10..30]);
        digest.update(&data[30..]);
        assert_eq!(whole, to_hex(&digest.finish()));

        let whole = Algorithm::Sha256.digest_to_string(data);
        let mut digest = Sha256::new();
        digest.update(&data[..1]);
        digest.update(&data[1..20]);
        digest.update(&data[20..]);
        assert_eq!(whole, to_hex(&digest.finish()));

        let whole = Algorithm::Sha512.digest_to_string(data);
        let mut digest = Sha512::new();
        digest.update(&data[..20]);
        digest.update(&data[20..]);
        assert_eq!(whole, to_hex(&digest.finish()));

        let whole = Algorithm::Md5.digest_to_string(data);
        let mut digest = Md5::new();
        digest.update(&data[..7]);
        digest.update(&data[7..]);
        assert_eq!(whole, to_hex(&digest.finish()));
    }

    #[test]
    fn the_digests_of_long_data_are_the_ones_of_java() {
        // A text of a thousand times the letter "a".
        let data = vec![b'a'; 1_000];

        assert_eq!(
            Algorithm::Sha1.digest_to_string(&data),
            "291e9a6c66994949b57ba5e650361e98fc36b1ba"
        );
        assert_eq!(
            Algorithm::Md5.digest_to_string(&data),
            "cabe45dcc9ae5b66ba86600cca6b8ba8"
        );
    }

    #[test]
    fn the_digests_are_told_apart() {
        assert!(is_sha1_digest("da39a3ee5e6b4b0d3255bfef95601890afd80709"));
        assert!(!is_sha1_digest("da39a3ee5e6b4b0d3255bfef95601890afd8070"));
        assert!(!is_sha1_digest("da39a3ee5e6b4b0d3255bfef95601890afd807zz"));

        assert!(is_sha512_digest(&"a".repeat(128)));
        assert!(!is_sha512_digest(&"a".repeat(127)));
    }

    #[test]
    fn the_digest_of_a_file_is_computed() {
        let path = std::env::temp_dir().join("hmcl-core-digest-test.txt");
        std::fs::write(&path, b"abc").expect("the file should be written");

        let digest = Algorithm::Sha1
            .digest_file_to_string(&path)
            .expect("the digest should be computed");
        assert_eq!(digest, "a9993e364706816aba3e25717850c26c9cd0d89d");

        let _ = std::fs::remove_file(&path);
    }
}
