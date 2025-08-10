/**
 * Validation utilities
 */

/**
 * Validate trading symbol format
 */
export function validateSymbol(symbol: string): boolean {
  // Format: BASE/QUOTE (e.g., BTC/USD, ETH/BTC)
  return /^[A-Z0-9]+\/[A-Z0-9]+$/.test(symbol);
}

/**
 * Validate email format
 */
export function validateEmail(email: string): boolean {
  const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
  return emailRegex.test(email);
}

/**
 * Validate price (positive number with up to 8 decimals)
 */
export function validatePrice(price: number): boolean {
  if (price <= 0) return false;
  const decimals = (price.toString().split('.')[1] || '').length;
  return decimals <= 8;
}

/**
 * Validate quantity (positive number)
 */
export function validateQuantity(quantity: number): boolean {
  return quantity > 0;
}

/**
 * Validate percentage (0-100)
 */
export function validatePercentage(value: number): boolean {
  return value >= 0 && value <= 100;
}

/**
 * Validate date range
 */
export function validateDateRange(start: Date, end: Date): boolean {
  return start < end && end <= new Date();
}

/**
 * Validate timeframe
 */
export function validateTimeframe(timeframe: string): boolean {
  const validTimeframes = ['1m', '5m', '15m', '30m', '1h', '4h', '1d', '1w'];
  return validTimeframes.includes(timeframe);
}

/**
 * Validate strategy parameters
 */
export function validateStrategyParams(params: Record<string, any>): {
  valid: boolean;
  errors: string[];
} {
  const errors: string[] = [];
  
  // Check RSI parameters
  if ('rsiPeriod' in params) {
    if (params.rsiPeriod < 2 || params.rsiPeriod > 100) {
      errors.push('RSI period must be between 2 and 100');
    }
  }
  
  if ('rsiOversold' in params && 'rsiOverbought' in params) {
    if (params.rsiOversold >= params.rsiOverbought) {
      errors.push('RSI oversold must be less than overbought');
    }
  }
  
  // Check MACD parameters
  if ('macdFast' in params && 'macdSlow' in params) {
    if (params.macdFast >= params.macdSlow) {
      errors.push('MACD fast period must be less than slow period');
    }
  }
  
  // Check stop loss and take profit
  if ('stopLoss' in params && params.stopLoss <= 0) {
    errors.push('Stop loss must be positive');
  }
  
  if ('takeProfit' in params && params.takeProfit <= 0) {
    errors.push('Take profit must be positive');
  }
  
  return {
    valid: errors.length === 0,
    errors,
  };
}

/**
 * Validate API key format
 */
export function validateApiKey(key: string): boolean {
  // Alpaca API key format
  if (key.startsWith('PK') || key.startsWith('SK')) {
    return key.length === 20;
  }
  // Generic validation
  return key.length >= 16 && /^[A-Za-z0-9_-]+$/.test(key);
}

/**
 * Validate manifest for analysis
 */
export function validateManifest(manifest: any): {
  valid: boolean;
  errors: string[];
} {
  const errors: string[] = [];
  
  if (!manifest.symbol) {
    errors.push('Symbol is required');
  } else if (typeof manifest.symbol === 'string' && !validateSymbol(manifest.symbol)) {
    errors.push('Invalid symbol format');
  }
  
  if (!manifest.timeframe || !validateTimeframe(manifest.timeframe)) {
    errors.push('Invalid timeframe');
  }
  
  if (!manifest.dateRange?.start || !manifest.dateRange?.end) {
    errors.push('Date range is required');
  }
  
  if (!manifest.strategy?.type) {
    errors.push('Strategy type is required');
  }
  
  return {
    valid: errors.length === 0,
    errors,
  };
}