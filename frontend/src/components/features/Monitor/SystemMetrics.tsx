/**
 * SystemMetrics - System resource monitoring component
 * Shows CPU, memory, disk, and network usage
 */
import React from 'react';
import styles from '../../MonitorPage/MonitorPage.module.css';

interface SystemMetricsProps {
  className?: string;
}

const SystemMetrics: React.FC<SystemMetricsProps> = ({ className }) => {
  return (
    <div className={`${styles.systemMetricsContainer} ${className}`}>
      <div className={styles.systemCard}>
        <div className={styles.systemCardHeader}>
          <h3 className={styles.systemCardTitle}>System Metrics</h3>
        </div>
        
        <div className={styles.comingSoon}>
          <p>🚧 System metrics monitoring coming soon!</p>
          <p>Will include:</p>
          <ul>
            <li>CPU usage monitoring</li>
            <li>Memory consumption tracking</li>
            <li>Disk I/O statistics</li>
            <li>Network throughput</li>
          </ul>
        </div>
      </div>
    </div>
  );
};

export default SystemMetrics;