/**
 * SystemMonitorPanel - Main system monitoring dashboard
 * Contains all system monitoring components and real-time updates
 */
import React, { useState, useEffect, useRef } from 'react';
import { io, Socket } from 'socket.io-client';
import SystemOverview from './SystemOverview';
import ServiceStatus from './ServiceStatus';
import DataStreams from './DataStreams';
import LivePositions from './LivePositions';
import SystemMetrics from './SystemMetrics';
import AlertsPanel from './AlertsPanel';
import styles from '../../MonitorPage/MonitorPage.module.css';

export interface SystemStatus {
  overall: 'healthy' | 'warning' | 'critical';
  uptime: number;
  timestamp: Date;
}

export interface ServiceInfo {
  name: string;
  status: 'running' | 'stopped' | 'error';
  port?: number;
  pid?: number;
  uptime?: number;
  memory?: number;
  cpu?: number;
  lastCheck: Date;
}

export interface StreamInfo {
  id: string;
  name: string;
  status: 'connected' | 'disconnected' | 'error';
  source: string;
  messageCount: number;
  lastMessage?: Date;
  latency?: number;
}

export interface PositionInfo {
  symbol: string;
  quantity: number;
  marketValue: number;
  unrealizedPnL: number;
  unrealizedPnLPercent: number;
  currentPrice: number;
  avgEntryPrice: number;
}

export interface SystemAlert {
  id: string;
  type: 'error' | 'warning' | 'info';
  message: string;
  timestamp: Date;
  source: string;
  resolved?: boolean;
}

interface SystemMonitorPanelProps {
  className?: string;
}

const SystemMonitorPanel: React.FC<SystemMonitorPanelProps> = ({ className }) => {
  // System monitoring state
  const [systemStatus, setSystemStatus] = useState<SystemStatus>({
    overall: 'healthy',
    uptime: 0,
    timestamp: new Date()
  });

  const [services, setServices] = useState<ServiceInfo[]>([]);
  const [streams, setStreams] = useState<StreamInfo[]>([]);
  const [positions, setPositions] = useState<PositionInfo[]>([]);
  const [alerts, setAlerts] = useState<SystemAlert[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [isConnected, setIsConnected] = useState(false);

  // Tab state for system dashboard
  const [activeTab, setActiveTab] = useState<'overview' | 'services' | 'streams' | 'positions' | 'alerts'>('overview');

  // WebSocket connection ref
  const socketRef = useRef<Socket | null>(null);

  // Fetch system data
  const fetchSystemData = async () => {
    try {
      setIsLoading(true);
      
      // Fetch system status
      const statusResponse = await fetch('/api/system/status');
      if (statusResponse.ok) {
        const statusData = await statusResponse.json();
        setSystemStatus(statusData);
      }

      // Fetch services
      const servicesResponse = await fetch('/api/system/services');
      if (servicesResponse.ok) {
        const servicesData = await servicesResponse.json();
        setServices(servicesData);
      }

      // Fetch streams
      const streamsResponse = await fetch('/api/system/streams');
      if (streamsResponse.ok) {
        const streamsData = await streamsResponse.json();
        setStreams(streamsData);
      }

      // Fetch positions
      const positionsResponse = await fetch('/api/positions');
      if (positionsResponse.ok) {
        const positionsData = await positionsResponse.json();
        if (positionsData.status === 'success') {
          setPositions(positionsData.data);
        }
      }

    } catch (error) {
      console.error('Error fetching system data:', error);
      setAlerts(prev => [...prev, {
        id: Date.now().toString(),
        type: 'error',
        message: `Failed to fetch system data: ${error instanceof Error ? error.message : 'Unknown error'}`,
        timestamp: new Date(),
        source: 'SystemMonitorPanel'
      }]);
    } finally {
      setIsLoading(false);
    }
  };

  // Initialize WebSocket connection for real-time updates
  useEffect(() => {
    // Initial data fetch
    fetchSystemData();
    
    // Connect to WebSocket for real-time updates
    const socket = io('http://localhost:5001', {
      transports: ['websocket'],
      path: '/socket.io/'
    });
    
    socketRef.current = socket;
    
    // Connection event handlers
    socket.on('connect', () => {
      console.log('✅ Connected to system monitoring WebSocket');
      setIsConnected(true);
      
      // Subscribe to system monitoring channels
      socket.emit('subscribe_system_monitoring');
    });
    
    socket.on('disconnect', () => {
      console.log('❌ Disconnected from system monitoring WebSocket');
      setIsConnected(false);
    });
    
    socket.on('connect_error', (error) => {
      console.error('WebSocket connection error:', error);
      setIsConnected(false);
    });
    
    // Real-time data event handlers
    socket.on('system_status_update', (data: SystemStatus) => {
      setSystemStatus({
        ...data,
        timestamp: new Date(data.timestamp)
      });
    });
    
    socket.on('services_update', (data: ServiceInfo[]) => {
      setServices(data.map(service => ({
        ...service,
        lastCheck: new Date(service.lastCheck)
      })));
    });
    
    socket.on('streams_update', (data: StreamInfo[]) => {
      setStreams(data.map(stream => ({
        ...stream,
        lastMessage: stream.lastMessage ? new Date(stream.lastMessage) : undefined
      })));
    });
    
    socket.on('positions_update', (data: PositionInfo[]) => {
      setPositions(data);
    });
    
    socket.on('alert', (alert: SystemAlert) => {
      setAlerts(prev => [{
        ...alert,
        timestamp: new Date(alert.timestamp),
        id: alert.id || Date.now().toString()
      }, ...prev]);
    });
    
    // Cleanup on unmount
    return () => {
      if (socket.connected) {
        socket.emit('unsubscribe_system_monitoring');
        socket.disconnect();
      }
    };
  }, []);

  if (isLoading) {
    return (
      <div className={`${styles.systemDashboard} ${className}`}>
        <div className={styles.systemTitle}>System Dashboard</div>
        <div className={styles.loadingContainer}>
          <div className={styles.loadingSpinner}>
            <div className={styles.phosphorGlow}>Loading system data...</div>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className={`${styles.systemDashboard} ${className}`}>
      <div className={styles.systemHeader}>
        <h2 className={styles.systemTitle}>System Dashboard</h2>
        <div className={styles.connectionStatus}>
          <span className={`${styles.statusIndicator} ${isConnected ? styles.healthy : styles.critical}`}></span>
          <span className={styles.connectionLabel}>
            {isConnected ? 'WebSocket Connected' : 'WebSocket Disconnected'}
          </span>
        </div>
      </div>
      
      {/* System Dashboard Tabs */}
      <div className={styles.systemTabs}>
        <button
          className={`${styles.systemTab} ${activeTab === 'overview' ? styles.active : ''}`}
          onClick={() => setActiveTab('overview')}
        >
          Overview
        </button>
        <button
          className={`${styles.systemTab} ${activeTab === 'services' ? styles.active : ''}`}
          onClick={() => setActiveTab('services')}
        >
          Services
        </button>
        <button
          className={`${styles.systemTab} ${activeTab === 'streams' ? styles.active : ''}`}
          onClick={() => setActiveTab('streams')}
        >
          Data Streams
        </button>
        <button
          className={`${styles.systemTab} ${activeTab === 'positions' ? styles.active : ''}`}
          onClick={() => setActiveTab('positions')}
        >
          Live Positions
        </button>
        <button
          className={`${styles.systemTab} ${activeTab === 'alerts' ? styles.active : ''}`}
          onClick={() => setActiveTab('alerts')}
        >
          Alerts ({alerts.filter(a => !a.resolved).length})
        </button>
      </div>

      {/* System Dashboard Content */}
      <div className={styles.systemContent}>
        {activeTab === 'overview' && (
          <SystemOverview
            systemStatus={systemStatus}
            services={services}
            streams={streams}
            positions={positions}
            alerts={alerts}
          />
        )}
        
        {activeTab === 'services' && (
          <ServiceStatus
            services={services}
            onRefresh={fetchSystemData}
          />
        )}
        
        {activeTab === 'streams' && (
          <DataStreams
            streams={streams}
            onRefresh={fetchSystemData}
          />
        )}
        
        {activeTab === 'positions' && (
          <LivePositions
            positions={positions}
            onRefresh={fetchSystemData}
          />
        )}
        
        {activeTab === 'alerts' && (
          <AlertsPanel
            alerts={alerts}
            onResolve={(alertId) => {
              setAlerts(prev => prev.map(alert =>
                alert.id === alertId ? { ...alert, resolved: true } : alert
              ));
            }}
            onClear={() => setAlerts(prev => prev.filter(alert => !alert.resolved))}
          />
        )}
      </div>
    </div>
  );
};

export default SystemMonitorPanel;