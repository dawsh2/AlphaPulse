import React from 'react';
import './DataFlowMonitor.css';

import type { Trade, OrderBook, Metrics } from '../types';

interface Props {
  trades: Trade[];
  orderbooks: Record<string, OrderBook>;
  metrics: Metrics | null;
}

export function DataFlowMonitor({ trades, orderbooks, metrics }: Props) {
  // Dynamically determine exchanges from actual data
  const activeExchanges = Array.from(new Set(trades.map(t => t.exchange))).sort();
  const exchanges = activeExchanges.length > 0 ? activeExchanges : ['coinbase']; // Fallback
  
  const getExchangeStats = (exchange: string) => {
    const exchangeTrades = trades.filter(t => t.exchange === exchange);
    const recentTrades = exchangeTrades.filter(t => 
      Date.now() - t.timestamp < 5000
    );
    
    return {
      tradesPerSec: recentTrades.length / 5,
      totalTrades: exchangeTrades.length,
      connected: recentTrades.length > 0
    };
  };

  return (
    <div className="data-flow-monitor">
      <div className="panel-header">
        <h3 className="panel-title">Data Flow Monitor</h3>
        <div className="flow-stats">
          <span>Total Trades: {trades.length}</span>
          <span>Active Streams: {Object.keys(orderbooks).length}</span>
        </div>
      </div>

      <div className="flow-visualization">
        <svg viewBox="0 0 800 300" className="flow-svg">
          {/* Exchange nodes */}
          {exchanges.map((exchange, index) => {
            const stats = getExchangeStats(exchange);
            const y = 50 + index * 80;
            
            return (
              <g key={exchange}>
                {/* Exchange node */}
                <rect
                  x="50"
                  y={y}
                  width="120"
                  height="50"
                  fill={stats.connected ? '#1a3a2a' : '#3a1a1a'}
                  stroke={stats.connected ? 'var(--accent-green)' : 'var(--accent-red)'}
                  strokeWidth="2"
                  rx="5"
                />
                <text x="110" y={y + 20} fill="var(--text-primary)" textAnchor="middle" fontSize="12">
                  {exchange.toUpperCase()}
                </text>
                <text x="110" y={y + 35} fill="var(--text-muted)" textAnchor="middle" fontSize="10">
                  {stats.tradesPerSec.toFixed(1)}/s
                </text>

                {/* Flow line */}
                {stats.connected && (
                  <line
                    x1="170"
                    y1={y + 25}
                    x2="350"
                    y2="150"
                    stroke="var(--accent-green)"
                    strokeWidth="2"
                    strokeDasharray="5,5"
                    className="flow-line"
                  />
                )}
              </g>
            );
          })}

          {/* Rust Collectors node */}
          <rect
            x="350"
            y="125"
            width="150"
            height="50"
            fill="#1a2a3a"
            stroke="var(--accent-blue)"
            strokeWidth="2"
            rx="5"
          />
          <text x="425" y="145" fill="var(--text-primary)" textAnchor="middle" fontSize="12">
            RUST COLLECTORS
          </text>
          <text x="425" y="160" fill="var(--text-muted)" textAnchor="middle" fontSize="10">
            {metrics?.trades_per_second?.toFixed(1) || 0}/s
          </text>

          {/* Redis Streams node */}
          <rect
            x="580"
            y="125"
            width="150"
            height="50"
            fill="#2a1a3a"
            stroke="var(--accent-yellow)"
            strokeWidth="2"
            rx="5"
          />
          <text x="655" y="145" fill="var(--text-primary)" textAnchor="middle" fontSize="12">
            REDIS STREAMS
          </text>
          <text x="655" y="160" fill="var(--text-muted)" textAnchor="middle" fontSize="10">
            {metrics?.redis_stream_length || 0} msgs
          </text>

          {/* Flow arrow from Rust to Redis */}
          <line
            x1="500"
            y1="150"
            x2="580"
            y2="150"
            stroke="var(--accent-blue)"
            strokeWidth="2"
            markerEnd="url(#arrowhead)"
          />

          {/* Arrow marker definition */}
          <defs>
            <marker
              id="arrowhead"
              markerWidth="10"
              markerHeight="10"
              refX="9"
              refY="3"
              orient="auto"
            >
              <polygon
                points="0 0, 10 3, 0 6"
                fill="var(--accent-blue)"
              />
            </marker>
          </defs>
        </svg>
      </div>

      <div className="flow-metrics">
        <div className="metric-card">
          <span className="metric-label">Latency</span>
          <span className="metric-value">{metrics?.latency_ms || 0}ms</span>
        </div>
        <div className="metric-card">
          <span className="metric-label">Connections</span>
          <span className="metric-value">{metrics?.active_connections || 0}</span>
        </div>
        <div className="metric-card">
          <span className="metric-label">OB Updates/s</span>
          <span className="metric-value">{metrics?.orderbook_updates_per_second?.toFixed(1) || 0}</span>
        </div>
      </div>
    </div>
  );
}