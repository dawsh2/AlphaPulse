//! TraceCollector - Simple debug version for testing
//! Standalone trace event collector for debugging message flow

use anyhow::Result;
use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::UNIX_EPOCH;
use network::time::safe_system_timestamp_ns;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, UnixListener, UnixStream};
use tracing::{debug, error, info, warn};

// Simple trace event structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEvent {
    pub trace_id: String,
    pub service: String,
    pub event_type: String,
    pub timestamp_ns: u64,
    pub duration_ns: Option<u64>,
    pub metadata: HashMap<String, String>,
}

// Simple statistics
#[derive(Debug, Clone, Serialize)]
pub struct TraceStats {
    pub events_processed: u64,
    pub active_traces: usize,
    pub services_seen: Vec<String>,
    pub events_per_second: f64,
    pub uptime_seconds: u64,
}

// Simple trace collector
pub struct SimpleTraceCollector {
    traces: Arc<DashMap<String, Vec<TraceEvent>>>,
    stats: Arc<RwLock<TraceStats>>,
    start_time: u64, // Store as nanoseconds since epoch
}

impl SimpleTraceCollector {
    pub fn new() -> Self {
        Self {
            traces: Arc::new(DashMap::new()),
            stats: Arc::new(RwLock::new(TraceStats {
                events_processed: 0,
                active_traces: 0,
                services_seen: Vec::new(),
                events_per_second: 0.0,
                uptime_seconds: 0,
            })),
            start_time: safe_system_timestamp_ns(),
        }
    }

    pub async fn start(&self) -> Result<()> {
        info!("🚀 Starting SimpleTraceCollector");

        // Create socket directory
        std::fs::create_dir_all("/tmp/torq").ok();

        // Remove existing socket
        let socket_path = "/tmp/torq/trace_collector.sock";
        if std::path::Path::new(socket_path).exists() {
            std::fs::remove_file(socket_path)?;
        }

        // Start Unix socket listener
        let listener = UnixListener::bind(socket_path)?;
        info!("📊 TraceCollector listening on {}", socket_path);

        // Start API server
        let _api_task = {
            let traces = self.traces.clone();
            let stats = self.stats.clone();
            tokio::spawn(async move {
                if let Err(e) = start_api_server(traces, stats).await {
                    error!("API server error: {}", e);
                }
            })
        };

        // Start statistics update task
        let _stats_task = {
            let traces = self.traces.clone();
            let stats = self.stats.clone();
            let start_time = self.start_time;
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
                loop {
                    interval.tick().await;
                    update_stats(&traces, &stats, start_time).await;
                }
            })
        };

        // Accept connections
        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    let traces = self.traces.clone();
                    let stats = self.stats.clone();

                    tokio::spawn(async move {
                        if let Err(e) = handle_connection(stream, traces, stats).await {
                            warn!("Connection error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                }
            }
        }
    }
}

async fn handle_connection(
    mut stream: UnixStream,
    traces: Arc<DashMap<String, Vec<TraceEvent>>>,
    stats: Arc<RwLock<TraceStats>>,
) -> Result<()> {
    debug!("📡 New trace connection");

    let mut buffer = vec![0u8; 8192];
    let mut incomplete_data = Vec::new();

    loop {
        match stream.read(&mut buffer).await {
            Ok(0) => {
                debug!("📡 Trace client disconnected");
                break;
            }
            Ok(bytes_read) => {
                incomplete_data.extend_from_slice(&buffer[..bytes_read]);

                // Process complete JSON events (newline delimited)
                let mut start = 0;
                while let Some(newline_pos) =
                    incomplete_data[start..].iter().position(|&b| b == b'\n')
                {
                    let end = start + newline_pos;
                    let json_bytes = &incomplete_data[start..end];

                    match parse_trace_event(json_bytes) {
                        Ok(event) => {
                            info!(
                                "📊 Trace Event: {} from {} (type: {})",
                                event.trace_id, event.service, event.event_type
                            );

                            // Store trace event
                            traces
                                .entry(event.trace_id.clone())
                                .or_insert_with(Vec::new)
                                .push(event);

                            // Update stats
                            {
                                let mut stats = stats.write();
                                stats.events_processed += 1;
                            }
                        }
                        Err(e) => {
                            warn!("Failed to parse trace event: {}", e);
                        }
                    }

                    start = end + 1;
                }

                // Keep remaining incomplete data
                if start < incomplete_data.len() {
                    incomplete_data.drain(..start);
                } else {
                    incomplete_data.clear();
                }
            }
            Err(e) => {
                warn!("Error reading from trace connection: {}", e);
                break;
            }
        }
    }

    Ok(())
}

fn parse_trace_event(json_bytes: &[u8]) -> Result<TraceEvent> {
    let json_str = std::str::from_utf8(json_bytes)?;
    let event: TraceEvent = serde_json::from_str(json_str)?;
    Ok(event)
}

async fn update_stats(
    traces: &Arc<DashMap<String, Vec<TraceEvent>>>,
    stats: &Arc<RwLock<TraceStats>>,
    start_time: u64,
) {
    let mut stats = stats.write();
    stats.active_traces = traces.len();
    let current_time = safe_system_timestamp_ns();
    stats.uptime_seconds = (current_time - start_time) / 1_000_000_000;

    // Calculate events per second
    if stats.uptime_seconds > 0 {
        stats.events_per_second = stats.events_processed as f64 / stats.uptime_seconds as f64;
    }

    // Collect unique services seen
    let mut services = std::collections::HashSet::new();
    for trace_events in traces.iter() {
        for event in trace_events.value() {
            services.insert(event.service.clone());
        }
    }
    stats.services_seen = services.into_iter().collect();
}

async fn start_api_server(
    traces: Arc<DashMap<String, Vec<TraceEvent>>>,
    stats: Arc<RwLock<TraceStats>>,
) -> Result<()> {
    let listener = TcpListener::bind("0.0.0.0:8080").await?;
    info!("🌐 TraceCollector API listening on http://0.0.0.0:8080");

    loop {
        match listener.accept().await {
            Ok((mut stream, addr)) => {
                info!("📡 API request from {}", addr);

                let traces = traces.clone();
                let stats = stats.clone();

                tokio::spawn(async move {
                    let mut buffer = [0u8; 1024];
                    if let Ok(n) = stream.read(&mut buffer).await {
                        let request = String::from_utf8_lossy(&buffer[..n]);
                        debug!("API request: {}", request.lines().next().unwrap_or(""));

                        let response = if request.contains("GET /api/traces") {
                            get_traces_response(&traces).await
                        } else if request.contains("GET /api/stats") {
                            get_stats_response(&stats).await
                        } else if request.contains("GET /api/health") {
                            get_health_response().await
                        } else {
                            get_debug_response(&traces, &stats).await
                        };

                        let _ = stream.write_all(response.as_bytes()).await;
                    }
                });
            }
            Err(e) => {
                error!("Failed to accept API connection: {}", e);
            }
        }
    }
}

async fn get_traces_response(traces: &Arc<DashMap<String, Vec<TraceEvent>>>) -> String {
    let mut all_events = Vec::new();
    for trace_events in traces.iter() {
        for event in trace_events.value() {
            all_events.push(event.clone());
        }
    }

    // Sort by timestamp (most recent first)
    all_events.sort_by(|a, b| b.timestamp_ns.cmp(&a.timestamp_ns));
    all_events.truncate(20); // Limit to 20 most recent

    let json = serde_json::to_string_pretty(&all_events).unwrap_or_else(|_| "[]".to_string());

    format!(
        "HTTP/1.1 200 OK\r\n\
         Content-Type: application/json\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Content-Length: {}\r\n\
         \r\n\
         {}",
        json.len(),
        json
    )
}

async fn get_stats_response(stats: &Arc<RwLock<TraceStats>>) -> String {
    let stats = stats.read().clone();
    let json = serde_json::to_string_pretty(&stats).unwrap_or_else(|_| "{}".to_string());

    format!(
        "HTTP/1.1 200 OK\r\n\
         Content-Type: application/json\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Content-Length: {}\r\n\
         \r\n\
         {}",
        json.len(),
        json
    )
}

async fn get_health_response() -> String {
    let health = serde_json::json!({
        "status": "healthy",
        "timestamp": safe_system_timestamp_ns() / 1_000_000 // Convert to milliseconds
    });

    let json = health.to_string();

    format!(
        "HTTP/1.1 200 OK\r\n\
         Content-Type: application/json\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Content-Length: {}\r\n\
         \r\n\
         {}",
        json.len(),
        json
    )
}

async fn get_debug_response(
    traces: &Arc<DashMap<String, Vec<TraceEvent>>>,
    stats: &Arc<RwLock<TraceStats>>,
) -> String {
    let stats = stats.read();
    let trace_count = traces.len();

    let html = format!(
        r#"
<!DOCTYPE html>
<html>
<head><title>TraceCollector Debug</title></head>
<body>
<h1>🚀 TraceCollector Debug Dashboard</h1>
<h2>📊 Statistics</h2>
<ul>
<li>Events Processed: {}</li>
<li>Active Traces: {}</li>
<li>Events/sec: {:.2}</li>
<li>Uptime: {} seconds</li>
<li>Services Seen: {:?}</li>
</ul>

<h2>🔍 Recent Trace Events</h2>
<div id="events" style="font-family: monospace; background: #f0f0f0; padding: 10px;">
Loading...
</div>

<script>
setInterval(async () => {{
    try {{
        const response = await fetch('/api/traces');
        const events = await response.json();
        const html = events.map(e => 
            `<div style="margin: 5px 0; padding: 5px; border: 1px solid #ccc;">
                <strong>${{e.service}}</strong> - ${{e.event_type}} 
                <br><small>Trace: ${{e.trace_id}}</small>
                <br><small>Time: ${{new Date(e.timestamp_ns / 1000000).toISOString()}}</small>
                ${{e.duration_ns ? '<br><small>Duration: ' + (e.duration_ns / 1000000).toFixed(2) + 'ms</small>' : ''}}
            </div>`
        ).join('');
        document.getElementById('events').innerHTML = html || 'No events yet...';
    }} catch (e) {{
        console.error('Failed to fetch events:', e);
    }}
}}, 1000);
</script>
</body>
</html>
        "#,
        stats.events_processed,
        trace_count,
        stats.events_per_second,
        stats.uptime_seconds,
        stats.services_seen
    );

    format!(
        "HTTP/1.1 200 OK\r\n\
         Content-Type: text/html\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Content-Length: {}\r\n\
         \r\n\
         {}",
        html.len(),
        html
    )
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let collector = SimpleTraceCollector::new();
    collector.start().await?;

    Ok(())
}
