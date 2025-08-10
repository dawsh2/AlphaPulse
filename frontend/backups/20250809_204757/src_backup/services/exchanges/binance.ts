// Binance Exchange WebSocket and REST API Integration
import type { MarketData, ExchangeService } from './types';

export class BinanceService implements ExchangeService {
  private ws: WebSocket | null = null;
  
  // Symbol mapping for Binance (uses lowercase, no slash)
  private symbolMap: Record<string, string> = {
    'BTC/USD': 'btcusdt',  // Binance uses USDT pairs
    'BTC/USDT': 'btcusdt',
    'ETH/USD': 'ethusdt',
    'ETH/USDT': 'ethusdt',
    'SOL/USD': 'solusdt',
    'SOL/USDT': 'solusdt'
  };

  connect(symbol: string, onData: (data: MarketData) => void): WebSocket | null {
    // Close existing connection
    this.disconnect();
    
    const binanceSymbol = this.symbolMap[symbol] || symbol.toLowerCase().replace('/', '');
    const streamName = `${binanceSymbol}@kline_1m`; // 1-minute klines
    
    const ws = new WebSocket(`wss://stream.binance.com:9443/ws/${streamName}`);
    this.ws = ws;
    
    ws.onopen = () => {
      console.log('[Binance] Connected to WebSocket');
    };
    
    ws.onmessage = (event) => {
      try {
        const message = JSON.parse(event.data);
        
        if (message.k) { // Kline data
          const kline = message.k;
          
          const candle: MarketData = {
            time: Math.floor(kline.t / 1000), // Convert ms to seconds
            open: parseFloat(kline.o),
            high: parseFloat(kline.h),
            low: parseFloat(kline.l),
            close: parseFloat(kline.c),
            volume: parseFloat(kline.v)
          };
          
          if (this.validateCandle(candle)) {
            onData(candle);
          }
        }
      } catch (error) {
        console.error('[Binance] Error parsing message:', error);
      }
    };
    
    ws.onerror = (error) => {
      console.error('[Binance] WebSocket error:', error);
    };
    
    ws.onclose = () => {
      console.log('[Binance] WebSocket disconnected');
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
    const binanceSymbol = this.symbolMap[symbol] || symbol.toUpperCase().replace('/', '');
    
    try {
      // Binance REST API for klines
      const response = await fetch(
        `https://api.binance.com/api/v3/klines?symbol=${binanceSymbol.toUpperCase()}&interval=1m&limit=${limit}`
      );
      
      if (!response.ok) {
        throw new Error(`Binance API error: ${response.statusText}`);
      }
      
      const data = await response.json();
      
      return data.map((candle: any[]) => ({
        time: Math.floor(candle[0] / 1000), // Convert ms to seconds
        open: parseFloat(candle[1]),
        high: parseFloat(candle[2]),
        low: parseFloat(candle[3]),
        close: parseFloat(candle[4]),
        volume: parseFloat(candle[5])
      })).filter(this.validateCandle);
    } catch (error) {
      console.error('[Binance] Failed to fetch historical data:', error);
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