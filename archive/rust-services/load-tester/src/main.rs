// AlphaPulse Load Testing Framework
use alphapulse_common::{
    shared_memory::{SharedMemoryWriter, OrderBookDeltaWriter, SharedTrade, SharedOrderBookDelta},
};
use clap::Parser;
use colored::Colorize;
use hdrhistogram::Histogram;
use indicatif::{ProgressBar, ProgressStyle};
use rand::{thread_rng, Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use sysinfo::System;
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::{info, warn};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Number of simulated exchanges
    #[arg(short = 'e', long, default_value_t = 3)]
    exchanges: usize,

    /// Target trades per second per exchange
    #[arg(short = 't', long, default_value_t = 10000)]
    trades_per_second: u64,

    /// Target orderbook updates per second per exchange
    #[arg(short = 'o', long, default_value_t = 5000)]
    orderbook_updates_per_second: u64,

    /// Test duration in seconds
    #[arg(short = 'd', long, default_value_t = 60)]
    duration_secs: u64,

    /// Number of symbols to simulate
    #[arg(short = 's', long, default_value_t = 10)]
    symbol_count: usize,

    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Output results to JSON file
    #[arg(long)]
    json_output: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadTestResults {
    pub config: LoadTestConfig,
    pub throughput: ThroughputMetrics,
    pub latency: LatencyMetrics,
    pub resources: ResourceMetrics,
    pub errors: ErrorMetrics,
    pub duration: Duration,
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadTestConfig {
    pub exchanges: usize,
    pub trades_per_second: u64,
    pub orderbook_updates_per_second: u64,
    pub duration_secs: u64,
    pub symbol_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThroughputMetrics {
    pub total_trades: u64,
    pub total_orderbook_updates: u64,
    pub trades_per_second: f64,
    pub orderbook_updates_per_second: f64,
    pub peak_trades_per_second: f64,
    pub peak_orderbook_per_second: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyMetrics {
    pub write_p50_ns: u64,
    pub write_p90_ns: u64,
    pub write_p99_ns: u64,
    pub write_p999_ns: u64,
    pub write_max_ns: u64,
    pub avg_latency_ns: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceMetrics {
    pub peak_cpu_percent: f32,
    pub peak_memory_mb: u64,
    pub avg_cpu_percent: f32,
    pub avg_memory_mb: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ErrorMetrics {
    pub write_errors: u64,
    pub buffer_overflows: u64,
    pub connection_errors: u64,
}

struct LoadGenerator {
    config: LoadTestConfig,
    running: Arc<AtomicBool>,
    trade_writers: Vec<Arc<RwLock<SharedMemoryWriter>>>,
    orderbook_writers: Vec<Arc<RwLock<OrderBookDeltaWriter>>>,
    metrics: Arc<RwLock<LiveMetrics>>,
}

struct LiveMetrics {
    trades_written: AtomicU64,
    orderbooks_written: AtomicU64,
    write_errors: AtomicU64,
    latency_histogram: RwLock<Histogram<u64>>,
    cpu_samples: Vec<f32>,
    memory_samples: Vec<u64>,
}

impl LoadGenerator {
    async fn new(config: LoadTestConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let mut trade_writers = Vec::new();
        let mut orderbook_writers = Vec::new();

        // Create shared memory writers for each exchange
        for i in 0..config.exchanges {
            let trade_path = format!("/tmp/alphapulse_loadtest/exchange_{}_trades", i);
            let orderbook_path = format!("/tmp/alphapulse_loadtest/exchange_{}_orderbook", i);

            // Clean up any existing files
            let _ = std::fs::remove_file(&trade_path);
            let _ = std::fs::remove_file(&orderbook_path);

            let trade_writer = SharedMemoryWriter::create(&trade_path, 100_000)?;
            let orderbook_writer = OrderBookDeltaWriter::create(&orderbook_path, 100_000)?;

            trade_writers.push(Arc::new(RwLock::new(trade_writer)));
            orderbook_writers.push(Arc::new(RwLock::new(orderbook_writer)));
        }

        let metrics = Arc::new(RwLock::new(LiveMetrics {
            trades_written: AtomicU64::new(0),
            orderbooks_written: AtomicU64::new(0),
            write_errors: AtomicU64::new(0),
            latency_histogram: RwLock::new(Histogram::new(3)?),
            cpu_samples: Vec::new(),
            memory_samples: Vec::new(),
        }));

        Ok(Self {
            config,
            running: Arc::new(AtomicBool::new(false)),
            trade_writers,
            orderbook_writers,
            metrics,
        })
    }

    async fn run(&mut self) -> LoadTestResults {
        self.running.store(true, Ordering::Relaxed);
        let start_time = Instant::now();

        // Create progress bar
        let pb = ProgressBar::new(self.config.duration_secs);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
                .unwrap()
                .progress_chars("#>-"),
        );

        // Start trade generators
        let mut trade_tasks = Vec::new();
        for (i, writer) in self.trade_writers.iter().enumerate() {
            let task = self.spawn_trade_generator(i, writer.clone());
            trade_tasks.push(task);
        }

        // Start orderbook generators
        let mut orderbook_tasks = Vec::new();
        for (i, writer) in self.orderbook_writers.iter().enumerate() {
            let task = self.spawn_orderbook_generator(i, writer.clone());
            orderbook_tasks.push(task);
        }

        // Start resource monitor
        let resource_task = self.spawn_resource_monitor();

        // Progress tracking
        let mut elapsed_secs = 0;
        while elapsed_secs < self.config.duration_secs {
            tokio::time::sleep(Duration::from_secs(1)).await;
            elapsed_secs += 1;
            pb.set_position(elapsed_secs);

            // Print live stats
            if elapsed_secs % 5 == 0 {
                self.print_live_stats().await;
            }
        }

        // Stop generation
        self.running.store(false, Ordering::Relaxed);
        pb.finish_with_message("Load test complete!");

        // Wait for tasks to complete
        for task in trade_tasks {
            let _ = task.await;
        }
        for task in orderbook_tasks {
            let _ = task.await;
        }
        resource_task.abort();

        // Calculate results
        self.calculate_results(start_time.elapsed()).await
    }

    fn spawn_trade_generator(
        &self,
        exchange_id: usize,
        writer: Arc<RwLock<SharedMemoryWriter>>,
    ) -> tokio::task::JoinHandle<()> {
        let running = self.running.clone();
        let metrics = self.metrics.clone();
        let symbol_count = self.config.symbol_count;
        let target_tps = self.config.trades_per_second;

        tokio::spawn(async move {
            let mut rng = rand::rngs::StdRng::from_entropy();
            let symbols = generate_symbols(symbol_count);
            let exchange_name = format!("exchange_{}", exchange_id);
            
            // Calculate interval for target TPS
            let interval_us = 1_000_000 / target_tps;
            let mut interval = interval(Duration::from_micros(interval_us));

            while running.load(Ordering::Relaxed) {
                interval.tick().await;

                let trade = generate_random_trade(&mut rng, &symbols, &exchange_name);
                let start = Instant::now();

                // Write trade
                let mut writer_guard = writer.write().await;
                match writer_guard.write_trade(&trade) {
                    Ok(_) => {
                        let latency_ns = start.elapsed().as_nanos() as u64;
                        
                        // Update metrics
                        let metrics_guard = metrics.read().await;
                        metrics_guard.trades_written.fetch_add(1, Ordering::Relaxed);
                        
                        let mut hist = metrics_guard.latency_histogram.write().await;
                        let _ = hist.record(latency_ns);
                    }
                    Err(e) => {
                        let metrics_guard = metrics.read().await;
                        metrics_guard.write_errors.fetch_add(1, Ordering::Relaxed);
                        warn!("Trade write error: {}", e);
                    }
                }
            }
        })
    }

    fn spawn_orderbook_generator(
        &self,
        exchange_id: usize,
        writer: Arc<RwLock<OrderBookDeltaWriter>>,
    ) -> tokio::task::JoinHandle<()> {
        let running = self.running.clone();
        let metrics = self.metrics.clone();
        let symbol_count = self.config.symbol_count;
        let target_ops = self.config.orderbook_updates_per_second;

        tokio::spawn(async move {
            let mut rng = rand::rngs::StdRng::from_entropy();
            let symbols = generate_symbols(symbol_count);
            let exchange_name = format!("exchange_{}", exchange_id);
            
            // Calculate interval for target OPS
            let interval_us = 1_000_000 / target_ops;
            let mut interval = interval(Duration::from_micros(interval_us));
            let mut version = 0u64;

            while running.load(Ordering::Relaxed) {
                interval.tick().await;

                let delta = generate_random_orderbook_delta(
                    &mut rng,
                    &symbols,
                    &exchange_name,
                    &mut version,
                );
                let start = Instant::now();

                // Write orderbook delta
                let mut writer_guard = writer.write().await;
                match writer_guard.write_delta(&delta) {
                    Ok(_) => {
                        let latency_ns = start.elapsed().as_nanos() as u64;
                        
                        // Update metrics
                        let metrics_guard = metrics.read().await;
                        metrics_guard.orderbooks_written.fetch_add(1, Ordering::Relaxed);
                        
                        let mut hist = metrics_guard.latency_histogram.write().await;
                        let _ = hist.record(latency_ns);
                    }
                    Err(e) => {
                        let metrics_guard = metrics.read().await;
                        metrics_guard.write_errors.fetch_add(1, Ordering::Relaxed);
                        warn!("Orderbook write error: {}", e);
                    }
                }
            }
        })
    }

    fn spawn_resource_monitor(&self) -> tokio::task::JoinHandle<()> {
        let running = self.running.clone();
        let metrics = self.metrics.clone();

        tokio::spawn(async move {
            let mut sys = System::new_all();
            let pid = sysinfo::Pid::from(std::process::id() as usize);
            
            while running.load(Ordering::Relaxed) {
                tokio::time::sleep(Duration::from_secs(1)).await;
                
                sys.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[pid]));
                if let Some(process) = sys.processes().get(&pid) {
                    let cpu = process.cpu_usage();
                    let memory = process.memory() / 1024; // Convert to MB
                    
                    let mut metrics_guard = metrics.write().await;
                    metrics_guard.cpu_samples.push(cpu);
                    metrics_guard.memory_samples.push(memory);
                }
            }
        })
    }

    async fn print_live_stats(&self) {
        let metrics = self.metrics.read().await;
        let trades = metrics.trades_written.load(Ordering::Relaxed);
        let orderbooks = metrics.orderbooks_written.load(Ordering::Relaxed);
        let errors = metrics.write_errors.load(Ordering::Relaxed);

        println!("\n{}", "=== Live Statistics ===".bright_cyan());
        println!("Trades written: {}", trades.to_string().green());
        println!("Orderbook updates: {}", orderbooks.to_string().green());
        if errors > 0 {
            println!("Errors: {}", errors.to_string().red());
        }
    }

    async fn calculate_results(&self, duration: Duration) -> LoadTestResults {
        let metrics = self.metrics.read().await;
        
        // Throughput metrics
        let total_trades = metrics.trades_written.load(Ordering::Relaxed);
        let total_orderbooks = metrics.orderbooks_written.load(Ordering::Relaxed);
        let duration_secs = duration.as_secs_f64();
        
        let throughput = ThroughputMetrics {
            total_trades,
            total_orderbook_updates: total_orderbooks,
            trades_per_second: total_trades as f64 / duration_secs,
            orderbook_updates_per_second: total_orderbooks as f64 / duration_secs,
            peak_trades_per_second: 0.0, // TODO: Track peaks
            peak_orderbook_per_second: 0.0,
        };

        // Latency metrics
        let hist = metrics.latency_histogram.read().await;
        let latency = LatencyMetrics {
            write_p50_ns: hist.value_at_percentile(50.0),
            write_p90_ns: hist.value_at_percentile(90.0),
            write_p99_ns: hist.value_at_percentile(99.0),
            write_p999_ns: hist.value_at_percentile(99.9),
            write_max_ns: hist.max(),
            avg_latency_ns: hist.mean() as u64,
        };

        // Resource metrics
        let resources = if !metrics.cpu_samples.is_empty() {
            ResourceMetrics {
                peak_cpu_percent: *metrics.cpu_samples.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap_or(&0.0),
                peak_memory_mb: *metrics.memory_samples.iter().max().unwrap_or(&0),
                avg_cpu_percent: metrics.cpu_samples.iter().sum::<f32>() / metrics.cpu_samples.len() as f32,
                avg_memory_mb: metrics.memory_samples.iter().sum::<u64>() / metrics.memory_samples.len() as u64,
            }
        } else {
            ResourceMetrics {
                peak_cpu_percent: 0.0,
                peak_memory_mb: 0,
                avg_cpu_percent: 0.0,
                avg_memory_mb: 0,
            }
        };

        // Error metrics
        let errors = ErrorMetrics {
            write_errors: metrics.write_errors.load(Ordering::Relaxed),
            buffer_overflows: 0, // TODO: Track buffer overflows
            connection_errors: 0, // N/A for shared memory
        };

        LoadTestResults {
            config: self.config.clone(),
            throughput,
            latency,
            resources,
            errors,
            duration,
            success: errors.write_errors == 0,
        }
    }
}

fn generate_symbols(count: usize) -> Vec<String> {
    let base_symbols = vec!["BTC", "ETH", "SOL", "AVAX", "MATIC", "ADA", "DOT", "LINK"];
    let quote_symbols = vec!["USD", "USDT", "EUR"];
    
    let mut symbols = Vec::new();
    for i in 0..count {
        let base = base_symbols[i % base_symbols.len()];
        let quote = quote_symbols[i % quote_symbols.len()];
        symbols.push(format!("{}/{}", base, quote));
    }
    symbols
}

fn generate_random_trade(rng: &mut impl Rng, symbols: &[String], exchange: &str) -> SharedTrade {
    let symbol = &symbols[rng.gen_range(0..symbols.len())];
    let price = 50000.0 + rng.gen_range(-5000.0..5000.0);
    let volume = rng.gen_range(0.001..10.0);
    let side = rng.gen_bool(0.5);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;
    let trade_id = format!("trade_{}", rng.gen::<u64>());

    SharedTrade::new(timestamp, symbol, exchange, price, volume, side, &trade_id)
}

fn generate_random_orderbook_delta(
    rng: &mut impl Rng,
    symbols: &[String],
    exchange: &str,
    version: &mut u64,
) -> SharedOrderBookDelta {
    let symbol = &symbols[rng.gen_range(0..symbols.len())];
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;
    
    *version += 1;
    let mut delta = SharedOrderBookDelta::new(timestamp, symbol, exchange, *version, *version - 1);
    
    // Add random price level changes
    let change_count = rng.gen_range(1..10);
    for _ in 0..change_count {
        let price = 50000.0 + rng.gen_range(-1000.0..1000.0);
        let volume = if rng.gen_bool(0.1) { 0.0 } else { rng.gen_range(0.1..5.0) };
        let is_ask = rng.gen_bool(0.5);
        let action = rng.gen_range(0..3); // Add, Update, Remove
        
        if !delta.add_change(price, volume, is_ask, action) {
            break; // Buffer full
        }
    }
    
    delta
}

fn print_results(results: &LoadTestResults) {
    println!("\n{}", "═══════════════════════════════════════".bright_blue());
    println!("{}", "       LOAD TEST RESULTS".bright_yellow().bold());
    println!("{}", "═══════════════════════════════════════".bright_blue());

    // Configuration
    println!("\n{}", "Configuration:".bright_cyan());
    println!("  Exchanges: {}", results.config.exchanges);
    println!("  Target TPS: {}", results.config.trades_per_second);
    println!("  Target OPS: {}", results.config.orderbook_updates_per_second);
    println!("  Duration: {}s", results.config.duration_secs);
    println!("  Symbols: {}", results.config.symbol_count);

    // Throughput
    println!("\n{}", "Throughput:".bright_cyan());
    println!("  Total Trades: {}", results.throughput.total_trades.to_string().green());
    println!("  Total Orderbook Updates: {}", results.throughput.total_orderbook_updates.to_string().green());
    println!("  Actual TPS: {:.0}", results.throughput.trades_per_second);
    println!("  Actual OPS: {:.0}", results.throughput.orderbook_updates_per_second);

    // Latency
    println!("\n{}", "Write Latency:".bright_cyan());
    println!("  P50:  {:>8} ns ({:.3} μs)", 
        results.latency.write_p50_ns,
        results.latency.write_p50_ns as f64 / 1000.0
    );
    println!("  P90:  {:>8} ns ({:.3} μs)", 
        results.latency.write_p90_ns,
        results.latency.write_p90_ns as f64 / 1000.0
    );
    println!("  P99:  {:>8} ns ({:.3} μs)", 
        results.latency.write_p99_ns,
        results.latency.write_p99_ns as f64 / 1000.0
    );
    println!("  P99.9:{:>8} ns ({:.3} μs)", 
        results.latency.write_p999_ns,
        results.latency.write_p999_ns as f64 / 1000.0
    );
    println!("  Max:  {:>8} ns ({:.3} μs)", 
        results.latency.write_max_ns.to_string().red(),
        results.latency.write_max_ns as f64 / 1000.0
    );

    // Resources
    println!("\n{}", "Resource Usage:".bright_cyan());
    println!("  Peak CPU: {:.1}%", results.resources.peak_cpu_percent);
    println!("  Avg CPU:  {:.1}%", results.resources.avg_cpu_percent);
    println!("  Peak Mem: {} MB", results.resources.peak_memory_mb);
    println!("  Avg Mem:  {} MB", results.resources.avg_memory_mb);

    // Errors
    if results.errors.write_errors > 0 {
        println!("\n{}", "Errors:".bright_red());
        println!("  Write Errors: {}", results.errors.write_errors);
    }

    // Summary
    println!("\n{}", "═══════════════════════════════════════".bright_blue());
    let status = if results.success {
        "✓ PASSED".green().bold()
    } else {
        "✗ FAILED".red().bold()
    };
    println!("Status: {}", status);
    
    // Performance vs Target
    let tps_achievement = (results.throughput.trades_per_second / results.config.trades_per_second as f64 * 100.0) as u32;
    let ops_achievement = (results.throughput.orderbook_updates_per_second / results.config.orderbook_updates_per_second as f64 * 100.0) as u32;
    
    println!("TPS Achievement: {}%", format_achievement(tps_achievement));
    println!("OPS Achievement: {}%", format_achievement(ops_achievement));
}

fn format_achievement(percentage: u32) -> colored::ColoredString {
    if percentage >= 95 {
        percentage.to_string().green()
    } else if percentage >= 80 {
        percentage.to_string().yellow()
    } else {
        percentage.to_string().red()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse arguments
    let args = Args::parse();

    // Initialize tracing
    let level = if args.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(level)
        .init();

    info!("🚀 Starting AlphaPulse Load Tester");

    // Create configuration
    let config = LoadTestConfig {
        exchanges: args.exchanges,
        trades_per_second: args.trades_per_second,
        orderbook_updates_per_second: args.orderbook_updates_per_second,
        duration_secs: args.duration_secs,
        symbol_count: args.symbol_count,
    };

    // Create and run load generator
    let mut generator = LoadGenerator::new(config).await?;
    let results = generator.run().await;

    // Print results
    print_results(&results);

    // Save to JSON if requested
    if let Some(output_path) = args.json_output {
        let json = serde_json::to_string_pretty(&results)?;
        std::fs::write(&output_path, json)?;
        info!("Results saved to: {}", output_path);
    }

    // Cleanup
    let _ = std::fs::remove_dir_all("/tmp/alphapulse_loadtest");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbol_generation() {
        let symbols = generate_symbols(10);
        assert_eq!(symbols.len(), 10);
        assert!(symbols.contains(&"BTC/USD".to_string()));
    }

    #[tokio::test]
    async fn test_small_load() {
        let config = LoadTestConfig {
            exchanges: 1,
            trades_per_second: 100,
            orderbook_updates_per_second: 50,
            duration_secs: 2,
            symbol_count: 5,
        };

        let mut generator = LoadGenerator::new(config).await.unwrap();
        let results = generator.run().await;

        assert!(results.throughput.total_trades > 0);
        assert!(results.throughput.total_orderbook_updates > 0);
        assert!(results.latency.write_p50_ns > 0);
        assert!(results.errors.write_errors == 0);
    }
}