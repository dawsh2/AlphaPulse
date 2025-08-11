/**
 * ChartManager Service
 * Manages multiple chart instances and their data connections
 */

import { exchangeManager } from '../exchanges';
import type { MarketData, ExchangeType } from '../exchanges';
import { dataStorage, dataFetcher } from '../data';

export interface ChartInstance {
  id: string;
  symbol: string;
  exchange: ExchangeType;
  timeframe: string;
  ws?: WebSocket;
  marketData: MarketData[];
  isLoadingData: boolean;
  livePrice: number | null;
}

class ChartManager {
  private charts: Map<string, ChartInstance> = new Map();
  private dataUpdateCallbacks: Map<string, (data: MarketData[]) => void> = new Map();
  private livePriceCallbacks: Map<string, (price: number) => void> = new Map();
  private liveDataBuffer: Map<string, MarketData[]> = new Map();
  private lastStorageTime: Map<string, number> = new Map();

  /**
   * Create a new chart instance
   */
  createChart(
    id: string,
    symbol: string,
    exchange: ExchangeType,
    timeframe: string
  ): ChartInstance {
    // Check if chart already exists
    const existingChart = this.charts.get(id);
    if (existingChart) {
      // Update existing chart instead of creating duplicate
      if (existingChart.symbol !== symbol || 
          existingChart.exchange !== exchange || 
          existingChart.timeframe !== timeframe) {
        // Close old WebSocket if config changed
        if (existingChart.ws) {
          existingChart.ws.close();
          existingChart.ws = undefined;
        }
        // Update config
        existingChart.symbol = symbol;
        existingChart.exchange = exchange;
        existingChart.timeframe = timeframe;
        existingChart.marketData = [];
        existingChart.isLoadingData = false;
        existingChart.livePrice = null;
        this.loadDataForChart(id);
      }
      return existingChart;
    }

    const chart: ChartInstance = {
      id,
      symbol,
      exchange,
      timeframe,
      marketData: [],
      isLoadingData: false,
      livePrice: null
    };

    this.charts.set(id, chart);
    this.loadDataForChart(id);
    return chart;
  }

  /**
   * Remove a chart instance
   */
  removeChart(id: string): void {
    console.log(`[ChartManager] removeChart called for chartId=${id}`);
    console.trace('[ChartManager] removeChart call stack');
    const chart = this.charts.get(id);
    if (chart) {
      // Disconnect WebSocket if exists
      if (chart.ws) {
        if (chart.ws.readyState === WebSocket.OPEN || 
            chart.ws.readyState === WebSocket.CONNECTING) {
          chart.ws.close();
        }
      }
      this.charts.delete(id);
      this.dataUpdateCallbacks.delete(id);
      this.livePriceCallbacks.delete(id);
      console.log(`[ChartManager] Chart ${id} removed. Remaining charts: ${this.charts.size}`);
    }
  }

  /**
   * Update chart configuration
   */
  async updateChart(
    id: string,
    updates: Partial<Pick<ChartInstance, 'symbol' | 'exchange' | 'timeframe'>>
  ): Promise<void> {
    const chart = this.charts.get(id);
    if (!chart) return;

    // Check if we need to reload data
    const needsReload = 
      updates.symbol !== undefined && updates.symbol !== chart.symbol ||
      updates.exchange !== undefined && updates.exchange !== chart.exchange ||
      updates.timeframe !== undefined && updates.timeframe !== chart.timeframe;

    // Update chart config
    Object.assign(chart, updates);

    if (needsReload) {
      // Disconnect old WebSocket
      if (chart.ws) {
        chart.ws.close();
        chart.ws = undefined;
      }

      // Reload data
      await this.loadDataForChart(id);
    }
  }

  /**
   * Load historical and live data for a chart
   */
  private async loadDataForChart(chartId: string): Promise<void> {
    const chart = this.charts.get(chartId);
    if (!chart) return;

    chart.isLoadingData = true;
    // Don't notify here - wait until we have actual data to avoid race condition

    try {
      // Set the exchange
      exchangeManager.setExchange(chart.exchange);
      const service = exchangeManager.getService();
      
      if (!service) {
        console.error('No exchange service available for', chart.exchange);
        chart.isLoadingData = false;
        this.notifyDataUpdate(chartId);
        return;
      }

      // Try to load from backend first
      console.log(`[ChartManager] Loading ${chart.symbol} from ${chart.exchange}...`);
      
      // First try to get data from backend
      try {
        const response = await fetch(`http://localhost:5002/api/crypto-data/${chart.symbol}?exchange=${chart.exchange}`);
        if (response.ok) {
          const backendData = await response.json();
          if (backendData && backendData.data && backendData.data.length > 0) {
            console.log(`[ChartManager] Loaded ${backendData.data.length} candles from backend`);
            
            // Convert backend data to MarketData format
            const backendCandles = backendData.data
              .sort((a: any, b: any) => a.timestamp - b.timestamp)
              .map((candle: any) => ({
                time: candle.timestamp,
                open: candle.open,
                high: candle.high,
                low: candle.low,
                close: candle.close,
                volume: candle.volume
              }));
            
            chart.marketData = backendCandles;
            
            console.log(`[ChartManager] Set chart data with ${backendCandles.length} candles`);
            
            // Check if we need to fetch recent data to fill the gap
            // Use reduce instead of Math.max with spread to avoid call stack issues
            const latestBackendTime = backendCandles.reduce((max, candle) => Math.max(max, candle.time), 0);
            const currentTime = Math.floor(Date.now() / 1000);
            const timeGapMinutes = (currentTime - latestBackendTime) / 60;
            
            console.log(`[ChartManager] Gap analysis: latest=${latestBackendTime}, current=${currentTime}, gap=${timeGapMinutes.toFixed(1)}min`);
            
            if (timeGapMinutes > 5) { // If gap is more than 5 minutes
              console.log(`[ChartManager] Gap of ${timeGapMinutes.toFixed(1)} minutes detected, fetching recent data...`);
              
              // Fetch recent data from API to fill the gap
              if ((chart.exchange === 'coinbase' || chart.exchange === 'kraken') && 
                  ['BTC/USD', 'ETH/USD', 'SOL/USD', 'LINK/USD'].includes(chart.symbol)) {
                
                // Fetch only the gap period (in days)
                const gapDays = Math.min(Math.ceil(timeGapMinutes / (24 * 60)), 1); // Max 1 day
                const result = await dataFetcher.fetchAndStoreHistoricalData(
                  chart.symbol, 
                  gapDays, 
                  chart.exchange
                );
                
                if (result.success) {
                  console.log(`[ChartManager] Fetched ${result.candleCount} gap candles`);
                  
                  // Reload data from backend to get the updated dataset
                  const updatedResponse = await fetch(`http://localhost:5002/api/crypto-data/${chart.symbol}?exchange=${chart.exchange}`);
                  if (updatedResponse.ok) {
                    const updatedData = await updatedResponse.json();
                    if (updatedData && updatedData.data && updatedData.data.length > 0) {
                      chart.marketData = updatedData.data
                        .sort((a: any, b: any) => a.timestamp - b.timestamp)
                        .map((candle: any) => ({
                          time: candle.timestamp,
                          open: candle.open,
                          high: candle.high,
                          low: candle.low,
                          close: candle.close,
                          volume: candle.volume
                        }));
                      console.log(`[ChartManager] Updated with ${chart.marketData.length} total candles`);
                    }
                  }
                }
              }
            }
            
            console.log(`[ChartManager] About to notify data update with ${chart.marketData.length} candles`);
            chart.isLoadingData = false;
            this.notifyDataUpdate(chartId);
            console.log(`[ChartManager] Data update notification sent`);
            
            // Connect WebSocket for live updates
            this.connectWebSocket(chartId);
            return;
          }
        }
      } catch (error) {
        console.log(`[ChartManager] Backend fetch failed, trying local storage:`, error);
      }
      
      // Fallback to local storage
      const cachedData = await dataStorage.queryCandles({
        symbol: chart.symbol,
        exchange: chart.exchange,
        interval: '1m',
        limit: 10000
      });
      
      if (cachedData.length > 0) {
        // Convert to MarketData format
        chart.marketData = cachedData
          .sort((a, b) => a.timestamp - b.timestamp)
          .map(candle => ({
            time: candle.timestamp,
            open: candle.open,
            high: candle.high,
            low: candle.low,
            close: candle.close,
            volume: candle.volume
          }));
        
        console.log(`[ChartManager] Loaded ${chart.marketData.length} candles from local storage`);
        chart.isLoadingData = false;
        this.notifyDataUpdate(chartId);
      } else {
        // No cached data, fetch from exchange
        console.log(`[ChartManager] No cached data, fetching from API...`);
        
        if ((chart.exchange === 'coinbase' || chart.exchange === 'kraken') && 
            ['BTC/USD', 'ETH/USD', 'SOL/USD', 'LINK/USD'].includes(chart.symbol)) {
          const result = await dataFetcher.fetchAndStoreHistoricalData(
            chart.symbol, 
            7, 
            chart.exchange
          );
          
          if (result.success) {
            console.log(`[ChartManager] Fetch complete, loading ${result.candleCount} candles from storage...`);
            // Load the newly fetched data
            const newData = await dataStorage.queryCandles({
              symbol: chart.symbol,
              exchange: chart.exchange,
              interval: '1m',
              limit: 10000
            });
            
            console.log(`[ChartManager] Loaded ${newData.length} candles from storage`);
            
            chart.marketData = newData
              .sort((a, b) => a.timestamp - b.timestamp)
              .map(candle => ({
                time: candle.timestamp,
                open: candle.open,
                high: candle.high,
                low: candle.low,
                close: candle.close,
                volume: candle.volume
              }));
              
            console.log(`[ChartManager] Chart data ready with ${chart.marketData.length} candles`);
            // Notify immediately after data is ready
            chart.isLoadingData = false;
            this.notifyDataUpdate(chartId);
          } else {
            console.error(`[ChartManager] Failed to fetch data: ${result.error}`);
            chart.isLoadingData = false;
            this.notifyDataUpdate(chartId);
          }
        } else {
          // Fallback to regular API fetch
          const historicalData = await service.fetchHistoricalData(chart.symbol, 30);
          chart.marketData = historicalData;
          chart.isLoadingData = false;
          this.notifyDataUpdate(chartId);
        }
      }
      
      // Connect WebSocket for live updates
      this.connectWebSocket(chartId);
      
    } catch (error) {
      console.error(`[ChartManager] Failed to load data for ${chart.symbol}:`, error);
      chart.isLoadingData = false;
      this.notifyDataUpdate(chartId);
    }
  }

  /**
   * Connect WebSocket for live data
   */
  private connectWebSocket(chartId: string): void {
    const chart = this.charts.get(chartId);
    if (!chart) return;

    // Don't create duplicate connections
    if (chart.ws && (chart.ws.readyState === WebSocket.CONNECTING || 
                     chart.ws.readyState === WebSocket.OPEN)) {
      console.log(`[ChartManager] WebSocket already exists for ${chartId}`);
      return;
    }

    const service = exchangeManager.getService();
    if (!service) return;

    let lastUpdateTime = 0;
    
    try {
      const ws = service.connect(chart.symbol, (newCandle: MarketData) => {
        // Update live price immediately
        chart.livePrice = newCandle.close;
        this.notifyLivePriceUpdate(chartId, newCandle.close);
        
        // Throttle chart updates to once per second
        const now = Date.now();
        if (now - lastUpdateTime < 1000) {
          return;
        }
        lastUpdateTime = now;
        
        // Update market data
        const existingIndex = chart.marketData.findIndex(d => d.time === newCandle.time);
        
        if (existingIndex >= 0) {
          chart.marketData[existingIndex] = newCandle;
        } else {
          chart.marketData.push(newCandle);
          // Keep a reasonable amount of data in memory
          if (chart.marketData.length > 10000) {
            chart.marketData.shift();
          }
        }
        
        // Store live data to backend (throttled to avoid excessive calls)
        this.storeLiveDataToBackend(chart.symbol, chart.exchange, newCandle);
        
        this.notifyDataUpdate(chartId);
      });
      
      if (ws) {
        chart.ws = ws;
      }
    } catch (error) {
      console.error(`[ChartManager] Failed to connect WebSocket for ${chartId}:`, error);
    }
  }

  /**
   * Register a callback for data updates
   */
  onDataUpdate(chartId: string, callback: (data: MarketData[]) => void): void {
    console.log(`[ChartManager] Registering data update callback for chartId=${chartId}`);
    this.dataUpdateCallbacks.set(chartId, callback);
    console.log(`[ChartManager] Callback registered. Total callbacks: ${this.dataUpdateCallbacks.size}`);
  }

  /**
   * Register a callback for live price updates
   */
  onLivePriceUpdate(chartId: string, callback: (price: number) => void): void {
    this.livePriceCallbacks.set(chartId, callback);
  }

  /**
   * Notify listeners of data update
   */
  private notifyDataUpdate(chartId: string): void {
    const chart = this.charts.get(chartId);
    const callback = this.dataUpdateCallbacks.get(chartId);
    console.log(`[ChartManager] notifyDataUpdate: chartId=${chartId}, hasChart=${!!chart}, hasCallback=${!!callback}, dataLength=${chart?.marketData?.length || 0}`);
    if (chart && callback) {
      console.log(`[ChartManager] Calling callback with ${chart.marketData.length} candles`);
      callback(chart.marketData);
    } else {
      console.log(`[ChartManager] Cannot notify - missing chart or callback`);
    }
  }

  /**
   * Notify listeners of live price update
   */
  private notifyLivePriceUpdate(chartId: string, price: number): void {
    const callback = this.livePriceCallbacks.get(chartId);
    if (callback) {
      callback(price);
    }
  }

  /**
   * Get chart instance
   */
  getChart(id: string): ChartInstance | undefined {
    return this.charts.get(id);
  }

  /**
   * Get all charts
   */
  getAllCharts(): ChartInstance[] {
    return Array.from(this.charts.values());
  }

  /**
   * Store live data to backend (throttled)
   */
  private async storeLiveDataToBackend(symbol: string, exchange: ExchangeType, candle: MarketData): Promise<void> {
    const key = `${symbol}-${exchange}`;
    const now = Date.now();
    const lastStorage = this.lastStorageTime.get(key) || 0;
    
    // Buffer the candle
    if (!this.liveDataBuffer.has(key)) {
      this.liveDataBuffer.set(key, []);
    }
    const buffer = this.liveDataBuffer.get(key)!;
    
    // Update or add candle to buffer
    const existingIndex = buffer.findIndex(c => c.time === candle.time);
    if (existingIndex >= 0) {
      buffer[existingIndex] = candle;
    } else {
      buffer.push(candle);
    }
    
    // Store to backend every 30 seconds or when buffer has 10+ candles
    if (now - lastStorage > 30000 || buffer.length >= 10) {
      try {
        const candlesToStore = buffer.splice(0); // Clear buffer
        
        const response = await fetch(`http://localhost:5002/api/market-data/save`, {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify({
            symbol,
            exchange,
            interval: '1m',
            candles: candlesToStore.map(c => ({
              timestamp: c.time,
              open: c.open,
              high: c.high,
              low: c.low,
              close: c.close,
              volume: c.volume
            }))
          })
        });
        
        if (response.ok) {
          console.log(`[ChartManager] Stored ${candlesToStore.length} live candles to backend`);
          this.lastStorageTime.set(key, now);
        } else {
          console.error(`[ChartManager] Failed to store live data:`, await response.text());
          // Put candles back in buffer if storage failed
          buffer.unshift(...candlesToStore);
        }
      } catch (error) {
        console.error(`[ChartManager] Error storing live data:`, error);
      }
    }
  }

  /**
   * Cleanup all charts
   */
  cleanup(): void {
    console.log(`[ChartManager] cleanup() called - removing ${this.charts.size} charts`);
    console.trace('[ChartManager] cleanup call stack');
    for (const chartId of this.charts.keys()) {
      this.removeChart(chartId);
    }
    
    // Clear buffers
    this.liveDataBuffer.clear();
    this.lastStorageTime.clear();
  }
}

// Singleton instance
export const chartManager = new ChartManager();