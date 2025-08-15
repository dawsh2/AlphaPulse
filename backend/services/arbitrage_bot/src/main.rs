mod executor;
mod validator;

use alphapulse_protocol::*;
use anyhow::{Context, Result};
use byteorder::{LittleEndian, ReadBytesExt};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::io::Read;
use std::os::unix::net::UnixStream;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::task;
use tracing::{debug, error, info, warn};

const RELAY_SOCKET_PATH: &str = "/tmp/alphapulse/relay.sock";
const MIN_PROFIT_USD: f64 = 15.0;
const MAX_GAS_PRICE_GWEI: u64 = 100;

struct ArbitrageBot {
    relay_stream: UnixStream,
    executor: Arc<executor::FlashLoanExecutor>,
    validator: Arc<validator::OpportunityValidator>,
    opportunities_queue: mpsc::Sender<ArbitrageOpportunityMessage>,
    stats: Arc<RwLock<BotStats>>,
}

#[derive(Debug, Default)]
struct BotStats {
    opportunities_received: u64,
    opportunities_validated: u64,
    opportunities_executed: u64,
    opportunities_failed: u64,
    total_profit_usd: f64,
    total_gas_spent_usd: f64,
}

impl ArbitrageBot {
    async fn new() -> Result<Self> {
        // Connect to relay server's Unix socket
        info!("🔌 Connecting to relay server at {}", RELAY_SOCKET_PATH);
        let mut relay_stream = UnixStream::connect(RELAY_SOCKET_PATH)
            .context("Failed to connect to relay server")?;
        
        // Set non-blocking mode for async reading
        relay_stream.set_nonblocking(false)?;
        
        // Create executor and validator
        let executor = Arc::new(executor::FlashLoanExecutor::new().await?);
        let validator = Arc::new(validator::OpportunityValidator::new().await?);
        
        // Create opportunity processing queue
        let (tx, mut rx) = mpsc::channel::<ArbitrageOpportunityMessage>(100);
        
        // Spawn opportunity processor
        let executor_clone = executor.clone();
        let validator_clone = validator.clone();
        tokio::spawn(async move {
            while let Some(opportunity) = rx.recv().await {
                Self::process_opportunity(
                    opportunity,
                    executor_clone.clone(),
                    validator_clone.clone(),
                ).await;
            }
        });
        
        Ok(Self {
            relay_stream,
            executor,
            validator,
            opportunities_queue: tx,
            stats: Arc::new(RwLock::new(BotStats::default())),
        })
    }
    
    async fn start(&mut self) -> Result<()> {
        info!("🚀 Starting Arbitrage Bot");
        info!("📊 Minimum profit threshold: ${}", MIN_PROFIT_USD);
        info!("⛽ Maximum gas price: {} gwei", MAX_GAS_PRICE_GWEI);
        
        // Start monitoring relay messages
        let mut buffer = vec![0u8; 65536];
        let mut pending_data = Vec::new();
        
        loop {
            // Read from Unix socket
            let bytes_read = match self.relay_stream.read(&mut buffer) {
                Ok(n) if n > 0 => n,
                Ok(_) => {
                    warn!("Relay connection closed");
                    break;
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    continue;
                }
                Err(e) => {
                    error!("Error reading from relay: {}", e);
                    break;
                }
            };
            
            pending_data.extend_from_slice(&buffer[..bytes_read]);
            
            // Process complete messages
            while pending_data.len() >= MessageHeader::SIZE {
                // Read header
                let header = MessageHeader::read_from_prefix(&pending_data[..MessageHeader::SIZE])
                    .context("Failed to read message header")?;
                
                if header.magic != MAGIC_BYTE {
                    error!("Invalid magic byte: {:02x}", header.magic);
                    pending_data.clear();
                    break;
                }
                
                let msg_type = header.get_type()?;
                let msg_len = header.get_length() as usize;
                let total_size = MessageHeader::SIZE + msg_len;
                
                if pending_data.len() < total_size {
                    break; // Wait for more data
                }
                
                // Handle ArbitrageOpportunity messages
                if msg_type == MessageType::ArbitrageOpportunity {
                    let msg_data = &pending_data[MessageHeader::SIZE..total_size];
                    match ArbitrageOpportunityMessage::decode(msg_data) {
                        Ok(opportunity) => {
                            info!("💰 Received arbitrage opportunity: {} - ${:.2} profit", 
                                opportunity.pair, 
                                opportunity.estimated_profit as f64 / 1e8);
                            
                            self.stats.write().opportunities_received += 1;
                            
                            // Queue for processing
                            if let Err(e) = self.opportunities_queue.send(opportunity).await {
                                error!("Failed to queue opportunity: {}", e);
                            }
                        }
                        Err(e) => {
                            error!("Failed to decode arbitrage opportunity: {}", e);
                        }
                    }
                }
                
                // Remove processed message
                pending_data.drain(..total_size);
            }
        }
        
        Ok(())
    }
    
    async fn process_opportunity(
        opportunity: ArbitrageOpportunityMessage,
        executor: Arc<executor::FlashLoanExecutor>,
        validator: Arc<validator::OpportunityValidator>,
    ) {
        let start = Instant::now();
        
        // Convert fixed point values back to floats
        let profit_usd = opportunity.estimated_profit as f64 / 1e8;
        let profit_percent = opportunity.profit_percent as f64 / 1e10;
        
        // Check minimum profit threshold
        if profit_usd < MIN_PROFIT_USD {
            debug!("Opportunity below threshold: ${:.2} < ${}", profit_usd, MIN_PROFIT_USD);
            return;
        }
        
        // Validate opportunity is still profitable
        match validator.validate(&opportunity).await {
            Ok(true) => {
                info!("✅ Opportunity validated: {} - ${:.2} ({:.2}%)", 
                    opportunity.pair, profit_usd, profit_percent * 100.0);
            }
            Ok(false) => {
                debug!("Opportunity no longer profitable after validation");
                return;
            }
            Err(e) => {
                warn!("Failed to validate opportunity: {}", e);
                return;
            }
        }
        
        // Execute flash loan
        match executor.execute(opportunity.clone()).await {
            Ok(result) => {
                info!("✅ EXECUTED: {} - Actual profit: ${:.2}, Gas: ${:.2}",
                    opportunity.pair,
                    result.actual_profit_usd,
                    result.gas_cost_usd);
                
                // Update stats
                // stats.write().opportunities_executed += 1;
                // stats.write().total_profit_usd += result.actual_profit_usd;
                // stats.write().total_gas_spent_usd += result.gas_cost_usd;
            }
            Err(e) => {
                error!("❌ Execution failed for {}: {}", opportunity.pair, e);
                // stats.write().opportunities_failed += 1;
            }
        }
        
        let latency_ms = start.elapsed().as_millis();
        debug!("Opportunity processing took {}ms", latency_ms);
    }
    
    fn print_stats(&self) {
        let stats = self.stats.read();
        info!("📈 Bot Statistics:");
        info!("  Opportunities received: {}", stats.opportunities_received);
        info!("  Opportunities validated: {}", stats.opportunities_validated);
        info!("  Opportunities executed: {}", stats.opportunities_executed);
        info!("  Opportunities failed: {}", stats.opportunities_failed);
        info!("  Total profit: ${:.2}", stats.total_profit_usd);
        info!("  Total gas spent: ${:.2}", stats.total_gas_spent_usd);
        info!("  Net profit: ${:.2}", stats.total_profit_usd - stats.total_gas_spent_usd);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("arbitrage_bot=info".parse()?)
                .add_directive("info".parse()?),
        )
        .init();
    
    info!("🤖 AlphaPulse Arbitrage Bot Starting...");
    
    // Create and start bot
    let mut bot = ArbitrageBot::new().await?;
    
    // Spawn stats printer
    let stats = bot.stats.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            // Print stats every minute
            let s = stats.read();
            if s.opportunities_received > 0 {
                info!("📊 Stats - Received: {}, Executed: {}, Failed: {}, Net: ${:.2}",
                    s.opportunities_received,
                    s.opportunities_executed,
                    s.opportunities_failed,
                    s.total_profit_usd - s.total_gas_spent_usd);
            }
        }
    });
    
    // Handle shutdown
    let shutdown = tokio::signal::ctrl_c();
    tokio::select! {
        result = bot.start() => {
            if let Err(e) = result {
                error!("Bot error: {}", e);
            }
        }
        _ = shutdown => {
            info!("🛑 Shutting down...");
        }
    }
    
    bot.print_stats();
    info!("👋 Arbitrage Bot stopped");
    
    Ok(())
}