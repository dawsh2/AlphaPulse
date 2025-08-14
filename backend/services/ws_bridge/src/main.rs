use alphapulse_protocol::*;
use zerocopy::FromBytes;
use anyhow::{Context, Result};
use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use metrics::{counter, gauge, histogram};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::os::unix::net::UnixStream;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, mpsc};
use tokio::task;
use tokio_tungstenite::{accept_async, tungstenite::Message};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

const WS_BIND_ADDR: &str = "127.0.0.1:8765";
const BROADCAST_BUFFER_SIZE: usize = 10000;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "msg_type")]
enum BroadcastMessage {
    #[serde(rename = "trade")]
    Trade {
        symbol: String,
        exchange: String,
        timestamp: u64,
        price: f64,
        volume: f64,
        side: String,
        data: TradeData,
    },
    #[serde(rename = "orderbook")]
    OrderBook {
        symbol: String,
        exchange: String,
        timestamp: u64,
        data: OrderBookData,
    },
}

#[derive(Debug, Clone, Serialize)]
struct TradeData {
    timestamp: u64,
    price: f64,
    volume: f64,
    side: String,
    symbol: String,
    exchange: String,
}

#[derive(Debug, Clone, Serialize)]
struct OrderBookData {
    bids: Vec<[f64; 2]>,
    asks: Vec<[f64; 2]>,
    timestamp: u64,
}

#[derive(Debug, Deserialize)]
struct ClientMessage {
    msg_type: String,
    #[serde(default)]
    channels: Vec<String>,
    #[serde(default)]
    symbols: Vec<String>,
}

struct BridgeServer {
    broadcast_tx: broadcast::Sender<BroadcastMessage>,
    symbol_mapper: Arc<RwLock<SymbolMapper>>,
}

impl BridgeServer {
    fn new() -> Self {
        let (broadcast_tx, _) = broadcast::channel(BROADCAST_BUFFER_SIZE);
        
        Self {
            broadcast_tx,
            symbol_mapper: Arc::new(RwLock::new(SymbolMapper::new())),
        }
    }

    async fn start(&self) -> Result<()> {
        let unix_reader_handle = self.spawn_unix_reader();
        let ws_server_handle = self.spawn_ws_server().await?;
        
        tokio::select! {
            result = unix_reader_handle => {
                error!("Unix reader exited: {:?}", result);
            }
            result = ws_server_handle => {
                error!("WebSocket server exited: {:?}", result);
            }
        }
        
        Ok(())
    }

    fn spawn_unix_reader(&self) -> task::JoinHandle<()> {
        let broadcast_tx = self.broadcast_tx.clone();
        let symbol_mapper = self.symbol_mapper.clone();
        
        task::spawn_blocking(move || {
            loop {
                // Connect to relay server instead of direct exchange socket
                let relay_socket_path = "/tmp/alphapulse/relay.sock";
                match UnixStream::connect(relay_socket_path) {
                    Ok(mut stream) => {
                        info!("Connected to relay server at {}", relay_socket_path);
                        
                        let mut buffer = vec![0u8; 65536];
                        let mut pending_data = Vec::new();
                        
                        loop {
                            use std::io::Read;
                            
                            match stream.read(&mut buffer) {
                                Ok(0) => {
                                    warn!("Unix socket closed");
                                    break;
                                }
                                Ok(n) => {
                                    pending_data.extend_from_slice(&buffer[..n]);
                                    
                                    while pending_data.len() >= MessageHeader::SIZE {
                                        let header = match MessageHeader::read_from_prefix(
                                            &pending_data[..MessageHeader::SIZE]
                                        ) {
                                            Some(h) => h,
                                            None => break,
                                        };
                                        
                                        if let Err(e) = header.validate() {
                                            error!("Invalid header: {}", e);
                                            pending_data.clear();
                                            break;
                                        }
                                        
                                        let total_size = MessageHeader::SIZE + header.get_length() as usize;
                                        
                                        if pending_data.len() < total_size {
                                            break;
                                        }
                                        
                                        let msg_data = &pending_data[MessageHeader::SIZE..total_size];
                                        
                                        match header.get_type() {
                                            Ok(MessageType::Trade) => {
                                                if let Some(trade) = TradeMessage::read_from_prefix(msg_data) {
                                                    Self::handle_trade(
                                                        &trade,
                                                        &broadcast_tx,
                                                        &symbol_mapper,
                                                    );
                                                }
                                            }
                                            Ok(MessageType::OrderBook) => {
                                                if let Ok(orderbook) = OrderBookMessage::decode(msg_data) {
                                                    Self::handle_orderbook(
                                                        &orderbook,
                                                        &broadcast_tx,
                                                        &symbol_mapper,
                                                    );
                                                }
                                            }
                                            Ok(MessageType::L2Snapshot) => {
                                                if let Ok(snapshot) = L2SnapshotMessage::decode(msg_data) {
                                                    Self::handle_l2_snapshot(
                                                        &snapshot,
                                                        &broadcast_tx,
                                                        &symbol_mapper,
                                                    );
                                                }
                                            }
                                            Ok(MessageType::L2Delta) => {
                                                if let Ok(delta) = L2DeltaMessage::decode(msg_data) {
                                                    Self::handle_l2_delta(
                                                        &delta,
                                                        &broadcast_tx,
                                                        &symbol_mapper,
                                                    );
                                                }
                                            }
                                            Ok(MessageType::Heartbeat) => {
                                                debug!("Received heartbeat");
                                            }
                                            _ => {}
                                        }
                                        
                                        pending_data.drain(..total_size);
                                        counter!("ws_bridge.messages_processed").increment(1);
                                    }
                                }
                                Err(e) => {
                                    error!("Unix socket read error: {}", e);
                                    break;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to connect to Unix socket: {}", e);
                    }
                }
                
                std::thread::sleep(Duration::from_secs(5));
            }
        })
    }

    fn handle_trade(
        trade: &TradeMessage,
        broadcast_tx: &broadcast::Sender<BroadcastMessage>,
        symbol_mapper: &RwLock<SymbolMapper>,
    ) {
        let mapper = symbol_mapper.read();
        let symbol = mapper.get_symbol(trade.symbol_id())
            .unwrap_or("UNKNOWN")
            .to_string();
        drop(mapper);
        
        let exchange = match trade.exchange_id() {
            1 => "kraken",
            2 => "coinbase",
            _ => "unknown",
        }.to_string();
        
        let side = match trade.side() {
            TradeSide::Buy => "buy",
            TradeSide::Sell => "sell",
            TradeSide::Unknown => "unknown",
        }.to_string();
        
        let msg = BroadcastMessage::Trade {
            symbol: symbol.clone(),
            exchange: exchange.clone(),
            timestamp: trade.timestamp_ns() / 1_000_000,
            price: trade.price_f64(),
            volume: trade.volume_f64(),
            side: side.clone(),
            data: TradeData {
                timestamp: trade.timestamp_ns() / 1_000_000,
                price: trade.price_f64(),
                volume: trade.volume_f64(),
                side,
                symbol,
                exchange,
            },
        };
        
        let _ = broadcast_tx.send(msg);
        histogram!("ws_bridge.trade_latency_us")
            .record(Instant::now().elapsed().as_micros() as f64);
    }

    fn handle_orderbook(
        orderbook: &OrderBookMessage,
        broadcast_tx: &broadcast::Sender<BroadcastMessage>,
        symbol_mapper: &RwLock<SymbolMapper>,
    ) {
        let mapper = symbol_mapper.read();
        let symbol = mapper.get_symbol(orderbook.symbol_id)
            .unwrap_or("UNKNOWN")
            .to_string();
        drop(mapper);
        
        let bids: Vec<[f64; 2]> = orderbook.bids.iter()
            .map(|level| [level.price_f64(), level.volume_f64()])
            .collect();
        
        let asks: Vec<[f64; 2]> = orderbook.asks.iter()
            .map(|level| [level.price_f64(), level.volume_f64()])
            .collect();
        
        let msg = BroadcastMessage::OrderBook {
            symbol,
            exchange: "kraken".to_string(),
            timestamp: orderbook.timestamp_ns / 1_000_000,
            data: OrderBookData {
                bids,
                asks,
                timestamp: orderbook.timestamp_ns / 1_000_000,
            },
        };
        
        let _ = broadcast_tx.send(msg);
    }

    fn handle_l2_snapshot(
        snapshot: &L2SnapshotMessage,
        broadcast_tx: &broadcast::Sender<BroadcastMessage>,
        symbol_mapper: &RwLock<SymbolMapper>,
    ) {
        let mapper = symbol_mapper.read();
        let symbol = mapper.get_symbol(snapshot.symbol_id)
            .unwrap_or("UNKNOWN")
            .to_string();
        drop(mapper);
        
        let bids: Vec<[f64; 2]> = snapshot.bids.iter()
            .map(|level| [level.price_f64(), level.volume_f64()])
            .collect();
        
        let asks: Vec<[f64; 2]> = snapshot.asks.iter()
            .map(|level| [level.price_f64(), level.volume_f64()])
            .collect();
        
        let exchange = match snapshot.exchange_id {
            1 => "kraken",
            2 => "coinbase",
            _ => "unknown",
        }.to_string();
        
        let msg = BroadcastMessage::OrderBook {
            symbol,
            exchange,
            timestamp: snapshot.timestamp_ns / 1_000_000,
            data: OrderBookData {
                bids,
                asks,
                timestamp: snapshot.timestamp_ns / 1_000_000,
            },
        };
        
        let _ = broadcast_tx.send(msg);
        info!("Sent L2 snapshot to clients");
    }

    fn handle_l2_delta(
        delta: &L2DeltaMessage,
        broadcast_tx: &broadcast::Sender<BroadcastMessage>,
        symbol_mapper: &RwLock<SymbolMapper>,
    ) {
        // For now, we'll skip delta messages as the frontend needs modification
        // to handle incremental updates. This is where you'd implement
        // delta application logic.
        debug!("Received L2 delta with {} updates", delta.updates.len());
    }

    async fn spawn_ws_server(&self) -> Result<task::JoinHandle<()>> {
        let addr: SocketAddr = WS_BIND_ADDR.parse()?;
        let listener = TcpListener::bind(&addr).await?;
        info!("WebSocket server listening on ws://{}/stream", addr);
        
        let broadcast_tx = self.broadcast_tx.clone();
        
        Ok(task::spawn(async move {
            while let Ok((stream, addr)) = listener.accept().await {
                let broadcast_tx = broadcast_tx.clone();
                task::spawn(handle_client(stream, addr, broadcast_tx));
            }
        }))
    }
}

async fn handle_client(
    stream: TcpStream,
    addr: SocketAddr,
    broadcast_tx: broadcast::Sender<BroadcastMessage>,
) {
    let ws_stream = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            error!("WebSocket handshake failed for {}: {}", addr, e);
            return;
        }
    };
    
    let client_id = Uuid::new_v4();
    info!("Client {} connected from {}", client_id, addr);
    
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    let mut broadcast_rx = broadcast_tx.subscribe();
    
    let (tx, mut rx) = mpsc::channel::<Message>(100);
    
    // Spawn task to forward messages to WebSocket
    let send_handle = task::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_sender.send(msg).await.is_err() {
                break;
            }
        }
    });
    
    // Spawn task to receive broadcast messages
    let tx_clone = tx.clone();
    let broadcast_handle = task::spawn(async move {
        while let Ok(msg) = broadcast_rx.recv().await {
            if let Ok(json) = serde_json::to_string(&msg) {
                if tx_clone.send(Message::Text(json)).await.is_err() {
                    break;
                }
            }
        }
    });
    
    // Handle incoming WebSocket messages
    while let Some(Ok(msg)) = ws_receiver.next().await {
        match msg {
            Message::Text(text) => {
                if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                    match client_msg.msg_type.as_str() {
                        "subscribe" => {
                            info!("Client {} subscribed to {:?}", client_id, client_msg.symbols);
                        }
                        "unsubscribe" => {
                            info!("Client {} unsubscribed", client_id);
                        }
                        _ => {}
                    }
                }
            }
            Message::Close(_) => break,
            Message::Ping(data) => {
                let _ = tx.send(Message::Pong(data)).await;
            }
            _ => {}
        }
    }
    
    broadcast_handle.abort();
    send_handle.abort();
    info!("Client {} disconnected", client_id);
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("ws_bridge=debug".parse()?)
                .add_directive("info".parse()?),
        )
        .init();

    info!("Starting WebSocket bridge service");
    
    let server = BridgeServer::new();
    
    tokio::select! {
        result = server.start() => {
            error!("Server exited: {:?}", result);
        }
        _ = tokio::signal::ctrl_c() => {
            info!("Shutting down WebSocket bridge");
        }
    }
    
    Ok(())
}