import React, { useState, useEffect } from 'react';
import { useLocation, useNavigate } from 'react-router-dom';
import styles from './ResearchPage.module.css';
import exploreStyles from './ExplorePage.module.css';
import { StrategyWorkbench } from '../components/StrategyBuilder/StrategyWorkbench';
import Editor from '@monaco-editor/react';
import * as monaco from 'monaco-editor';
import { dataStorage } from '../services/data';
import type { DatasetInfo } from '../services/data';
import { notebookService } from '../services/notebookService';
import { SqlQueryInterface } from '../components/research/SqlQueryInterface';
import { StrategyCard } from '../components/research/StrategyCard';
import { TearsheetModal } from '../components/research/TearsheetModal';
import { SearchControls } from '../components/research/SearchControls';
import { ResultsInfo } from '../components/research/ResultsInfo';
import { LoadMoreButtons } from '../components/research/LoadMoreButtons';
import { EmptyState } from '../components/research/EmptyState';
import { MobileSwipeIndicator } from '../components/research/MobileSwipeIndicator';
import { MobileOverlay } from '../components/research/MobileOverlay';
import { BuilderWelcome } from '../components/research/BuilderWelcome';
import { SidebarTabs } from '../components/research/SidebarTabs';
import { DataEmptyState } from '../components/research/DataEmptyState';
import { NotebookTemplatesList } from '../components/research/NotebookTemplatesList';
import { SavedNotebooksList } from '../components/research/SavedNotebooksList';
import { ExploreView } from '../components/research/ExploreView';
import { BuilderView } from '../components/research/BuilderView';
import { NotebookView } from '../components/research/NotebookView';
import { DataCardComponent } from '../components/research/DataCard';

// Types
interface CodeSnippet {
  id: string;
  name: string;
  code: string;
  description?: string;
}

interface NotebookTemplate {
  id: string;
  title: string;
  description: string;
  cells: NotebookCell[];
}

interface AiMessage {
  role: 'assistant' | 'user';
  content: string;
  timestamp?: string;
}

interface NotebookCell {
  id: string;
  type: 'code' | 'markdown' | 'ai-chat';
  content: string;
  output?: string;
  isExecuting?: boolean;
  showAiAnalysis?: boolean;
  isAiChat?: boolean;
  parentCellId?: string;
  aiMessages?: AiMessage[];
  chatInput?: string;
}

interface SavedNotebook {
  id: string;
  name: string;
  lastModified: string;
  cells: NotebookCell[];
}

interface Strategy {
  id: string;
  title: string;
  description: string;
  color: string;
  tags: string[];
  creator?: string;
  comingSoon?: boolean;
  metrics?: {
    sharpe: number;
    annualReturn: number;
    maxDrawdown: number;
    winRate: number;
  };
  behavior?: 'trending' | 'meanrev' | 'breakout' | 'volatility';
  risk?: 'conservative' | 'moderate' | 'aggressive';
  timeframe?: 'intraday' | 'swing' | 'position';
}

interface TearsheetData {
  strategy: Strategy;
  isOpen: boolean;
}

type SidebarTab = 'builder' | 'notebooks';
type MainView = 'explore' | 'notebook' | 'builder' | 'data';
type SortBy = 'new' | 'sharpe' | 'returns' | 'name' | 'winrate';

// Strategy data - matching ExplorePage
const coreStrategies: Strategy[] = [
  {
    id: 'ema-cross',
    title: 'EMA Cross',
    description: 'Classic trend-following strategy using exponential moving average crossovers.',
    color: 'blue',
    tags: ['MA cross', 'simple', 'S&P-500', 'NASDAQ', 'Russell-2000', 'SPY', 'QQQ', 'IWM'],
    creator: 'alexchen',
    behavior: 'trending',
    risk: 'moderate',
    timeframe: 'swing',
    metrics: {
      sharpe: 1.82,
      annualReturn: 24.5,
      maxDrawdown: -8.3,
      winRate: 68
    }
  },
  {
    id: 'mean-reversion',
    title: 'RSI Mean Reversion',
    description: 'Trades oversold bounces and overbought reversals using RSI divergences.',
    color: 'orange',
    tags: ['RSI', 'reversal', 'S&P-500', 'NASDAQ', 'SPY', 'QQQ'],
    creator: 'sarahkim',
    behavior: 'meanrev',
    risk: 'conservative',
    timeframe: 'swing',
    metrics: {
      sharpe: 2.15,
      annualReturn: 31.2,
      maxDrawdown: -6.7,
      winRate: 72
    }
  },
  {
    id: 'momentum',
    title: 'Momentum Breakout',
    description: 'Captures explosive moves after consolidation periods.',
    color: 'green',
    tags: ['breakout', 'volume', 'S&P-500', 'NASDAQ', 'Russell-2000', 'SPY', 'QQQ', 'IWM'],
    creator: 'mikejohnson',
    behavior: 'breakout',
    risk: 'aggressive',
    timeframe: 'intraday',
    metrics: {
      sharpe: 1.58,
      annualReturn: 28.9,
      maxDrawdown: -14.2,
      winRate: 61
    }
  },
  {
    id: 'custom',
    title: 'Strategy Builder',
    description: 'Create your own strategy with visual tools.',
    color: 'cyan',
    tags: ['custom', 'builder', 'any-universe'],
    metrics: {
      sharpe: 0,
      annualReturn: 0,
      maxDrawdown: 0,
      winRate: 0
    }
  }
];

const statisticalStrategies: Strategy[] = [
  {
    id: 'pairs-trading',
    title: 'Pairs Trading',
    description: 'Market-neutral strategy trading correlated pairs divergence.',
    color: 'purple',
    tags: ['pairs', 'neutral', 'S&P-500', 'NASDAQ', 'sector-ETFs'],
    creator: 'quantdave',
    behavior: 'meanrev',
    risk: 'conservative',
    timeframe: 'position',
    metrics: {
      sharpe: 2.54,
      annualReturn: 28.7,
      maxDrawdown: -4.2,
      winRate: 76
    }
  },
  {
    id: 'volatility-harvest',
    title: 'Vol Harvester',
    description: 'Profits from volatility spikes and VIX contango.',
    color: 'red',
    tags: ['VIX', 'options', 'VXX', 'UVXY', 'volatility-ETFs'],
    creator: 'voltrader',
    behavior: 'volatility',
    risk: 'aggressive',
    timeframe: 'swing',
    metrics: {
      sharpe: 1.95,
      annualReturn: 35.8,
      maxDrawdown: -15.3,
      winRate: 65
    }
  },
  {
    id: 'bollinger-squeeze',
    title: 'Bollinger Squeeze',
    description: 'Trades volatility expansion after consolidation.',
    color: 'teal',
    tags: ['BB', 'squeeze', 'S&P-500', 'NASDAQ', 'SPY', 'QQQ'],
    behavior: 'breakout',
    risk: 'moderate',
    timeframe: 'intraday',
    metrics: {
      sharpe: 1.67,
      annualReturn: 22.4,
      maxDrawdown: -9.8,
      winRate: 69
    }
  }
];

const mlStrategies: Strategy[] = [
  {
    id: 'trend-rider',
    title: 'Trend Rider XL',
    description: 'Multi-timeframe trend following with dynamic position sizing.',
    color: 'indigo',
    tags: ['multi-TF', 'adaptive', 'S&P-500', 'NASDAQ', 'Russell-2000', 'SPY', 'QQQ', 'IWM'],
    creator: 'trendmaster',
    behavior: 'trending',
    risk: 'moderate',
    timeframe: 'position',
    metrics: {
      sharpe: 2.91,
      annualReturn: 42.5,
      maxDrawdown: -9.8,
      winRate: 71
    }
  },
  {
    id: 'gap-fade',
    title: 'Gap Fade Pro',
    description: 'Fades opening gaps with statistical edge.',
    color: 'pink',
    tags: ['gaps', 'open', 'S&P-500', 'NASDAQ', 'SPY', 'QQQ', 'individual-stocks'],
    creator: 'gapfader',
    behavior: 'meanrev',
    risk: 'moderate',
    timeframe: 'intraday',
    metrics: {
      sharpe: 1.93,
      annualReturn: 27.8,
      maxDrawdown: -7.2,
      winRate: 74
    }
  }
];

const additionalStrategies: Strategy[] = [
  {
    id: 'macd-cross',
    title: 'MACD Cross Signal',
    description: 'Classic MACD signal line crossover with histogram confirmation.',
    color: 'blue',
    tags: ['trending', 'MACD', 'crossover', 'histogram', 'swing', 'moderate-risk', 'beginner', 'momentum', 'S&P-500', 'NASDAQ', 'SPY', 'QQQ'],
    metrics: {
      sharpe: 1.64,
      annualReturn: 21.3,
      maxDrawdown: -11.2,
      winRate: 59
    }
  },
  {
    id: 'stoch-rsi',
    title: 'Stochastic RSI',
    description: 'Combines Stochastic and RSI for precise overbought/oversold signals.',
    color: 'orange',
    tags: ['mean-reversion', 'stochastic', 'RSI', 'oversold', 'overbought', 'intraday', 'moderate-risk', 'intermediate', 'S&P-500', 'NASDAQ', 'SPY', 'QQQ'],
    metrics: {
      sharpe: 2.08,
      annualReturn: 26.7,
      maxDrawdown: -8.9,
      winRate: 71
    }
  },
  {
    id: 'channel-breakout',
    title: 'Channel Breakout',
    description: 'Trades breakouts from established support and resistance channels.',
    color: 'green',
    tags: ['breakout', 'channels', 'support-resistance', 'swing', 'moderate-risk', 'technical', 'intermediate', 'S&P-500', 'NASDAQ', 'Russell-2000', 'SPY', 'QQQ', 'IWM'],
    metrics: {
      sharpe: 1.77,
      annualReturn: 29.4,
      maxDrawdown: -13.6,
      winRate: 54
    }
  },
  {
    id: 'vwap-reversion',
    title: 'VWAP Reversion',
    description: 'Mean reversion strategy using Volume Weighted Average Price.',
    color: 'purple',
    tags: ['mean-reversion', 'VWAP', 'volume', 'intraday', 'low-risk', 'institutional', 'beginner', 'SPY', 'S&P-500', 'ETF'],
    creator: 'flowtrader',
    metrics: {
      sharpe: 2.31,
      annualReturn: 19.8,
      maxDrawdown: -5.4,
      winRate: 78
    }
  },
  {
    id: 'buy-the-dip',
    title: 'Buy the Dip',
    description: 'Systematic dip buying with risk management and trend filters.',
    color: 'red',
    tags: ['mean-reversion', 'dip-buying', 'swing', 'contrarian', 'beginner', 'systematic', 'bull-market', 'SPY', 'QQQ', 'ETF'],
    creator: 'dipbuyerxyz',
    metrics: {
      sharpe: 1.52,
      annualReturn: 31.7,
      maxDrawdown: -16.8,
      winRate: 64
    }
  },
  {
    id: 'ma-ribbon',
    title: 'MA Ribbon',
    description: 'Multiple moving averages create a trend-following ribbon system.',
    color: 'teal',
    tags: ['trending', 'MA-ribbon', 'multi-timeframe', 'swing', 'intermediate', 'systematic', 'momentum', 'S&P-500', 'NASDAQ', 'SPY', 'QQQ'],
    metrics: {
      sharpe: 1.89,
      annualReturn: 25.1,
      maxDrawdown: -10.3,
      winRate: 62
    }
  },
  {
    id: 'fibonacci-retrace',
    title: 'Fibonacci Retracement',
    description: 'Trades pullbacks to key Fibonacci retracement levels.',
    color: 'indigo',
    tags: ['mean-reversion', 'fibonacci', 'technical', 'swing', 'intermediate', 'pullback', 'support-resistance', 'S&P-500', 'NASDAQ', 'individual-stocks'],
    metrics: {
      sharpe: 1.73,
      annualReturn: 23.9,
      maxDrawdown: -12.1,
      winRate: 67
    }
  },
  {
    id: 'iron-condor',
    title: 'Iron Condor',
    description: 'Options strategy profiting from low volatility and time decay.',
    color: 'pink',
    tags: ['volatility', 'options', 'theta-decay', 'range-bound', 'advanced', 'premium-selling', 'market-neutral', 'SPX', 'RUT', 'NDX', 'index-options'],
    metrics: {
      sharpe: 2.17,
      annualReturn: 18.4,
      maxDrawdown: -7.3,
      winRate: 82
    }
  },
  {
    id: 'news-sentiment',
    title: 'News Sentiment',
    description: 'NLP-driven strategy using real-time news sentiment analysis.',
    color: 'cyan',
    tags: ['sentiment', 'NLP', 'news', 'event-driven', 'intraday', 'alternative-data', 'advanced', 'systematic', 'S&P-500', 'individual-stocks'],
    comingSoon: true
  }
];

const cryptoStrategies: Strategy[] = [
  {
    id: 'crypto-arbitrage',
    title: 'Crypto Arbitrage',
    description: 'Cross-exchange arbitrage capturing price differences between crypto exchanges.',
    color: 'orange',
    tags: ['crypto', 'arbitrage', 'market-neutral', 'systematic', 'high-frequency', 'bitcoin', 'ethereum', 'advanced', 'BTC', 'ETH', 'multi-exchange'],
    creator: 'cryptoarb',
    metrics: {
      sharpe: 3.12,
      annualReturn: 45.8,
      maxDrawdown: -4.2,
      winRate: 89
    }
  },
  {
    id: 'defi-yield-farming',
    title: 'DeFi Yield Farming',
    description: 'Automated yield optimization across DeFi protocols and liquidity pools.',
    color: 'green',
    tags: ['crypto', 'DeFi', 'yield-farming', 'liquidity', 'ethereum', 'position', 'moderate-risk', 'advanced', 'ETH', 'USDC', 'USDT', 'stablecoins'],
    creator: 'defifarmer',
    metrics: {
      sharpe: 2.67,
      annualReturn: 78.4,
      maxDrawdown: -23.1,
      winRate: 76
    }
  },
  {
    id: 'bitcoin-halving',
    title: 'Bitcoin Halving Cycle',
    description: 'Long-term strategy based on Bitcoin halving cycles and market psychology.',
    color: 'indigo',
    tags: ['crypto', 'bitcoin', 'halving', 'cycle', 'position', 'long-term', 'macro', 'beginner', 'BTC', 'bitcoin-only'],
    creator: 'hodlmaster',
    metrics: {
      sharpe: 1.95,
      annualReturn: 127.3,
      maxDrawdown: -45.8,
      winRate: 71
    }
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
    }
  }
];

const forexStrategies: Strategy[] = [
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
    }
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
    }
  }
];

const commoditiesStrategies: Strategy[] = [
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
    }
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
    }
  }
];

// Combine all strategies
const allStrategies = [
  ...coreStrategies,
  ...statisticalStrategies,
  ...mlStrategies,
  ...additionalStrategies,
  ...cryptoStrategies,
  ...forexStrategies,
  ...commoditiesStrategies
];

// Data cards - using similar structure to strategies
interface DataCard {
  id: string;
  title: string;
  description: string;
  color: string;
  tags: string[];
  provider?: string;
  frequency?: string;
  coverage?: string;
  dataType?: 'market' | 'economic' | 'alternative' | 'custom';
  metrics?: {
    dataPoints?: string;
    dateRange?: string;
    updateFreq?: string;
    reliability?: string;
    latency?: string;
    coverage?: string;
    frequency?: string;
  };
}

const marketDataCards: DataCard[] = [
  {
    id: 'sp500',
    title: 'S&P 500 Index',
    description: 'Real-time and historical data for all S&P 500 constituents with corporate actions.',
    color: 'blue',
    tags: ['equities', 'index', 'us-market', 'large-cap', 'real-time'],
    provider: 'polygon',
    frequency: '1min',
    coverage: '2000-present',
    dataType: 'market',
    metrics: {
      dataPoints: '2.5B+',
      dateRange: '2000-2024',
      updateFreq: 'Real-time'
    }
  },
  {
    id: 'nasdaq100',
    title: 'NASDAQ 100',
    description: 'Technology-focused index with complete tick data and market depth.',
    color: 'green',
    tags: ['equities', 'index', 'tech', 'growth', 'real-time'],
    provider: 'polygon',
    frequency: '1min',
    coverage: '2000-present',
    dataType: 'market',
    metrics: {
      reliability: '99.9%',
      latency: '<10ms',
      coverage: '100%',
      frequency: '1min'
    }
  },
  {
    id: 'russell2000',
    title: 'Russell 2000',
    description: 'Small-cap universe with detailed fundamental data and earnings.',
    color: 'purple',
    tags: ['equities', 'index', 'small-cap', 'us-market'],
    provider: 'polygon',
    frequency: '1min',
    coverage: '2005-present',
    dataType: 'market',
    metrics: {
      reliability: '99.5%',
      latency: '<20ms',
      coverage: '98%',
      frequency: '1min'
    }
  },
  {
    id: 'value-stocks',
    title: 'Value Stock Universe',
    description: 'Curated dataset of value stocks with P/E < 15 and P/B < 1.5.',
    color: 'teal',
    tags: ['equities', 'value', 'fundamental', 'screener', 'custom'],
    provider: 'custom',
    frequency: 'daily',
    coverage: '2010-present',
    dataType: 'custom'
  },
  {
    id: 'options-flow',
    title: 'Options Flow',
    description: 'Real-time options flow, unusual activity, and Greek exposures.',
    color: 'orange',
    tags: ['options', 'derivatives', 'flow', 'gamma', 'real-time'],
    provider: 'cboe',
    frequency: 'tick',
    coverage: '2020-present',
    dataType: 'market',
    metrics: {
      reliability: '99.8%',
      latency: '<5ms',
      coverage: '100%',
      frequency: 'Tick'
    }
  },
  {
    id: 'penny-stocks',
    title: 'Penny Stocks OTC',
    description: 'OTC and pink sheet stocks under $5 with bid-ask spreads.',
    color: 'red',
    tags: ['equities', 'penny-stocks', 'otc', 'high-risk', 'speculative'],
    provider: 'otcmarkets',
    frequency: '15min',
    coverage: '2015-present',
    dataType: 'market',
    metrics: {
      dataPoints: '180M+',
      dateRange: '2015-2024',
      updateFreq: '15-min'
    }
  },
  {
    id: 'futures-commodities',
    title: 'Commodity Futures',
    description: 'Energy, metals, and agriculture futures with COT reports.',
    color: 'brown',
    tags: ['futures', 'commodities', 'oil', 'gold', 'wheat'],
    provider: 'cme',
    frequency: '1min',
    coverage: '1990-present',
    dataType: 'market',
    metrics: {
      dataPoints: '3.2B+',
      dateRange: '1990-2024',
      updateFreq: 'Real-time'
    }
  },
  {
    id: 'forex-majors',
    title: 'Forex Major Pairs',
    description: 'EUR/USD, GBP/USD, USD/JPY and other major currency pairs.',
    color: 'green',
    tags: ['forex', 'currencies', 'fx', 'majors', '24-5'],
    provider: 'oanda',
    frequency: 'tick',
    coverage: '2000-present',
    dataType: 'market',
    metrics: {
      dataPoints: '8.5B+',
      dateRange: '2000-2024',
      updateFreq: 'Tick-level'
    }
  },
  {
    id: 'corporate-bonds',
    title: 'Corporate Bonds',
    description: 'Investment grade and high yield corporate bond prices and yields.',
    color: 'blue',
    tags: ['bonds', 'fixed-income', 'credit', 'yield'],
    provider: 'ice',
    frequency: 'daily',
    coverage: '2005-present',
    dataType: 'market',
    metrics: {
      dataPoints: '125M+',
      dateRange: '2005-2024',
      updateFreq: 'Daily'
    }
  }
];

const cryptoDataCards: DataCard[] = [
  {
    id: 'btc-spot',
    title: 'Bitcoin Spot',
    description: 'Aggregated BTC spot prices from major exchanges with volume data.',
    color: 'orange',
    tags: ['crypto', 'bitcoin', 'spot', 'aggregate', '24-7'],
    provider: 'coinbase',
    frequency: '1sec',
    coverage: '2015-present',
    dataType: 'market',
    metrics: {
      reliability: '99.9%',
      latency: '<1ms',
      coverage: '100%',
      frequency: '1sec'
    }
  },
  {
    id: 'eth-defi',
    title: 'Ethereum DeFi',
    description: 'DeFi protocol data including TVL, yields, and liquidations.',
    color: 'indigo',
    tags: ['crypto', 'ethereum', 'defi', 'yield', 'tvl'],
    provider: 'defillama',
    frequency: '5min',
    coverage: '2020-present',
    dataType: 'market',
    metrics: {
      reliability: '98.0%',
      latency: '<50ms',
      coverage: '90%',
      frequency: '5min'
    }
  },
  {
    id: 'btc-hashrate',
    title: 'BTC Hash Rate',
    description: 'Bitcoin network hash rate, difficulty adjustments, and miner revenue.',
    color: 'red',
    tags: ['crypto', 'bitcoin', 'mining', 'hashrate', 'on-chain'],
    provider: 'glassnode',
    frequency: 'daily',
    coverage: '2010-present',
    dataType: 'alternative'
  },
  {
    id: 'stablecoin-flows',
    title: 'Stablecoin Flows',
    description: 'USDT, USDC, DAI flows between exchanges and chains.',
    color: 'green',
    tags: ['crypto', 'stablecoin', 'flows', 'liquidity', 'on-chain'],
    provider: 'nansen',
    frequency: '1hour',
    coverage: '2019-present',
    dataType: 'alternative'
  },
  {
    id: 'nft-collections',
    title: 'NFT Collections',
    description: 'Floor prices, volumes, and holder distribution for top NFT collections.',
    color: 'purple',
    tags: ['crypto', 'nft', 'ethereum', 'opensea', 'collectibles'],
    provider: 'opensea',
    frequency: '10min',
    coverage: '2021-present',
    dataType: 'alternative',
    metrics: {
      dataPoints: '158M+',
      dateRange: '2021-2024',
      updateFreq: '10-min'
    }
  },
  {
    id: 'crypto-derivatives',
    title: 'Crypto Derivatives',
    description: 'Perpetual futures, options, and structured products across exchanges.',
    color: 'orange',
    tags: ['crypto', 'derivatives', 'futures', 'options', 'perps'],
    provider: 'deribit',
    frequency: 'tick',
    coverage: '2018-present',
    dataType: 'market',
    metrics: {
      dataPoints: '2.3B+',
      dateRange: '2018-2024',
      updateFreq: 'Tick-level'
    }
  }
];

const economicDataCards: DataCard[] = [
  {
    id: 'm2-money',
    title: 'M2 Money Supply',
    description: 'US M2 money supply with velocity and correlation to asset prices.',
    color: 'green',
    tags: ['macro', 'monetary', 'fed', 'liquidity', 'economic'],
    provider: 'fred',
    frequency: 'weekly',
    coverage: '1980-present',
    dataType: 'economic'
  },
  {
    id: 'yield-curve',
    title: 'Yield Curve',
    description: 'Complete US Treasury yield curve with spreads and inversions.',
    color: 'blue',
    tags: ['rates', 'bonds', 'treasury', 'macro', 'economic'],
    provider: 'fred',
    frequency: 'daily',
    coverage: '1990-present',
    dataType: 'economic'
  },
  {
    id: 'inflation-nowcast',
    title: 'Inflation Nowcast',
    description: 'Real-time inflation expectations from TIPS, surveys, and models.',
    color: 'red',
    tags: ['inflation', 'cpi', 'tips', 'nowcast', 'economic'],
    provider: 'cleveland-fed',
    frequency: 'daily',
    coverage: '2000-present',
    dataType: 'economic'
  },
  {
    id: 'gdp-nowcast',
    title: 'GDP Nowcast',
    description: 'Real-time GDP predictions using high-frequency data.',
    color: 'purple',
    tags: ['gdp', 'growth', 'nowcast', 'economic', 'forecast'],
    provider: 'atlanta-fed',
    frequency: 'daily',
    coverage: '2011-present',
    dataType: 'economic'
  },
  {
    id: 'pmi-manufacturing',
    title: 'PMI Manufacturing',
    description: 'ISM manufacturing and services PMI with regional breakdowns.',
    color: 'indigo',
    tags: ['pmi', 'manufacturing', 'ism', 'industrial', 'services'],
    provider: 'ism',
    frequency: 'monthly',
    coverage: '1948-present',
    dataType: 'economic',
    metrics: {
      dataPoints: '912+',
      dateRange: '1948-2024',
      updateFreq: 'Monthly'
    }
  },
  {
    id: 'retail-sales',
    title: 'Retail Sales',
    description: 'Monthly retail sales data by category and region.',
    color: 'pink',
    tags: ['retail', 'sales', 'consumer', 'spending', 'commerce'],
    provider: 'census',
    frequency: 'monthly',
    coverage: '1992-present',
    dataType: 'economic',
    metrics: {
      dataPoints: '384+',
      dateRange: '1992-2024',
      updateFreq: 'Monthly'
    }
  }
];

const alternativeDataCards: DataCard[] = [
  {
    id: 'satellite-data',
    title: 'Satellite Analytics',
    description: 'Parking lot traffic, oil storage, agricultural yields from satellite imagery.',
    color: 'cyan',
    tags: ['satellite', 'alternative', 'retail', 'commodities', 'ai'],
    provider: 'orbital-insight',
    frequency: 'weekly',
    coverage: '2018-present',
    dataType: 'alternative'
  },
  {
    id: 'social-sentiment',
    title: 'Social Sentiment',
    description: 'Aggregated sentiment from Reddit, Twitter, and news sources.',
    color: 'pink',
    tags: ['sentiment', 'social', 'nlp', 'reddit', 'twitter'],
    provider: 'sentimentrader',
    frequency: '1hour',
    coverage: '2020-present',
    dataType: 'alternative'
  },
  {
    id: 'earnings-transcripts',
    title: 'Earnings Call NLP',
    description: 'Parsed earnings transcripts with sentiment and topic analysis.',
    color: 'teal',
    tags: ['earnings', 'nlp', 'sentiment', 'fundamental', 'alternative'],
    provider: 'alphasense',
    frequency: 'quarterly',
    coverage: '2010-present',
    dataType: 'alternative'
  },
  {
    id: 'web-traffic',
    title: 'Web Traffic Data',
    description: 'E-commerce traffic, app downloads, and user engagement metrics.',
    color: 'orange',
    tags: ['web-traffic', 'alternative', 'e-commerce', 'consumer'],
    provider: 'similarweb',
    frequency: 'daily',
    coverage: '2015-present',
    dataType: 'alternative'
  },
  {
    id: 'social-sentiment',
    title: 'Social Sentiment',
    description: 'Reddit WSB, Twitter, and StockTwits sentiment analysis.',
    color: 'purple',
    tags: ['alternative', 'sentiment', 'social', 'reddit', 'twitter'],
    provider: 'swaggy',
    frequency: '5min',
    coverage: '2020-present',
    dataType: 'alternative',
    metrics: {
      dataPoints: '420M+',
      dateRange: '2020-2024',
      updateFreq: '5-min'
    }
  },
  {
    id: 'web-traffic',
    title: 'Web Traffic',
    description: 'Company website traffic, app downloads, and user engagement.',
    color: 'green',
    tags: ['alternative', 'web', 'traffic', 'apps', 'engagement'],
    provider: 'similarweb',
    frequency: 'daily',
    coverage: '2015-present',
    dataType: 'alternative',
    metrics: {
      dataPoints: '3.3M+',
      dateRange: '2015-2024',
      updateFreq: 'Daily'
    }
  },
  {
    id: 'insider-trading',
    title: 'Insider Trading',
    description: 'SEC Form 4 filings, insider buys/sells, and 10b5-1 plans.',
    color: 'red',
    tags: ['alternative', 'insider', 'sec', 'form-4', 'executives'],
    provider: 'sec',
    frequency: 'real-time',
    coverage: '2003-present',
    dataType: 'alternative',
    metrics: {
      dataPoints: '4.5M+',
      dateRange: '2003-2024',
      updateFreq: 'Real-time'
    }
  },
  {
    id: 'esg-scores',
    title: 'ESG Ratings',
    description: 'Environmental, social, and governance scores and controversies.',
    color: 'green',
    tags: ['alternative', 'esg', 'sustainability', 'governance'],
    provider: 'msci',
    frequency: 'monthly',
    coverage: '2010-present',
    dataType: 'alternative',
    metrics: {
      dataPoints: '168K+',
      dateRange: '2010-2024',
      updateFreq: 'Monthly'
    }
  },
  {
    id: 'job-postings',
    title: 'Job Postings',
    description: 'Company job listings, salary ranges, and skill requirements.',
    color: 'pink',
    tags: ['alternative', 'jobs', 'hiring', 'employment', 'skills'],
    provider: 'indeed',
    frequency: 'daily',
    coverage: '2012-present',
    dataType: 'alternative',
    metrics: {
      dataPoints: '45M+',
      dateRange: '2012-2024',
      updateFreq: 'Daily'
    }
  }
];

// Combine all data cards
const allDataCards = [
  ...marketDataCards,
  ...cryptoDataCards,
  ...economicDataCards,
  ...alternativeDataCards
];

const ResearchPage: React.FC = () => {
  const location = useLocation();
  const navigate = useNavigate();
  
  // State management
  const [activeTab, setActiveTab] = useState<SidebarTab>('builder');
  const [mainView, setMainView] = useState<MainView>('explore');
  const [isBuilderOpen, setIsBuilderOpen] = useState(false);
  const [selectedTemplate, setSelectedTemplate] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [collapsedCategories, setCollapsedCategories] = useState<Set<string>>(new Set());
  const [backendData, setBackendData] = useState<any>(null);
  const [loadingBackendData, setLoadingBackendData] = useState(false);
  const [notebookCells, setNotebookCells] = useState<NotebookCell[]>([]);
  const [activeCell, setActiveCell] = useState<string | null>(null);
  const [sidebarOpen, setSidebarOpen] = useState(false); // Mobile sidebar state
  const [isMobile, setIsMobile] = useState(window.innerWidth <= 768);
  const [touchStart, setTouchStart] = useState<number | null>(null);
  const [touchEnd, setTouchEnd] = useState<number | null>(null);
  const [datasets, setDatasets] = useState<DatasetInfo[]>([]);
  const [loadingDatasets, setLoadingDatasets] = useState(false);
  // Initialize with correct theme detection
  const [theme, setTheme] = useState(() => {
    const isDark = document.documentElement.getAttribute('data-theme') === 'dark' ||
                   (!document.documentElement.getAttribute('data-theme') && 
                    window.matchMedia('(prefers-color-scheme: dark)').matches);
    return isDark ? 'vs-dark' : 'cream-light';
  });
  const editorTheme = theme; // Use the same theme for the editor
  
  // Explore page state
  const [exploreSearchQuery, setExploreSearchQuery] = useState('');
  const [selectedStrategy, setSelectedStrategy] = useState<string | null>(null);
  const [tearsheet, setTearsheet] = useState<TearsheetData>({ strategy: null as any, isOpen: false });
  const [dataDetails, setDataDetails] = useState<{ data: DataCard | null; isOpen: boolean }>({ data: null, isOpen: false });
  const [hoveredCard, setHoveredCard] = useState<string | null>(null);
  const [sortBy, setSortBy] = useState<SortBy>('sharpe');
  const [searchTerms, setSearchTerms] = useState<string[]>([]);
  const [displayLimit, setDisplayLimit] = useState(18);
  const [sortDropdownOpen, setSortDropdownOpen] = useState(false);
  const [exploreViewType, setExploreViewType] = useState<'strategies' | 'data'>('strategies');

  // Mock data for notebooks
  const codeSnippets: Record<string, CodeSnippet[]> = {
    'Data Loading': [
      {
        id: 'load_signals',
        name: 'Load Signals',
        code: `import admf\n\n# Load signals with filtering\nsignals = admf.load_signals(\n    strategy_type='bollinger_bands',\n    min_sharpe=1.0,\n    symbols=['AAPL', 'MSFT']\n)\nprint(f"Loaded {len(signals)} signal traces")`,
        description: 'Load strategy signals from ADMF registry'
      },
      {
        id: 'load_executions',
        name: 'Load Executions',
        code: `# Load execution data\nexecutions = admf.load_executions(\n    signal_hash='sig_a7f8d9e6',\n    include_trades=True\n)\nprint(f"Found {len(executions)} execution records")`,
        description: 'Load execution data for analysis'
      }
    ],
    'Performance Metrics': [
      {
        id: 'performance_table',
        name: 'Performance Table',
        code: `from analysis_lib import performance_table\n\n# Generate comprehensive performance metrics\nmetrics = performance_table(signals)\nmetrics.sort_values('sharpe_ratio', ascending=False).head(10)`,
        description: 'Calculate key performance metrics'
      },
      {
        id: 'sharpe_calculation',
        name: 'Sharpe Ratio',
        code: `# Calculate Sharpe ratio\ndef calculate_sharpe_ratio(returns, risk_free_rate=0.02):\n    excess_returns = returns - risk_free_rate / 252\n    return excess_returns.mean() / excess_returns.std() * np.sqrt(252)\n\nsharpe = calculate_sharpe_ratio(strategy_returns)\nprint(f"Sharpe Ratio: {sharpe:.2f}")`,
        description: 'Calculate annualized Sharpe ratio'
      }
    ],
    'Visualizations': [
      {
        id: 'equity_curves',
        name: 'Equity Curves',
        code: `import matplotlib.pyplot as plt\nfrom analysis_lib import plot_equity_curves\n\n# Plot multiple strategy equity curves\nfig = plot_equity_curves(\n    signals,\n    benchmark='SPY',\n    title='Strategy Performance Comparison'\n)\nfig.show()`,
        description: 'Plot strategy equity curves with benchmark'
      }
    ]
  };

  const notebookTemplates: NotebookTemplate[] = [
    {
      id: 'strategy_comparison',
      title: 'Strategy Comparison Analysis',
      description: 'Compare multiple strategies across key performance metrics',
      cells: [
        {
          id: 'cell-1',
          type: 'markdown',
          content: '# Strategy Comparison Analysis\n\nComparing multiple strategies across key performance metrics and risk characteristics.'
        },
        {
          id: 'cell-2',
          type: 'code',
          content: `import admf\nfrom analysis_lib import *\n\n# Load strategies to compare\nstrategies = admf.load_signals(['momentum', 'mean_reversion'], min_sharpe=1.0)\nprint(f"Loaded {len(strategies)} strategies for comparison")`
        }
      ]
    },
    {
      id: 'performance_summary',
      title: 'Complete Performance Analysis',
      description: 'Comprehensive analysis of strategy performance',
      cells: [
        {
          id: 'cell-1',
          type: 'markdown',
          content: '# Performance Summary Report\n\nComprehensive analysis of strategy performance including returns, risk metrics, and trade statistics.'
        }
      ]
    }
  ];

  const savedNotebooks: SavedNotebook[] = [
    {
      id: 'notebook-1',
      name: 'NVDA Momentum Analysis',
      lastModified: '2025-01-15',
      cells: []
    },
    {
      id: 'notebook-2',
      name: 'Portfolio Optimization',
      lastModified: '2025-01-14',
      cells: []
    }
  ];

  // Theme detection
  useEffect(() => {
    // Check if monaco is available before defining theme
    if (typeof monaco !== 'undefined' && monaco.editor) {
      // Define the cream theme once
      monaco.editor.defineTheme('cream-light', {
        base: 'vs',
        inherit: true,
        rules: [],
        colors: {
          'editor.background': '#faf7f0', // Cream/eggshell color
          'editor.foreground': '#33332d',
          'editor.lineHighlightBackground': '#f5f2ea',
          'editor.selectionBackground': '#e5e0d5',
          'editorCursor.foreground': '#33332d',
          'editorLineNumber.foreground': '#8b8680',
          'editorLineNumber.activeForeground': '#33332d'
        }
      });
    }
    
    // Detect current theme
    const updateTheme = () => {
      const isDark = document.documentElement.getAttribute('data-theme') === 'dark' ||
                     (!document.documentElement.getAttribute('data-theme') && 
                      window.matchMedia('(prefers-color-scheme: dark)').matches);
      
      setTheme(isDark ? 'vs-dark' : 'cream-light');
    };
    
    updateTheme();
    
    // Listen for theme changes
    const observer = new MutationObserver(updateTheme);
    observer.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ['data-theme']
    });
    
    return () => observer.disconnect();
  }, []);

  // Initialize with default notebook cells
  useEffect(() => {
    setNotebookCells([
      {
        id: 'cell-1',
        type: 'markdown',
        content: '# Research Notebook\n\nWelcome to the AlphaPulse research environment. Use the sidebar to access code snippets, templates, and saved notebooks.'
      },
      {
        id: 'cell-2',
        type: 'code',
        content: `import admf\nimport pandas as pd\nimport numpy as np\nfrom analysis_lib import *\n\n# Load sample data\nsignals = admf.load_signals(strategy_type='ema_cross', limit=5)\nprint(f"Loaded {len(signals)} signal traces for analysis")`
      }
    ]);
  }, []);

  // Handle window resize for mobile detection
  // Load datasets when data tab is active or when in explore view with data type selected
  useEffect(() => {
    if (mainView === 'data' || (mainView === 'explore' && exploreViewType === 'data')) {
      // Load Parquet datasets from backend
      if (datasets.length === 0 && !loadingDatasets) {
        setLoadingDatasets(true);
        console.log('Fetching datasets from backend...');
        fetch('http://localhost:5001/api/data/summary')
          .then(response => response.json())
          .then(data => {
            console.log('Backend data received:', data);
            // Convert backend format to frontend DatasetInfo format
            const backendDatasets = data.symbols.map((symbol: any) => ({
              symbol: symbol.symbol,
              exchange: symbol.exchange,
              interval: '1m', // Backend stores 1m data
              startTime: new Date(symbol.first_bar).getTime() / 1000,
              endTime: new Date(symbol.last_bar).getTime() / 1000,
              candleCount: symbol.bar_count,
              lastUpdated: Date.now()
            }));
            console.log('Setting datasets:', backendDatasets);
            setDatasets(backendDatasets);
            setLoadingDatasets(false);
          })
          .catch(error => {
            console.error('Failed to load datasets from backend:', error);
            // Fallback to IndexedDB
            dataStorage.getDatasets()
              .then(data => {
                setDatasets(data);
                setLoadingDatasets(false);
              })
              .catch(fallbackError => {
                console.error('Failed to load datasets from IndexedDB:', fallbackError);
                setLoadingDatasets(false);
              });
          });
      }
      
      // Load backend catalog data
      if (!backendData) {
        setLoadingBackendData(true);
        fetch('http://localhost:5001/api/catalog/list')
          .then(res => res.json())
          .then(data => {
            setBackendData(data);
            setLoadingBackendData(false);
          })
          .catch(error => {
            console.error('Failed to load backend catalog:', error);
            setLoadingBackendData(false);
          });
      }
    }
  }, [mainView, exploreViewType]);

  useEffect(() => {
    const handleResize = () => {
      setIsMobile(window.innerWidth <= 768);
    };

    window.addEventListener('resize', handleResize);
    return () => window.removeEventListener('resize', handleResize);
  }, []);

  // Touch event handlers for swipe gestures
  const handleTouchStart = (e: React.TouchEvent) => {
    setTouchEnd(null);
    setTouchStart(e.targetTouches[0].clientY);
  };

  const handleTouchMove = (e: React.TouchEvent) => {
    setTouchEnd(e.targetTouches[0].clientY);
  };

  const handleTouchEnd = () => {
    if (!touchStart || !touchEnd) return;
    
    const distance = touchStart - touchEnd;
    const isSwipeUp = distance > 50;
    const isSwipeDown = distance < -50;
    
    if (isSwipeUp && !sidebarOpen && isMobile) {
      // Swipe up to open sidebar
      setSidebarOpen(true);
    } else if (isSwipeDown && sidebarOpen && isMobile) {
      // Swipe down to close sidebar
      setSidebarOpen(false);
    }
  };

  // Check if opened from Explore page with strategy data or builder request
  useEffect(() => {
    if (location.state?.strategy) {
      const strategy = location.state.strategy;
      const analysisCell: NotebookCell = {
        id: `cell-${Date.now()}`,
        type: 'markdown',
        content: `# ${strategy.title} Analysis\n\nAnalyzing strategy: **${strategy.title}**\n\n**Description:** ${strategy.description}\n\n**Creator:** ${strategy.creator ? `@${strategy.creator}` : 'Unknown'}\n\n**Tags:** ${strategy.tags.join(', ')}`
      };
      
      const codeCell: NotebookCell = {
        id: `cell-${Date.now() + 1}`,
        type: 'code',
        content: `# Load strategy data for analysis\nimport admf\n\n# Load ${strategy.title} strategy data\nsignals = admf.load_signals(strategy_id='${strategy.id}')\nprint(f"Loaded strategy: ${strategy.title}")\nprint(f"Expected Sharpe: ${strategy.metrics?.sharpe || 'N/A'}")\nprint(f"Expected Annual Return: ${strategy.metrics?.annualReturn || 'N/A'}%")`
      };
      
      setNotebookCells([analysisCell, codeCell]);
      setActiveTab('notebooks');
      setMainView('notebook');
    } else if (location.state?.openBuilder) {
      setActiveTab('builder');
      setMainView('builder');
    }
  }, [location.state]);

  // Event handlers
  const handleTabSwitch = (tab: SidebarTab) => {
    setActiveTab(tab);
    
    // When builder button is clicked, open default template
    if (tab === 'builder') {
      setMainView('builder');
      // Set a default 'New Strategy' template
      setSelectedTemplate('new_strategy');
    } 
    // When notebook button is clicked, open default notebook template
    else if (tab === 'notebooks') {
      setMainView('notebook');
      // Load a default notebook template
      if (notebookTemplates.length > 0) {
        loadTemplate(notebookTemplates[0]); // Load first template as default
      }
    }
  };
  
  const handleOpenBuilder = () => {
    setActiveTab('builder');
    setMainView('builder');
    setSelectedTemplate('new_strategy');
  };

  const toggleCategory = (category: string) => {
    setCollapsedCategories(prev => {
      const newSet = new Set(prev);
      if (newSet.has(category)) {
        newSet.delete(category);
      } else {
        newSet.add(category);
      }
      return newSet;
    });
  };

  const insertSnippet = (snippet: CodeSnippet) => {
    const newCell: NotebookCell = {
      id: `cell-${Date.now()}`,
      type: 'code',
      content: snippet.code
    };
    setNotebookCells(prev => [...prev, newCell]);
    setMainView('notebook');
  };

  const loadTemplate = (template: NotebookTemplate) => {
    setNotebookCells(template.cells);
    setMainView('notebook');
  };

  const addCell = (type: 'code' | 'markdown', afterId?: string) => {
    const newCell: NotebookCell = {
      id: `cell-${Date.now()}`,
      type,
      content: type === 'markdown' ? '# New Section' : '# Add your code here'
    };

    if (afterId) {
      setNotebookCells(prev => {
        const index = prev.findIndex(cell => cell.id === afterId);
        const newCells = [...prev];
        newCells.splice(index + 1, 0, newCell);
        return newCells;
      });
    } else {
      setNotebookCells(prev => [...prev, newCell]);
    }
  };

  const deleteCell = (cellId: string) => {
    setNotebookCells(prev => prev.filter(cell => cell.id !== cellId));
  };

  const addCellAfter = (cellId: string, type: 'code' | 'markdown') => {
    const newCell: NotebookCell = {
      id: `cell-${Date.now()}`,
      type,
      content: type === 'markdown' ? '# New Section' : '# Add your code here'
    };
    
    setNotebookCells(prev => {
      const index = prev.findIndex(cell => cell.id === cellId);
      const newCells = [...prev];
      newCells.splice(index + 1, 0, newCell);
      return newCells;
    });
  };

  const toggleAiAnalysis = (cellId: string) => {
    setNotebookCells(prev =>
      prev.map(cell =>
        cell.id === cellId ? { ...cell, showAiAnalysis: !cell.showAiAnalysis } : cell
      )
    );
  };

  const updateCellContent = (cellId: string, content: string) => {
    setNotebookCells(prev => 
      prev.map(cell => 
        cell.id === cellId ? { ...cell, content } : cell
      )
    );
  };

  const executeCell = async (cellId: string) => {
    // Find the cell to execute
    const cell = notebookCells.find(c => c.id === cellId);
    if (!cell) return;

    // Mark cell as executing
    setNotebookCells(prev => 
      prev.map(c => 
        c.id === cellId ? { ...c, isExecuting: true } : c
      )
    );

    try {
      // Execute the code through Jupyter backend
      const result = await notebookService.executeCode(cell.content);
      
      // Update cell with output or error
      setNotebookCells(prev => 
        prev.map(c => 
          c.id === cellId 
            ? { 
                ...c, 
                isExecuting: false, 
                output: result.error || result.output || 'No output'
              } 
            : c
        )
      );
    } catch (error) {
      // Handle execution error
      setNotebookCells(prev => 
        prev.map(c => 
          c.id === cellId 
            ? { 
                ...c, 
                isExecuting: false, 
                output: `Error: ${error instanceof Error ? error.message : 'Unknown error'}`
              } 
            : c
        )
      );
    }
    
    // Auto-scroll to the cell with output
    setTimeout(() => {
      const element = document.getElementById(`cell-${cellId}`);
      if (element) {
        const outputElement = element.querySelector('[class*="cellOutput"]');
        if (outputElement) {
          outputElement.scrollIntoView({ behavior: 'smooth', block: 'end' });
        } else {
          element.scrollIntoView({ behavior: 'smooth', block: 'center' });
        }
      }
    }, 100);
  };

  // Explore page handlers
  const handleTagClick = (tag: string) => {
    setSearchTerms(prev => {
      if (prev.includes(tag)) {
        return prev.filter(t => t !== tag);
      } else {
        return [...prev, tag];
      }
    });
  };

  const handleStrategySelect = (strategy: Strategy) => {
    if (!strategy.comingSoon) {
      if (strategy.id === 'custom') {
        setActiveTab('builder');
        setMainView('builder');
        setSelectedTemplate('custom');
      } else {
        setTearsheet({ strategy, isOpen: true });
      }
    }
  };

  const handleNotebookClick = (e: React.MouseEvent, strategy: Strategy) => {
    e.stopPropagation();
    
    // On mobile, open the builder view instead of notebook
    if (isMobile) {
      setActiveTab('builder');
      setMainView('builder');
      setSelectedTemplate(strategy.id);
      setSidebarOpen(false); // Close sidebar on mobile after selection
      return;
    }
    
    // Desktop behavior - open notebook
    const analysisCell: NotebookCell = {
      id: `cell-${Date.now()}`,
      type: 'markdown',
      content: `# ${strategy.title} Analysis\n\nAnalyzing strategy: **${strategy.title}**\n\n**Description:** ${strategy.description}\n\n**Creator:** ${strategy.creator ? `@${strategy.creator}` : 'Unknown'}\n\n**Tags:** ${strategy.tags.join(', ')}`
    };
    
    const codeCell: NotebookCell = {
      id: `cell-${Date.now() + 1}`,
      type: 'code',
      content: `# Load strategy data for analysis\nimport admf\n\n# Load ${strategy.title} strategy data\nsignals = admf.load_signals(strategy_id='${strategy.id}')\nprint(f"Loaded strategy: ${strategy.title}")\nprint(f"Expected Sharpe: ${strategy.metrics?.sharpe || 'N/A'}")\nprint(f"Expected Annual Return: ${strategy.metrics?.annualReturn || 'N/A'}%")`
    };
    
    setNotebookCells([analysisCell, codeCell]);
    setActiveTab('notebooks');
    setMainView('notebook');
  };

  const handleDataNotebookClick = (dataCard: DataCard) => {
    // On mobile, open the builder view
    if (isMobile) {
      setActiveTab('builder');
      setMainView('builder');
      setSidebarOpen(false);
      return;
    }
    
    // Desktop behavior - open notebook with data analysis
    const analysisCell: NotebookCell = {
      id: `cell-${Date.now()}`,
      type: 'markdown',
      content: `# ${dataCard.title} Data Analysis\n\nAnalyzing data source: **${dataCard.title}**\n\n**Description:** ${dataCard.description}\n\n**Provider:** ${dataCard.provider ? `@${dataCard.provider}` : 'Unknown'}\n\n**Coverage:** ${dataCard.coverage || 'N/A'}\n\n**Update Frequency:** ${dataCard.frequency || 'N/A'}\n\n**Tags:** ${dataCard.tags.join(', ')}`
    };
    
    const codeCell: NotebookCell = {
      id: `cell-${Date.now() + 1}`,
      type: 'code',
      content: `import admf
import pandas as pd
from snippets import data, analysis, visualization

# Connect to data source
data_source = admf.connect_data("${dataCard.id}")
df = data_source.fetch_latest(limit=1000)

# Display data overview
print(f"Data shape: {df.shape}")
print(f"Date range: {df.index.min()} to {df.index.max()}")
print(f"\\nFirst 5 rows:")
print(df.head())

# Basic statistics
print(f"\\nSummary statistics:")
print(df.describe())`
    };
    
    setNotebookCells([analysisCell, codeCell]);
    setActiveCell(codeCell.id);
    setActiveTab('notebooks');
    setMainView('notebook');
  };
  
  const handleDeployClick = (e: React.MouseEvent, strategy: Strategy) => {
    e.stopPropagation();
    navigate('/monitor', { state: { strategy } });
  };

  const filterAndSortStrategies = () => {
    let filtered = allStrategies;

    // Multi-tag filter
    const allSearchTerms = [...searchTerms];
    if (exploreSearchQuery.trim()) {
      allSearchTerms.push(...exploreSearchQuery.toLowerCase().split(' ').filter(term => term.length > 0));
    }

    if (allSearchTerms.length > 0) {
      filtered = filtered.filter(strategy => {
        const searchableText = [
          strategy.title.toLowerCase(),
          strategy.description.toLowerCase(),
          ...strategy.tags.map(tag => tag.toLowerCase())
        ];
        
        if (strategy.creator) {
          searchableText.push(strategy.creator.toLowerCase());
          searchableText.push(`@${strategy.creator.toLowerCase()}`);
        }
        
        return allSearchTerms.every(term => 
          searchableText.some(text => text.includes(term))
        );
      });
    }

    // Sort
    return filtered.sort((a, b) => {
      if (!a.metrics || !b.metrics) return 0;
      
      switch (sortBy) {
        case 'new':
          // Reverse order to show newest first (higher indices first)
          return allStrategies.indexOf(b) - allStrategies.indexOf(a);
        case 'sharpe':
          return b.metrics.sharpe - a.metrics.sharpe;
        case 'returns':
          return b.metrics.annualReturn - a.metrics.annualReturn;
        case 'winrate':
          return b.metrics.winRate - a.metrics.winRate;
        case 'name':
          return a.title.localeCompare(b.title);
        default:
          return 0;
      }
    });
  };

  const filterAndSortDataCards = () => {
    console.log('filterAndSortDataCards called with datasets:', datasets);
    // Convert backend datasets to DataCard format
    const backendDataCards: DataCard[] = datasets.map((dataset, index) => ({
      id: `backend-${dataset.symbol}-${dataset.exchange}`,
      title: `${dataset.symbol} (Live)`,
      dataType: 'Market Data', 
      description: `Live ${dataset.interval} market data from ${dataset.exchange} exchange`,
      provider: dataset.exchange.charAt(0).toUpperCase() + dataset.exchange.slice(1),
      tags: [
        dataset.symbol.includes('BTC') || dataset.symbol.includes('ETH') || 
        dataset.symbol.includes('SOL') || dataset.symbol.includes('LINK') ? 'crypto' : 'stocks',
        dataset.exchange,
        dataset.interval,
        'live',
        dataset.symbol.toLowerCase().includes('btc') ? 'bitcoin' : '',
        dataset.symbol.toLowerCase().includes('btc') ? 'btc' : '',
        dataset.symbol.toLowerCase().includes('eth') ? 'ethereum' : '',
        dataset.symbol.toLowerCase().includes('sol') ? 'solana' : '',
        dataset.symbol.toLowerCase().includes('link') ? 'chainlink' : ''
      ].filter(tag => tag !== ''),
      frequency: dataset.interval,
      period: `${new Date(dataset.startTime * 1000).toLocaleDateString()} - ${new Date(dataset.endTime * 1000).toLocaleDateString()}`,
      color: dataset.symbol.includes('BTC') ? '#f7931a' : 
             dataset.symbol.includes('ETH') ? '#627eea' : 
             dataset.symbol.includes('SOL') ? '#14f195' :
             dataset.symbol.includes('LINK') ? '#2a5ada' : '#00c805',
      records: dataset.candleCount.toLocaleString(),
      size: `${(dataset.candleCount * 0.1).toFixed(1)} MB`, // Estimate
      lastUpdated: new Date(dataset.lastUpdated).toLocaleDateString()
    }));
    
    // Combine static cards with backend data
    let filtered = [...allDataCards, ...backendDataCards];

    // Multi-tag filter
    const allSearchTerms = [...searchTerms];
    if (exploreSearchQuery.trim()) {
      allSearchTerms.push(...exploreSearchQuery.toLowerCase().split(' ').filter(term => term.length > 0));
    }

    if (allSearchTerms.length > 0) {
      filtered = filtered.filter(dataCard => {
        const searchableText = [
          dataCard.title.toLowerCase(),
          dataCard.description.toLowerCase(),
          ...dataCard.tags.map(tag => tag.toLowerCase())
        ];
        
        if (dataCard.provider) {
          searchableText.push(dataCard.provider.toLowerCase());
          searchableText.push(`@${dataCard.provider.toLowerCase()}`);
        }
        
        if (dataCard.dataType) {
          searchableText.push(dataCard.dataType.toLowerCase());
        }
        
        return allSearchTerms.every(term => 
          searchableText.some(text => text.includes(term))
        );
      });
    }

    // Sort by name for now (can add more sort options later)
    return filtered.sort((a, b) => {
      switch (sortBy) {
        case 'name':
          return a.title.localeCompare(b.title);
        case 'new':
          return allDataCards.indexOf(b) - allDataCards.indexOf(a);
        default:
          return a.title.localeCompare(b.title);
      }
    });
  };

  const renderStrategyCard = (strategy: Strategy) => {
    return (
      <StrategyCard
        key={strategy.id}
        strategy={strategy}
        isHovered={hoveredCard === strategy.id}
        onMouseEnter={() => setHoveredCard(strategy.id)}
        onMouseLeave={() => setHoveredCard(null)}
        onStrategySelect={() => handleStrategySelect(strategy)}
        onTagClick={handleTagClick}
        onNotebookClick={(e) => handleNotebookClick(e, strategy)}
        onDeployClick={(e) => handleDeployClick(e, strategy)}
        searchTerms={searchTerms}
      />
    );
  };

  const renderDataCard = (dataCard: DataCard) => {
    return (
      <DataCardComponent
        key={dataCard.id}
        data={dataCard}
        isHovered={hoveredCard === dataCard.id}
        onMouseEnter={() => setHoveredCard(dataCard.id)}
        onMouseLeave={() => setHoveredCard(null)}
        onDataSelect={() => {
          setDataDetails({ data: dataCard, isOpen: true });
        }}
        onNotebookClick={(e) => {
          e.stopPropagation();
          handleDataNotebookClick(dataCard);
        }}
        onTagClick={handleTagClick}
        searchTerms={searchTerms}
      />
    );
  };

  const renderSidebarContent = () => {
    // When in explore view, check if we're showing strategies or data
    if (mainView === 'explore') {
      // If viewing data, show data categories
      if (exploreViewType === 'data') {
        const dataCards = filterAndSortDataCards();
        const dataCategories = {
          'Crypto': dataCards.filter(d => d.tags?.includes('crypto')),
          'Stocks': dataCards.filter(d => d.tags?.includes('stocks')),
          'Live Data': dataCards.filter(d => d.tags?.includes('live')),
          'Historical': dataCards.filter(d => !d.tags?.includes('live')),
          'Coinbase': dataCards.filter(d => d.tags?.includes('coinbase')),
          'Kraken': dataCards.filter(d => d.tags?.includes('kraken')),
          'High Frequency': dataCards.filter(d => d.frequency === '1m' || d.frequency === '5m'),
          'Daily': dataCards.filter(d => d.frequency === '1d' || d.frequency === 'Daily')
        };

        return (
          <div className={styles.tabContent}>
            {/* Categories with data cards */}
            {Object.entries(dataCategories).map(([category, categoryData]) => (
              categoryData.length > 0 && (
                <div key={category} className={styles.strategyCategory}>
                  <div 
                    className={`${styles.categoryHeader} ${collapsedCategories.has(category) ? styles.collapsed : ''}`}
                    onClick={() => toggleCategory(category)}
                  >
                    <span className={styles.categoryArrow}>▼</span>
                    <span>{category} ({categoryData.length})</span>
                  </div>
                  {!collapsedCategories.has(category) && (
                    <div className={styles.strategyList}>
                      {categoryData.slice(0, 5).map(data => (
                        <div 
                          key={data.id}
                          className={styles.strategyItem}
                          onClick={() => {
                            setDataDetails({ data, isOpen: true });
                          }}
                        >
                          <div className={styles.strategyName}>{data.title}</div>
                          <div className={styles.strategyDesc}>
                            {data.provider} • {data.records || 'N/A'} records
                          </div>
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              )
            ))}
          </div>
        );
      } else {
        // Show strategies when in strategy view
        const strategies = filterAndSortStrategies();
        const strategyCategories = {
          'Trending': strategies.filter(s => s.tags.includes('trending')),
          'Mean Reversion': strategies.filter(s => s.tags.includes('mean-reversion')),
          'Momentum': strategies.filter(s => s.tags.includes('momentum')),
          'Machine Learning': strategies.filter(s => s.tags.includes('ml')),
          'High Frequency': strategies.filter(s => s.tags.includes('high-frequency')),
          'Options': strategies.filter(s => s.tags.includes('options')),
          'Crypto': strategies.filter(s => s.tags.includes('crypto')),
          'Forex': strategies.filter(s => s.tags.includes('forex'))
        };

        return (
          <div className={styles.tabContent}>
            {/* Categories with strategies - no header text */}
            {Object.entries(strategyCategories).map(([category, categoryStrategies]) => (
              categoryStrategies.length > 0 && (
                <div key={category} className={styles.strategyCategory}>
                  <div 
                    className={`${styles.categoryHeader} ${collapsedCategories.has(category) ? styles.collapsed : ''}`}
                    onClick={() => toggleCategory(category)}
                  >
                    <span className={styles.categoryArrow}>▼</span>
                    <span>{category} ({categoryStrategies.length})</span>
                  </div>
                  {!collapsedCategories.has(category) && (
                    <div className={styles.strategyList}>
                      {categoryStrategies.slice(0, 5).map(strategy => (
                        <div 
                          key={strategy.id}
                          className={styles.strategyItem}
                          onClick={() => {
                            setTearsheet({ strategy, isOpen: true });
                          }}
                        >
                          <div className={styles.strategyName}>{strategy.title}</div>
                          <div className={styles.strategyDesc}>
                            {strategy.metrics.sharpe.toFixed(2)} Sharpe • {strategy.metrics.winRate}% Win
                          </div>
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              )
            ))}
          </div>
        );
      }
    }
    
    // Data Explorer view
    if (mainView === 'data') {
      return (
        <div className={styles.tabContent}>
          {/* Cached Datasets */}
          <div className={styles.dataCategory}>
            <div 
              className={`${styles.categoryHeader} ${collapsedCategories.has('Cached Data') ? styles.collapsed : ''}`}
              onClick={() => toggleCategory('Cached Data')}
              style={{ display: 'flex', alignItems: 'center' }}
            >
              <span className={styles.categoryArrow}>▼</span>
              <span>Market Data (Parquet Files)</span>
              <button 
                className={styles.refreshBtn}
                onClick={async (e) => {
                  e.stopPropagation();
                  setLoadingDatasets(true);
                  
                  try {
                    const response = await fetch('http://localhost:5001/api/data/summary');
                    const data = await response.json();
                    const backendDatasets = data.symbols.map((symbol: any) => ({
                      symbol: symbol.symbol,
                      exchange: symbol.exchange,
                      interval: '1m',
                      startTime: new Date(symbol.first_bar).getTime() / 1000,
                      endTime: new Date(symbol.last_bar).getTime() / 1000,
                      candleCount: symbol.bar_count,
                      lastUpdated: Date.now()
                    }));
                    setDatasets(backendDatasets);
                  } catch (error) {
                    console.error('Failed to refresh datasets:', error);
                  }
                  setLoadingDatasets(false);
                }}
                style={{ 
                  marginLeft: 'auto',
                  padding: '2px 8px',
                  fontSize: '12px',
                  background: 'transparent',
                  border: '1px solid var(--border)',
                  borderRadius: '4px',
                  cursor: 'pointer'
                }}
              >
                Refresh
              </button>
            </div>
            {!collapsedCategories.has('Cached Data') && (
              <div className={styles.datasetList}>
                {loadingDatasets ? (
                  <div className={styles.datasetItem}>
                    <div className={styles.datasetName}>Loading datasets...</div>
                  </div>
                ) : datasets.length === 0 ? (
                  <div className={styles.datasetItem}>
                    <div className={styles.datasetName}>No cached data yet</div>
                    <div className={styles.datasetInfo}>Open the Monitor page to fetch and cache market data</div>
                  </div>
                ) : (
                  datasets.map((dataset, index) => {
                    const startDate = new Date(dataset.startTime * 1000).toLocaleDateString();
                    const endDate = new Date(dataset.endTime * 1000).toLocaleDateString();
                    const duration = Math.round((dataset.endTime - dataset.startTime) / (60 * 60 * 24));
                    
                    return (
                      <div key={index} className={styles.datasetItem} onClick={() => {
                        // Export dataset as JSON
                        dataStorage.exportToJSON({
                          symbol: dataset.symbol,
                          exchange: dataset.exchange,
                          interval: dataset.interval
                        }).then(json => {
                          const blob = new Blob([json], { type: 'application/json' });
                          const url = URL.createObjectURL(blob);
                          const a = document.createElement('a');
                          a.href = url;
                          a.download = `${dataset.symbol}_${dataset.exchange}_${dataset.interval}.json`;
                          a.click();
                          URL.revokeObjectURL(url);
                        });
                      }}>
                        <div className={styles.datasetName}>
                          {dataset.symbol} • {dataset.exchange.toUpperCase()} • {dataset.interval}
                        </div>
                        <div className={styles.datasetInfo}>
                          {dataset.candleCount.toLocaleString()} candles • {duration} days • {startDate} to {endDate}
                        </div>
                      </div>
                    );
                  })
                )}
              </div>
            )}
          </div>

          {/* Data Analysis Tools */}
          <div className={styles.dataCategory}>
            <div 
              className={`${styles.categoryHeader} ${collapsedCategories.has('Analysis') ? styles.collapsed : ''}`}
              onClick={() => toggleCategory('Analysis')}
              style={{ display: 'flex', alignItems: 'center' }}
            >
              <span className={styles.categoryArrow}>▼</span>
              <span>Data Analysis</span>
            </div>
            {!collapsedCategories.has('Analysis') && (
              <div className={styles.datasetList}>
                {datasets.length >= 2 ? (
                  <div className={styles.datasetItem} 
                    onClick={async () => {
                      try {
                        // Assume first two datasets are BTC and ETH
                        const symbol1 = datasets[0].symbol.replace('/', '-');
                        const symbol2 = datasets[1].symbol.replace('/', '-');
                        
                        const response = await fetch(`http://localhost:5001/api/data/correlation/${symbol1}/${symbol2}`);
                        const data = await response.json();
                        
                        alert(`Correlation between ${datasets[0].symbol} and ${datasets[1].symbol}: ${data.correlation?.toFixed(4) || 'N/A'}\n\n` +
                              `${datasets[0].symbol} Stats:\n` +
                              `- Volatility: ${data.symbol1_stats?.annualized_volatility?.toFixed(4) || 'N/A'}\n` +
                              `- Sharpe Ratio: ${data.symbol1_stats?.sharpe_ratio?.toFixed(4) || 'N/A'}\n\n` +
                              `${datasets[1].symbol} Stats:\n` +
                              `- Volatility: ${data.symbol2_stats?.annualized_volatility?.toFixed(4) || 'N/A'}\n` +
                              `- Sharpe Ratio: ${data.symbol2_stats?.sharpe_ratio?.toFixed(4) || 'N/A'}`);
                      } catch (error) {
                        console.error('Correlation analysis failed:', error);
                        alert('Failed to calculate correlation. Make sure the backend is running.');
                      }
                    }}
                    style={{ cursor: 'pointer', backgroundColor: 'var(--color-bg-secondary)' }}
                  >
                    <div className={styles.datasetName}>🔗 Correlation Analysis</div>
                    <div className={styles.datasetInfo}>
                      Calculate correlation between {datasets[0]?.symbol || 'Symbol 1'} and {datasets[1]?.symbol || 'Symbol 2'}
                    </div>
                  </div>
                ) : (
                  <div className={styles.datasetItem}>
                    <div className={styles.datasetName}>Need at least 2 datasets for analysis</div>
                    <div className={styles.datasetInfo}>Load more data from the Monitor page</div>
                  </div>
                )}
                
                <div className={styles.datasetItem}
                  onClick={() => {
                    // SQL Query interface - simple example
                    const query = prompt('Enter DuckDB SQL Query (SELECT only):', 
                      'SELECT symbol, AVG(close) as avg_price, COUNT(*) as bars FROM ohlcv GROUP BY symbol');
                    
                    if (query && query.trim().toUpperCase().startsWith('SELECT')) {
                      fetch('http://localhost:5001/api/data/query', {
                        method: 'POST',
                        headers: { 'Content-Type': 'application/json' },
                        body: JSON.stringify({ query })
                      })
                      .then(response => response.json())
                      .then(data => {
                        console.log('Query results:', data);
                        alert(`Query executed successfully!\nRows: ${data.rows}\nColumns: ${data.columns?.join(', ')}\n\nCheck console for full results.`);
                      })
                      .catch(error => {
                        console.error('Query failed:', error);
                        alert('Query failed. Check the console for details.');
                      });
                    }
                  }}
                  style={{ cursor: 'pointer', backgroundColor: 'var(--color-bg-secondary)' }}
                >
                  <div className={styles.datasetName}>💾 SQL Query Interface</div>
                  <div className={styles.datasetInfo}>Run custom SQL queries on DuckDB data</div>
                </div>
              </div>
            )}
          </div>
          
          {/* Parquet Files (Backend) */}
          <div className={styles.dataCategory}>
            <div 
              className={`${styles.categoryHeader} ${collapsedCategories.has('Parquet Files') ? styles.collapsed : ''}`}
              onClick={() => toggleCategory('Parquet Files')}
            >
              <span className={styles.categoryArrow}>▼</span>
              <span>Parquet Files (Backend Catalog)</span>
            </div>
            {!collapsedCategories.has('Parquet Files') && (
              <div className={styles.datasetList}>
                {loadingBackendData ? (
                  <div className={styles.datasetItem}>
                    <div className={styles.datasetName}>Loading backend catalog...</div>
                  </div>
                ) : backendData?.bars?.length > 0 ? (
                  backendData.bars.map((file: any, index: number) => (
                    <div key={index} className={styles.datasetItem} onClick={() => {}}>
                      <div className={styles.datasetName}>
                        {file.symbol} • {file.timeframe}
                      </div>
                      <div className={styles.datasetInfo}>
                        catalog/data/bar/{file.filename} • {(file.size / 1024 / 1024).toFixed(1)}MB
                      </div>
                    </div>
                  ))
                ) : (
                  <div className={styles.datasetItem}>
                    <div className={styles.datasetName}>No backend data available</div>
                    <div className={styles.datasetInfo}>Data will appear here once collected</div>
                  </div>
                )}
              </div>
            )}
          </div>
          
          <div className={styles.dataCategory}>
            <div 
              className={`${styles.categoryHeader} ${collapsedCategories.has('Signals') ? styles.collapsed : ''}`}
              onClick={() => toggleCategory('Signals')}
            >
              <span className={styles.categoryArrow}>▼</span>
              <span>Signals & Features</span>
            </div>
            {!collapsedCategories.has('Signals') && (
              <div className={styles.datasetList}>
                {backendData?.signals?.length > 0 ? (
                  backendData.signals.map((file: any, index: number) => (
                    <div key={index} className={styles.datasetItem} onClick={() => {}}>
                      <div className={styles.datasetName}>{file.filename}</div>
                      <div className={styles.datasetInfo}>
                        {(file.size / 1024 / 1024).toFixed(1)}MB • Features
                      </div>
                    </div>
                  ))
                ) : (
                  <>
                    <div className={styles.datasetItem} onClick={() => {}}>
                      <div className={styles.datasetName}>momentum_signals.parquet</div>
                      <div className={styles.datasetInfo}>500K rows • 120MB • Features</div>
                    </div>
                    <div className={styles.datasetItem} onClick={() => {}}>
                      <div className={styles.datasetName}>ml_features_v2.parquet</div>
                      <div className={styles.datasetInfo}>2M rows • 380MB • ML features</div>
                    </div>
                  </>
                )}
              </div>
            )}
          </div>
          
          <div className={styles.dataCategory}>
            <div 
              className={`${styles.categoryHeader} ${collapsedCategories.has('Backtests') ? styles.collapsed : ''}`}
              onClick={() => toggleCategory('Backtests')}
            >
              <span className={styles.categoryArrow}>▼</span>
              <span>Backtest Results</span>
            </div>
            {!collapsedCategories.has('Backtests') && (
              <div className={styles.datasetList}>
                {backendData?.backtests?.length > 0 ? (
                  backendData.backtests.map((file: any, index: number) => (
                    <div key={index} className={styles.datasetItem} onClick={() => {}}>
                      <div className={styles.datasetName}>{file.filename}</div>
                      <div className={styles.datasetInfo}>
                        {(file.size / 1024 / 1024).toFixed(1)}MB • Performance
                      </div>
                    </div>
                  ))
                ) : (
                  <div className={styles.datasetItem} onClick={() => {}}>
                    <div className={styles.datasetName}>ema_cross_results.parquet</div>
                    <div className={styles.datasetInfo}>10K rows • 5MB • Performance</div>
                  </div>
                )}
              </div>
            )}
          </div>
          
          {/* Quick Actions */}
          <div className={styles.dataActions}>
            <button className={styles.dataActionBtn}>
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <path d="M12 5v14M5 12h14"></path>
              </svg>
              Upload Dataset
            </button>
            <button className={styles.dataActionBtn}>
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect>
                <line x1="9" y1="9" x2="15" y2="9"></line>
                <line x1="9" y1="15" x2="15" y2="15"></line>
              </svg>
              SQL Query
            </button>
          </div>
        </div>
      );
    }
    
    switch (activeTab) {
      case 'notebooks':
        return (
          <div className={styles.tabContent}>
            {/* Code Snippets Section */}
            {Object.entries(codeSnippets).map(([category, snippets]) => (
              <div key={category} className={styles.snippetCategory}>
                <div 
                  className={`${styles.categoryHeader} ${collapsedCategories.has(category) ? styles.collapsed : ''}`}
                  onClick={() => toggleCategory(category)}
                >
                  <span className={styles.categoryArrow}>▼</span>
                  <span>{category}</span>
                </div>
                {!collapsedCategories.has(category) && (
                  <div className={styles.snippetList}>
                    {snippets
                      .filter(snippet => 
                        snippet.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
                        snippet.code.toLowerCase().includes(searchQuery.toLowerCase())
                      )
                      .map(snippet => (
                        <div 
                          key={snippet.id} 
                          className={styles.snippetItem}
                          onClick={() => insertSnippet(snippet)}
                        >
                          <div>
                            <div className={styles.snippetName}>{snippet.name}</div>
                            {snippet.description && (
                              <div className={styles.snippetDesc}>{snippet.description}</div>
                            )}
                          </div>
                          <span className={styles.insertIcon}>+</span>
                        </div>
                      ))}
                  </div>
                )}
              </div>
            ))}
            
            {/* Templates Section */}
            <div className={styles.templateCategory}>
              <div className={`${styles.categoryHeader} ${collapsedCategories.has('Templates') ? styles.collapsed : ''}`} onClick={() => toggleCategory('Templates')}>
                <span className={styles.categoryArrow}>▼</span>
                <span>Analysis Templates</span>
              </div>
              {!collapsedCategories.has('Templates') && (
                <NotebookTemplatesList 
                  templates={notebookTemplates}
                  onTemplateSelect={loadTemplate}
                />
              )}
            </div>
            
            {/* Saved Notebooks Section */}
            <div className={styles.notebookBrowser}>
              <div className={styles.notebookCategory}>
                <div className={`${styles.categoryHeader} ${collapsedCategories.has('Saved Notebooks') ? styles.collapsed : ''}`} onClick={() => toggleCategory('Saved Notebooks')}>
                  <span className={styles.categoryArrow}>▼</span>
                  <span>Saved Notebooks</span>
                </div>
                {!collapsedCategories.has('Saved Notebooks') && (
                  <SavedNotebooksList notebooks={savedNotebooks} />
                )}
              </div>
            </div>
          </div>
        );

      case 'builder':
        return (
          <div className={styles.tabContent}>
            {/* Strategies Section */}
            <div className={styles.strategyCategory}>
              <div 
                className={`${styles.categoryHeader} ${collapsedCategories.has('Strategies') ? styles.collapsed : ''}`}
                onClick={() => toggleCategory('Strategies')}
              >
                <span className={styles.categoryArrow}>▼</span>
                <span>Strategies</span>
              </div>
              {!collapsedCategories.has('Strategies') && (
                <div className={styles.strategyList}>
                  <div 
                    className={styles.strategyItem}
                    onClick={() => {
                      setSelectedTemplate('custom');
                      setMainView('builder');
                    }}
                  >
                    <div className={styles.strategyName}>New Strategy</div>
                    <div className={styles.strategyDesc}>Create from scratch</div>
                  </div>
                  <div 
                    className={styles.strategyItem}
                    onClick={() => {
                      setSelectedTemplate('oversold_bounce');
                      setMainView('builder');
                    }}
                  >
                    <div className={styles.strategyName}>Oversold Bounce</div>
                    <div className={styles.strategyDesc}>RSI mean reversion</div>
                  </div>
                </div>
              )}
            </div>
            
            {/* Templates Section */}
            <div className={styles.templateCategory}>
              <div 
                className={`${styles.categoryHeader} ${collapsedCategories.has('Templates') ? styles.collapsed : ''}`}
                onClick={() => toggleCategory('Templates')}
              >
                <span className={styles.categoryArrow}>▼</span>
                <span>Templates</span>
              </div>
              {!collapsedCategories.has('Templates') && (
                <div className={styles.templateList}>
                  <div 
                    className={styles.templateItem}
                    onClick={() => {
                      setSelectedTemplate('signal_analysis');
                      setMainView('builder');
                    }}
                  >
                    <div className={styles.templateName}>Signal Analysis</div>
                    <div className={styles.templateDesc}>Analyze signals across search space</div>
                  </div>
                </div>
              )}
            </div>
          </div>
        );

      default:
        return null;
    }
  };

  const renderMainContent = () => {
    if (mainView === 'explore') {
      const strategies = filterAndSortStrategies();
      const dataCards = filterAndSortDataCards();
      return (
        <ExploreView
          exploreSearchQuery={exploreSearchQuery}
          setExploreSearchQuery={setExploreSearchQuery}
          sortBy={sortBy}
          setSortBy={setSortBy}
          sortDropdownOpen={sortDropdownOpen}
          setSortDropdownOpen={setSortDropdownOpen}
          searchTerms={searchTerms}
          onTagClick={handleTagClick}
          onNewStrategy={() => {
            setActiveTab('builder');
            setMainView('builder');
            console.log('Opening new strategy builder');
          }}
          displayLimit={displayLimit}
          totalResults={strategies.length}
          strategies={strategies}
          renderStrategyCard={renderStrategyCard}
          onLoadMore={() => setDisplayLimit(prev => prev + 12)}
          onShowAll={() => setDisplayLimit(strategies.length)}
          tearsheet={tearsheet}
          setTearsheet={setTearsheet}
          onNotebookClick={(strategy) => handleNotebookClick(new MouseEvent('click') as any, strategy)}
          viewType={exploreViewType}
          setViewType={setExploreViewType}
          dataCards={dataCards}
          renderDataCard={renderDataCard}
          dataDetails={dataDetails}
          setDataDetails={setDataDetails}
          onDataNotebookClick={handleDataNotebookClick}
        />
      );
    }
    
    if (mainView === 'builder') {
      return (
        <BuilderView
          selectedTemplate={selectedTemplate}
          setSelectedTemplate={setSelectedTemplate}
          setActiveTab={setActiveTab}
          setMainView={setMainView}
        />
      );
    }
    
    return (
      <NotebookView
        notebookCells={notebookCells}
        setNotebookCells={setNotebookCells}
        activeCell={activeCell}
        setActiveCell={setActiveCell}
        deleteCell={deleteCell}
        executeCell={executeCell}
        updateCellContent={updateCellContent}
        addCellAfter={addCellAfter}
        toggleAiAnalysis={toggleAiAnalysis}
        editorTheme={editorTheme}
        addCell={addCell}
      />
    );
  };

  return (
    <div 
      className={styles.researchContainer}
      onTouchStart={handleTouchStart}
      onTouchMove={handleTouchMove}
      onTouchEnd={handleTouchEnd}
    >
      {/* Overlay for Mobile */}
      <MobileOverlay 
        show={isMobile && sidebarOpen} 
        onClose={() => setSidebarOpen(false)} 
      />
      
      {/* Swipe Indicator for Mobile */}
      <MobileSwipeIndicator show={isMobile && !sidebarOpen} />
      
      {/* Sidebar */}
      <aside className={`${styles.snippetsSidebar} ${sidebarOpen ? styles.open : ''}`}>
        <div className={styles.sidebarHeader}>
          <SidebarTabs
            mainView={mainView}
            setMainView={setMainView}
            handleTabSwitch={handleTabSwitch}
          />
        </div>
        
        <div className={styles.sidebarContent}>
          {/* Add a header when in explore mode to clarify what's shown */}
          {mainView === 'explore' && (
            <div style={{ 
              padding: '8px 16px', 
              borderBottom: '1px solid var(--border)',
              marginBottom: '8px',
              fontSize: '12px',
              fontWeight: 600,
              color: 'var(--text-secondary)'
            }}>
              {exploreViewType === 'data' ? 'Data Catalog' : 'Strategy Directory'}
            </div>
          )}
          {renderSidebarContent()}
        </div>
      </aside>

      {/* Main Content */}
      <main className={styles.mainArea}>
        {renderMainContent()}
      </main>
    </div>
  );
};

export default ResearchPage;
