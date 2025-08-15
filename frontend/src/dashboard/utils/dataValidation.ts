import type { OrderBook, OrderBookLevel } from '../types';

export interface ValidationResult<T> {
  data: T;
  isValid: boolean;
  errors: string[];
}

export class OrderBookValidator {
  static validateLevel(level: OrderBookLevel): boolean {
    return level.price > 0 && 
           level.size > 0 && 
           !isNaN(level.price) && 
           !isNaN(level.size) &&
           isFinite(level.price) &&
           isFinite(level.size);
  }

  static validateOrderBook(orderbook: OrderBook, symbol: string): ValidationResult<OrderBook> {
    const errors: string[] = [];
    
    // Filter out invalid levels
    const validBids = orderbook.bids.filter(level => {
      const isValid = this.validateLevel(level);
      if (!isValid) {
        errors.push(`Invalid bid level: price=${level.price}, size=${level.size}`);
      }
      return isValid;
    });

    const validAsks = orderbook.asks.filter(level => {
      const isValid = this.validateLevel(level);
      if (!isValid) {
        errors.push(`Invalid ask level: price=${level.price}, size=${level.size}`);
      }
      return isValid;
    });

    // Sort to ensure correct ordering
    const sortedBids = [...validBids].sort((a, b) => b.price - a.price);
    const sortedAsks = [...validAsks].sort((a, b) => a.price - b.price);

    // Check for crossed book
    const bestBid = sortedBids[0]?.price || 0;
    const bestAsk = sortedAsks[0]?.price || 0;
    
    if (bestBid > 0 && bestAsk > 0 && bestBid >= bestAsk) {
      errors.push(`Crossed book detected for ${symbol}: bid=${bestBid} >= ask=${bestAsk}`);
      // Fix crossed book by removing overlapping levels
      const midPoint = (bestBid + bestAsk) / 2;
      const fixedBids = sortedBids.filter(b => b.price < midPoint);
      const fixedAsks = sortedAsks.filter(a => a.price > midPoint);
      
      return {
        data: {
          bids: fixedBids,
          asks: fixedAsks,
          timestamp: orderbook.timestamp,
          symbol_hash: orderbook.symbol_hash,
          symbol: orderbook.symbol
        },
        isValid: false,
        errors
      };
    }

    // Check for reasonable spread (not too wide)
    const spread = bestAsk - bestBid;
    const spreadPercent = bestBid > 0 ? (spread / bestBid) * 100 : 0;
    if (spreadPercent > 10) {
      errors.push(`Unusually wide spread: ${spreadPercent.toFixed(2)}%`);
    }

    return {
      data: {
        bids: sortedBids,
        asks: sortedAsks,
        timestamp: orderbook.timestamp,
        symbol_hash: orderbook.symbol_hash,
        symbol: orderbook.symbol
      },
      isValid: errors.length === 0,
      errors
    };
  }
}

export class PriceValidator {
  private static readonly PRICE_LIMITS: Record<string, { min: number; max: number }> = {
    'BTC-USD': { min: 1000, max: 1000000 },
    'ETH-USD': { min: 100, max: 100000 },
    'BTC-USDT': { min: 1000, max: 1000000 },
    'ETH-USDT': { min: 100, max: 100000 },
    'default': { min: 0.00001, max: 1000000000 }
  };

  static validatePrice(price: number, symbol: string): boolean {
    const limits = this.PRICE_LIMITS[symbol] || this.PRICE_LIMITS.default;
    return price >= limits.min && price <= limits.max;
  }

  static detectAnomalousPrice(price: number, recentPrices: number[]): boolean {
    if (recentPrices.length < 5) return false;
    
    const avg = recentPrices.reduce((a, b) => a + b, 0) / recentPrices.length;
    const deviation = Math.abs(price - avg) / avg;
    
    // Flag if price deviates more than 20% from recent average
    return deviation > 0.2;
  }
}

export class VolumeValidator {
  static validateVolume(volume: number): boolean {
    return volume > 0 && volume < 1000000000 && isFinite(volume);
  }

  static detectWashTrading(volumes: number[], timeWindowMs: number): boolean {
    // Simple wash trading detection: look for repetitive volumes
    if (volumes.length < 10) return false;
    
    const volumeCounts = new Map<number, number>();
    volumes.forEach(v => {
      const rounded = Math.round(v * 1000) / 1000;
      volumeCounts.set(rounded, (volumeCounts.get(rounded) || 0) + 1);
    });
    
    // If any volume appears more than 30% of the time, might be wash trading
    const maxCount = Math.max(...volumeCounts.values());
    return maxCount / volumes.length > 0.3;
  }
}

export class TimestampValidator {
  static validateTimestamp(timestamp: number): boolean {
    const now = Date.now();
    const oneHourAgo = now - 3600000;
    const oneHourAhead = now + 3600000;
    
    // Timestamp should be within reasonable bounds
    return timestamp > oneHourAgo && timestamp < oneHourAhead;
  }

  static detectStaleData(timestamp: number, maxAgeMs: number = 5000): boolean {
    return Date.now() - timestamp > maxAgeMs;
  }
}