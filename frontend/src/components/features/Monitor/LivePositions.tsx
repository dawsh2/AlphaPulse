/**
 * LivePositions - Real-time portfolio positions monitoring
 * Shows current positions with live P&L updates
 */
import React from 'react';
import type { PositionInfo } from './SystemMonitorPanel';
import styles from '../../MonitorPage/MonitorPage.module.css';

interface LivePositionsProps {
  positions: PositionInfo[];
  onRefresh: () => void;
}

const LivePositions: React.FC<LivePositionsProps> = ({ positions, onRefresh }) => {
  const formatCurrency = (value: number): string => {
    return new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency: 'USD',
      minimumFractionDigits: 2
    }).format(value);
  };

  const formatPercent = (value: number): string => {
    return `${value >= 0 ? '+' : ''}${value.toFixed(2)}%`;
  };

  const totalValue = positions.reduce((sum, pos) => sum + pos.marketValue, 0);
  const totalPnL = positions.reduce((sum, pos) => sum + pos.unrealizedPnL, 0);

  return (
    <div className={styles.livePositionsContainer}>
      {/* Positions Header */}
      <div className={styles.systemCard}>
        <div className={styles.systemCardHeader}>
          <h3 className={styles.systemCardTitle}>Live Positions</h3>
          <button className={styles.refreshButton} onClick={onRefresh}>
            🔄 Refresh
          </button>
        </div>

        {/* Portfolio Summary */}
        <div className={styles.portfolioSummary}>
          <div className={styles.summaryItem}>
            <span className={styles.summaryCount}>{formatCurrency(totalValue)}</span>
            <span className={styles.summaryLabel}>Total Value</span>
          </div>
          <div className={styles.summaryItem}>
            <span className={`${styles.summaryCount} ${totalPnL >= 0 ? styles.positive : styles.negative}`}>
              {formatCurrency(totalPnL)}
            </span>
            <span className={styles.summaryLabel}>Unrealized P&L</span>
          </div>
          <div className={styles.summaryItem}>
            <span className={styles.summaryCount}>{positions.length}</span>
            <span className={styles.summaryLabel}>Open Positions</span>
          </div>
        </div>
      </div>

      {/* Positions List */}
      <div className={styles.systemCard}>
        {positions.length === 0 ? (
          <div className={styles.emptyState}>
            <p>No open positions found.</p>
          </div>
        ) : (
          <div className={styles.positionsList}>
            {positions.map((position) => (
              <div key={position.symbol} className={styles.positionItem}>
                <div className={styles.positionHeader}>
                  <span className={styles.positionSymbol}>{position.symbol}</span>
                  <span className={styles.positionQuantity}>
                    {position.quantity > 0 ? 'LONG' : 'SHORT'} {Math.abs(position.quantity)}
                  </span>
                </div>
                
                <div className={styles.positionMetrics}>
                  <div className={styles.positionMetric}>
                    <span className={styles.metricLabel}>Market Value</span>
                    <span className={styles.metricValue}>{formatCurrency(position.marketValue)}</span>
                  </div>
                  <div className={styles.positionMetric}>
                    <span className={styles.metricLabel}>Current Price</span>
                    <span className={styles.metricValue}>{formatCurrency(position.currentPrice)}</span>
                  </div>
                  <div className={styles.positionMetric}>
                    <span className={styles.metricLabel}>Avg Entry</span>
                    <span className={styles.metricValue}>{formatCurrency(position.avgEntryPrice)}</span>
                  </div>
                  <div className={styles.positionMetric}>
                    <span className={styles.metricLabel}>Unrealized P&L</span>
                    <span className={`${styles.metricValue} ${position.unrealizedPnL >= 0 ? styles.positive : styles.negative}`}>
                      {formatCurrency(position.unrealizedPnL)} ({formatPercent(position.unrealizedPnLPercent)})
                    </span>
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
};

export default LivePositions;