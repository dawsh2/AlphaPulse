// Coinbase Exchange WebSocket and REST API Integration
import type { MarketData, ExchangeService } from './types';

export class CoinbaseService implements ExchangeService {
  private ws: WebSocket | null = null;
  
  // Symbol mapping for Coinbase
  private symbolMap: Record<string, string> = {
    'BTC/USD': 'BTC-USD',
    'ETH/USD': 'ETH-USD',
    'SOL/USD': 'SOL-USD',
    'BTC/USDT': 'BTC-USDT',
    'ETH/USDT': 'ETH-USDT'
  };

  connect(symbol: string, onData: (data: MarketData) => void): WebSocket | null {
    // Close existing connection
    this.disconnect();
    
    const coinbaseSymbol = this.symbolMap[symbol] || symbol.replace('/', '-');
    
    // Using Coinbase Exchange WebSocket Feed
    const ws = new WebSocket('wss://ws-feed.exchange.coinbase.com');
    this.ws = ws;
    
    ws.onopen = () => {
      console.log('[Coinbase] Connected to WebSocket');
      
      // Subscribe to matches channel for real-time trades
      // and ticker channel for price updates
      const subscribeMsg = {
        type: 'subscribe',
        product_ids: [coinbaseSymbol],
        channels: ['ticker', 'matches']
      };
      
      ws.send(JSON.stringify(subscribeMsg));
    };
    
    // Track candle building
    let currentCandle: MarketData | null = null;
    let lastMinute = 0;
    
    ws.onmessage = (event) => {
      try {
        const message = JSON.parse(event.data);
        
        if (message.type === 'ticker' || message.type === 'match') {
          const price = parseFloat(message.price);
          const size = parseFloat(message.size || message.last_size || 0);
          const timestamp = Math.floor(new Date(message.time).getTime() / 1000);
          const minute = Math.floor(timestamp / 60) * 60;
          
          // Start new candle on new minute
          if (!currentCandle || minute > lastMinute) {
            // Send previous candle if exists
            if (currentCandle && this.validateCandle(currentCandle)) {
              onData(currentCandle);
            }
            
            // For a new minute, we should ideally fetch the actual OHLC from REST API
            // But for real-time updates, we'll build from trades
            // The first trade of a minute is the open, not all OHLC values
            currentCandle = {
              time: minute,
              open: price,
              high: price,
              low: price,
              close: price,
              volume: size
            };
            lastMinute = minute;
            
            // Note: The first candle when connecting might be incomplete
            // The frontend should fetch recent complete candles via REST API first
          } else {
            // Update current candle - this properly tracks OHLC
            currentCandle.high = Math.max(currentCandle.high, price);
            currentCandle.low = Math.min(currentCandle.low, price);
            currentCandle.close = price;
            currentCandle.volume += size;
            
            // Send update for current candle
            if (this.validateCandle(currentCandle)) {
              onData({ ...currentCandle });
            }
          }
        }
      } catch (error) {
        console.error('[Coinbase] Error parsing message:', error);
      }
    };
    
    ws.onerror = (error) => {
      console.error('[Coinbase] WebSocket error:', error);
    };
    
    ws.onclose = () => {
      console.log('[Coinbase] WebSocket disconnected');
    };
    
    return ws;
  }
  
  disconnect(): void {
    if (this.ws) {
      if (this.ws.readyState === WebSocket.OPEN || 
          this.ws.readyState === WebSocket.CONNECTING) {
        this.ws.close();
      }
      this.ws = null;
    }
  }
  
  async fetchHistoricalData(symbol: string, limit: number = 30): Promise<MarketData[]> {
    const coinbaseSymbol = this.symbolMap[symbol] || symbol.replace('/', '-');
    
    try {
      // Calculate time range (last 30 minutes)
      const end = new Date();
      const start = new Date(end.getTime() - limit * 60 * 1000);
      
      // Coinbase REST API for candles
      // Granularity 60 = 1 minute
      const response = await fetch(
        `https://api.exchange.coinbase.com/products/${coinbaseSymbol}/candles?` +
        `start=${start.toISOString()}&end=${end.toISOString()}&granularity=60`
      );
      
      if (!response.ok) {
        throw new Error(`Coinbase API error: ${response.statusText}`);
      }
      
      const data = await response.json();
      
      // Coinbase returns data in reverse chronological order
      // Format: [timestamp, low, high, open, close, volume]
      return data
        .reverse()
        .map((candle: number[]) => ({
          time: candle[0],
          open: candle[3],
          high: candle[2],
          low: candle[1],
          close: candle[4],
          volume: candle[5]
        }))
        .filter(this.validateCandle);
    } catch (error) {
      console.error('[Coinbase] Failed to fetch historical data:', error);
      throw error;
    }
  }
  
  validateCandle(candle: MarketData): boolean {
    return candle.open > 0 && 
           candle.close > 0 &&
           candle.high >= candle.low &&
           candle.high >= candle.open &&
           candle.high >= candle.close &&
           candle.low <= candle.open &&
           candle.low <= candle.close;
  }
}