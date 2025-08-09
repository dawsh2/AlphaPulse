// Data fetcher for retrieving and storing historical market data
import { CoinbaseService } from '../exchanges/coinbase';
import { dataStorage } from './DataStorage';
import type { StoredMarketData } from './DataTypes';
import type { MarketData } from '../exchanges/types';

export class DataFetcher {
  private coinbase = new CoinbaseService();
  
  /**
   * Fetch and store historical data from Coinbase
   * @param symbol - Trading pair (e.g., 'BTC/USD')
   * @param days - Number of days to fetch (default 7)
   */
  async fetchAndStoreHistoricalData(
    symbol: string, 
    days: number = 7
  ): Promise<{ success: boolean; candleCount: number; error?: string }> {
    try {
      console.log(`[DataFetcher] Fetching ${days} days of ${symbol} from Coinbase...`);
      
      // Calculate time range
      const endTime = Date.now();
      const startTime = endTime - (days * 24 * 60 * 60 * 1000);
      const minutesNeeded = days * 24 * 60;
      
      // Coinbase API limits to 300 candles per request
      const maxCandlesPerRequest = 300;
      const requests = Math.ceil(minutesNeeded / maxCandlesPerRequest);
      
      const allCandles: StoredMarketData[] = [];
      let currentEnd = Math.floor(endTime / 1000);
      
      // Fetch in batches (going backwards in time)
      for (let i = 0; i < requests; i++) {
        const batchMinutes = Math.min(maxCandlesPerRequest, minutesNeeded - (i * maxCandlesPerRequest));
        const batchStart = currentEnd - (batchMinutes * 60);
        
        try {
          // Fetch batch from Coinbase REST API
          const response = await this.fetchCoinbaseBatch(symbol, batchStart, currentEnd);
          
          // Convert to storage format
          const storedCandles = response.map(candle => ({
            symbol,
            exchange: 'coinbase',
            interval: '1m',
            timestamp: candle.time,
            open: candle.open,
            high: candle.high,
            low: candle.low,
            close: candle.close,
            volume: candle.volume,
            metadata: {
              fetchedAt: Date.now(),
              source: 'coinbase-rest'
            }
          }));
          
          allCandles.push(...storedCandles);
          
          // Move to next batch
          currentEnd = batchStart;
          
          // Small delay to avoid rate limiting
          if (i < requests - 1) {
            await new Promise(resolve => setTimeout(resolve, 100));
          }
        } catch (error) {
          console.error(`[DataFetcher] Error fetching batch ${i + 1}/${requests}:`, error);
        }
      }
      
      // Sort by timestamp
      allCandles.sort((a, b) => a.timestamp - b.timestamp);
      
      // Store in IndexedDB
      console.log(`[DataFetcher] Storing ${allCandles.length} candles...`);
      await dataStorage.saveCandles(allCandles);
      
      // Also save to backend if available
      await this.saveToBackend(allCandles);
      
      return {
        success: true,
        candleCount: allCandles.length
      };
    } catch (error) {
      console.error('[DataFetcher] Error:', error);
      return {
        success: false,
        candleCount: 0,
        error: error instanceof Error ? error.message : 'Unknown error'
      };
    }
  }
  
  /**
   * Fetch a batch of candles from Coinbase REST API
   */
  private async fetchCoinbaseBatch(
    symbol: string, 
    startTime: number, 
    endTime: number
  ): Promise<MarketData[]> {
    const coinbaseSymbol = symbol.replace('/', '-');
    
    // Calculate granularity (60 = 1 minute)
    const start = new Date(startTime * 1000).toISOString();
    const end = new Date(endTime * 1000).toISOString();
    
    const response = await fetch(
      `https://api.exchange.coinbase.com/products/${coinbaseSymbol}/candles?` +
      `start=${start}&end=${end}&granularity=60`
    );
    
    if (!response.ok) {
      throw new Error(`Coinbase API error: ${response.statusText}`);
    }
    
    const data = await response.json();
    
    // Coinbase returns: [timestamp, low, high, open, close, volume]
    return data
      .reverse() // Coinbase returns in reverse chronological order
      .map((candle: number[]) => ({
        time: candle[0],
        open: candle[3],
        high: candle[2],
        low: candle[1],
        close: candle[4],
        volume: candle[5]
      }));
  }
  
  /**
   * Save data to backend catalog (if API is available)
   */
  private async saveToBackend(candles: StoredMarketData[]): Promise<void> {
    try {
      const response = await fetch('http://localhost:5000/api/market-data/save', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          symbol: candles[0]?.symbol,
          exchange: candles[0]?.exchange,
          interval: candles[0]?.interval,
          candles: candles.map(c => ({
            timestamp: c.timestamp,
            open: c.open,
            high: c.high,
            low: c.low,
            close: c.close,
            volume: c.volume
          }))
        })
      });
      
      if (response.ok) {
        console.log('[DataFetcher] Saved to backend catalog');
      }
    } catch (error) {
      console.log('[DataFetcher] Backend not available, using local storage only');
    }
  }
  
  /**
   * Check if we need to update data for a symbol
   */
  async needsUpdate(symbol: string, exchange: string = 'coinbase'): Promise<boolean> {
    const latest = await dataStorage.getLatestCandle(symbol, exchange);
    
    if (!latest) return true;
    
    // Check if data is older than 5 minutes
    const fiveMinutesAgo = Math.floor(Date.now() / 1000) - 300;
    return latest.timestamp < fiveMinutesAgo;
  }
  
  /**
   * Update data if needed (incremental update)
   */
  async updateIfNeeded(symbol: string, exchange: string = 'coinbase'): Promise<void> {
    if (await this.needsUpdate(symbol, exchange)) {
      console.log(`[DataFetcher] Updating ${symbol} data...`);
      
      const latest = await dataStorage.getLatestCandle(symbol, exchange);
      const startTime = latest ? latest.timestamp : Math.floor(Date.now() / 1000) - (7 * 24 * 60 * 60);
      const endTime = Math.floor(Date.now() / 1000);
      
      // Fetch only missing data
      const response = await this.fetchCoinbaseBatch(symbol, startTime, endTime);
      
      const storedCandles = response.map(candle => ({
        symbol,
        exchange,
        interval: '1m',
        timestamp: candle.time,
        open: candle.open,
        high: candle.high,
        low: candle.low,
        close: candle.close,
        volume: candle.volume,
        metadata: {
          fetchedAt: Date.now(),
          source: 'coinbase-rest'
        }
      }));
      
      await dataStorage.saveCandles(storedCandles);
      console.log(`[DataFetcher] Updated with ${storedCandles.length} new candles`);
    }
  }
}

// Singleton instance
export const dataFetcher = new DataFetcher();