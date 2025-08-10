// IndexedDB-based storage for market data
import type { 
  StoredMarketData, 
  DatasetInfo, 
  DataQuery, 
  DataStorageConfig
} from './DataTypes';

import { DEFAULT_CONFIG } from './DataTypes';

export class DataStorage {
  private db: IDBDatabase | null = null;
  private config: DataStorageConfig;
  
  constructor(config: Partial<DataStorageConfig> = {}) {
    this.config = { ...DEFAULT_CONFIG, ...config };
  }
  
  async init(): Promise<void> {
    return new Promise((resolve, reject) => {
      const request = indexedDB.open(this.config.dbName, this.config.version);
      
      request.onerror = () => reject(request.error);
      request.onsuccess = () => {
        this.db = request.result;
        resolve();
      };
      
      request.onupgradeneeded = (event) => {
        const db = (event.target as IDBOpenDBRequest).result;
        
        // Create market data store
        if (!db.objectStoreNames.contains('marketData')) {
          const store = db.createObjectStore('marketData', { 
            keyPath: 'id', 
            autoIncrement: true 
          });
          
          // Create indexes for efficient querying
          store.createIndex('symbol', 'symbol', { unique: false });
          store.createIndex('exchange', 'exchange', { unique: false });
          store.createIndex('timestamp', 'timestamp', { unique: false });
          store.createIndex('composite', ['symbol', 'exchange', 'interval', 'timestamp'], { 
            unique: true 
          });
        }
        
        // Create dataset info store
        if (!db.objectStoreNames.contains('datasets')) {
          const store = db.createObjectStore('datasets', { 
            keyPath: ['symbol', 'exchange', 'interval'] 
          });
          store.createIndex('symbol', 'symbol', { unique: false });
          store.createIndex('lastUpdated', 'lastUpdated', { unique: false });
        }
      };
    });
  }
  
  async saveCandles(candles: StoredMarketData[]): Promise<void> {
    if (!this.db) await this.init();
    
    const transaction = this.db!.transaction(['marketData', 'datasets'], 'readwrite');
    const marketStore = transaction.objectStore('marketData');
    const datasetStore = transaction.objectStore('datasets');
    
    // Group by symbol/exchange/interval for dataset info
    const datasets = new Map<string, DatasetInfo>();
    
    for (const candle of candles) {
      const key = `${candle.symbol}-${candle.exchange}-${candle.interval}`;
      
      // Check if candle already exists
      const index = marketStore.index('composite');
      const existingRequest = index.get([
        candle.symbol, 
        candle.exchange, 
        candle.interval, 
        candle.timestamp
      ]);
      
      await new Promise((resolve, reject) => {
        existingRequest.onsuccess = () => {
          if (!existingRequest.result) {
            // Add new candle
            const addRequest = marketStore.add(candle);
            addRequest.onsuccess = resolve;
            addRequest.onerror = reject;
          } else {
            // Update existing candle
            const updateRequest = marketStore.put({
              ...existingRequest.result,
              ...candle,
              id: existingRequest.result.id
            });
            updateRequest.onsuccess = resolve;
            updateRequest.onerror = reject;
          }
        };
        existingRequest.onerror = reject;
      });
      
      // Update dataset info
      if (!datasets.has(key)) {
        datasets.set(key, {
          symbol: candle.symbol,
          exchange: candle.exchange,
          interval: candle.interval || '1m',
          startTime: candle.timestamp,
          endTime: candle.timestamp,
          candleCount: 1,
          lastUpdated: Date.now()
        });
      } else {
        const info = datasets.get(key)!;
        info.startTime = Math.min(info.startTime, candle.timestamp);
        info.endTime = Math.max(info.endTime, candle.timestamp);
        info.candleCount++;
      }
    }
    
    // Update dataset info
    for (const info of datasets.values()) {
      await new Promise((resolve, reject) => {
        const request = datasetStore.put(info);
        request.onsuccess = resolve;
        request.onerror = reject;
      });
    }
    
    // Cleanup old data if needed
    await this.cleanupOldData();
  }
  
  async queryCandles(query: DataQuery): Promise<StoredMarketData[]> {
    if (!this.db) await this.init();
    
    const transaction = this.db!.transaction(['marketData'], 'readonly');
    const store = transaction.objectStore('marketData');
    
    return new Promise((resolve, reject) => {
      const results: StoredMarketData[] = [];
      let request: IDBRequest;
      
      // Use composite index for efficient querying
      if (query.symbol && query.exchange && query.interval) {
        const index = store.index('composite');
        const range = IDBKeyRange.bound(
          [query.symbol, query.exchange, query.interval, query.startTime || 0],
          [query.symbol, query.exchange, query.interval, query.endTime || Infinity]
        );
        request = index.openCursor(range);
      } else if (query.symbol) {
        const index = store.index('symbol');
        request = index.openCursor(IDBKeyRange.only(query.symbol));
      } else {
        request = store.openCursor();
      }
      
      request.onsuccess = () => {
        const cursor = request.result;
        if (cursor) {
          const data = cursor.value;
          
          // Apply filters
          if ((!query.exchange || data.exchange === query.exchange) &&
              (!query.interval || data.interval === query.interval) &&
              (!query.startTime || data.timestamp >= query.startTime) &&
              (!query.endTime || data.timestamp <= query.endTime)) {
            results.push(data);
          }
          
          if (!query.limit || results.length < query.limit) {
            cursor.continue();
          } else {
            resolve(results);
          }
        } else {
          resolve(results);
        }
      };
      
      request.onerror = () => reject(request.error);
    });
  }
  
  async getDatasets(): Promise<DatasetInfo[]> {
    if (!this.db) await this.init();
    
    const transaction = this.db!.transaction(['datasets'], 'readonly');
    const store = transaction.objectStore('datasets');
    
    return new Promise((resolve, reject) => {
      const request = store.getAll();
      request.onsuccess = () => resolve(request.result);
      request.onerror = () => reject(request.error);
    });
  }
  
  async getLatestCandle(symbol: string, exchange: string): Promise<StoredMarketData | null> {
    const candles = await this.queryCandles({
      symbol,
      exchange,
      limit: 1
    });
    
    if (candles.length === 0) return null;
    
    // Sort by timestamp and return latest
    return candles.reduce((latest, candle) => 
      candle.timestamp > latest.timestamp ? candle : latest
    );
  }
  
  private async cleanupOldData(): Promise<void> {
    if (!this.db) return;
    
    const datasets = await this.getDatasets();
    
    for (const dataset of datasets) {
      const candles = await this.queryCandles({
        symbol: dataset.symbol,
        exchange: dataset.exchange,
        interval: dataset.interval
      });
      
      if (candles.length > this.config.maxCandles) {
        // Sort by timestamp and keep only recent data
        candles.sort((a, b) => b.timestamp - a.timestamp);
        const toDelete = candles.slice(this.config.maxCandles);
        
        const transaction = this.db.transaction(['marketData'], 'readwrite');
        const store = transaction.objectStore('marketData');
        
        for (const candle of toDelete) {
          if (candle.id) {
            store.delete(candle.id);
          }
        }
      }
    }
  }
  
  async clearAll(): Promise<void> {
    if (!this.db) await this.init();
    
    const transaction = this.db!.transaction(['marketData', 'datasets'], 'readwrite');
    transaction.objectStore('marketData').clear();
    transaction.objectStore('datasets').clear();
    
    return new Promise((resolve, reject) => {
      transaction.oncomplete = () => resolve();
      transaction.onerror = () => reject(transaction.error);
    });
  }
  
  async exportToJSON(query: DataQuery): Promise<string> {
    const candles = await this.queryCandles(query);
    return JSON.stringify(candles, null, 2);
  }
  
  async importFromJSON(json: string): Promise<void> {
    const candles = JSON.parse(json) as StoredMarketData[];
    await this.saveCandles(candles);
  }
}

// Singleton instance
export const dataStorage = new DataStorage();