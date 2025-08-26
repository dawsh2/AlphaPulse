//! Market Data Capture Infrastructure
//!
//! Captures live market data for later replay in tests. This enables
//! deterministic testing with real market conditions.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Serialize, Deserialize};
use tokio::sync::mpsc;

/// Configuration for market data capture
#[derive(Debug, Clone)]
pub struct CaptureConfig {
    pub output_directory: PathBuf,
    pub max_file_size_mb: u64,
    pub compression_enabled: bool,
    pub capture_duration_secs: Option<u64>,
    pub exchanges: Vec<String>,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            output_directory: PathBuf::from("./captured_data"),
            max_file_size_mb: 100, // 100MB per file
            compression_enabled: true,
            capture_duration_secs: None, // Capture indefinitely
            exchanges: vec!["polygon".to_string(), "binance".to_string()],
        }
    }
}

/// Captured market event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapturedEvent {
    pub timestamp_ns: u64,
    pub exchange: String,
    pub event_type: String,
    pub raw_data: Vec<u8>,
    pub parsed_data: Option<serde_json::Value>,
}

/// Market data capture engine
pub struct MarketDataCapture {
    config: CaptureConfig,
    current_file: Option<BufWriter<File>>,
    current_file_size: u64,
    current_file_path: Option<PathBuf>,
    event_count: u64,
}

impl MarketDataCapture {
    pub fn new(config: CaptureConfig) -> Result<Self, Box<dyn std::error::Error>> {
        // Create output directory if it doesn't exist
        std::fs::create_dir_all(&config.output_directory)?;
        
        Ok(Self {
            config,
            current_file: None,
            current_file_size: 0,
            current_file_path: None,
            event_count: 0,
        })
    }
    
    pub async fn start_capture(&mut self) -> Result<mpsc::Receiver<CapturedEvent>, Box<dyn std::error::Error>> {
        let (tx, rx) = mpsc::channel(1000);
        
        // Start capture for each configured exchange
        for exchange in &self.config.exchanges {
            self.start_exchange_capture(exchange.clone(), tx.clone()).await?;
        }
        
        Ok(rx)
    }
    
    async fn start_exchange_capture(
        &self,
        exchange: String,
        event_sender: mpsc::Sender<CapturedEvent>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match exchange.as_str() {
            "polygon" => {
                self.start_polygon_capture(event_sender).await?;
            }
            "binance" => {
                self.start_binance_capture(event_sender).await?;
            }
            _ => {
                return Err(format!("Unsupported exchange: {}", exchange).into());
            }
        }
        
        Ok(())
    }
    
    async fn start_polygon_capture(
        &self,
        event_sender: mpsc::Sender<CapturedEvent>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Connect to Polygon websocket and capture events
        tokio::spawn(async move {
            // This would connect to the actual Polygon WebSocket
            // For now, we'll simulate with test data
            
            loop {
                // Simulate receiving a pool swap event
                let event = CapturedEvent {
                    timestamp_ns: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_nanos() as u64,
                    exchange: "polygon".to_string(),
                    event_type: "pool_swap".to_string(),
                    raw_data: vec![1, 2, 3, 4], // Mock raw event data
                    parsed_data: Some(serde_json::json!({
                        "pool": "0x8ad599c3A0ff1De082011EFDDc58f1908eb6e6D8",
                        "token0_amount": "1000000000000000000",
                        "token1_amount": "2000000000",
                        "block_number": 19000000
                    })),
                };
                
                if event_sender.send(event).await.is_err() {
                    break; // Receiver dropped
                }
                
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        });
        
        Ok(())
    }
    
    async fn start_binance_capture(
        &self,
        event_sender: mpsc::Sender<CapturedEvent>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Similar implementation for Binance
        tokio::spawn(async move {
            loop {
                let event = CapturedEvent {
                    timestamp_ns: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_nanos() as u64,
                    exchange: "binance".to_string(),
                    event_type: "trade".to_string(),
                    raw_data: vec![5, 6, 7, 8],
                    parsed_data: Some(serde_json::json!({
                        "symbol": "BTCUSDT",
                        "price": "45000.00",
                        "quantity": "0.1",
                        "timestamp": 1234567890
                    })),
                };
                
                if event_sender.send(event).await.is_err() {
                    break;
                }
                
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }
        });
        
        Ok(())
    }
    
    pub async fn capture_to_file(&mut self, event: CapturedEvent) -> Result<(), Box<dyn std::error::Error>> {
        // Check if we need to rotate to a new file
        if self.should_rotate_file()? {
            self.rotate_file()?;
        }
        
        // Ensure we have a file open
        if self.current_file.is_none() {
            self.create_new_file()?;
        }
        
        // Serialize event
        let serialized = if self.config.compression_enabled {
            // Use compression (mock implementation)
            serde_json::to_vec(&event)?
        } else {
            serde_json::to_vec(&event)?
        };
        
        // Write to file
        if let Some(file) = &mut self.current_file {
            file.write_all(&serialized)?;
            file.write_all(b"\n")?; // Line delimiter
            self.current_file_size += serialized.len() as u64 + 1;
        }
        
        self.event_count += 1;
        
        Ok(())
    }
    
    fn should_rotate_file(&self) -> Result<bool, Box<dyn std::error::Error>> {
        if self.current_file.is_none() {
            return Ok(true);
        }
        
        let max_size_bytes = self.config.max_file_size_mb * 1024 * 1024;
        Ok(self.current_file_size >= max_size_bytes)
    }
    
    fn rotate_file(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Close current file
        if let Some(mut file) = self.current_file.take() {
            file.flush()?;
        }
        
        println!("Rotated capture file. Events captured: {}", self.event_count);
        
        // Reset for new file
        self.current_file_size = 0;
        self.create_new_file()?;
        
        Ok(())
    }
    
    fn create_new_file(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let filename = format!("market_data_capture_{}.jsonl", timestamp);
        let filepath = self.config.output_directory.join(&filename);
        
        let file = File::create(&filepath)?;
        let writer = BufWriter::new(file);
        
        self.current_file = Some(writer);
        self.current_file_path = Some(filepath);
        
        println!("Created new capture file: {}", filename);
        
        Ok(())
    }
    
    pub fn get_capture_stats(&self) -> CaptureStats {
        CaptureStats {
            events_captured: self.event_count,
            current_file_size_bytes: self.current_file_size,
            current_file_path: self.current_file_path.clone(),
        }
    }
}

impl Drop for MarketDataCapture {
    fn drop(&mut self) {
        if let Some(mut file) = self.current_file.take() {
            let _ = file.flush();
        }
        
        println!("Market data capture stopped. Total events: {}", self.event_count);
    }
}

/// Capture statistics
#[derive(Debug, Clone)]
pub struct CaptureStats {
    pub events_captured: u64,
    pub current_file_size_bytes: u64,
    pub current_file_path: Option<PathBuf>,
}

/// Utility functions for capture management
pub struct CaptureManager;

impl CaptureManager {
    /// Start a capture session with the given configuration
    pub async fn start_capture_session(config: CaptureConfig) -> Result<(), Box<dyn std::error::Error>> {
        let mut capture = MarketDataCapture::new(config)?;
        let mut event_receiver = capture.start_capture().await?;
        
        // Process captured events
        while let Some(event) = event_receiver.recv().await {
            capture.capture_to_file(event).await?;
            
            // Print periodic stats
            if capture.event_count % 1000 == 0 {
                let stats = capture.get_capture_stats();
                println!("Capture stats: {} events, {:.2} MB", 
                        stats.events_captured, 
                        stats.current_file_size_bytes as f64 / (1024.0 * 1024.0));
            }
        }
        
        Ok(())
    }
    
    /// List available captured data files
    pub fn list_capture_files(directory: &Path) -> Result<Vec<CaptureFileInfo>, Box<dyn std::error::Error>> {
        let mut files = Vec::new();
        
        for entry in std::fs::read_dir(directory)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().map_or(false, |ext| ext == "jsonl") {
                let metadata = entry.metadata()?;
                let filename = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                
                files.push(CaptureFileInfo {
                    path: path.clone(),
                    filename,
                    size_bytes: metadata.len(),
                    modified_time: metadata.modified()?,
                });
            }
        }
        
        // Sort by modification time (newest first)
        files.sort_by(|a, b| b.modified_time.cmp(&a.modified_time));
        
        Ok(files)
    }
}

/// Information about a captured data file
#[derive(Debug, Clone)]
pub struct CaptureFileInfo {
    pub path: PathBuf,
    pub filename: String,
    pub size_bytes: u64,
    pub modified_time: SystemTime,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_capture_configuration() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        
        let config = CaptureConfig {
            output_directory: temp_dir.path().to_path_buf(),
            max_file_size_mb: 1, // Small for testing
            compression_enabled: false,
            capture_duration_secs: Some(1),
            exchanges: vec!["polygon".to_string()],
        };
        
        let mut capture = MarketDataCapture::new(config).expect("Failed to create capture");
        
        // Test capturing a few events
        let test_event = CapturedEvent {
            timestamp_ns: 1234567890123456789,
            exchange: "polygon".to_string(),
            event_type: "test".to_string(),
            raw_data: vec![1, 2, 3, 4],
            parsed_data: Some(serde_json::json!({"test": true})),
        };
        
        capture.capture_to_file(test_event.clone()).await.expect("Failed to capture event");
        capture.capture_to_file(test_event).await.expect("Failed to capture second event");
        
        let stats = capture.get_capture_stats();
        assert_eq!(stats.events_captured, 2);
        assert!(stats.current_file_size_bytes > 0);
        assert!(stats.current_file_path.is_some());
    }
    
    #[test]
    fn test_capture_file_listing() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        
        // Create some test files
        let test_file1 = temp_dir.path().join("test1.jsonl");
        let test_file2 = temp_dir.path().join("test2.jsonl");
        
        std::fs::write(&test_file1, "test data 1").expect("Failed to write test file");
        std::fs::write(&test_file2, "test data 2").expect("Failed to write test file");
        
        let files = CaptureManager::list_capture_files(temp_dir.path())
            .expect("Failed to list files");
        
        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|f| f.filename == "test1.jsonl"));
        assert!(files.iter().any(|f| f.filename == "test2.jsonl"));
    }
}