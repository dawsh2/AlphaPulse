//! Compression Engine
//!
//! Provides compression and decompression capabilities for network transport
//! using LZ4, Zstd, and Snappy algorithms optimized for different use cases.

use crate::{TransportError, Result};
use serde::{Deserialize, Serialize};

/// Compression type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompressionType {
    /// No compression - lowest latency
    None,
    /// LZ4 - fast compression/decompression
    #[cfg(feature = "compression")]
    Lz4,
    /// Zstd - better compression ratio
    #[cfg(feature = "compression")]
    Zstd,
    /// Snappy - Google's fast compression
    #[cfg(feature = "compression")]
    Snappy,
}

/// Compression engine for bandwidth optimization
pub struct CompressionEngine {
    compression_type: CompressionType,
    #[cfg(feature = "compression")]
    zstd_level: i32,
}

impl CompressionEngine {
    /// Create new compression engine
    pub fn new(compression_type: CompressionType) -> Self {
        Self {
            compression_type,
            #[cfg(feature = "compression")]
            zstd_level: 3, // Default compression level for Zstd
        }
    }

    /// Create engine with custom Zstd compression level
    #[cfg(feature = "compression")]
    pub fn with_zstd_level(compression_type: CompressionType, level: i32) -> Self {
        Self {
            compression_type,
            zstd_level: level,
        }
    }

    /// Compress data using configured algorithm
    pub fn compress(&self, data: &[u8]) -> Result<Vec<u8>> {
        if data.is_empty() {
            return Ok(Vec::new());
        }

        match self.compression_type {
            CompressionType::None => Ok(data.to_vec()),
            
            #[cfg(feature = "compression")]
            CompressionType::Lz4 => {
                lz4::block::compress(data, None, true)
                    .map_err(|e| TransportError::compression("lz4", format!("Compression failed: {}", e)))
            }
            
            #[cfg(feature = "compression")]
            CompressionType::Zstd => {
                zstd::bulk::compress(data, self.zstd_level)
                    .map_err(|e| TransportError::compression("zstd", format!("Compression failed: {}", e)))
            }
            
            #[cfg(feature = "compression")]
            CompressionType::Snappy => {
                let mut encoder = snap::raw::Encoder::new();
                encoder.compress_vec(data)
                    .map_err(|e| TransportError::compression("snappy", format!("Compression failed: {}", e)))
            }

            #[cfg(not(feature = "compression"))]
            _ => Err(TransportError::configuration(
                "Compression feature not enabled",
                Some("compression")
            )),
        }
    }

    /// Decompress data using configured algorithm
    pub fn decompress(&self, data: &[u8]) -> Result<Vec<u8>> {
        if data.is_empty() {
            return Ok(Vec::new());
        }

        match self.compression_type {
            CompressionType::None => Ok(data.to_vec()),
            
            #[cfg(feature = "compression")]
            CompressionType::Lz4 => {
                lz4::block::decompress(data, None)
                    .map_err(|e| TransportError::compression("lz4", format!("Decompression failed: {}", e)))
            }
            
            #[cfg(feature = "compression")]
            CompressionType::Zstd => {
                // Use reasonable max size to prevent memory exhaustion
                const MAX_DECOMPRESSED_SIZE: usize = 64 * 1024 * 1024; // 64MB
                zstd::bulk::decompress(data, MAX_DECOMPRESSED_SIZE)
                    .map_err(|e| TransportError::compression("zstd", format!("Decompression failed: {}", e)))
            }
            
            #[cfg(feature = "compression")]
            CompressionType::Snappy => {
                let mut decoder = snap::raw::Decoder::new();
                decoder.decompress_vec(data)
                    .map_err(|e| TransportError::compression("snappy", format!("Decompression failed: {}", e)))
            }

            #[cfg(not(feature = "compression"))]
            _ => Err(TransportError::configuration(
                "Compression feature not enabled", 
                Some("compression")
            )),
        }
    }

    /// Get compression ratio for given data (for metrics)
    pub fn compression_ratio(&self, original_size: usize, compressed_size: usize) -> f64 {
        if original_size == 0 {
            return 1.0;
        }
        compressed_size as f64 / original_size as f64
    }

    /// Get compression type
    pub fn compression_type(&self) -> CompressionType {
        self.compression_type
    }

    /// Check if compression is enabled
    pub fn is_enabled(&self) -> bool {
        !matches!(self.compression_type, CompressionType::None)
    }

    /// Estimate compressed size without actually compressing
    pub fn estimate_compressed_size(&self, original_size: usize) -> usize {
        match self.compression_type {
            CompressionType::None => original_size,
            #[cfg(feature = "compression")]
            CompressionType::Lz4 => {
                // LZ4 typically achieves ~50-70% compression
                (original_size as f64 * 0.6) as usize
            }
            #[cfg(feature = "compression")]
            CompressionType::Zstd => {
                // Zstd typically achieves ~40-60% compression
                (original_size as f64 * 0.5) as usize
            }
            #[cfg(feature = "compression")]
            CompressionType::Snappy => {
                // Snappy typically achieves ~60-80% compression
                (original_size as f64 * 0.7) as usize
            }
            #[cfg(not(feature = "compression"))]
            _ => original_size,
        }
    }

    /// Get compression algorithm info
    pub fn algorithm_info(&self) -> CompressionInfo {
        match self.compression_type {
            CompressionType::None => CompressionInfo {
                name: "none",
                speed: CompressionSpeed::Fastest,
                ratio: CompressionRatio::Lowest,
                description: "No compression",
            },
            #[cfg(feature = "compression")]
            CompressionType::Lz4 => CompressionInfo {
                name: "lz4",
                speed: CompressionSpeed::VeryFast,
                ratio: CompressionRatio::Low,
                description: "LZ4 - extremely fast compression",
            },
            #[cfg(feature = "compression")]
            CompressionType::Zstd => CompressionInfo {
                name: "zstd",
                speed: CompressionSpeed::Fast,
                ratio: CompressionRatio::High,
                description: "Zstandard - high compression ratio",
            },
            #[cfg(feature = "compression")]
            CompressionType::Snappy => CompressionInfo {
                name: "snappy",
                speed: CompressionSpeed::VeryFast,
                ratio: CompressionRatio::Medium,
                description: "Snappy - fast compression by Google",
            },
            #[cfg(not(feature = "compression"))]
            _ => CompressionInfo {
                name: "disabled",
                speed: CompressionSpeed::Fastest,
                ratio: CompressionRatio::Lowest,
                description: "Compression feature disabled",
            },
        }
    }
}

/// Compression algorithm information
#[derive(Debug, Clone)]
pub struct CompressionInfo {
    pub name: &'static str,
    pub speed: CompressionSpeed,
    pub ratio: CompressionRatio,
    pub description: &'static str,
}

/// Compression speed characteristics
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CompressionSpeed {
    Fastest,   // No compression
    VeryFast,  // LZ4, Snappy
    Fast,      // Zstd level 1-3
    Medium,    // Zstd level 4-6
    Slow,      // Zstd level 7-12
    Slowest,   // Zstd level 13-22
}

/// Compression ratio characteristics
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CompressionRatio {
    Lowest,   // No compression
    Low,      // LZ4
    Medium,   // Snappy
    High,     // Zstd
    Highest,  // Zstd max level
}

/// Adaptive compression that selects algorithm based on data characteristics
pub struct AdaptiveCompression {
    engines: Vec<CompressionEngine>,
    sample_size: usize,
    compression_threshold: f64,
}

impl AdaptiveCompression {
    /// Create adaptive compression engine
    pub fn new() -> Self {
        let mut engines = vec![CompressionEngine::new(CompressionType::None)];
        
        #[cfg(feature = "compression")]
        {
            engines.push(CompressionEngine::new(CompressionType::Lz4));
            engines.push(CompressionEngine::new(CompressionType::Snappy));
            engines.push(CompressionEngine::new(CompressionType::Zstd));
        }

        Self {
            engines,
            sample_size: 1024, // Sample first 1KB to choose algorithm
            compression_threshold: 0.9, // Only compress if ratio < 90%
        }
    }

    /// Select best compression algorithm for given data
    pub fn select_algorithm(&self, data: &[u8]) -> CompressionType {
        if data.len() < 64 {
            // Don't compress very small messages
            return CompressionType::None;
        }

        // Sample data to test compression effectiveness
        let sample_size = std::cmp::min(self.sample_size, data.len());
        let sample = &data[..sample_size];

        #[cfg(feature = "compression")]
        {
            // Test LZ4 first (fastest)
            if let Ok(compressed) = self.engines[1].compress(sample) {
                let ratio = compressed.len() as f64 / sample.len() as f64;
                if ratio < self.compression_threshold {
                    return CompressionType::Lz4;
                }
            }
        }

        CompressionType::None
    }

    /// Compress with automatic algorithm selection
    pub fn auto_compress(&self, data: &[u8]) -> Result<(Vec<u8>, CompressionType)> {
        let algorithm = self.select_algorithm(data);
        let engine = self.engines.iter()
            .find(|e| e.compression_type() == algorithm)
            .unwrap();
        
        let compressed = engine.compress(data)?;
        Ok((compressed, algorithm))
    }
}

impl Default for AdaptiveCompression {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_compression() {
        let engine = CompressionEngine::new(CompressionType::None);
        let data = b"hello world";
        
        let compressed = engine.compress(data).unwrap();
        let decompressed = engine.decompress(&compressed).unwrap();
        
        assert_eq!(data, decompressed.as_slice());
        assert_eq!(compressed, data);
    }

    #[cfg(feature = "compression")]
    #[test]
    fn test_lz4_compression() {
        let engine = CompressionEngine::new(CompressionType::Lz4);
        let data = b"hello world ".repeat(100); // Repetitive data compresses well
        
        let compressed = engine.compress(&data).unwrap();
        let decompressed = engine.decompress(&compressed).unwrap();
        
        assert_eq!(data, decompressed);
        assert!(compressed.len() < data.len()); // Should be compressed
    }

    #[cfg(feature = "compression")]
    #[test]
    fn test_zstd_compression() {
        let engine = CompressionEngine::new(CompressionType::Zstd);
        let data = b"The quick brown fox jumps over the lazy dog ".repeat(50);
        
        let compressed = engine.compress(&data).unwrap();
        let decompressed = engine.decompress(&compressed).unwrap();
        
        assert_eq!(data, decompressed);
        assert!(compressed.len() < data.len());
    }

    #[cfg(feature = "compression")]
    #[test]
    fn test_snappy_compression() {
        let engine = CompressionEngine::new(CompressionType::Snappy);
        let data = b"Lorem ipsum dolor sit amet ".repeat(20);
        
        let compressed = engine.compress(&data).unwrap();
        let decompressed = engine.decompress(&compressed).unwrap();
        
        assert_eq!(data, decompressed);
        assert!(compressed.len() < data.len());
    }

    #[test]
    fn test_empty_data() {
        let engine = CompressionEngine::new(CompressionType::None);
        let empty: &[u8] = &[];
        
        let compressed = engine.compress(empty).unwrap();
        let decompressed = engine.decompress(&compressed).unwrap();
        
        assert!(compressed.is_empty());
        assert!(decompressed.is_empty());
    }

    #[test]
    fn test_compression_ratio() {
        let engine = CompressionEngine::new(CompressionType::None);
        
        assert_eq!(engine.compression_ratio(100, 50), 0.5);
        assert_eq!(engine.compression_ratio(0, 0), 1.0);
        assert_eq!(engine.compression_ratio(100, 150), 1.5);
    }

    #[test]
    fn test_algorithm_info() {
        let engine = CompressionEngine::new(CompressionType::None);
        let info = engine.algorithm_info();
        
        assert_eq!(info.name, "none");
        assert_eq!(info.speed, CompressionSpeed::Fastest);
        assert_eq!(info.ratio, CompressionRatio::Lowest);
    }

    #[test]
    fn test_adaptive_compression() {
        let adaptive = AdaptiveCompression::new();
        
        // Small data should not be compressed
        let small_data = b"hello";
        assert_eq!(adaptive.select_algorithm(small_data), CompressionType::None);
        
        // Large repetitive data should be compressed (if compression enabled)
        let large_data = b"repeat ".repeat(200);
        let algorithm = adaptive.select_algorithm(&large_data);
        
        #[cfg(feature = "compression")]
        assert_ne!(algorithm, CompressionType::None);
        
        #[cfg(not(feature = "compression"))]
        assert_eq!(algorithm, CompressionType::None);
    }

    #[test]
    fn test_size_estimation() {
        let engine = CompressionEngine::new(CompressionType::None);
        assert_eq!(engine.estimate_compressed_size(1000), 1000);
        
        #[cfg(feature = "compression")]
        {
            let lz4_engine = CompressionEngine::new(CompressionType::Lz4);
            assert!(lz4_engine.estimate_compressed_size(1000) < 1000);
        }
    }
}