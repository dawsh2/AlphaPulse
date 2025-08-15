use alphapulse_protocol::{ArbitrageOpportunityMessage, MessageType};
use anyhow::Result;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use ethers::prelude::*;
use parking_lot::RwLock;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tokio::time;
use tracing::{debug, error, info, warn};

mod config;
mod dex;
mod executor;
mod simulator;

use config::Config;
use dex::{DexRouter, TokenInfo};
use executor::CapitalArbExecutor;
use simulator::ArbSimulator;

const RELAY_SOCKET_PATH: &str = "/tmp/alphapulse_relay.sock";
const RECONNECT_DELAY: Duration = Duration::from_secs(5);

#[derive(Debug, Clone)]
pub struct ArbOpportunity {
    pub timestamp_ns: u64,
    pub pair: String,
    pub token_a: Address,
    pub token_b: Address,
    pub dex_buy_router: Address,
    pub dex_sell_router: Address,
    pub price_a: f64,
    pub price_b: f64,
    pub liquidity_a: f64,
    pub liquidity_b: f64,
    pub estimated_profit_usd: f64,
    pub gas_estimate: u64,
}

pub struct CapitalArbBot {
    config: Arc<Config>,
    executor: Arc<CapitalArbExecutor>,
    simulator: Arc<ArbSimulator>,
    opportunities: Arc<RwLock<Vec<ArbOpportunity>>>,
    metrics: Arc<Metrics>,
}

#[derive(Default)]
struct Metrics {
    opportunities_received: RwLock<u64>,
    opportunities_executed: RwLock<u64>,
    opportunities_simulated: RwLock<u64>,
    total_profit: RwLock<f64>,
    failed_executions: RwLock<u64>,
}

impl CapitalArbBot {
    pub async fn new(config: Config) -> Result<Self> {
        let provider = Provider::<Http>::try_from(&config.rpc_url)?;
        let wallet = config.private_key.parse::<LocalWallet>()?
            .with_chain_id(config.chain_id);
        let client = Arc::new(SignerMiddleware::new(provider, wallet));

        let executor = Arc::new(CapitalArbExecutor::new(client.clone(), config.clone()).await?);
        let simulator = Arc::new(ArbSimulator::new(client.clone()));

        Ok(Self {
            config: Arc::new(config),
            executor,
            simulator,
            opportunities: Arc::new(RwLock::new(Vec::new())),
            metrics: Arc::new(Metrics::default()),
        })
    }

    pub async fn run(&self) -> Result<()> {
        info!("Starting Capital-Based Arbitrage Bot");
        info!("Using own capital - no flash loans");
        info!("Chain ID: {}", self.config.chain_id);
        info!("Min profit threshold: ${}", self.config.min_profit_usd);

        // Check wallet balances
        self.executor.check_balances().await?;

        // Start opportunity processor
        let processor_handle = self.start_opportunity_processor();

        // Connect to relay server
        let relay_handle = self.start_relay_connection();

        // Start metrics server
        self.start_metrics_server().await;

        // Wait for tasks
        tokio::select! {
            _ = processor_handle => {
                error!("Opportunity processor stopped");
            }
            _ = relay_handle => {
                error!("Relay connection stopped");
            }
        }

        Ok(())
    }

    fn start_relay_connection(&self) -> tokio::task::JoinHandle<()> {
        let opportunities = self.opportunities.clone();
        let metrics = self.metrics.clone();
        
        tokio::spawn(async move {
            loop {
                match Self::connect_and_listen(opportunities.clone(), metrics.clone()).await {
                    Ok(_) => {
                        info!("Relay connection closed normally");
                    }
                    Err(e) => {
                        error!("Relay connection error: {}", e);
                    }
                }
                
                info!("Reconnecting to relay in {:?}", RECONNECT_DELAY);
                tokio::time::sleep(RECONNECT_DELAY).await;
            }
        })
    }

    async fn connect_and_listen(
        opportunities: Arc<RwLock<Vec<ArbOpportunity>>>,
        metrics: Arc<Metrics>,
    ) -> Result<()> {
        info!("Connecting to relay server at {}", RELAY_SOCKET_PATH);
        
        // Use blocking I/O in separate thread for Unix socket
        let (tx, mut rx) = mpsc::channel::<ArbitrageOpportunityMessage>(100);
        
        std::thread::spawn(move || {
            if let Err(e) = Self::unix_socket_reader(tx) {
                error!("Unix socket reader error: {}", e);
            }
        });

        // Process messages
        while let Some(msg) = rx.recv().await {
            let opp = ArbOpportunity {
                timestamp_ns: msg.timestamp_ns,
                pair: msg.pair.clone(),
                token_a: msg.token_a.parse()?,
                token_b: msg.token_b.parse()?,
                dex_buy_router: msg.dex_buy_router.parse()?,
                dex_sell_router: msg.dex_sell_router.parse()?,
                price_a: msg.price_buy as f64 / 1e9,  // Convert from fixed point
                price_b: msg.price_sell as f64 / 1e9,
                liquidity_a: msg.liquidity_buy as f64 / 1e9,
                liquidity_b: msg.liquidity_sell as f64 / 1e9,
                estimated_profit_usd: msg.estimated_profit as f64 / 1e9,
                gas_estimate: msg.gas_estimate as u64,
            };

            debug!("Received opportunity: {} profit=${:.2}", opp.pair, opp.estimated_profit_usd);
            
            *metrics.opportunities_received.write() += 1;
            opportunities.write().push(opp);
        }

        Ok(())
    }

    fn unix_socket_reader(tx: mpsc::Sender<ArbitrageOpportunityMessage>) -> Result<()> {
        let mut stream = UnixStream::connect(RELAY_SOCKET_PATH)?;
        info!("Connected to relay server");

        // Send subscription for arbitrage opportunities
        stream.write_u8(MessageType::ArbitrageOpportunity as u8)?;
        stream.flush()?;

        let mut buffer = vec![0u8; 65536];
        
        loop {
            // Read message type
            let msg_type = stream.read_u8()?;
            
            if msg_type != MessageType::ArbitrageOpportunity as u8 {
                warn!("Unexpected message type: {}", msg_type);
                continue;
            }

            // Read message length
            let length = stream.read_u32::<LittleEndian>()? as usize;
            
            if length > buffer.len() {
                buffer.resize(length, 0);
            }

            // Read message data
            stream.read_exact(&mut buffer[..length])?;

            // Decode message
            match ArbitrageOpportunityMessage::decode(&buffer[..length]) {
                Ok(msg) => {
                    if tx.blocking_send(msg).is_err() {
                        error!("Failed to send message to processor");
                        break;
                    }
                }
                Err(e) => {
                    error!("Failed to decode arbitrage message: {}", e);
                }
            }
        }

        Ok(())
    }

    fn start_opportunity_processor(&self) -> tokio::task::JoinHandle<()> {
        let opportunities = self.opportunities.clone();
        let executor = self.executor.clone();
        let simulator = self.simulator.clone();
        let config = self.config.clone();
        let metrics = self.metrics.clone();

        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_millis(100));
            
            loop {
                interval.tick().await;

                // Get opportunities to process
                let opps = {
                    let mut opps = opportunities.write();
                    std::mem::take(&mut *opps)
                };

                for opp in opps {
                    // Check age
                    let age_ms = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_nanos() as u64
                        - opp.timestamp_ns;
                    let age_ms = age_ms / 1_000_000;

                    if age_ms > config.max_opportunity_age_ms {
                        debug!("Opportunity too old: {}ms", age_ms);
                        continue;
                    }

                    // Check profit threshold
                    if opp.estimated_profit_usd < config.min_profit_usd {
                        debug!("Opportunity below threshold: ${:.2}", opp.estimated_profit_usd);
                        continue;
                    }

                    info!("Processing opportunity: {} profit=${:.2} age={}ms", 
                        opp.pair, opp.estimated_profit_usd, age_ms);

                    // Simulate first if enabled
                    if config.simulation_mode {
                        *metrics.opportunities_simulated.write() += 1;
                        match simulator.simulate(&opp).await {
                            Ok(sim_profit) => {
                                info!("Simulation result: ${:.2}", sim_profit);
                                if sim_profit < config.min_profit_usd {
                                    warn!("Simulation shows insufficient profit");
                                    continue;
                                }
                            }
                            Err(e) => {
                                error!("Simulation failed: {}", e);
                                continue;
                            }
                        }
                    }

                    // Execute trade
                    match executor.execute(&opp).await {
                        Ok(profit) => {
                            info!("Execution successful! Profit: ${:.2}", profit);
                            *metrics.opportunities_executed.write() += 1;
                            *metrics.total_profit.write() += profit;
                        }
                        Err(e) => {
                            error!("Execution failed: {}", e);
                            *metrics.failed_executions.write() += 1;
                        }
                    }
                }
            }
        })
    }

    async fn start_metrics_server(&self) {
        let metrics = self.metrics.clone();
        
        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(60));
            
            loop {
                interval.tick().await;
                
                let received = *metrics.opportunities_received.read();
                let executed = *metrics.opportunities_executed.read();
                let simulated = *metrics.opportunities_simulated.read();
                let failed = *metrics.failed_executions.read();
                let profit = *metrics.total_profit.read();
                
                info!("=== Metrics ===");
                info!("Opportunities received: {}", received);
                info!("Opportunities simulated: {}", simulated);
                info!("Opportunities executed: {}", executed);
                info!("Failed executions: {}", failed);
                info!("Total profit: ${:.2}", profit);
                info!("Success rate: {:.1}%", 
                    if executed + failed > 0 {
                        executed as f64 / (executed + failed) as f64 * 100.0
                    } else {
                        0.0
                    }
                );
            }
        });
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("capital_arb_bot=debug,info")
        .init();

    // Load configuration
    let config = Config::from_env()?;

    // Create and run bot
    let bot = CapitalArbBot::new(config).await?;
    bot.run().await?;

    Ok(())
}
