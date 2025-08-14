// Metrics collection for monitoring
use metrics::{counter, gauge, histogram};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

pub struct MetricsCollector {
    trades_processed: AtomicU64,
    messages_received: AtomicU64,
    errors_count: AtomicU64,
    buffer_size: AtomicUsize,
    last_update: AtomicU64,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            trades_processed: AtomicU64::new(0),
            messages_received: AtomicU64::new(0),
            errors_count: AtomicU64::new(0),
            buffer_size: AtomicUsize::new(0),
            last_update: AtomicU64::new(0),
        }
    }
    
    pub fn record_trade(&self, exchange: &str, symbol: &str) {
        self.trades_processed.fetch_add(1, Ordering::Relaxed);
        counter!("trades_processed", "exchange" => exchange.to_string(), "symbol" => symbol.to_string()).increment(1);
    }
    
    pub fn record_message(&self, exchange: &str, msg_type: &str) {
        self.messages_received.fetch_add(1, Ordering::Relaxed);
        counter!("messages_received", "exchange" => exchange.to_string(), "type" => msg_type.to_string()).increment(1);
    }
    
    pub fn record_error(&self, exchange: &str, error_type: &str) {
        self.errors_count.fetch_add(1, Ordering::Relaxed);
        counter!("errors", "exchange" => exchange.to_string(), "type" => error_type.to_string()).increment(1);
    }
    
    pub fn record_buffer_size(&self, size: usize, buffer_name: &str) {
        self.buffer_size.store(size, Ordering::Relaxed);
        gauge!("buffer_size", "buffer" => buffer_name.to_string()).set(size as f64);
    }
    
    pub fn record_buffer_overflow(&self, buffer_name: &str) {
        counter!("buffer_overflow", "buffer" => buffer_name.to_string()).increment(1);
    }
    
    pub fn record_latency(&self, latency_ms: f64, operation: &str) {
        histogram!("operation_latency_ms", "operation" => operation.to_string()).record(latency_ms);
    }
    
    pub fn record_websocket_status(&self, exchange: &str, connected: bool) {
        let status = if connected { 1.0 } else { 0.0 };
        gauge!("websocket_connected", "exchange" => exchange.to_string()).set(status);
    }
    
    // Alias for compatibility
    pub fn record_websocket_connection_status(&self, exchange: &str, connected: bool) {
        self.record_websocket_status(exchange, connected);
    }
    
    pub fn record_websocket_reconnection(&self, exchange: &str) {
        counter!("websocket_reconnections", "exchange" => exchange.to_string()).increment(1);
    }
    
    pub fn record_trade_processed(&self, exchange: &str, symbol: &str) {
        self.record_trade(exchange, symbol);
    }
    
    pub fn record_websocket_message(&self, exchange: &str, msg_type: &str) {
        self.record_message(exchange, msg_type);
    }
    
    pub fn record_http_request(&self, endpoint: &str, status: u16) {
        counter!("http_requests", "endpoint" => endpoint.to_string(), "status" => status.to_string()).increment(1);
    }
    
    pub fn record_http_latency(&self, latency_ms: f64, endpoint: &str) {
        histogram!("http_latency_ms", "endpoint" => endpoint.to_string()).record(latency_ms);
    }
    
    pub fn record_redis_operation(&self, operation: &str, success: bool) {
        let status = if success { "success" } else { "failure" };
        counter!("redis_operations", "operation" => operation.to_string(), "status" => status.to_string()).increment(1);
    }
    
    pub fn record_redis_latency(&self, latency_ms: f64, operation: &str) {
        histogram!("redis_latency_ms", "operation" => operation.to_string()).record(latency_ms);
    }
    
    pub fn record_batch_size(&self, size: usize, batch_type: &str) {
        histogram!("batch_size", "type" => batch_type.to_string()).record(size as f64);
    }
    
    pub fn record_uptime(&self) {
        gauge!("uptime_seconds").set(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as f64
        );
    }
    
    pub fn record_memory_usage(&self, bytes: u64) {
        gauge!("memory_usage_bytes").set(bytes as f64);
    }
    
    pub fn get_stats(&self) -> (u64, u64, u64) {
        (
            self.trades_processed.load(Ordering::Relaxed),
            self.messages_received.load(Ordering::Relaxed),
            self.errors_count.load(Ordering::Relaxed),
        )
    }
}