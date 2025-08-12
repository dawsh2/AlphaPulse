#!/usr/bin/env python3
"""
WebSocket Trade Recorder Service
Records live trades from Coinbase and Kraken to build historical tick database
"""

import asyncio
import websockets
import json
import time
import pandas as pd
from datetime import datetime
from typing import Dict, Any, List
import logging
import signal
import sys
from db_manager import get_db_manager

# Setup logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)

class TradeRecorder:
    """Base class for trade recording"""
    
    def __init__(self, exchange: str):
        self.exchange = exchange
        self.trades_buffer = []
        self.buffer_size = 100  # Batch insert every 100 trades
        self.stats = {
            'trades_received': 0,
            'trades_saved': 0,
            'errors': 0,
            'start_time': time.time()
        }
        self.running = True
        self.db_manager = get_db_manager()
        
    def save_trades(self, trades: List[Dict[str, Any]]):
        """Save trades using the database manager"""
        if not trades:
            return
        
        try:
            # Queue trades for async insertion
            self.db_manager.queue_trades(trades)
            
            saved_count = len(trades)
            self.stats['trades_saved'] += saved_count
            
            logger.info(f"{self.exchange}: Queued {saved_count} trades for database insertion")
            
        except Exception as e:
            logger.error(f"{self.exchange}: Error queueing trades: {e}")
            self.stats['errors'] += 1
    
    def add_trade(self, trade: Dict[str, Any]):
        """Add trade to buffer and save if buffer is full"""
        self.trades_buffer.append(trade)
        self.stats['trades_received'] += 1
        
        if len(self.trades_buffer) >= self.buffer_size:
            self.save_trades(self.trades_buffer)
            self.trades_buffer = []
    
    def flush_buffer(self):
        """Force save any buffered trades"""
        if self.trades_buffer:
            self.save_trades(self.trades_buffer)
            self.trades_buffer = []
    
    def print_stats(self):
        """Print current statistics"""
        runtime = time.time() - self.stats['start_time']
        rate = self.stats['trades_received'] / runtime if runtime > 0 else 0
        
        print(f"\n{self.exchange.upper()} Statistics:")
        print(f"  Trades received: {self.stats['trades_received']:,}")
        print(f"  Trades saved: {self.stats['trades_saved']:,}")
        print(f"  Errors: {self.stats['errors']}")
        print(f"  Rate: {rate:.1f} trades/sec")
        print(f"  Runtime: {runtime/60:.1f} minutes")


class CoinbaseRecorder(TradeRecorder):
    """Records trades from Coinbase WebSocket"""
    
    def __init__(self, symbols: List[str] = ['BTC-USD', 'ETH-USD']):
        super().__init__('coinbase')
        self.symbols = symbols
        self.ws_url = 'wss://ws-feed.exchange.coinbase.com'
    
    async def connect_and_record(self):
        """Connect to Coinbase WebSocket and record trades"""
        try:
            async with websockets.connect(self.ws_url) as websocket:
                # Subscribe to matches channel
                subscribe_msg = {
                    "type": "subscribe",
                    "channels": [
                        {
                            "name": "matches",
                            "product_ids": self.symbols
                        }
                    ]
                }
                
                await websocket.send(json.dumps(subscribe_msg))
                logger.info(f"Coinbase: Subscribed to {self.symbols}")
                
                # Listen for trades
                while self.running:
                    try:
                        message = await asyncio.wait_for(websocket.recv(), timeout=30)
                        data = json.loads(message)
                        
                        if data.get('type') == 'match':
                            # Process trade
                            trade = {
                                'timestamp': pd.Timestamp(data['time']).timestamp(),
                                'datetime': pd.Timestamp(data['time']),
                                'symbol': data['product_id'].replace('-', '/'),
                                'exchange': 'coinbase',
                                'price': float(data['price']),
                                'size': float(data['size']),
                                'side': data['side'],
                                'trade_id': str(data['trade_id'])
                            }
                            
                            self.add_trade(trade)
                            
                            # Log every 100th trade
                            if self.stats['trades_received'] % 100 == 0:
                                logger.info(f"Coinbase: {self.stats['trades_received']} trades received")
                    
                    except asyncio.TimeoutError:
                        logger.debug("Coinbase: No message received in 30s (normal)")
                    except Exception as e:
                        logger.error(f"Coinbase: Error processing message: {e}")
                        self.stats['errors'] += 1
        
        except Exception as e:
            logger.error(f"Coinbase: Connection error: {e}")
            self.stats['errors'] += 1
        
        finally:
            self.flush_buffer()


class KrakenRecorder(TradeRecorder):
    """Records trades from Kraken WebSocket"""
    
    def __init__(self, pairs: List[str] = ['XBT/USD', 'ETH/USD']):
        super().__init__('kraken')
        self.pairs = pairs
        self.ws_url = 'wss://ws.kraken.com'
    
    async def connect_and_record(self):
        """Connect to Kraken WebSocket and record trades"""
        try:
            async with websockets.connect(self.ws_url) as websocket:
                # Subscribe to trade channel
                for pair in self.pairs:
                    subscribe_msg = {
                        "event": "subscribe",
                        "pair": [pair],
                        "subscription": {
                            "name": "trade"
                        }
                    }
                    
                    await websocket.send(json.dumps(subscribe_msg))
                
                logger.info(f"Kraken: Subscribed to {self.pairs}")
                
                # Listen for trades
                while self.running:
                    try:
                        message = await asyncio.wait_for(websocket.recv(), timeout=30)
                        data = json.loads(message)
                        
                        # Log subscription confirmations and heartbeats
                        if isinstance(data, dict):
                            if data.get('event') == 'subscriptionStatus':
                                logger.info(f"Kraken subscription status: {data}")
                            elif data.get('event') == 'heartbeat':
                                logger.debug("Kraken: Heartbeat received")
                            else:
                                logger.debug(f"Kraken: Other message type: {data}")
                        
                        # Kraken sends trades as arrays
                        elif isinstance(data, list) and len(data) >= 3:
                            channel_id = data[0]
                            trades = data[1]
                            channel_name = data[2]
                            pair = data[3] if len(data) > 3 else 'XBT/USD'
                            
                            logger.debug(f"Kraken: Array message - channel: {channel_name}, pair: {pair}, trades count: {len(trades) if isinstance(trades, list) else 'N/A'}")
                            
                            if channel_name == 'trade':
                                for idx, trade_data in enumerate(trades):
                                    # trade_data format: [price, volume, time, side, type, misc]
                                    # Generate unique trade ID using timestamp and index
                                    timestamp = float(trade_data[2])
                                    # Create unique ID with microsecond precision and index
                                    trade_id = f"kraken_{int(timestamp * 1000000)}_{idx}"
                                    
                                    trade = {
                                        'timestamp': timestamp,
                                        'datetime': pd.Timestamp(timestamp, unit='s'),
                                        'symbol': pair.replace('XBT', 'BTC'),
                                        'exchange': 'kraken',
                                        'price': float(trade_data[0]),
                                        'size': float(trade_data[1]),
                                        'side': 'buy' if trade_data[3] == 'b' else 'sell',
                                        'trade_id': trade_id
                                    }
                                    
                                    self.add_trade(trade)
                                
                                # Log every 100th trade
                                if self.stats['trades_received'] % 100 == 0:
                                    logger.info(f"Kraken: {self.stats['trades_received']} trades received")
                    
                    except asyncio.TimeoutError:
                        logger.debug("Kraken: No message received in 30s (normal)")
                    except Exception as e:
                        logger.error(f"Kraken: Error processing message: {e}")
                        logger.debug(f"Kraken: Message that caused error: {message[:200] if 'message' in locals() else 'No message'}")
                        self.stats['errors'] += 1
        
        except Exception as e:
            logger.error(f"Kraken: Connection error: {e}")
            self.stats['errors'] += 1
        
        finally:
            self.flush_buffer()


class MultiExchangeRecorder:
    """Manages multiple exchange recorders"""
    
    def __init__(self):
        self.recorders = {
            'coinbase': CoinbaseRecorder(['BTC-USD', 'ETH-USD']),
            'kraken': KrakenRecorder(['XBT/USD', 'ETH/USD'])
        }
        self.running = True
    
    async def run_all(self):
        """Run all recorders concurrently"""
        tasks = []
        
        for name, recorder in self.recorders.items():
            logger.info(f"Starting {name} recorder...")
            tasks.append(asyncio.create_task(recorder.connect_and_record()))
        
        # Print stats periodically
        stats_task = asyncio.create_task(self.print_stats_loop())
        tasks.append(stats_task)
        
        # Wait for all tasks
        await asyncio.gather(*tasks)
    
    async def print_stats_loop(self):
        """Print statistics every 30 seconds"""
        while self.running:
            await asyncio.sleep(30)
            
            print("\n" + "=" * 60)
            print(f"TRADE RECORDER STATS - {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
            print("=" * 60)
            
            for recorder in self.recorders.values():
                recorder.print_stats()
    
    def stop(self):
        """Stop all recorders"""
        self.running = False
        for recorder in self.recorders.values():
            recorder.running = False
            recorder.flush_buffer()
        
        print("\n" + "=" * 60)
        print("FINAL STATISTICS")
        print("=" * 60)
        
        for recorder in self.recorders.values():
            recorder.print_stats()


def main():
    """Main entry point"""
    print("=" * 60)
    print("WEBSOCKET TRADE RECORDER")
    print("=" * 60)
    print("\nRecording trades from:")
    print("  - Coinbase: BTC-USD, ETH-USD")
    print("  - Kraken: BTC/USD, ETH/USD")
    print("\nPress Ctrl+C to stop recording")
    print("-" * 60)
    
    # Create recorder
    recorder = MultiExchangeRecorder()
    
    # Handle shutdown
    def signal_handler(sig, frame):
        print("\n\nShutting down...")
        recorder.stop()
        sys.exit(0)
    
    signal.signal(signal.SIGINT, signal_handler)
    
    # Run async event loop
    try:
        asyncio.run(recorder.run_all())
    except KeyboardInterrupt:
        print("\nShutdown requested")
        recorder.stop()


if __name__ == "__main__":
    main()