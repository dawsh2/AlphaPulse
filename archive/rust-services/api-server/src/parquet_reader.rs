// Parquet file reader for historical market data
use arrow::array::{Array, Float64Array, Int64Array};
use arrow::record_batch::RecordBatch;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use std::collections::BTreeMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use tracing::{info, warn, error};
use crate::handlers::candles::Candle;

pub struct ParquetReader {
    data_dir: PathBuf,
}

impl ParquetReader {
    pub fn new(data_dir: impl AsRef<Path>) -> Self {
        Self {
            data_dir: data_dir.as_ref().to_path_buf(),
        }
    }

    /// Read historical candles from Parquet files
    pub async fn read_historical_candles(
        &self,
        symbol: &str,
        exchange: &str,
        start_time: Option<i64>,
        end_time: Option<i64>,
    ) -> anyhow::Result<Vec<Candle>> {
        // Convert symbol format (BTC-USD -> BTC_USD for file paths)
        let file_symbol = symbol.replace("-", "_").replace("/", "_");
        
        // Build path to parquet files
        let parquet_dir = self.data_dir
            .join("data")
            .join("parquet")
            .join(exchange)
            .join(&file_symbol);
        
        if !parquet_dir.exists() {
            warn!("Parquet directory does not exist: {:?}", parquet_dir);
            return Ok(Vec::new());
        }
        
        let mut all_candles = BTreeMap::new();
        
        // Read all parquet files in the directory
        let entries = std::fs::read_dir(&parquet_dir)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("parquet") {
                info!("Reading parquet file: {:?}", path);
                
                match self.read_parquet_file(&path, start_time, end_time).await {
                    Ok(candles) => {
                        for candle in candles {
                            // Use BTreeMap to automatically sort by time and deduplicate
                            all_candles.insert(candle.time, candle);
                        }
                    }
                    Err(e) => {
                        error!("Error reading parquet file {:?}: {}", path, e);
                    }
                }
            }
        }
        
        // Convert BTreeMap to Vec (already sorted by time)
        let candles: Vec<Candle> = all_candles.into_values().collect();
        
        info!("Read {} total candles from parquet files for {}/{}", 
              candles.len(), exchange, symbol);
        
        Ok(candles)
    }
    
    /// Read a single parquet file
    async fn read_parquet_file(
        &self,
        path: &Path,
        start_time: Option<i64>,
        end_time: Option<i64>,
    ) -> anyhow::Result<Vec<Candle>> {
        let file = File::open(path)?;
        let builder = ParquetRecordBatchReaderBuilder::try_new(file)?;
        let mut reader = builder.build()?;
        
        let mut candles = Vec::new();
        
        while let Some(batch_result) = reader.next() {
            let batch = batch_result?;
            let batch_candles = self.extract_candles_from_batch(&batch, start_time, end_time)?;
            candles.extend(batch_candles);
        }
        
        Ok(candles)
    }
    
    /// Extract candles from an Arrow RecordBatch
    fn extract_candles_from_batch(
        &self,
        batch: &RecordBatch,
        start_time: Option<i64>,
        end_time: Option<i64>,
    ) -> anyhow::Result<Vec<Candle>> {
        // Get column arrays
        let timestamp_array = batch
            .column_by_name("timestamp")
            .ok_or_else(|| anyhow::anyhow!("Missing timestamp column"))?
            .as_any()
            .downcast_ref::<Int64Array>()
            .ok_or_else(|| anyhow::anyhow!("Invalid timestamp column type"))?;
        
        let open_array = batch
            .column_by_name("open")
            .ok_or_else(|| anyhow::anyhow!("Missing open column"))?
            .as_any()
            .downcast_ref::<Float64Array>()
            .ok_or_else(|| anyhow::anyhow!("Invalid open column type"))?;
        
        let high_array = batch
            .column_by_name("high")
            .ok_or_else(|| anyhow::anyhow!("Missing high column"))?
            .as_any()
            .downcast_ref::<Float64Array>()
            .ok_or_else(|| anyhow::anyhow!("Invalid high column type"))?;
        
        let low_array = batch
            .column_by_name("low")
            .ok_or_else(|| anyhow::anyhow!("Missing low column"))?
            .as_any()
            .downcast_ref::<Float64Array>()
            .ok_or_else(|| anyhow::anyhow!("Invalid low column type"))?;
        
        let close_array = batch
            .column_by_name("close")
            .ok_or_else(|| anyhow::anyhow!("Missing close column"))?
            .as_any()
            .downcast_ref::<Float64Array>()
            .ok_or_else(|| anyhow::anyhow!("Invalid close column type"))?;
        
        let volume_array = batch
            .column_by_name("volume")
            .ok_or_else(|| anyhow::anyhow!("Missing volume column"))?
            .as_any()
            .downcast_ref::<Float64Array>()
            .ok_or_else(|| anyhow::anyhow!("Invalid volume column type"))?;
        
        let mut candles = Vec::new();
        
        for i in 0..batch.num_rows() {
            if timestamp_array.is_null(i) {
                continue;
            }
            
            let timestamp = timestamp_array.value(i);
            
            // Filter by time range if specified
            if let Some(start) = start_time {
                if timestamp < start {
                    continue;
                }
            }
            if let Some(end) = end_time {
                if timestamp > end {
                    continue;
                }
            }
            
            candles.push(Candle {
                time: timestamp,
                open: open_array.value(i),
                high: high_array.value(i),
                low: low_array.value(i),
                close: close_array.value(i),
                volume: volume_array.value(i),
            });
        }
        
        Ok(candles)
    }
}