/**
 * Strategy templates and presets
 */

export interface StrategyTemplate {
  id: string;
  title: string;
  description: string;
  color: string;
  tags: string[];
  metrics: {
    sharpe: number;
    annualReturn: number;
    maxDrawdown: number;
    winRate: number;
  };
  category?: 'core' | 'statistical' | 'ml' | 'crypto' | 'forex' | 'commodities';
  requiresPremium?: boolean;
}

export const coreStrategies: StrategyTemplate[] = [
  {
    id: 'trend-following',
    title: 'Trend Following',
    description: 'Classic momentum strategy that rides market trends using moving averages and breakouts.',
    color: 'blue',
    tags: ['momentum', 'moving-averages', 'breakout', 'trend', 'classic', 'beginner-friendly', 'all-markets', 'robust', 'MA', 'EMA', 'crossover', 'systematic'],
    metrics: {
      sharpe: 1.45,
      annualReturn: 18.5,
      maxDrawdown: -12.3,
      winRate: 42
    },
    category: 'core'
  },
  {
    id: 'mean-reversion',
    title: 'Mean Reversion',
    description: 'Exploits overbought/oversold conditions using RSI and Bollinger Bands.',
    color: 'green',
    tags: ['mean-reversion', 'RSI', 'bollinger-bands', 'oversold', 'overbought', 'counter-trend', 'volatility', 'statistical', 'reversal', 'oscillator', 'BB', 'relative-strength'],
    metrics: {
      sharpe: 1.72,
      annualReturn: 15.3,
      maxDrawdown: -8.7,
      winRate: 58
    },
    category: 'core'
  },
  {
    id: 'pairs-trading',
    title: 'Statistical Arbitrage',
    description: 'Market-neutral strategy trading correlated asset pairs.',
    color: 'purple',
    tags: ['pairs-trading', 'arbitrage', 'market-neutral', 'correlation', 'cointegration', 'statistical', 'hedge', 'low-risk', 'quantitative', 'spread', 'convergence', 'statistical-arbitrage'],
    metrics: {
      sharpe: 2.31,
      annualReturn: 12.8,
      maxDrawdown: -4.5,
      winRate: 71
    },
    category: 'core'
  }
];

export const statisticalStrategies: StrategyTemplate[] = [
  {
    id: 'volatility-breakout',
    title: 'Volatility Breakout',
    description: 'Trades explosive moves when volatility expands beyond normal ranges.',
    color: 'orange',
    tags: ['volatility', 'breakout', 'ATR', 'expansion', 'momentum', 'squeeze', 'range', 'explosive', 'dynamic', 'adaptive', 'volatility-expansion', 'keltner'],
    metrics: {
      sharpe: 1.89,
      annualReturn: 24.7,
      maxDrawdown: -15.2,
      winRate: 48
    },
    category: 'statistical'
  },
  {
    id: 'calendar-spread',
    title: 'Calendar Effects',
    description: 'Exploits seasonal patterns and calendar anomalies in markets.',
    color: 'teal',
    tags: ['seasonal', 'calendar', 'anomaly', 'month-end', 'quarter-end', 'turn-of-month', 'day-of-week', 'systematic', 'pattern', 'cycle', 'timing', 'seasonality'],
    metrics: {
      sharpe: 1.56,
      annualReturn: 11.2,
      maxDrawdown: -6.8,
      winRate: 62
    },
    category: 'statistical'
  }
];

export const mlStrategies: StrategyTemplate[] = [
  {
    id: 'ml-ensemble',
    title: 'ML Ensemble',
    description: 'Combines multiple machine learning models for robust predictions.',
    color: 'indigo',
    tags: ['machine-learning', 'ensemble', 'random-forest', 'XGBoost', 'neural-network', 'AI', 'prediction', 'classification', 'feature-engineering', 'advanced', 'ML', 'deep-learning'],
    metrics: {
      sharpe: 2.15,
      annualReturn: 28.9,
      maxDrawdown: -11.4,
      winRate: 64
    },
    category: 'ml',
    requiresPremium: true
  },
  {
    id: 'sentiment-analysis',
    title: 'Sentiment Trading',
    description: 'Analyzes news and social media sentiment for trading signals.',
    color: 'pink',
    tags: ['sentiment', 'NLP', 'news', 'social-media', 'twitter', 'reddit', 'text-analysis', 'alternative-data', 'crowd-psychology', 'behavioral', 'sentiment-analysis', 'natural-language'],
    metrics: {
      sharpe: 1.93,
      annualReturn: 21.4,
      maxDrawdown: -9.8,
      winRate: 59
    },
    category: 'ml'
  }
];

export const cryptoStrategies: StrategyTemplate[] = [
  {
    id: 'defi-yield',
    title: 'DeFi Yield Farming',
    description: 'Optimizes yield across DeFi protocols with impermanent loss protection.',
    color: 'violet',
    tags: ['DeFi', 'yield-farming', 'liquidity', 'AMM', 'impermanent-loss', 'crypto', 'staking', 'APY', 'protocol', 'Ethereum', 'BSC', 'polygon', 'arbitrum', 'optimism'],
    metrics: {
      sharpe: 3.21,
      annualReturn: 45.7,
      maxDrawdown: -23.4,
      winRate: 0
    },
    category: 'crypto'
  },
  {
    id: 'altcoin-momentum',
    title: 'Altcoin Momentum',
    description: 'Momentum strategy for high-beta altcoins during bull market phases.',
    color: 'red',
    tags: ['crypto', 'altcoin', 'momentum', 'high-risk', 'bull-market', 'swing', 'aggressive', 'volatile', 'ETH', 'SOL', 'ADA', 'MATIC', 'altcoins'],
    metrics: {
      sharpe: 1.43,
      annualReturn: 89.2,
      maxDrawdown: -67.4,
      winRate: 58
    },
    category: 'crypto'
  }
];

export const forexStrategies: StrategyTemplate[] = [
  {
    id: 'carry-trade',
    title: 'Currency Carry Trade',
    description: 'Profits from interest rate differentials between currency pairs.',
    color: 'purple',
    tags: ['forex', 'carry-trade', 'interest-rates', 'position', 'macro', 'fundamental', 'conservative', 'systematic', 'EUR-USD', 'GBP-USD', 'USD-JPY', 'AUD-USD'],
    metrics: {
      sharpe: 2.08,
      annualReturn: 22.7,
      maxDrawdown: -8.9,
      winRate: 74
    },
    category: 'forex'
  },
  {
    id: 'london-breakout',
    title: 'London Breakout',
    description: 'Trades volatility expansion during London market opening hours.',
    color: 'teal',
    tags: ['forex', 'breakout', 'london-session', 'intraday', 'volatility', 'timezone', 'moderate-risk', 'systematic', 'EUR-USD', 'GBP-USD', 'USD-JPY'],
    metrics: {
      sharpe: 1.89,
      annualReturn: 28.4,
      maxDrawdown: -11.7,
      winRate: 63
    },
    category: 'forex'
  }
];

export const commoditiesStrategies: StrategyTemplate[] = [
  {
    id: 'gold-volatility',
    title: 'Gold Volatility',
    description: 'Trades gold price volatility during economic uncertainty periods.',
    color: 'orange',
    tags: ['commodities', 'gold', 'volatility', 'safe-haven', 'macro', 'swing', 'moderate-risk', 'hedging', 'GLD', 'GOLD', 'IAU', 'gold-ETFs'],
    metrics: {
      sharpe: 1.76,
      annualReturn: 19.8,
      maxDrawdown: -9.2,
      winRate: 69
    },
    category: 'commodities'
  },
  {
    id: 'oil-contango',
    title: 'Oil Contango',
    description: 'Profits from oil futures contango and backwardation patterns.',
    color: 'red',
    tags: ['commodities', 'oil', 'futures', 'contango', 'calendar-spreads', 'position', 'advanced', 'systematic', 'USO', 'OIL', 'UCO', 'oil-ETFs', 'WTI', 'Brent'],
    metrics: {
      sharpe: 2.31,
      annualReturn: 24.6,
      maxDrawdown: -7.8,
      winRate: 78
    },
    category: 'commodities'
  }
];

export const allStrategies = [
  ...coreStrategies,
  ...statisticalStrategies,
  ...mlStrategies,
  ...cryptoStrategies,
  ...forexStrategies,
  ...commoditiesStrategies
];

export const strategyCategories = [
  { id: 'core', label: 'Core Strategies', strategies: coreStrategies },
  { id: 'statistical', label: 'Statistical', strategies: statisticalStrategies },
  { id: 'ml', label: 'Machine Learning', strategies: mlStrategies },
  { id: 'crypto', label: 'Crypto', strategies: cryptoStrategies },
  { id: 'forex', label: 'Forex', strategies: forexStrategies },
  { id: 'commodities', label: 'Commodities', strategies: commoditiesStrategies },
];