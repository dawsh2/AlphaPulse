import React from 'react';
import styles from './GrafanaEmbed.module.css';

export const GrafanaIngestionChart: React.FC = () => {
  // Panel 9 from the fixed_monitor.json - Real-Time Ingestion Rate (per second)
  // Using direct panel URL for embedding
  const grafanaUrl = 'http://localhost:3000';
  const dashboardUid = '30597fec-f389-45ee-a852-ff2378e70db9';
  
  // Build the embed URL for the specific panel (panel ID 9 is the ingestion rate chart)
  const embedUrl = `${grafanaUrl}/d-solo/${dashboardUid}?orgId=1&from=now-5m&to=now&panelId=9&refresh=1s&theme=dark`;

  return (
    <div className={styles.grafanaContainer} style={{ height: '300px' }}>
      <iframe
        src={embedUrl}
        width="100%"
        height="100%"
        frameBorder="0"
        title="Real-Time Ingestion Rate"
        style={{
          border: 'none',
          borderRadius: '8px'
        }}
      />
    </div>
  );
};