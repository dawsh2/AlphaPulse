// Exchange service factory and manager
import { KrakenService } from './kraken';
import { BinanceService } from './binance';
import { CoinbaseService } from './coinbase';
import type { ExchangeType, ExchangeService } from './types';

// Re-export only types (not runtime values)
export type { MarketData, ExchangeType, ExchangeService, ExchangeConfig } from './types';

export class ExchangeManager {
  private services: Map<ExchangeType, ExchangeService> = new Map();
  private currentExchange: ExchangeType = 'coinbase';
  private currentService: ExchangeService | null = null;
  
  constructor() {
    // Initialize available services
    this.services.set('kraken', new KrakenService());
    this.services.set('binance', new BinanceService());
    this.services.set('coinbase', new CoinbaseService());
  }
  
  setExchange(exchange: ExchangeType): void {
    // Disconnect from current exchange
    if (this.currentService) {
      this.currentService.disconnect();
    }
    
    this.currentExchange = exchange;
    this.currentService = this.services.get(exchange) || null;
  }
  
  getExchange(): ExchangeType {
    return this.currentExchange;
  }
  
  getService(): ExchangeService | null {
    if (!this.currentService) {
      this.currentService = this.services.get(this.currentExchange) || null;
    }
    return this.currentService;
  }
  
  // Available exchanges for UI
  getAvailableExchanges(): { value: ExchangeType; label: string }[] {
    return [
      { value: 'coinbase', label: 'Coinbase' },
      { value: 'binance', label: 'Binance' },
      { value: 'kraken', label: 'Kraken' }
    ];
  }
  
  // Get supported symbols for current exchange
  getSupportedSymbols(): string[] {
    switch (this.currentExchange) {
      case 'coinbase':
        return ['BTC/USD', 'ETH/USD', 'SOL/USD', 'LINK/USD'];
      case 'binance':
        return ['BTC/USDT', 'ETH/USDT', 'SOL/USDT', 'BNB/USDT'];
      case 'kraken':
        return ['BTC/USD', 'ETH/USD', 'SOL/USD'];
      default:
        return ['BTC/USD'];
    }
  }
}

// Singleton instance
export const exchangeManager = new ExchangeManager();