// Kraken Exchange WebSocket and REST API Integration
import type { MarketData, ExchangeService } from './types';

export class KrakenService implements ExchangeService {
  private ws: WebSocket | null = null;
  
  // Symbol mapping for Kraken
  private symbolMap: Record<string, string> = {
    'BTC/USD': 'BTC/USD',
    'ETH/USD': 'ETH/USD',
    'SOL/USD': 'SOL/USD'
  };
  
  // REST API symbol mapping (different format)
  private restSymbolMap: Record<string, string> = {
    'BTC/USD': 'XBTUSD',
    'ETH/USD': 'ETHUSD',
    'SOL/USD': 'SOLUSD'
  };

  connect(symbol: string, onData: (data: MarketData) => void): WebSocket | null {
    // Close existing connection
    this.disconnect();
    
    const ws = new WebSocket('wss://ws.kraken.com/v2');
    this.ws = ws;
    
    ws.onopen = () => {
      console.log('[Kraken] Connected to WebSocket');
      
      const subscribeMsg = {
        method: 'subscribe',
        params: {
          channel: 'ohlc',
          symbol: [this.symbolMap[symbol] || symbol],
          interval: 1 // 1-minute bars
        },
        req_id: Date.now()
      };
      
      ws.send(JSON.stringify(subscribeMsg));
    };
    
    ws.onmessage = (event) => {
      try {
        const message = JSON.parse(event.data);
        
        if (message.channel === 'ohlc' && message.data && message.data.length > 0) {
          message.data.forEach((ohlcData: any) => {
            const timestamp = Math.floor(new Date(ohlcData.timestamp).getTime() / 1000);
            const minuteTimestamp = Math.floor(timestamp / 60) * 60;
            
            const candle: MarketData = {
              time: minuteTimestamp,
              open: parseFloat(ohlcData.open),
              high: parseFloat(ohlcData.high),
              low: parseFloat(ohlcData.low),
              close: parseFloat(ohlcData.close),
              volume: parseFloat(ohlcData.volume)
            };
            
            if (this.validateCandle(candle)) {
              onData(candle);
            }
          });
        }
      } catch (error) {
        console.error('[Kraken] Error parsing message:', error);
      }
    };
    
    ws.onerror = (error) => {
      console.error('[Kraken] WebSocket error:', error);
    };
    
    ws.onclose = () => {
      console.log('[Kraken] WebSocket disconnected');
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
    const krakenPair = this.restSymbolMap[symbol] || symbol;
    
    try {
      const response = await fetch(
        `https://api.kraken.com/0/public/OHLC?pair=${krakenPair}&interval=1`
      );
      const data = await response.json();
      
      if (data.error && data.error.length > 0) {
        throw new Error(`Kraken API error: ${data.error.join(', ')}`);
      }
      
      const pairKey = Object.keys(data.result).find(k => k !== 'last');
      if (!pairKey) {
        throw new Error('No data found for symbol');
      }
      
      const ohlcData = data.result[pairKey];
      
      return ohlcData
        .slice(-limit)
        .map((candle: any[]) => ({
          time: parseInt(candle[0]),
          open: parseFloat(candle[1]),
          high: parseFloat(candle[2]),
          low: parseFloat(candle[3]),
          close: parseFloat(candle[4]),
          volume: parseFloat(candle[6])
        }))
        .filter(this.validateCandle);
    } catch (error) {
      console.error('[Kraken] Failed to fetch historical data:', error);
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