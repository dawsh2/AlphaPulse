/**
 * Token utilities for standardizing token names and price display
 * Updated to use token addresses for accurate identification
 */

// Verified token addresses on Polygon (prevents fake token confusion)
export const VERIFIED_TOKEN_ADDRESSES: Record<string, {
  symbol: string;
  name: string;
  decimals: number;
  priority: number;
  category: 'stablecoin' | 'major' | 'native' | 'defi' | 'other';
}> = {
  '0x2791bca1f2de4661ed88a30c99a7a9449aa84174': {
    symbol: 'USDC',
    name: 'USD Coin',
    decimals: 6,
    priority: 1000,
    category: 'stablecoin'
  },
  '0xc2132d05d31c914a87c6611c10748aeb04b58e8f': {
    symbol: 'USDT', 
    name: 'Tether USD',
    decimals: 6,
    priority: 1000,
    category: 'stablecoin'
  },
  '0x8f3cf7ad23cd3cadbd9735aff958023239c6a063': {
    symbol: 'DAI',
    name: 'Dai Stablecoin',
    decimals: 18,
    priority: 1000,
    category: 'stablecoin'
  },
  '0x7ceb23fd6bc0add59e62ac25578270cff1b9f619': {
    symbol: 'WETH',
    name: 'Wrapped Ether',
    decimals: 18,
    priority: 500,
    category: 'major'
  },
  '0x1bfd67037b42cf73acf2047067bd4f2c47d9bfd6': {
    symbol: 'WBTC',
    name: 'Wrapped BTC',
    decimals: 8,
    priority: 499,
    category: 'major'
  },
  '0x0d500b1d8e8ef31e21c99d1db9a6444d3adf1270': {
    symbol: 'WMATIC',
    name: 'Wrapped Matic',
    decimals: 18,
    priority: 100,
    category: 'native'
  },
  '0x455e53724f9266ca11607ef1e22d3f2c4c5f34b1': {
    symbol: 'LINK',
    name: 'ChainLink Token',
    decimals: 18,
    priority: 50,
    category: 'defi'
  }
};

// Known fake/malicious token addresses
export const KNOWN_FAKE_TOKENS: Record<string, {
  fakeSymbol: string;
  realTokenAddress: string;
  reason: string;
}> = {
  '0x4c28f48448720e9000907bc2611f73022fdce1fa': {
    fakeSymbol: 'WETH',
    realTokenAddress: '0x7ceb23fd6bc0add59e62ac25578270cff1b9f619',
    reason: 'Missing core WETH functions (deposit/withdraw)'
  }
};

// Token aliases - maps various representations to canonical names
export const TOKEN_ALIASES: Record<string, string> = {
  // Wrapped MATIC/POL variations
  'WPOL': 'WMATIC',
  'POL': 'MATIC',
  'MATIC': 'MATIC',
  'WMATIC': 'WMATIC',
  
  // Wrapped ETH variations
  'WETH': 'WETH',
  'ETH': 'ETH',
  
  // Wrapped BTC variations
  'WBTC': 'WBTC',
  'BTC': 'BTC',
  
  // Stablecoins
  'USDC': 'USDC',
  'USDT': 'USDT',
  'DAI': 'DAI',
  'BUSD': 'BUSD',
  'TUSD': 'TUSD',
  'USDC.E': 'USDC',  // Bridged USDC
  
  // Other common tokens
  'LINK': 'LINK',
  'UNI': 'UNI',
  'AAVE': 'AAVE',
  'SUSHI': 'SUSHI',
  'COMP': 'COMP',
  'MKR': 'MKR',
  'SNX': 'SNX',
  'YFI': 'YFI',
  'CRV': 'CRV',
  'BAL': 'BAL',
};

// Token priorities for determining quote currency (higher = more likely to be quote)
// Stablecoins have highest priority, then major assets, then others
export const TOKEN_PRIORITY: Record<string, number> = {
  // Stablecoins (highest priority as quote currency)
  'USDC': 1000,
  'USDT': 999,
  'DAI': 998,
  'BUSD': 997,
  'TUSD': 996,
  
  // Major assets (medium priority)
  'WETH': 500,
  'ETH': 499,
  'WBTC': 498,
  'BTC': 497,
  
  // Native token (lower priority)
  'WMATIC': 100,
  'MATIC': 99,
  
  // Other tokens (lowest priority)
  'LINK': 50,
  'UNI': 49,
  'AAVE': 48,
  'SUSHI': 47,
};

/**
 * Get canonical token name from various representations
 */
export function getCanonicalTokenName(token: string): string {
  const upperToken = token.toUpperCase();
  return TOKEN_ALIASES[upperToken] || upperToken;
}

/**
 * Get token priority (higher number = more likely to be quote currency)
 */
export function getTokenPriority(token: string): number {
  const canonical = getCanonicalTokenName(token);
  return TOKEN_PRIORITY[canonical] || 0;
}

/**
 * Determine if price should be inverted based on token pair
 * Returns true if token0 should be priced in terms of token1
 * Returns false if token1 should be priced in terms of token0
 */
export function shouldInvertPrice(token0: string, token1: string): boolean {
  const priority0 = getTokenPriority(token0);
  const priority1 = getTokenPriority(token1);
  
  // If token0 has lower priority, it should be priced in terms of token1
  // E.g., LINK/USDC - LINK has lower priority, so show LINK price in USDC
  return priority0 < priority1;
}

/**
 * Format token pair for display with correct ordering
 */
export function formatTokenPair(token0: string, token1: string): string {
  const canonical0 = getCanonicalTokenName(token0);
  const canonical1 = getCanonicalTokenName(token1);
  
  // Order based on priority - lower priority first (base), higher priority second (quote)
  if (shouldInvertPrice(token0, token1)) {
    return `${canonical0}/${canonical1}`;
  } else {
    return `${canonical1}/${canonical0}`;
  }
}

/**
 * Get the correct price for display based on token ordering
 * Backend sends price as token1/token0
 */
export function getDisplayPrice(
  price: number,
  token0: string,
  token1: string
): number {
  // Backend price is token1 per token0
  // If we should invert (token0 has lower priority), return as-is
  // Otherwise, return reciprocal
  
  if (shouldInvertPrice(token0, token1)) {
    // token0 is base, token1 is quote - price is already correct
    return price;
  } else {
    // token1 is base, token0 is quote - need to invert
    return price > 0 ? 1 / price : 0;
  }
}

/**
 * Format price for display with appropriate decimal places
 */
export function formatPrice(price: number): string {
  if (price === 0) return '0';
  
  if (price < 0.000001) {
    return price.toExponential(2);
  } else if (price < 0.01) {
    return price.toFixed(6);
  } else if (price < 1) {
    return price.toFixed(4);
  } else if (price < 100) {
    return price.toFixed(2);
  } else if (price < 10000) {
    return price.toFixed(0);
  } else {
    return price.toExponential(2);
  }
}

/**
 * Normalize a token pair string (handle various separators)
 */
export function normalizeTokenPair(pair: string): [string, string] {
  const tokens = pair.split(/[-\\/]/);
  if (tokens.length !== 2) {
    console.warn(`Invalid pair format: ${pair}`);
    return ['', ''];
  }
  return [tokens[0].trim(), tokens[1].trim()];
}

/**
 * Check if two token pairs are equivalent (accounting for aliases)
 */
export function areTokenPairsEquivalent(
  pair1: [string, string],
  pair2: [string, string]
): boolean {
  const [token1a, token1b] = pair1.map(getCanonicalTokenName);
  const [token2a, token2b] = pair2.map(getCanonicalTokenName);
  
  // Check both orderings
  return (
    (token1a === token2a && token1b === token2b) ||
    (token1a === token2b && token1b === token2a)
  );
}

/**
 * Check if a token address is verified/legitimate
 */
export function isVerifiedToken(address: string): boolean {
  const normalizedAddress = address.toLowerCase();
  return normalizedAddress in VERIFIED_TOKEN_ADDRESSES;
}

/**
 * Check if a token address is known to be fake/malicious
 */
export function isKnownFakeToken(address: string): boolean {
  const normalizedAddress = address.toLowerCase();
  return normalizedAddress in KNOWN_FAKE_TOKENS;
}

/**
 * Get token priority by address (more secure than symbol-based)
 */
export function getTokenPriorityByAddress(address: string): number {
  const normalizedAddress = address.toLowerCase();
  const tokenInfo = VERIFIED_TOKEN_ADDRESSES[normalizedAddress];
  
  if (tokenInfo) {
    return tokenInfo.priority;
  }
  
  // Known fake tokens get negative priority
  if (isKnownFakeToken(address)) {
    return -1000;
  }
  
  // Unknown tokens get zero priority
  return 0;
}

/**
 * Get verified token info by address
 */
export function getVerifiedTokenInfo(address: string) {
  const normalizedAddress = address.toLowerCase();
  return VERIFIED_TOKEN_ADDRESSES[normalizedAddress] || null;
}

/**
 * Get fake token info if this address is known to be fake
 */
export function getFakeTokenInfo(address: string) {
  const normalizedAddress = address.toLowerCase();
  return KNOWN_FAKE_TOKENS[normalizedAddress] || null;
}

/**
 * Determine if price should be inverted based on token addresses (more secure)
 */
export function shouldInvertPriceByAddress(token0Address: string, token1Address: string): boolean {
  const priority0 = getTokenPriorityByAddress(token0Address);
  const priority1 = getTokenPriorityByAddress(token1Address);
  
  // If token0 has higher priority than token1, we need to invert
  // This ensures lower priority tokens are priced in higher priority tokens
  return priority0 > priority1;
}

/**
 * Validate a pool for potential fake tokens and display warnings
 */
export function validatePoolTokens(pools: Array<{ 
  poolAddress: string;
  token0Address?: string;
  token1Address?: string;
  pair: string;
  [key: string]: any;
}>): {
  validPools: typeof pools;
  suspiciousPools: typeof pools;
  warnings: string[];
} {
  const validPools: typeof pools = [];
  const suspiciousPools: typeof pools = [];
  const warnings: string[] = [];
  
  for (const pool of pools) {
    let isSuspicious = false;
    
    if (pool.token0Address && isKnownFakeToken(pool.token0Address)) {
      const fakeInfo = getFakeTokenInfo(pool.token0Address);
      warnings.push(`🚨 Pool ${pool.poolAddress} contains FAKE ${fakeInfo?.fakeSymbol} token: ${fakeInfo?.reason}`);
      isSuspicious = true;
    }
    
    if (pool.token1Address && isKnownFakeToken(pool.token1Address)) {
      const fakeInfo = getFakeTokenInfo(pool.token1Address);
      warnings.push(`🚨 Pool ${pool.poolAddress} contains FAKE ${fakeInfo?.fakeSymbol} token: ${fakeInfo?.reason}`);
      isSuspicious = true;
    }
    
    if (isSuspicious) {
      suspiciousPools.push(pool);
    } else {
      validPools.push(pool);
    }
  }
  
  return { validPools, suspiciousPools, warnings };
}

/**
 * Group pools by canonical token pairs
 */
export function groupPoolsByCanonicalPair(
  pools: Array<{ pair: string; [key: string]: any }>
): Map<string, typeof pools> {
  const grouped = new Map<string, typeof pools>();
  
  for (const pool of pools) {
    const [token0, token1] = normalizeTokenPair(pool.pair);
    const canonicalPair = formatTokenPair(token0, token1);
    
    const existing = grouped.get(canonicalPair) || [];
    existing.push(pool);
    grouped.set(canonicalPair, existing);
  }
  
  return grouped;
}