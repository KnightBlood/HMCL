//! The compression of the data.
//!
//! The launcher reads the files of the game, which are compressed with
//! deflate, and it writes the files of the launcher, which are compressed
//! with the same format.

use std::io::{Read, Write};

use flate2::read::{DeflateDecoder, GzDecoder, ZlibDecoder};
use flate2::write::{DeflateEncoder, GzEncoder, ZlibEncoder};
use flate2::Compression;

use crate::error::Error;

/// Compresses the data with the zlib format.
///
/// The zlib format is the one the files of the game use.
pub fn compress(data: &[u8]) -> Result<Vec<u8>, Error> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data).map_err(Error::io_without_path)?;
    encoder.finish().map_err(Error::io_without_path)
}

/// Compresses the data with the gzip format.
pub fn compress_gzip(data: &[u8]) -> Result<Vec<u8>, Error> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data).map_err(Error::io_without_path)?;
    encoder.finish().map_err(Error::io_without_path)
}

/// Compresses the data with the deflate format, without a header.
pub fn compress_deflate(data: &[u8]) -> Result<Vec<u8>, Error> {
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data).map_err(Error::io_without_path)?;
    encoder.finish().map_err(Error::io_without_path)
}

/// Decompresses the data of the zlib format.
pub fn decompress(data: &[u8]) -> Result<Vec<u8>, Error> {
    decompress_with(data, |reader| Box::new(ZlibDecoder::new(reader)))
}

/// Decompresses the data of the gzip format.
pub fn decompress_gzip(data: &[u8]) -> Result<Vec<u8>, Error> {
    decompress_with(data, |reader| Box::new(GzDecoder::new(reader)))
}

/// Decompresses the data of the deflate format, without a header.
pub fn decompress_deflate(data: &[u8]) -> Result<Vec<u8>, Error> {
    decompress_with(data, |reader| Box::new(DeflateDecoder::new(reader)))
}

/// Decompresses the data with the reader of the format.
fn decompress_with<'a>(
    data: &'a [u8],
    reader: impl FnOnce(&'a [u8]) -> Box<dyn Read + 'a>,
) -> Result<Vec<u8>, Error> {
    let mut decoder = reader(data);
    let mut result = Vec::new();
    decoder
        .read_to_end(&mut result)
        .map_err(Error::io_without_path)?;
    Ok(result)
}

/// Returns whether the compression of the launcher is the same as the
/// compression of the reference implementation.
///
/// Some systems replace the compression of the runtime with another one,
/// such as zlib-ng, which writes the data in another way. The files the
/// launcher writes are checked against the digests the servers give, so the
/// launcher has to know whether the files it writes are the same as the
/// files of the other launchers.
pub fn is_zlib_compatible() -> bool {
    // The compression of five zero bytes, as the reference implementation
    // writes it.
    const EXPECTED: [u8; 11] = [120, 156, 99, 96, 0, 2, 0, 0, 5, 0, 1];

    match compress(&[0; 5]) {
        Ok(compressed) => compressed == EXPECTED,
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The data the tests compress and decompress.
    fn sample() -> Vec<u8> {
        b"The quick brown fox jumps over the lazy dog. \
          The quick brown fox jumps over the lazy dog."
            .to_vec()
    }

    #[test]
    fn the_data_is_compressed_and_decompressed_again() {
        let data = sample();

        let compressed = compress(&data).expect("the data should be compressed");
        assert!(compressed.len() < data.len());
        assert_eq!(
            decompress(&compressed).expect("the data should be decompressed"),
            data
        );

        let compressed = compress_gzip(&data).expect("the data should be compressed");
        assert_eq!(
            decompress_gzip(&compressed).expect("the data should be decompressed"),
            data
        );

        let compressed = compress_deflate(&data).expect("the data should be compressed");
        assert_eq!(
            decompress_deflate(&compressed).expect("the data should be decompressed"),
            data
        );
    }

    #[test]
    fn the_empty_data_is_compressed_and_decompressed_again() {
        let compressed = compress(b"").expect("the data should be compressed");
        assert_eq!(
            decompress(&compressed).expect("the data should be decompressed"),
            Vec::<u8>::new()
        );
    }

    #[test]
    fn the_compression_is_the_one_of_the_reference_implementation() {
        // The compression of five zero bytes, as the reference implementation
        // writes it.
        const EXPECTED: [u8; 11] = [120, 156, 99, 96, 0, 2, 0, 0, 5, 0, 1];

        assert_eq!(
            compress(&[0; 5]).expect("the data should be compressed"),
            EXPECTED
        );
        assert!(is_zlib_compatible());
    }

    #[test]
    fn the_data_that_is_not_compressed_is_refused() {
        assert!(decompress(b"this is not compressed").is_err());
    }
}
