import React, { useState, useEffect } from 'react';
import './PrometheusMetrics.css';

export function PrometheusMetrics() {
  const [metrics, setMetrics] = useState<any>({});
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const fetchMetrics = async () => {
      try {
        const response = await fetch('/api/metrics');
        const text = await response.text();
        
        // Parse Prometheus metrics format
        const parsed: any = {};
        const lines = text.split('\n');
        
        lines.forEach(line => {
          if (line.startsWith('#') || !line.trim()) return;
          
          const match = line.match(/^([a-z_]+)(?:\{[^}]*\})?\s+(.+)$/);
          if (match) {
            const [, name, value] = match;
            parsed[name] = parseFloat(value);
          }
        });
        
        // Debug: Log parsed metrics
        console.log('Parsed Prometheus metrics:', Object.keys(parsed).length, 'metrics');
        
        setMetrics(parsed);
        setLoading(false);
        setError(null);
      } catch (error) {
        console.error('Failed to fetch metrics:', error);
        setError(error instanceof Error ? error.message : 'Failed to fetch metrics');
        setLoading(false);
      }
    };

    fetchMetrics();
    const interval = setInterval(fetchMetrics, 5000);
    
    return () => clearInterval(interval);
  }, []);

  if (loading) {
    return (
      <div className="prometheus-metrics">
        <div className="panel-header">
          <h3 className="panel-title">Prometheus Metrics</h3>
        </div>
        <div className="loading">Loading metrics...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="prometheus-metrics">
        <div className="panel-header">
          <h3 className="panel-title">Prometheus Metrics</h3>
        </div>
        <div className="error">Error: {error}</div>
      </div>
    );
  }

  return (
    <div className="prometheus-metrics">
      <div className="panel-header">
        <h3 className="panel-title">Prometheus Metrics</h3>
        <a 
          href="http://localhost:9090" 
          target="_blank" 
          rel="noopener noreferrer"
          className="prometheus-link"
        >
          Open Prometheus →
        </a>
      </div>

      <div className="metrics-grid">
        <div className="metric-item">
          <span className="metric-name">HTTP Requests</span>
          <span className="metric-value">
            {metrics.http_requests_total?.toLocaleString() || 0}
          </span>
        </div>

        <div className="metric-item">
          <span className="metric-name">WebSocket Connections</span>
          <span className="metric-value">
            {metrics.websocket_connections_active?.toLocaleString() || 0}
          </span>
        </div>

        <div className="metric-item">
          <span className="metric-name">Redis Stream Length</span>
          <span className="metric-value">
            {metrics.redis_stream_length?.toLocaleString() || 0}
          </span>
        </div>

        <div className="metric-item">
          <span className="metric-name">System Memory</span>
          <span className="metric-value">
            {metrics.system_memory_percent?.toFixed(1) || 0}%
          </span>
        </div>

        <div className="metric-item">
          <span className="metric-name">System CPU</span>
          <span className="metric-value">
            {metrics.system_cpu_percent?.toFixed(1) || 0}%
          </span>
        </div>

        <div className="metric-item">
          <span className="metric-name">System Memory</span>
          <span className="metric-value">
            {metrics.system_memory_percent?.toFixed(1) || 0}%
          </span>
        </div>
      </div>
    </div>
  );
}