import React from 'react';
import styles from '../../MonitorPage/MonitorPage.module.css';

type SidebarTab = 'metrics' | 'events' | 'strategies';

interface MetricData {
  totalPnL: number;
  winRate: number;
  sharpeRatio: number;
  maxDrawdown: number;
  totalTrades: number;
  avgTrade: number;
}

interface EventData {
  time: string;
  type: 'buy' | 'sell' | 'signal';
  description: string;
}

interface Strategy {
  id: string;
  name: string;
  winRate: number;
  pnl: number;
  active?: boolean;
}

interface MetricsSidebarProps {
  sidebarTab: SidebarTab;
  setSidebarTab: (tab: SidebarTab) => void;
  mockMetrics: MetricData;
  eventData: EventData[];
  mockStrategies: Strategy[];
  currentBar: number;
  selectedStrategy: string;
  setSelectedStrategy: (strategy: string) => void;
  styles: Record<string, string>;
}

export type { SidebarTab };

const MetricsSidebar: React.FC<MetricsSidebarProps> = ({
  sidebarTab,
  setSidebarTab,
  mockMetrics,
  eventData,
  mockStrategies,
  currentBar,
  selectedStrategy,
  setSelectedStrategy,
  styles
}) => {
  const getVisibleEvents = () => {
    const visibleEventCount = Math.floor(currentBar / 10);
    return eventData.slice(0, visibleEventCount);
  };

  return (
    <div className={styles.sidebar}>
      <div className={styles.sidebarHeader}>
        <div className={styles.sidebarTabs}>
          <button
            className={`${styles.sidebarTab} ${sidebarTab === 'metrics' ? styles.active : ''}`}
            onClick={() => setSidebarTab('metrics')}
          >
            Metrics
          </button>
          <button
            className={`${styles.sidebarTab} ${sidebarTab === 'events' ? styles.active : ''}`}
            onClick={() => setSidebarTab('events')}
          >
            Events
          </button>
          <button
            className={`${styles.sidebarTab} ${sidebarTab === 'strategies' ? styles.active : ''}`}
            onClick={() => setSidebarTab('strategies')}
          >
            Strategies
          </button>
        </div>
      </div>

      <div className={styles.sidebarContent}>
        {/* Metrics Tab */}
        {sidebarTab === 'metrics' && (
          <div className={styles.metricsGrid}>
            <div className={styles.metricCard}>
              <div className={styles.metricLabel}>Total P&L</div>
              <div className={`${styles.metricValue} ${mockMetrics.totalPnL > 0 ? styles.positive : styles.negative}`}>
                {mockMetrics.totalPnL > 0 ? '+' : ''}${mockMetrics.totalPnL.toFixed(2)}
              </div>
            </div>
            <div className={styles.metricCard}>
              <div className={styles.metricLabel}>Win Rate</div>
              <div className={styles.metricValue}>{mockMetrics.winRate.toFixed(1)}%</div>
            </div>
            <div className={styles.metricCard}>
              <div className={styles.metricLabel}>Sharpe Ratio</div>
              <div className={styles.metricValue}>{mockMetrics.sharpeRatio.toFixed(2)}</div>
            </div>
            <div className={styles.metricCard}>
              <div className={styles.metricLabel}>Max Drawdown</div>
              <div className={`${styles.metricValue} ${styles.negative}`}>
                {mockMetrics.maxDrawdown.toFixed(1)}%
              </div>
            </div>
            <div className={styles.metricCard}>
              <div className={styles.metricLabel}>Total Trades</div>
              <div className={styles.metricValue}>{mockMetrics.totalTrades}</div>
            </div>
            <div className={styles.metricCard}>
              <div className={styles.metricLabel}>Avg Trade</div>
              <div className={`${styles.metricValue} ${mockMetrics.avgTrade > 0 ? styles.positive : styles.negative}`}>
                {mockMetrics.avgTrade > 0 ? '+' : ''}${mockMetrics.avgTrade.toFixed(2)}
              </div>
            </div>
          </div>
        )}

        {/* Events Tab */}
        {sidebarTab === 'events' && (
          <div className={styles.eventLog}>
            {getVisibleEvents().map((event, index) => (
              <div key={index} className={styles.eventItem}>
                <span className={styles.eventTime}>{event.time}</span>
                <span className={`${styles.eventType} ${event.type === 'buy' ? styles.buy : event.type === 'sell' ? styles.sell : ''}`}>
                  {event.type.toUpperCase()}
                </span>
                <span className={styles.eventMessage}>{event.description}</span>
              </div>
            ))}
          </div>
        )}

        {/* Strategies Tab */}
        {sidebarTab === 'strategies' && (
          <div className={styles.strategyList}>
            {mockStrategies.map((strategy) => (
              <div
                key={strategy.id}
                className={`${styles.strategyItem} ${strategy.active ? styles.active : ''}`}
                onClick={() => setSelectedStrategy(strategy.name)}
              >
                <div className={styles.strategyName}>{strategy.name}</div>
                <div className={styles.strategyStats}>
                  <span>Win: {strategy.winRate}%</span>
                  <span>P&L: +{strategy.pnl.toFixed(1)}%</span>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
};

export default MetricsSidebar;