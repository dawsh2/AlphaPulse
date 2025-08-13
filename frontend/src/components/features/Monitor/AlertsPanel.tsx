/**
 * AlertsPanel - System alerts and notifications management
 * Shows active alerts with resolution and management capabilities
 */
import React from 'react';
import type { SystemAlert } from './SystemMonitorPanel';
import styles from '../../MonitorPage/MonitorPage.module.css';

interface AlertsPanelProps {
  alerts: SystemAlert[];
  onResolve: (alertId: string) => void;
  onClear: () => void;
}

const AlertsPanel: React.FC<AlertsPanelProps> = ({ alerts, onResolve, onClear }) => {
  const activeAlerts = alerts.filter(alert => !alert.resolved);
  const resolvedAlerts = alerts.filter(alert => alert.resolved);

  const getAlertIcon = (type: string): string => {
    switch (type) {
      case 'error':
        return '❌';
      case 'warning':
        return '⚠️';
      case 'info':
        return 'ℹ️';
      default:
        return '🔔';
    }
  };

  const getAlertColor = (type: string): string => {
    switch (type) {
      case 'error':
        return 'critical';
      case 'warning':
        return 'warning';
      case 'info':
        return 'healthy';
      default:
        return 'warning';
    }
  };

  return (
    <div className={styles.alertsPanelContainer}>
      {/* Alerts Header */}
      <div className={styles.systemCard}>
        <div className={styles.systemCardHeader}>
          <h3 className={styles.systemCardTitle}>System Alerts</h3>
          {resolvedAlerts.length > 0 && (
            <button className={styles.refreshButton} onClick={onClear}>
              🗑️ Clear Resolved
            </button>
          )}
        </div>

        {/* Alerts Summary */}
        <div className={styles.alertsSummary}>
          <div className={styles.summaryItem}>
            <span className={`${styles.statusIndicator} ${styles.critical}`}></span>
            <span className={styles.summaryCount}>
              {activeAlerts.filter(a => a.type === 'error').length}
            </span>
            <span className={styles.summaryLabel}>Critical</span>
          </div>
          <div className={styles.summaryItem}>
            <span className={`${styles.statusIndicator} ${styles.warning}`}></span>
            <span className={styles.summaryCount}>
              {activeAlerts.filter(a => a.type === 'warning').length}
            </span>
            <span className={styles.summaryLabel}>Warning</span>
          </div>
          <div className={styles.summaryItem}>
            <span className={`${styles.statusIndicator} ${styles.healthy}`}></span>
            <span className={styles.summaryCount}>
              {activeAlerts.filter(a => a.type === 'info').length}
            </span>
            <span className={styles.summaryLabel}>Info</span>
          </div>
        </div>
      </div>

      {/* Active Alerts */}
      {activeAlerts.length > 0 && (
        <div className={styles.systemCard}>
          <div className={styles.systemCardHeader}>
            <h4 className={styles.systemCardTitle}>Active Alerts</h4>
          </div>
          
          <div className={styles.alertsList}>
            {activeAlerts.map((alert) => (
              <div key={alert.id} className={`${styles.alertItem} ${styles[alert.type]}`}>
                <div className={styles.alertContent}>
                  <div className={styles.alertHeader}>
                    <span className={styles.alertIcon}>{getAlertIcon(alert.type)}</span>
                    <span className={`${styles.alertType} ${styles[getAlertColor(alert.type)]}`}>
                      {alert.type.toUpperCase()}
                    </span>
                    <span className={styles.alertSource}>{alert.source}</span>
                    <span className={styles.alertTime}>
                      {alert.timestamp.toLocaleTimeString()}
                    </span>
                  </div>
                  
                  <div className={styles.alertMessage}>
                    {alert.message}
                  </div>
                </div>
                
                <div className={styles.alertActions}>
                  <button
                    className={styles.resolveButton}
                    onClick={() => onResolve(alert.id)}
                  >
                    ✓ Resolve
                  </button>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* No Active Alerts */}
      {activeAlerts.length === 0 && (
        <div className={styles.systemCard}>
          <div className={styles.emptyState}>
            <p>✅ No active alerts. System is running smoothly!</p>
          </div>
        </div>
      )}

      {/* Resolved Alerts */}
      {resolvedAlerts.length > 0 && (
        <div className={styles.systemCard}>
          <div className={styles.systemCardHeader}>
            <h4 className={styles.systemCardTitle}>Resolved Alerts ({resolvedAlerts.length})</h4>
          </div>
          
          <div className={styles.alertsList}>
            {resolvedAlerts.slice(0, 5).map((alert) => (
              <div key={alert.id} className={`${styles.alertItem} ${styles.resolved}`}>
                <div className={styles.alertContent}>
                  <div className={styles.alertHeader}>
                    <span className={styles.alertIcon}>✅</span>
                    <span className={styles.alertType}>RESOLVED</span>
                    <span className={styles.alertSource}>{alert.source}</span>
                    <span className={styles.alertTime}>
                      {alert.timestamp.toLocaleTimeString()}
                    </span>
                  </div>
                  
                  <div className={styles.alertMessage}>
                    {alert.message}
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
};

export default AlertsPanel;