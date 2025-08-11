/**
 * Refactored Monitor Page Container
 */

import React, { useState, useEffect, useCallback } from 'react';
import { LiveChart } from './LiveChart';
import { MetricsPanel } from './MetricsPanel';
import { EventStream } from './EventStream';
import { useMarketData } from '../../../hooks/useMarketData';
import { useWebSocket } from '../../../hooks/useWebSocket';
import { formatCurrency, formatPercent } from '../../../utils/format';
import { EXCHANGES, TIMEFRAMES } from '../../../constants/markets';
import type { MarketBar } from '../../../types';
import styles from './Monitor.module.css';

interface MonitorContainerProps {
  symbol?: string;
  exchange?: keyof typeof EXCHANGES;
  timeframe?: keyof typeof TIMEFRAMES;
}

export const MonitorContainer: React.FC<MonitorContainerProps> = ({
  symbol = 'BTC/USD',
  exchange = 'COINBASE',
  timeframe = '1m',
}) => {
  // Market data state (hook needs to be implemented or imported)
  const [marketData, setMarketData] = useState<any[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [connected, setConnected] = useState(false);
  const [livePrice, setLivePrice] = useState<number | null>(null);
  
  // TODO: Implement proper market data fetching
  const refresh = useCallback(() => {
    console.log('Refreshing market data for', symbol, timeframe);
    // Implementation needed
  }, [symbol, timeframe]);

  // Events WebSocket
  const {
    lastMessage: lastEvent,
    connected: eventsConnected,
  } = useWebSocket(`${import.meta.env.VITE_WS_URL || 'ws://localhost:5001'}/events`, {
    autoConnect: true,
    onMessage: (event) => {
      addEvent({
        id: `event_${Date.now()}`,
        time: Math.floor(Date.now() / 1000),
        type: event.type || 'info',
        description: event.description,
        metadata: event.metadata,
      });
    },
  });

  // Local state
  const [events, setEvents] = useState<any[]>([]);
  const [selectedTab, setSelectedTab] = useState<'metrics' | 'events' | 'strategies'>('metrics');
  const [crosshairPrice, setCrosshairPrice] = useState<number | null>(null);

  // Mock metrics (in production, these would come from API)
  const metrics = {
    totalPnL: 12456.78,
    winRate: 65.4,
    sharpeRatio: 1.82,
    maxDrawdown: -12.3,
    totalTrades: 247,
    avgTrade: 50.43,
  };

  // Mock strategies
  const strategies = [
    { id: '1', name: 'Mean Reversion v2', winRate: 68, pnl: 12.3, active: true },
    { id: '2', name: 'Momentum Breakout', winRate: 52, pnl: 8.7, active: false },
    { id: '3', name: 'Market Making', winRate: 71, pnl: 5.2, active: false },
  ];

  // Add event helper
  const addEvent = useCallback((event: any) => {
    setEvents(prev => [...prev, event].slice(-100)); // Keep last 100 events
  }, []);

  // Generate mock events on mount
  useEffect(() => {
    const mockEvents = [
      {
        id: 'event_1',
        time: Math.floor(Date.now() / 1000) - 300,
        type: 'signal' as const,
        description: `RSI oversold signal detected for ${symbol}`,
      },
      {
        id: 'event_2',
        time: Math.floor(Date.now() / 1000) - 200,
        type: 'buy' as const,
        description: `Bought 0.5 ${symbol} at ${livePrice ? formatCurrency(livePrice) : '$0'}`,
      },
      {
        id: 'event_3',
        time: Math.floor(Date.now() / 1000) - 100,
        type: 'info' as const,
        description: 'Strategy "Mean Reversion v2" activated',
      },
    ];
    setEvents(mockEvents);
  }, [symbol, livePrice]);

  // Price display component
  const PriceDisplay = () => (
    <div className={styles.priceDisplay}>
      <div className={styles.symbol}>{symbol}</div>
      <div className={styles.price}>
        {livePrice ? formatCurrency(livePrice) : '--'}
      </div>
      {marketData.length > 1 && (
        <div className={styles.priceChange}>
          {(() => {
            const change = ((livePrice || 0) - marketData[0].close) / marketData[0].close;
            return (
              <span className={change >= 0 ? styles.positive : styles.negative}>
                {change >= 0 ? '↑' : '↓'} {formatPercent(Math.abs(change))}
              </span>
            );
          })()}
        </div>
      )}
      {crosshairPrice && (
        <div className={styles.crosshairPrice}>
          Crosshair: {formatCurrency(crosshairPrice)}
        </div>
      )}
    </div>
  );

  // Strategy list component
  const StrategyList = () => (
    <div className={styles.strategyList}>
      {strategies.map(strategy => (
        <div 
          key={strategy.id} 
          className={`${styles.strategyItem} ${strategy.active ? styles.active : ''}`}
        >
          <div className={styles.strategyInfo}>
            <div className={styles.strategyName}>{strategy.name}</div>
            <div className={styles.strategyStats}>
              <span className={styles.strategyStat}>
                Win: {formatPercent(strategy.winRate / 100, 0)}
              </span>
              <span className={styles.strategyStat}>
                P&L: {formatPercent(strategy.pnl / 100)}
              </span>
            </div>
          </div>
          <div className={`${styles.strategyToggle} ${strategy.active ? styles.active : ''}`} />
        </div>
      ))}
    </div>
  );

  if (loading) {
    return <div className={styles.loading}>Loading market data...</div>;
  }

  if (error) {
    return (
      <div className={styles.error}>
        Error loading data: {error}
        <button onClick={refresh}>Retry</button>
      </div>
    );
  }

  return (
    <div className={styles.monitorContainer}>
      <div className={styles.header}>
        <PriceDisplay />
        <div className={styles.connectionStatus}>
          <span className={connected ? styles.connected : styles.disconnected}>
            {connected ? '● Live' : '○ Disconnected'}
          </span>
          <span className={styles.exchange}>{exchange}</span>
        </div>
      </div>

      <div className={styles.mainContent}>
        <div className={styles.chartSection}>
          <LiveChart
            data={marketData}
            height={500}
            showVolume={true}
            onCrosshairMove={setCrosshairPrice}
          />
        </div>

        <div className={styles.sidebar}>
          <div className={styles.tabs}>
            <button
              className={`${styles.tab} ${selectedTab === 'metrics' ? styles.active : ''}`}
              onClick={() => setSelectedTab('metrics')}
            >
              Metrics
            </button>
            <button
              className={`${styles.tab} ${selectedTab === 'events' ? styles.active : ''}`}
              onClick={() => setSelectedTab('events')}
            >
              Events
            </button>
            <button
              className={`${styles.tab} ${selectedTab === 'strategies' ? styles.active : ''}`}
              onClick={() => setSelectedTab('strategies')}
            >
              Strategies
            </button>
          </div>

          <div className={styles.tabContent}>
            {selectedTab === 'metrics' && <MetricsPanel metrics={metrics} />}
            {selectedTab === 'events' && <EventStream events={events} />}
            {selectedTab === 'strategies' && <StrategyList />}
          </div>
        </div>
      </div>
    </div>
  );
};