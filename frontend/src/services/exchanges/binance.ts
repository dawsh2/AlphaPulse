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
    try {
      // Use backend API instead of direct Binance API to avoid CORS
      const backendSymbol = symbol.replace('/', '-'); // Convert BTC/USD to BTC-USD for URL
      const response = await fetch(
        `http://localhost:8080/api/crypto-data/${backendSymbol}?exchange=coinbase&limit=${limit}`
      );
      
      if (!response.ok) {
        throw new Error(`Backend API error: ${response.statusText}`);
      }
      
      const result = await response.json();
      
      // Convert backend format to our MarketData format
      return result.data.map((item: any) => ({
        time: Math.floor(item.timestamp / 1000), // Convert ms to seconds if needed
        open: item.open,
        high: item.high,
        low: item.low,
        close: item.close,
        volume: item.volume
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