import React from 'react';
import './SystemStatus.css';
import type { SystemStatus as SystemStatusType } from '../types';

interface Props {
  status: SystemStatusType;
}

export function SystemStatus({ status }: Props) {
  const formatUptime = (seconds: number) => {
    const days = Math.floor(seconds / 86400);
    const hours = Math.floor((seconds % 86400) / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    return `${days}d ${hours}h ${minutes}m`;
  };

  return (
    <div className="system-status">
      <div className="panel-header">
        <h3 className="panel-title">System Status</h3>
        <span className="uptime">Uptime: {formatUptime(status.uptime_seconds)}</span>
      </div>

      <div className="status-grid">
        <div className="status-item">
          <span className="label">CPU</span>
          <div className="progress-bar">
            <div 
              className="progress-fill cpu"
              style={{ width: `${status.cpu_percent}%` }}
            />
          </div>
          <span className="value">{status.cpu_percent.toFixed(1)}%</span>
        </div>

        <div className="status-item">
          <span className="label">Memory</span>
          <div className="progress-bar">
            <div 
              className="progress-fill memory"
              style={{ width: `${status.memory_percent}%` }}
            />
          </div>
          <span className="value">{status.memory_percent.toFixed(1)}%</span>
        </div>

        <div className="status-item">
          <span className="label">Disk</span>
          <div className="progress-bar">
            <div 
              className="progress-fill disk"
              style={{ width: `${status.disk_percent}%` }}
            />
          </div>
          <span className="value">{status.disk_percent.toFixed(1)}%</span>
        </div>

        <div className="status-item">
          <span className="label">Network</span>
          <div className="network-stats">
            <span>↓ {status.network_rx_kb.toFixed(1)} KB/s</span>
            <span>↑ {status.network_tx_kb.toFixed(1)} KB/s</span>
          </div>
        </div>
      </div>
    </div>
  );
}