import React from 'react';
import styles from '../../MonitorPage/MonitorPage.module.css';
import type { EventData } from '../../../types/monitor.types';

type PrimaryTab = 'charts' | 'system';
type ChartTab = 'metrics' | 'events' | 'strategies';
type SidebarTab = ChartTab; // Keep for backward compatibility

interface MetricData {
  totalPnL: number;
  winRate: number;
  sharpeRatio: number;
  maxDrawdown: number;
  totalTrades: number;
  avgTrade: number;
}

interface Strategy {
  id: string;
  name: string;
  winRate: number;
  pnl: number;
  active?: boolean;
}

interface MetricsSidebarProps {
  primaryTab: PrimaryTab;
  setPrimaryTab: (tab: PrimaryTab) => void;
  chartTab: ChartTab;
  setChartTab: (tab: ChartTab) => void;
  mockMetrics: MetricData;
  eventData: EventData[];
  mockStrategies: Strategy[];
  currentBar: number;
  selectedStrategy: string;
  setSelectedStrategy: (strategy: string) => void;
  styles: Record<string, string>;
  // Legacy props for backward compatibility
  sidebarTab?: SidebarTab;
  setSidebarTab?: (tab: SidebarTab) => void;
}

export type { SidebarTab, PrimaryTab, ChartTab };

const MetricsSidebar: React.FC<MetricsSidebarProps> = ({
  primaryTab,
  setPrimaryTab,
  chartTab,
  setChartTab,
  mockMetrics,
  eventData,
  mockStrategies,
  currentBar,
  selectedStrategy,
  setSelectedStrategy,
  styles,
  // Legacy props for backward compatibility
  sidebarTab,
  setSidebarTab
}) => {
  // Handle backward compatibility
  const activePrimaryTab = primaryTab || 'charts';
  const activeChartTab = chartTab || sidebarTab || 'metrics';
  const handlePrimaryTabChange = setPrimaryTab || (() => {});
  const handleChartTabChange = setChartTab || setSidebarTab || (() => {});
  const getVisibleEvents = () => {
    const visibleEventCount = Math.floor(currentBar / 10);
    return eventData.slice(0, visibleEventCount);
  };

  return (
    <div className={styles.sidebar}>
      {/* Primary Tabs - Charts vs System */}
      <div className={styles.sidebarHeader}>
        <div className={styles.sidebarTabs}>
          <button
            className={`${styles.sidebarTab} ${activePrimaryTab === 'charts' ? styles.active : ''}`}
            onClick={() => handlePrimaryTabChange('charts')}
          >
            Charts
          </button>
          <button
            className={`${styles.sidebarTab} ${activePrimaryTab === 'system' ? styles.active : ''}`}
            onClick={() => handlePrimaryTabChange('system')}
          >
            System
          </button>
        </div>
      </div>

      <div className={styles.sidebarContent}>
        {/* Charts Tab - Contains existing functionality */}
        {activePrimaryTab === 'charts' && (
          <>
            {/* Chart Sub-tabs */}
            <div className={styles.subTabsContainer}>
              <div className={styles.sidebarTabs}>
                <button
                  className={`${styles.sidebarTab} ${styles.subTab} ${activeChartTab === 'metrics' ? styles.active : ''}`}
                  onClick={() => handleChartTabChange('metrics')}
                >
                  Metrics
                </button>
                <button
                  className={`${styles.sidebarTab} ${styles.subTab} ${activeChartTab === 'events' ? styles.active : ''}`}
                  onClick={() => handleChartTabChange('events')}
                >
                  Events
                </button>
                <button
                  className={`${styles.sidebarTab} ${styles.subTab} ${activeChartTab === 'strategies' ? styles.active : ''}`}
                  onClick={() => handleChartTabChange('strategies')}
                >
                  Strategies
                </button>
              </div>
            </div>

            {/* Chart Tab Content */}
            <div className={styles.chartTabContent}>
              {/* Metrics Tab */}
              {activeChartTab === 'metrics' && (
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
              {activeChartTab === 'events' && (
                <div className={styles.eventLog}>
                  {getVisibleEvents().map((event, index) => (
                    <div key={index} className={styles.eventItem}>
                      <span className={styles.eventTime}>{event.time}</span>
                      <span className={`${styles.eventType} ${event.type === 'signal' ? styles.signal : event.type === 'order' ? styles.order : ''}`}>
                        {event.type.toUpperCase()}
                      </span>
                      <span className={styles.eventMessage}>{event.description}</span>
                    </div>
                  ))}
                </div>
              )}

              {/* Strategies Tab */}
              {activeChartTab === 'strategies' && (
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
          </>
        )}

        {/* System Tab - Sidebar shows minimal info when system dashboard is active */}
        {activePrimaryTab === 'system' && (
          <div className={styles.systemSidebarInfo}>
            <div className={styles.systemInfoCard}>
              <h3>System Monitoring</h3>
              <p>Real-time system metrics and service status are displayed in the main dashboard.</p>
              
              <div className={styles.quickStats}>
                <div className={styles.quickStat}>
                  <span className={styles.quickStatLabel}>Mode:</span>
                  <span className={styles.quickStatValue}>Live Monitoring</span>
                </div>
                <div className={styles.quickStat}>
                  <span className={styles.quickStatLabel}>Update Rate:</span>
                  <span className={styles.quickStatValue}>Real-time (WebSocket)</span>
                </div>
              </div>
              
              <div className={styles.monitoringTip}>
                <p>💡 Tip: The system dashboard updates in real-time via WebSocket connections for instant visibility into system health.</p>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

export default MetricsSidebar;