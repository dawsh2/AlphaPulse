// Metrics collection for AlphaPulse services
use metrics::{counter, gauge, histogram};
use std::time::Instant;

pub struct MetricsCollector {
    start_time: Instant,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
        }
    }

    // Trade processing metrics
    pub fn record_trade_processed(&self, exchange: &str, symbol: &str) {
        counter!("trades_processed_total", "exchange" => exchange.to_string(), "symbol" => symbol.to_string()).increment(1);
    }

    pub fn record_processing_latency(&self, latency_ms: f64, exchange: &str) {
        histogram!("processing_latency_ms", "exchange" => exchange.to_string()).record(latency_ms);
    }

    pub fn record_batch_size(&self, size: usize, exchange: &str) {
        histogram!("batch_size", "exchange" => exchange.to_string()).record(size as f64);
    }

    // Redis metrics
    pub fn record_redis_operation(&self, operation: &str, success: bool) {
        let status = if success { "success".to_string() } else { "error".to_string() };
        counter!("redis_operations_total", "operation" => operation.to_string(), "status" => status).increment(1);
    }

    pub fn record_redis_latency(&self, latency_ms: f64, operation: &str) {
        histogram!("redis_operation_latency_ms", "operation" => operation.to_string()).record(latency_ms);
    }

    // WebSocket metrics
    pub fn record_websocket_message(&self, exchange: &str, message_type: &str) {
        counter!("websocket_messages_total", "exchange" => exchange.to_string(), "type" => message_type.to_string()).increment(1);
    }

    pub fn record_websocket_connection_status(&self, exchange: &str, connected: bool) {
        let status = if connected { 1.0 } else { 0.0 };
        gauge!("websocket_connected", "exchange" => exchange.to_string()).set(status);
    }

    pub fn record_websocket_reconnection(&self, exchange: &str) {
        counter!("websocket_reconnections_total", "exchange" => exchange.to_string()).increment(1);
    }

    // Memory and performance metrics
    pub fn record_memory_usage(&self, bytes: u64) {
        gauge!("memory_usage_bytes").set(bytes as f64);
    }

    pub fn record_cpu_usage(&self, percentage: f64) {
        gauge!("cpu_usage_percent").set(percentage);
    }

    pub fn record_uptime(&self) {
        let uptime_seconds = self.start_time.elapsed().as_secs() as f64;
        gauge!("uptime_seconds").set(uptime_seconds);
    }

    // Buffer metrics
    pub fn record_buffer_size(&self, size: usize, buffer_type: &str) {
        gauge!("buffer_size", "type" => buffer_type.to_string()).set(size as f64);
    }

    pub fn record_buffer_overflow(&self, buffer_type: &str) {
        counter!("buffer_overflows_total", "type" => buffer_type.to_string()).increment(1);
    }

    // HTTP API metrics
    pub fn record_http_request(&self, method: &str, path: &str, status_code: u16) {
        counter!("http_requests_total", 
               "method" => method.to_string(), 
               "path" => path.to_string(), 
               "status" => status_code.to_string())
            .increment(1);
    }

    pub fn record_http_latency(&self, latency_ms: f64, method: &str, path: &str) {
        histogram!("http_request_duration_ms", "method" => method.to_string(), "path" => path.to_string())
            .record(latency_ms);
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}