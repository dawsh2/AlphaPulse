/**
 * SystemOverview - High-level system health dashboard
 * Shows overall system status, key metrics, and quick stats
 */
import React from 'react';
import type { SystemStatus, ServiceInfo, StreamInfo, PositionInfo, SystemAlert } from './SystemMonitorPanel';
import styles from '../../MonitorPage/MonitorPage.module.css';

interface SystemOverviewProps {
  systemStatus: SystemStatus;
  services: ServiceInfo[];
  streams: StreamInfo[];
  positions: PositionInfo[];
  alerts: SystemAlert[];
}

const SystemOverview: React.FC<SystemOverviewProps> = ({
  systemStatus,
  services,
  streams,
  positions,
  alerts
}) => {
  // Calculate summary statistics
  const runningServices = services.filter(s => s.status === 'running').length;
  const connectedStreams = streams.filter(s => s.status === 'connected').length;
  const totalPortfolioValue = positions.reduce((sum, pos) => sum + pos.marketValue, 0);
  const totalUnrealizedPnL = positions.reduce((sum, pos) => sum + pos.unrealizedPnL, 0);
  const activeAlerts = alerts.filter(a => !a.resolved).length;
  const criticalAlerts = alerts.filter(a => !a.resolved && a.type === 'error').length;

  const formatUptime = (seconds: number): string => {
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    return `${hours}h ${minutes}m`;
  };

  const formatCurrency = (value: number): string => {
    return new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency: 'USD',
      minimumFractionDigits: 2
    }).format(value);
  };

  const getStatusColor = (status: string): string => {
    switch (status) {
      case 'healthy':
      case 'running':
      case 'connected':
        return 'healthy';
      case 'warning':
        return 'warning';
      case 'critical':
      case 'error':
      case 'stopped':
      case 'disconnected':
        return 'critical';
      default:
        return 'warning';
    }
  };

  return (
    <div className={styles.systemOverviewContainer}>
      {/* System Health Header */}
      <div className={styles.systemCard}>
        <div className={styles.systemCardHeader}>
          <h3 className={styles.systemCardTitle}>
            <span className={`${styles.statusIndicator} ${styles[getStatusColor(systemStatus.overall)]}`}></span>
            AlphaPulse System Status
          </h3>
          <div className={styles.systemDetails}>
            <span>Uptime: {formatUptime(systemStatus.uptime)}</span>
            <span>Last Check: {systemStatus.timestamp.toLocaleTimeString()}</span>
          </div>
        </div>
        
        <div className={styles.systemGrid}>
          {/* System Health Card */}
          <div className={styles.overviewCard}>
            <h4 className={styles.overviewCardTitle}>System Health</h4>
            <div className={styles.overviewMetric}>
              <span className={styles.overviewValue}>
                <span className={`${styles.statusIndicator} ${styles[getStatusColor(systemStatus.overall)]}`}></span>
                {systemStatus.overall.toUpperCase()}
              </span>
            </div>
          </div>

          {/* Services Status */}
          <div className={styles.overviewCard}>
            <h4 className={styles.overviewCardTitle}>Services</h4>
            <div className={styles.overviewMetric}>
              <span className={styles.overviewValue}>
                {runningServices} / {services.length}
              </span>
              <span className={styles.overviewLabel}>Running</span>
            </div>
          </div>

          {/* Data Streams */}
          <div className={styles.overviewCard}>
            <h4 className={styles.overviewCardTitle}>Data Streams</h4>
            <div className={styles.overviewMetric}>
              <span className={styles.overviewValue}>
                {connectedStreams} / {streams.length}
              </span>
              <span className={styles.overviewLabel}>Connected</span>
            </div>
          </div>

          {/* Portfolio Value */}
          <div className={styles.overviewCard}>
            <h4 className={styles.overviewCardTitle}>Portfolio Value</h4>
            <div className={styles.overviewMetric}>
              <span className={styles.overviewValue}>
                {formatCurrency(totalPortfolioValue)}
              </span>
              <span className={`${styles.overviewLabel} ${totalUnrealizedPnL >= 0 ? styles.positive : styles.negative}`}>
                {totalUnrealizedPnL >= 0 ? '+' : ''}{formatCurrency(totalUnrealizedPnL)} P&L
              </span>
            </div>
          </div>

          {/* Active Alerts */}
          <div className={styles.overviewCard}>
            <h4 className={styles.overviewCardTitle}>Alerts</h4>
            <div className={styles.overviewMetric}>
              <span className={`${styles.overviewValue} ${criticalAlerts > 0 ? styles.critical : ''}`}>
                {activeAlerts}
              </span>
              <span className={styles.overviewLabel}>
                {criticalAlerts > 0 ? `${criticalAlerts} Critical` : 'Active'}
              </span>
            </div>
          </div>

          {/* Active Positions */}
          <div className={styles.overviewCard}>
            <h4 className={styles.overviewCardTitle}>Positions</h4>
            <div className={styles.overviewMetric}>
              <span className={styles.overviewValue}>
                {positions.length}
              </span>
              <span className={styles.overviewLabel}>Open</span>
            </div>
          </div>
        </div>
      </div>

      {/* Quick Service Status */}
      <div className={styles.systemCard}>
        <div className={styles.systemCardHeader}>
          <h3 className={styles.systemCardTitle}>Core Services</h3>
        </div>
        
        <div className={styles.serviceQuickList}>
          {services.slice(0, 6).map((service) => (
            <div key={service.name} className={styles.serviceQuickItem}>
              <div className={styles.systemInfo}>
                <span className={`${styles.statusIndicator} ${styles[getStatusColor(service.status)]}`}></span>
                <span className={styles.serviceName}>{service.name}</span>
              </div>
              <div className={styles.systemDetails}>
                {service.port && <span>:{service.port}</span>}
                {service.uptime && <span>{formatUptime(service.uptime)}</span>}
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Recent Alerts */}
      {activeAlerts > 0 && (
        <div className={styles.systemCard}>
          <div className={styles.systemCardHeader}>
            <h3 className={styles.systemCardTitle}>Recent Alerts</h3>
          </div>
          
          <div className={styles.alertQuickList}>
            {alerts.filter(a => !a.resolved).slice(0, 3).map((alert) => (
              <div key={alert.id} className={`${styles.alertQuickItem} ${styles[alert.type]}`}>
                <div className={styles.alertInfo}>
                  <span className={`${styles.statusIndicator} ${styles[alert.type]}`}></span>
                  <span className={styles.alertMessage}>{alert.message}</span>
                </div>
                <div className={styles.alertMeta}>
                  <span>{alert.source}</span>
                  <span>{alert.timestamp.toLocaleTimeString()}</span>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* System Performance Snapshot */}
      <div className={styles.systemCard}>
        <div className={styles.systemCardHeader}>
          <h3 className={styles.systemCardTitle}>Performance Snapshot</h3>
        </div>
        
        <div className={styles.performanceGrid}>
          <div className={styles.performanceItem}>
            <span className={styles.performanceLabel}>Memory Usage</span>
            <span className={styles.performanceValue}>
              {Math.round(services.reduce((sum, s) => sum + (s.memory || 0), 0))} MB
            </span>
          </div>
          <div className={styles.performanceItem}>
            <span className={styles.performanceLabel}>CPU Usage</span>
            <span className={styles.performanceValue}>
              {Math.round(services.reduce((sum, s) => sum + (s.cpu || 0), 0) / services.length || 0)}%
            </span>
          </div>
          <div className={styles.performanceItem}>
            <span className={styles.performanceLabel}>Data Latency</span>
            <span className={styles.performanceValue}>
              {Math.round(streams.reduce((sum, s) => sum + (s.latency || 0), 0) / streams.length || 0)} ms
            </span>
          </div>
          <div className={styles.performanceItem}>
            <span className={styles.performanceLabel}>Messages/min</span>
            <span className={styles.performanceValue}>
              {streams.reduce((sum, s) => sum + s.messageCount, 0)}
            </span>
          </div>
        </div>
      </div>
    </div>
  );
};

export default SystemOverview;