//! Per-message deflate compression for WebSocket frames.
//!
//! Wraps `flate2` to provide a simple compress / decompress API that can be
//! wired into the WebSocket message pipeline.  When `enabled` is `false` or
//! the payload is smaller than `min_size`, the data is returned unchanged.

use flate2::Compression;
use flate2::read::DeflateDecoder;
use flate2::write::DeflateEncoder;
use server_core::Error;
use std::io::{Read, Write};

/// Compressor / decompressor for WebSocket per-message deflate (RFC 7692).
pub struct MessageCompressor {
    /// When `false`, `compress` is a no-op and returns the input unchanged.
    pub enabled: bool,
    /// `flate2` compression level 0–9 (0 = store, 1 = fast, 9 = best).
    pub compression_level: u32,
    /// Minimum payload size in bytes below which compression is skipped.
    pub min_size: usize,
}

impl MessageCompressor {
    pub fn new(enabled: bool, level: u32, min_size: usize) -> Self {
        Self {
            enabled,
            compression_level: level.min(9),
            min_size,
        }
    }

    /// Returns `true` when the payload should be compressed based on the
    /// current configuration and payload length.
    pub fn should_compress(&self, data: &[u8]) -> bool {
        self.enabled && data.len() >= self.min_size
    }

    /// Deflate-compress `data`.  Returns the compressed bytes, or the
    /// original bytes when compression is disabled or below `min_size`.
    pub fn compress(&self, data: &[u8]) -> Vec<u8> {
        if !self.should_compress(data) {
            return data.to_vec();
        }
        let level = Compression::new(self.compression_level);
        let mut encoder = DeflateEncoder::new(Vec::new(), level);
        // Encoding only fails on I/O errors; writing to a Vec never fails.
        encoder.write_all(data).unwrap_or_default();
        encoder.finish().unwrap_or_else(|_| data.to_vec())
    }

    /// Deflate-decompress `data`.  Returns an error if the data is malformed.
    pub fn decompress(&self, data: &[u8]) -> server_core::Result<Vec<u8>> {
        let mut decoder = DeflateDecoder::new(data);
        let mut out = Vec::new();
        decoder
            .read_to_end(&mut out)
            .map_err(|e| Error::Transport(format!("deflate decompress: {e}")))?;
        Ok(out)
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(payload: &[u8]) -> Vec<u8> {
        let c = MessageCompressor::new(true, 6, 0);
        let compressed = c.compress(payload);
        c.decompress(&compressed).expect("decompress must succeed")
    }

    #[test]
    fn test_compress_decompress_roundtrip() {
        let data = b"hello world hello world hello world";
        let recovered = roundtrip(data);
        assert_eq!(recovered, data);
    }

    #[test]
    fn test_compression_actually_shrinks_large_payload() {
        // A highly compressible 1 KiB payload.
        let data = vec![b'a'; 1024];
        let c = MessageCompressor::new(true, 6, 0);
        let compressed = c.compress(&data);
        assert!(
            compressed.len() < data.len(),
            "compressed ({} bytes) should be smaller than original ({} bytes)",
            compressed.len(),
            data.len()
        );
    }

    #[test]
    fn test_disabled_compressor_is_passthrough() {
        let c = MessageCompressor::new(false, 6, 0);
        let data = b"unchanged payload";
        assert_eq!(c.compress(data), data);
    }

    #[test]
    fn test_min_size_skips_small_payload() {
        let c = MessageCompressor::new(true, 6, 1024);
        let data = b"short";
        // Payload shorter than min_size — should be returned as-is.
        assert_eq!(c.compress(data), data);
        assert!(!c.should_compress(data));
    }

    #[test]
    fn test_decompress_invalid_data_returns_error() {
        let c = MessageCompressor::new(true, 6, 0);
        let result = c.decompress(b"this is not deflate data \xff\xfe");
        assert!(result.is_err(), "invalid deflate data must return Err");
    }
}
