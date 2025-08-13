import React from 'react';
import styles from './GrafanaEmbed.module.css';

interface GrafanaEmbedProps {
  dashboardUid: string;
  panelId?: number;
  theme?: 'light' | 'dark';
  refresh?: string; // e.g., '5s', '1m'
  timeRange?: {
    from: string; // e.g., 'now-5m'
    to: string;   // e.g., 'now'
  };
}

export const GrafanaEmbed: React.FC<GrafanaEmbedProps> = ({
  dashboardUid,
  panelId,
  theme = 'dark',
  refresh = '5s',
  timeRange = { from: 'now-5m', to: 'now' }
}) => {
  const grafanaUrl = 'http://localhost:3000'; // Grafana base URL
  
  // Build the embed URL
  const buildEmbedUrl = () => {
    let url = `${grafanaUrl}/d/${dashboardUid}`;
    
    // If specific panel, use panel view
    if (panelId) {
      url = `${grafanaUrl}/d-solo/${dashboardUid}?panelId=${panelId}`;
    }
    
    // Add query parameters
    const params = new URLSearchParams({
      orgId: '1',
      theme: theme,
      refresh: refresh,
      from: timeRange.from,
      to: timeRange.to,
      kiosk: 'true' // Hide Grafana UI chrome
    });
    
    return `${url}${url.includes('?') ? '&' : '?'}${params.toString()}`;
  };

  return (
    <div className={styles.grafanaContainer}>
      <iframe
        src={buildEmbedUrl()}
        width="100%"
        height="100%"
        frameBorder="0"
        title="Grafana Dashboard"
      />
    </div>
  );
};

// Example usage for specific panels
export const SystemStatusPanel: React.FC = () => (
  <GrafanaEmbed 
    dashboardUid="your-dashboard-uid" 
    panelId={1} 
    refresh="1s"
  />
);

export const TradeFlowChart: React.FC = () => (
  <GrafanaEmbed 
    dashboardUid="your-dashboard-uid" 
    panelId={2}
    timeRange={{ from: 'now-30m', to: 'now' }}
  />
);

export const FullDashboard: React.FC = () => (
  <GrafanaEmbed 
    dashboardUid="your-dashboard-uid"
    refresh="5s"
  />
);