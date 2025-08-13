/**
 * DataStreams - Data connection monitoring component
 * Shows real-time status of market data feeds and WebSocket connections
 */
import React, { useState } from 'react';
import type { StreamInfo } from './SystemMonitorPanel';
import styles from '../../MonitorPage/MonitorPage.module.css';

interface DataStreamsProps {
  streams: StreamInfo[];
  onRefresh: () => void;
}

const DataStreams: React.FC<DataStreamsProps> = ({ streams, onRefresh }) => {
  const [sortBy, setSortBy] = useState<'name' | 'status' | 'latency' | 'messages'>('name');
  const [filterStatus, setFilterStatus] = useState<'all' | 'connected' | 'disconnected' | 'error'>('all');

  const formatLatency = (ms: number): string => {
    if (!ms) return 'N/A';
    if (ms < 1000) {
      return `${ms.toFixed(0)} ms`;
    }
    return `${(ms / 1000).toFixed(1)} s`;
  };

  const formatMessageCount = (count: number): string => {
    if (count >= 1000000) {
      return `${(count / 1000000).toFixed(1)}M`;
    } else if (count >= 1000) {
      return `${(count / 1000).toFixed(1)}K`;
    }
    return count.toString();
  };

  const getStatusColor = (status: string): string => {
    switch (status) {
      case 'connected':
        return 'healthy';
      case 'disconnected':
        return 'stopped';
      case 'error':
        return 'critical';
      default:
        return 'warning';
    }
  };

  const getStreamIcon = (source: string): string => {
    if (source.toLowerCase().includes('alpaca')) {
      return '🦙';
    } else if (source.toLowerCase().includes('coinbase')) {
      return '🟠';
    } else if (source.toLowerCase().includes('binance')) {
      return '🟡';
    } else if (source.toLowerCase().includes('websocket') || source.toLowerCase().includes('ws')) {
      return '🔌';
    } else if (source.toLowerCase().includes('rest') || source.toLowerCase().includes('api')) {
      return '🌐';
    } else {
      return '📡';
    }
  };

  const getLatencyStatus = (latency: number): 'good' | 'fair' | 'poor' => {
    if (!latency) return 'good';
    if (latency < 100) return 'good';
    if (latency < 500) return 'fair';
    return 'poor';
  };

  // Filter and sort streams
  const filteredStreams = streams
    .filter(stream => filterStatus === 'all' || stream.status === filterStatus)
    .sort((a, b) => {
      switch (sortBy) {
        case 'name':
          return a.name.localeCompare(b.name);
        case 'status':
          return a.status.localeCompare(b.status);
        case 'latency':
          return (a.latency || 0) - (b.latency || 0);
        case 'messages':
          return b.messageCount - a.messageCount;
        default:
          return 0;
      }
    });

  const connectedCount = streams.filter(s => s.status === 'connected').length;
  const disconnectedCount = streams.filter(s => s.status === 'disconnected').length;
  const errorCount = streams.filter(s => s.status === 'error').length;
  const avgLatency = streams.length > 0 
    ? streams.reduce((sum, s) => sum + (s.latency || 0), 0) / streams.length 
    : 0;

  return (
    <div className={styles.dataStreamsContainer}>
      {/* Streams Status Header */}
      <div className={styles.systemCard}>
        <div className={styles.systemCardHeader}>
          <h3 className={styles.systemCardTitle}>Data Streams</h3>
          <button className={styles.refreshButton} onClick={onRefresh}>
            🔄 Refresh
          </button>
        </div>

        {/* Streams Summary */}
        <div className={styles.streamsSummary}>
          <div className={styles.summaryItem}>
            <span className={`${styles.statusIndicator} ${styles.healthy}`}></span>
            <span className={styles.summaryCount}>{connectedCount}</span>
            <span className={styles.summaryLabel}>Connected</span>
          </div>
          <div className={styles.summaryItem}>
            <span className={`${styles.statusIndicator} ${styles.stopped}`}></span>
            <span className={styles.summaryCount}>{disconnectedCount}</span>
            <span className={styles.summaryLabel}>Disconnected</span>
          </div>
          <div className={styles.summaryItem}>
            <span className={`${styles.statusIndicator} ${styles.critical}`}></span>
            <span className={styles.summaryCount}>{errorCount}</span>
            <span className={styles.summaryLabel}>Error</span>
          </div>
          <div className={styles.summaryItem}>
            <span className={styles.summaryCount}>{formatLatency(avgLatency)}</span>
            <span className={styles.summaryLabel}>Avg Latency</span>
          </div>
        </div>
      </div>

      {/* Filters and Controls */}
      <div className={styles.systemCard}>
        <div className={styles.serviceControls}>
          <div className={styles.filterGroup}>
            <label className={styles.filterLabel}>Filter by Status:</label>
            <select
              className={styles.filterSelect}
              value={filterStatus}
              onChange={(e) => setFilterStatus(e.target.value as any)}
            >
              <option value="all">All Streams</option>
              <option value="connected">Connected</option>
              <option value="disconnected">Disconnected</option>
              <option value="error">Error</option>
            </select>
          </div>

          <div className={styles.filterGroup}>
            <label className={styles.filterLabel}>Sort by:</label>
            <select
              className={styles.filterSelect}
              value={sortBy}
              onChange={(e) => setSortBy(e.target.value as any)}
            >
              <option value="name">Name</option>
              <option value="status">Status</option>
              <option value="latency">Latency</option>
              <option value="messages">Message Count</option>
            </select>
          </div>
        </div>
      </div>

      {/* Streams List */}
      <div className={styles.systemCard}>
        <div className={styles.streamsList}>
          {filteredStreams.length === 0 ? (
            <div className={styles.emptyState}>
              <p>No data streams found matching the current filter.</p>
            </div>
          ) : (
            filteredStreams.map((stream) => (
              <div key={stream.id} className={styles.streamItem}>
                {/* Stream Info */}
                <div className={styles.streamInfo}>
                  <div className={styles.streamHeader}>
                    <span className={styles.streamIcon}>{getStreamIcon(stream.source)}</span>
                    <span className={styles.streamName}>{stream.name}</span>
                    <span className={`${styles.statusIndicator} ${styles[getStatusColor(stream.status)]}`}></span>
                    <span className={`${styles.streamStatus} ${styles[getStatusColor(stream.status)]}`}>
                      {stream.status.toUpperCase()}
                    </span>
                  </div>

                  <div className={styles.streamDetails}>
                    <div className={styles.streamDetail}>
                      <span className={styles.detailLabel}>Source:</span>
                      <span className={styles.detailValue}>{stream.source}</span>
                    </div>
                    <div className={styles.streamDetail}>
                      <span className={styles.detailLabel}>Messages:</span>
                      <span className={styles.detailValue}>{formatMessageCount(stream.messageCount)}</span>
                    </div>
                    {stream.lastMessage && (
                      <div className={styles.streamDetail}>
                        <span className={styles.detailLabel}>Last Message:</span>
                        <span className={styles.detailValue}>{stream.lastMessage.toLocaleTimeString()}</span>
                      </div>
                    )}
                  </div>
                </div>

                {/* Stream Metrics */}
                <div className={styles.streamMetrics}>
                  <div className={styles.metricGroup}>
                    <div className={styles.metric}>
                      <span className={styles.metricLabel}>Latency</span>
                      <span className={`${styles.metricValue} ${styles[getLatencyStatus(stream.latency || 0)]}`}>
                        {formatLatency(stream.latency || 0)}
                      </span>
                    </div>
                    <div className={styles.metric}>
                      <span className={styles.metricLabel}>Messages/min</span>
                      <span className={styles.metricValue}>
                        {formatMessageCount(stream.messageCount)}
                      </span>
                    </div>
                  </div>

                  {/* Latency Bar */}
                  {stream.latency && (
                    <div className={styles.performanceBar}>
                      <div className={styles.performanceBarLabel}>Latency Status</div>
                      <div className={styles.performanceBarTrack}>
                        <div
                          className={styles.performanceBarFill}
                          style={{
                            width: `${Math.min(100, (stream.latency / 1000) * 100)}%`,
                            backgroundColor: 
                              stream.latency < 100 ? '#51cf66' : 
                              stream.latency < 500 ? '#ffd43b' : '#ff6b6b'
                          }}
                        />
                      </div>
                    </div>
                  )}

                  {/* Message Throughput Bar */}
                  <div className={styles.performanceBar}>
                    <div className={styles.performanceBarLabel}>Message Throughput</div>
                    <div className={styles.performanceBarTrack}>
                      <div
                        className={styles.performanceBarFill}
                        style={{
                          width: `${Math.min(100, (stream.messageCount / 1000) * 100)}%`,
                          backgroundColor: '#00d4ff'
                        }}
                      />
                    </div>
                  </div>
                </div>

                {/* Connection Health */}
                <div className={styles.connectionHealth}>
                  <div className={styles.healthIndicator}>
                    <span className={`${styles.statusIndicator} ${styles[getStatusColor(stream.status)]}`}></span>
                    <span className={styles.healthLabel}>Connection</span>
                  </div>
                  {stream.latency && (
                    <div className={styles.healthIndicator}>
                      <span className={`${styles.statusIndicator} ${styles[getLatencyStatus(stream.latency)]}`}></span>
                      <span className={styles.healthLabel}>Latency</span>
                    </div>
                  )}
                  <div className={styles.healthIndicator}>
                    <span className={`${styles.statusIndicator} ${stream.messageCount > 0 ? styles.healthy : styles.warning}`}></span>
                    <span className={styles.healthLabel}>Data Flow</span>
                  </div>
                </div>
              </div>
            ))
          )}
        </div>
      </div>
    </div>
  );
};

export default DataStreams;