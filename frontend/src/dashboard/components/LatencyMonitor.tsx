import React, { useState, useEffect, useMemo } from 'react';
import type { Trade } from '../types';

interface LatencyMonitorProps {
  trades: Trade[];
}

interface LatencyStats {
  avg: number;
  min: number;
  max: number;
  p95: number;
  count: number;
}

export const LatencyMonitor: React.FC<LatencyMonitorProps> = ({ trades }) => {
  const [timeWindow, setTimeWindow] = useState<number>(60); // seconds

  // Calculate latency statistics from recent trades
  const latencyStats = useMemo(() => {
    const cutoffTime = Date.now() - (timeWindow * 1000);
    const recentTrades = trades.filter(t => t.timestamp > cutoffTime);
    
    const validTrades = recentTrades.filter(t => t.latency_total_us != null);
    
    if (validTrades.length === 0) {
      return {
        total: { avg: 0, min: 0, max: 0, p95: 0, count: 0 },
        collector_to_relay: { avg: 0, min: 0, max: 0, p95: 0, count: 0 },
        relay_to_bridge: { avg: 0, min: 0, max: 0, p95: 0, count: 0 },
        bridge_to_frontend: { avg: 0, min: 0, max: 0, p95: 0, count: 0 }
      };
    }

    const calculateStats = (values: number[]): LatencyStats => {
      if (values.length === 0) return { avg: 0, min: 0, max: 0, p95: 0, count: 0 };
      
      const sorted = [...values].sort((a, b) => a - b);
      const sum = values.reduce((a, b) => a + b, 0);
      const p95Index = Math.floor(values.length * 0.95);
      
      return {
        avg: sum / values.length,
        min: sorted[0],
        max: sorted[sorted.length - 1],
        p95: sorted[p95Index] || sorted[sorted.length - 1],
        count: values.length
      };
    };

    return {
      total: calculateStats(validTrades.map(t => t.latency_total_us!)),
      collector_to_relay: calculateStats(validTrades.map(t => t.latency_collector_to_relay_us!).filter(x => x != null)),
      relay_to_bridge: calculateStats(validTrades.map(t => t.latency_relay_to_bridge_us!).filter(x => x != null)),
      bridge_to_frontend: calculateStats(validTrades.map(t => t.latency_bridge_to_frontend_us!).filter(x => x != null))
    };
  }, [trades, timeWindow]);

  const formatLatency = (microseconds: number): string => {
    if (microseconds < 1000) {
      return `${Math.round(microseconds)}μs`;
    } else if (microseconds < 1000000) {
      return `${(microseconds / 1000).toFixed(1)}ms`;
    } else {
      return `${(microseconds / 1000000).toFixed(2)}s`;
    }
  };

  const getLatencyColor = (microseconds: number): string => {
    if (microseconds < 100) return '#22c55e'; // green - excellent
    if (microseconds < 500) return '#84cc16'; // lime - good
    if (microseconds < 1000) return '#eab308'; // yellow - ok
    if (microseconds < 5000) return '#f97316'; // orange - concerning
    return '#ef4444'; // red - poor
  };

  return (
    <div className="latency-monitor">
      <div className="latency-header">
        <h3>📊 End-to-End Latency Monitor</h3>
        <div className="time-controls">
          <label>Time Window:</label>
          <select value={timeWindow} onChange={(e) => setTimeWindow(Number(e.target.value))}>
            <option value={30}>30s</option>
            <option value={60}>1m</option>
            <option value={300}>5m</option>
            <option value={900}>15m</option>
          </select>
        </div>
      </div>

      <div className="latency-grid">
        {/* Total End-to-End Latency */}
        <div className="latency-section total">
          <h4 style={{ color: getLatencyColor(latencyStats.total.avg) }}>
            🎯 Total E2E Latency
          </h4>
          <div className="latency-stats">
            <div className="stat-row">
              <span>Average:</span>
              <span style={{ color: getLatencyColor(latencyStats.total.avg) }}>
                {formatLatency(latencyStats.total.avg)}
              </span>
            </div>
            <div className="stat-row">
              <span>P95:</span>
              <span style={{ color: getLatencyColor(latencyStats.total.p95) }}>
                {formatLatency(latencyStats.total.p95)}
              </span>
            </div>
            <div className="stat-row">
              <span>Range:</span>
              <span>
                {formatLatency(latencyStats.total.min)} - {formatLatency(latencyStats.total.max)}
              </span>
            </div>
            <div className="stat-row">
              <span>Samples:</span>
              <span>{latencyStats.total.count}</span>
            </div>
          </div>
        </div>

        {/* Pipeline Breakdown */}
        <div className="latency-section pipeline">
          <h4>🔗 Pipeline Breakdown</h4>
          
          <div className="pipeline-stage">
            <div className="stage-name">Collector → Relay</div>
            <div className="stage-latency">
              {latencyStats.collector_to_relay.count > 0 ? (
                <span style={{ color: getLatencyColor(latencyStats.collector_to_relay.avg) }}>
                  {formatLatency(latencyStats.collector_to_relay.avg)}
                </span>
              ) : (
                <span className="no-data">No data</span>
              )}
            </div>
          </div>

          <div className="pipeline-stage">
            <div className="stage-name">Relay → WebSocket Bridge</div>
            <div className="stage-latency">
              {latencyStats.relay_to_bridge.count > 0 ? (
                <span style={{ color: getLatencyColor(latencyStats.relay_to_bridge.avg) }}>
                  {formatLatency(latencyStats.relay_to_bridge.avg)}
                </span>
              ) : (
                <span className="no-data">No data</span>
              )}
            </div>
          </div>

          <div className="pipeline-stage">
            <div className="stage-name">Bridge → Frontend</div>
            <div className="stage-latency">
              {latencyStats.bridge_to_frontend.count > 0 ? (
                <span style={{ color: getLatencyColor(latencyStats.bridge_to_frontend.avg) }}>
                  {formatLatency(latencyStats.bridge_to_frontend.avg)}
                </span>
              ) : (
                <span className="no-data">No data</span>
              )}
            </div>
          </div>
        </div>
      </div>

      <style jsx>{`
        .latency-monitor {
          background: var(--bg-tertiary);
          border: 1px solid var(--border-color);
          border-radius: 8px;
          padding: 1rem;
          margin: 1rem 0;
        }

        .latency-header {
          display: flex;
          justify-content: space-between;
          align-items: center;
          margin-bottom: 1rem;
          border-bottom: 1px solid var(--border-color);
          padding-bottom: 0.5rem;
        }

        .latency-header h3 {
          margin: 0;
          color: var(--text-primary);
          font-size: 16px;
        }

        .time-controls {
          display: flex;
          align-items: center;
          gap: 0.5rem;
          font-size: 12px;
        }

        .time-controls select {
          background: var(--bg-secondary);
          color: var(--text-primary);
          border: 1px solid var(--border-color);
          border-radius: 4px;
          padding: 0.25rem;
        }

        .latency-grid {
          display: grid;
          grid-template-columns: 1fr 1fr;
          gap: 1rem;
        }

        .latency-section {
          background: var(--bg-secondary);
          border: 1px solid var(--border-color);
          border-radius: 6px;
          padding: 0.75rem;
        }

        .latency-section h4 {
          margin: 0 0 0.75rem 0;
          font-size: 14px;
          font-weight: 600;
        }

        .latency-stats {
          display: flex;
          flex-direction: column;
          gap: 0.5rem;
        }

        .stat-row {
          display: flex;
          justify-content: space-between;
          font-size: 12px;
        }

        .stat-row span:first-child {
          color: var(--text-secondary);
        }

        .stat-row span:last-child {
          font-weight: 600;
          font-family: monospace;
        }

        .pipeline-stage {
          display: flex;
          justify-content: space-between;
          align-items: center;
          padding: 0.5rem;
          margin-bottom: 0.5rem;
          background: var(--bg-tertiary);
          border-radius: 4px;
        }

        .stage-name {
          font-size: 12px;
          color: var(--text-secondary);
        }

        .stage-latency {
          font-family: monospace;
          font-weight: 600;
          font-size: 12px;
        }

        .no-data {
          color: var(--text-secondary);
          font-style: italic;
        }

        @media (max-width: 768px) {
          .latency-grid {
            grid-template-columns: 1fr;
          }
        }
      `}</style>
    </div>
  );
};