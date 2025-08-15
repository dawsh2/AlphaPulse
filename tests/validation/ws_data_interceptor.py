#!/usr/bin/env python3
"""
WebSocket Data Interceptor
Captures all messages from the WebSocket bridge for validation
"""

import asyncio
import json
import time
from datetime import datetime
from typing import Dict, List, Any, Optional
from dataclasses import dataclass, field, asdict
import websockets
import logging

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)


@dataclass
class CapturedMessage:
    """Represents a captured WebSocket message"""
    timestamp: float
    msg_type: str
    symbol: Optional[str]
    symbol_hash: Optional[str]
    price: Optional[float]
    volume: Optional[float]
    exchange: Optional[str]
    pair: Optional[str]
    raw_data: Dict[str, Any]
    
    @classmethod
    def from_ws_message(cls, raw_msg: str, capture_time: float) -> 'CapturedMessage':
        """Parse WebSocket message into structured format"""
        try:
            data = json.loads(raw_msg)
            msg_type = data.get('msg_type', 'unknown')
            
            # Parse symbol format (exchange:pair)
            symbol = data.get('symbol')
            exchange = None
            pair = None
            if symbol and ':' in symbol:
                parts = symbol.split(':')
                exchange = parts[0]
                pair = parts[1] if len(parts) > 1 else None
            
            return cls(
                timestamp=capture_time,
                msg_type=msg_type,
                symbol=symbol,
                symbol_hash=data.get('symbol_hash'),
                price=data.get('price'),
                volume=data.get('volume'),
                exchange=exchange,
                pair=pair,
                raw_data=data
            )
        except Exception as e:
            logger.error(f"Failed to parse message: {e}")
            return cls(
                timestamp=capture_time,
                msg_type='error',
                symbol=None,
                symbol_hash=None,
                price=None,
                volume=None,
                exchange=None,
                pair=None,
                raw_data={'error': str(e), 'raw': raw_msg}
            )


class WebSocketInterceptor:
    """Captures and stores WebSocket messages"""
    
    def __init__(self, ws_url: str = "ws://localhost:8765"):
        self.ws_url = ws_url
        self.messages: List[CapturedMessage] = []
        self.is_running = False
        self.start_time: Optional[float] = None
        self.stats = {
            'total_messages': 0,
            'trade_messages': 0,
            'orderbook_messages': 0,
            'symbol_mapping_messages': 0,
            'errors': 0,
            'unique_symbols': set(),
            'unique_exchanges': set()
        }
    
    async def connect_and_capture(self, duration: int = 60):
        """Connect to WebSocket and capture messages for specified duration"""
        self.start_time = time.time()
        self.is_running = True
        
        try:
            async with websockets.connect(self.ws_url) as websocket:
                logger.info(f"Connected to {self.ws_url}")
                
                # Set up timeout for capture duration
                end_time = self.start_time + duration
                
                while self.is_running and time.time() < end_time:
                    try:
                        # Wait for message with timeout
                        remaining = end_time - time.time()
                        if remaining <= 0:
                            break
                            
                        message = await asyncio.wait_for(
                            websocket.recv(), 
                            timeout=min(remaining, 1.0)
                        )
                        
                        # Capture and parse message
                        capture_time = time.time()
                        captured = CapturedMessage.from_ws_message(message, capture_time)
                        self.messages.append(captured)
                        
                        # Update statistics
                        self._update_stats(captured)
                        
                        # Log every 100th message
                        if len(self.messages) % 100 == 0:
                            logger.info(f"Captured {len(self.messages)} messages")
                            
                    except asyncio.TimeoutError:
                        continue
                    except Exception as e:
                        logger.error(f"Error receiving message: {e}")
                        self.stats['errors'] += 1
                        
        except Exception as e:
            logger.error(f"WebSocket connection error: {e}")
            raise
        finally:
            self.is_running = False
            elapsed = time.time() - self.start_time
            logger.info(f"Capture complete: {len(self.messages)} messages in {elapsed:.2f}s")
    
    def _update_stats(self, msg: CapturedMessage):
        """Update capture statistics"""
        self.stats['total_messages'] += 1
        
        if msg.msg_type == 'trade':
            self.stats['trade_messages'] += 1
        elif msg.msg_type == 'orderbook':
            self.stats['orderbook_messages'] += 1
        elif msg.msg_type == 'symbol_mapping':
            self.stats['symbol_mapping_messages'] += 1
        elif msg.msg_type == 'error':
            self.stats['errors'] += 1
        
        if msg.symbol:
            self.stats['unique_symbols'].add(msg.symbol)
        if msg.exchange:
            self.stats['unique_exchanges'].add(msg.exchange)
    
    def get_messages_by_symbol(self, symbol: str) -> List[CapturedMessage]:
        """Get all messages for a specific symbol"""
        return [msg for msg in self.messages if msg.symbol == symbol]
    
    def get_messages_by_type(self, msg_type: str) -> List[CapturedMessage]:
        """Get all messages of a specific type"""
        return [msg for msg in self.messages if msg.msg_type == msg_type]
    
    def get_price_series(self, symbol: str) -> List[tuple[float, float]]:
        """Get time series of prices for a symbol"""
        series = []
        for msg in self.messages:
            if msg.symbol == symbol and msg.price is not None:
                series.append((msg.timestamp, msg.price))
        return sorted(series, key=lambda x: x[0])
    
    def save_to_file(self, filepath: str):
        """Save captured messages to JSON file"""
        data = {
            'capture_info': {
                'start_time': self.start_time,
                'duration': time.time() - self.start_time if self.start_time else 0,
                'ws_url': self.ws_url,
                'total_messages': len(self.messages)
            },
            'statistics': {
                **self.stats,
                'unique_symbols': list(self.stats['unique_symbols']),
                'unique_exchanges': list(self.stats['unique_exchanges'])
            },
            'messages': [asdict(msg) for msg in self.messages]
        }
        
        with open(filepath, 'w') as f:
            json.dump(data, f, indent=2, default=str)
        logger.info(f"Saved {len(self.messages)} messages to {filepath}")
    
    def load_from_file(self, filepath: str):
        """Load captured messages from JSON file"""
        with open(filepath, 'r') as f:
            data = json.load(f)
        
        self.messages = []
        for msg_data in data['messages']:
            self.messages.append(CapturedMessage(**msg_data))
        
        logger.info(f"Loaded {len(self.messages)} messages from {filepath}")
        return data.get('capture_info', {}), data.get('statistics', {})
    
    def get_summary(self) -> Dict[str, Any]:
        """Get summary of captured data"""
        if not self.messages:
            return {'error': 'No messages captured'}
        
        price_ranges = {}
        for symbol in self.stats['unique_symbols']:
            prices = [msg.price for msg in self.messages 
                     if msg.symbol == symbol and msg.price is not None]
            if prices:
                price_ranges[symbol] = {
                    'min': min(prices),
                    'max': max(prices),
                    'avg': sum(prices) / len(prices),
                    'count': len(prices)
                }
        
        return {
            'total_messages': len(self.messages),
            'duration': time.time() - self.start_time if self.start_time else 0,
            'messages_per_second': len(self.messages) / (time.time() - self.start_time) if self.start_time else 0,
            'statistics': {
                **self.stats,
                'unique_symbols': list(self.stats['unique_symbols']),
                'unique_exchanges': list(self.stats['unique_exchanges'])
            },
            'price_ranges': price_ranges
        }


async def main():
    """Example usage"""
    interceptor = WebSocketInterceptor()
    
    # Capture for 30 seconds
    await interceptor.connect_and_capture(duration=30)
    
    # Print summary
    summary = interceptor.get_summary()
    print(json.dumps(summary, indent=2, default=str))
    
    # Save to file
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    interceptor.save_to_file(f"captured_ws_data_{timestamp}.json")
    
    # Example: Get all QuickSwap WETH-USDC trades
    weth_usdc_trades = interceptor.get_messages_by_symbol("quickswap:WETH-USDC")
    print(f"\nFound {len(weth_usdc_trades)} WETH-USDC messages")
    
    # Get price series
    price_series = interceptor.get_price_series("quickswap:WETH-USDC")
    if price_series:
        print(f"Price range: ${price_series[0][1]:.2f} - ${price_series[-1][1]:.2f}")


if __name__ == "__main__":
    asyncio.run(main())