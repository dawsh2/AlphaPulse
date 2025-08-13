/**
 * ServiceStatus - Detailed service monitoring component
 * Shows status, performance metrics, and management controls for all services
 */
import React, { useState } from 'react';
import type { ServiceInfo } from './SystemMonitorPanel';
import styles from '../../MonitorPage/MonitorPage.module.css';

interface ServiceStatusProps {
  services: ServiceInfo[];
  onRefresh: () => void;
}

const ServiceStatus: React.FC<ServiceStatusProps> = ({ services, onRefresh }) => {
  const [sortBy, setSortBy] = useState<'name' | 'status' | 'memory' | 'cpu'>('name');
  const [filterStatus, setFilterStatus] = useState<'all' | 'running' | 'stopped' | 'error'>('all');

  const formatUptime = (seconds: number): string => {
    if (!seconds) return 'N/A';
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    const secs = seconds % 60;
    
    if (hours > 0) {
      return `${hours}h ${minutes}m`;
    } else if (minutes > 0) {
      return `${minutes}m ${secs}s`;
    } else {
      return `${secs}s`;
    }
  };

  const formatMemory = (mb: number): string => {
    if (!mb) return 'N/A';
    if (mb >= 1024) {
      return `${(mb / 1024).toFixed(1)} GB`;
    }
    return `${mb.toFixed(0)} MB`;
  };

  const getStatusColor = (status: string): string => {
    switch (status) {
      case 'running':
        return 'healthy';
      case 'stopped':
        return 'stopped';
      case 'error':
        return 'critical';
      default:
        return 'warning';
    }
  };

  const getServiceIcon = (serviceName: string): string => {
    if (serviceName.toLowerCase().includes('flask') || serviceName.toLowerCase().includes('api')) {
      return '🌐';
    } else if (serviceName.toLowerCase().includes('jupyter')) {
      return '📊';
    } else if (serviceName.toLowerCase().includes('frontend') || serviceName.toLowerCase().includes('vite')) {
      return '⚡';
    } else if (serviceName.toLowerCase().includes('websocket') || serviceName.toLowerCase().includes('socket')) {
      return '🔌';
    } else if (serviceName.toLowerCase().includes('database') || serviceName.toLowerCase().includes('db')) {
      return '🗄️';
    } else {
      return '⚙️';
    }
  };

  // Filter and sort services
  const filteredServices = services
    .filter(service => filterStatus === 'all' || service.status === filterStatus)
    .sort((a, b) => {
      switch (sortBy) {
        case 'name':
          return a.name.localeCompare(b.name);
        case 'status':
          return a.status.localeCompare(b.status);
        case 'memory':
          return (b.memory || 0) - (a.memory || 0);
        case 'cpu':
          return (b.cpu || 0) - (a.cpu || 0);
        default:
          return 0;
      }
    });

  const runningCount = services.filter(s => s.status === 'running').length;
  const stoppedCount = services.filter(s => s.status === 'stopped').length;
  const errorCount = services.filter(s => s.status === 'error').length;

  return (
    <div className={styles.serviceStatusContainer}>
      {/* Service Status Header */}
      <div className={styles.systemCard}>
        <div className={styles.systemCardHeader}>
          <h3 className={styles.systemCardTitle}>Service Status</h3>
          <button className={styles.refreshButton} onClick={onRefresh}>
            🔄 Refresh
          </button>
        </div>

        {/* Service Summary */}
        <div className={styles.serviceSummary}>
          <div className={styles.summaryItem}>
            <span className={`${styles.statusIndicator} ${styles.healthy}`}></span>
            <span className={styles.summaryCount}>{runningCount}</span>
            <span className={styles.summaryLabel}>Running</span>
          </div>
          <div className={styles.summaryItem}>
            <span className={`${styles.statusIndicator} ${styles.stopped}`}></span>
            <span className={styles.summaryCount}>{stoppedCount}</span>
            <span className={styles.summaryLabel}>Stopped</span>
          </div>
          <div className={styles.summaryItem}>
            <span className={`${styles.statusIndicator} ${styles.critical}`}></span>
            <span className={styles.summaryCount}>{errorCount}</span>
            <span className={styles.summaryLabel}>Error</span>
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
              <option value="all">All Services</option>
              <option value="running">Running</option>
              <option value="stopped">Stopped</option>
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
              <option value="memory">Memory Usage</option>
              <option value="cpu">CPU Usage</option>
            </select>
          </div>
        </div>
      </div>

      {/* Services List */}
      <div className={styles.systemCard}>
        <div className={styles.servicesList}>
          {filteredServices.length === 0 ? (
            <div className={styles.emptyState}>
              <p>No services found matching the current filter.</p>
            </div>
          ) : (
            filteredServices.map((service) => (
              <div key={service.name} className={styles.serviceItem}>
                {/* Service Info */}
                <div className={styles.serviceInfo}>
                  <div className={styles.serviceHeader}>
                    <span className={styles.serviceIcon}>{getServiceIcon(service.name)}</span>
                    <span className={styles.serviceName}>{service.name}</span>
                    <span className={`${styles.statusIndicator} ${styles[getStatusColor(service.status)]}`}></span>
                    <span className={`${styles.serviceStatus} ${styles[getStatusColor(service.status)]}`}>
                      {service.status.toUpperCase()}
                    </span>
                  </div>

                  <div className={styles.serviceDetails}>
                    {service.port && (
                      <div className={styles.serviceDetail}>
                        <span className={styles.detailLabel}>Port:</span>
                        <span className={styles.detailValue}>{service.port}</span>
                      </div>
                    )}
                    {service.pid && (
                      <div className={styles.serviceDetail}>
                        <span className={styles.detailLabel}>PID:</span>
                        <span className={styles.detailValue}>{service.pid}</span>
                      </div>
                    )}
                    {service.uptime && (
                      <div className={styles.serviceDetail}>
                        <span className={styles.detailLabel}>Uptime:</span>
                        <span className={styles.detailValue}>{formatUptime(service.uptime)}</span>
                      </div>
                    )}
                  </div>
                </div>

                {/* Performance Metrics */}
                <div className={styles.serviceMetrics}>
                  <div className={styles.metricGroup}>
                    <div className={styles.metric}>
                      <span className={styles.metricLabel}>Memory</span>
                      <span className={styles.metricValue}>
                        {formatMemory(service.memory || 0)}
                      </span>
                    </div>
                    <div className={styles.metric}>
                      <span className={styles.metricLabel}>CPU</span>
                      <span className={styles.metricValue}>
                        {service.cpu ? `${service.cpu.toFixed(1)}%` : 'N/A'}
                      </span>
                    </div>
                  </div>

                  {/* Performance Bars */}
                  {service.memory && (
                    <div className={styles.performanceBar}>
                      <div className={styles.performanceBarLabel}>Memory Usage</div>
                      <div className={styles.performanceBarTrack}>
                        <div
                          className={styles.performanceBarFill}
                          style={{
                            width: `${Math.min(100, (service.memory / 512) * 100)}%`,
                            backgroundColor: service.memory > 256 ? '#ff6b6b' : '#51cf66'
                          }}
                        />
                      </div>
                    </div>
                  )}

                  {service.cpu && (
                    <div className={styles.performanceBar}>
                      <div className={styles.performanceBarLabel}>CPU Usage</div>
                      <div className={styles.performanceBarTrack}>
                        <div
                          className={styles.performanceBarFill}
                          style={{
                            width: `${Math.min(100, service.cpu)}%`,
                            backgroundColor: service.cpu > 70 ? '#ff6b6b' : service.cpu > 30 ? '#ffd43b' : '#51cf66'
                          }}
                        />
                      </div>
                    </div>
                  )}
                </div>

                {/* Last Check */}
                <div className={styles.serviceLastCheck}>
                  <span className={styles.lastCheckLabel}>Last Check:</span>
                  <span className={styles.lastCheckValue}>
                    {service.lastCheck.toLocaleTimeString()}
                  </span>
                </div>
              </div>
            ))
          )}
        </div>
      </div>
    </div>
  );
};

export default ServiceStatus;