/**
 * SystemDashboard - Professional metrics dashboard with real-time visualizations
 * Displays system metrics with charts, gauges, and live data streams
 */
import React, { useState, useEffect, useRef } from 'react';
import { io, Socket } from 'socket.io-client';
import { Line, Bar } from 'react-chartjs-2';
import {
  Chart as ChartJS,
  CategoryScale,
  LinearScale,
  PointElement,
  LineElement,
  BarElement,
  Title,
  Tooltip,
  Legend,
  Filler,
  ArcElement
} from 'chart.js';
import styles from './SystemDashboard.module.css';
import { GrafanaIngestionChart } from './GrafanaIngestionChart';
import { PostgresRealtimeChart } from './PostgresRealtimeChart';
import { ContinuousStreamChart } from './ContinuousStreamChart';

// Register ChartJS components
ChartJS.register(
  CategoryScale,
  LinearScale,
  PointElement,
  LineElement,
  BarElement,
  Title,
  Tooltip,
  Legend,
  Filler,
  ArcElement
);

interface MetricData {
  timestamp: Date;
  value: number;
}

interface ServiceHealth {
  name: string;
  status: 'healthy' | 'warning' | 'critical' | 'offline';
  cpu: number;
  memory: number;
  uptime: number;
}

interface DataStream {
  id: string;
  name: string;
  exchange: string;
  type: 'trades' | 'orderbook' | 'L2';
  status: 'connected' | 'disconnected' | 'error';
  messagesPerSecond: number;
  totalMessages: number;
  latency: number;
  lastMessage: Date | null;
  dataSize: number; // MB written to disk
  errorRate: number;
}

interface SystemDashboardProps {
  className?: string;
}

const SystemDashboard: React.FC<SystemDashboardProps> = ({ className }) => {
  // WebSocket connection
  const socketRef = useRef<Socket | null>(null);
  const [isConnected, setIsConnected] = useState(false);

  // Metrics history (for charts)
  const [cpuHistory, setCpuHistory] = useState<MetricData[]>([]);
  const [memoryHistory, setMemoryHistory] = useState<MetricData[]>([]);
  const [networkHistory, setNetworkHistory] = useState<{ rx: MetricData[], tx: MetricData[] }>({ rx: [], tx: [] });
  const [latencyHistory, setLatencyHistory] = useState<MetricData[]>([]);

  // Current metrics (for gauges and numbers)
  const [currentCpu, setCurrentCpu] = useState(0);
  const [currentMemory, setCurrentMemory] = useState(0);
  const [currentDisk, setCurrentDisk] = useState(0);
  const [systemUptime, setSystemUptime] = useState(0);
  const [activeConnections, setActiveConnections] = useState(0);
  const [requestsPerSecond, setRequestsPerSecond] = useState(0);

  // Services status
  const [services, setServices] = useState<ServiceHealth[]>([]);
  const [dataStreams, setDataStreams] = useState<DataStream[]>([]);
  const [alerts, setAlerts] = useState<any[]>([]);
  
  // Initialize with default streams
  useEffect(() => {
    setDataStreams([
      {
        id: 'coinbase_l2',
        name: 'Coinbase L2 Orderbook',
        exchange: 'Coinbase',
        type: 'L2',
        status: 'disconnected',
        messagesPerSecond: 0,
        totalMessages: 0,
        latency: 0,
        lastMessage: null,
        dataSize: 0,
        errorRate: 0
      },
      {
        id: 'kraken_l2',
        name: 'Kraken L2 Orderbook',
        exchange: 'Kraken',
        type: 'L2',
        status: 'disconnected',
        messagesPerSecond: 0,
        totalMessages: 0,
        latency: 0,
        lastMessage: null,
        dataSize: 0,
        errorRate: 0
      }
    ]);
  }, []);

  const MAX_DATA_POINTS = 60; // Keep last 60 seconds of data

  // Helper to add data point to history
  const addDataPoint = (
    history: MetricData[],
    value: number,
    maxPoints: number = MAX_DATA_POINTS
  ): MetricData[] => {
    const newPoint = { timestamp: new Date(), value };
    const newHistory = [...history, newPoint];
    return newHistory.slice(-maxPoints);
  };

  // Initialize WebSocket connection
  useEffect(() => {
    const socket = io('http://localhost:5001', {
      transports: ['websocket'],
      path: '/socket.io/'
    });

    socketRef.current = socket;

    socket.on('connect', () => {
      console.log('✅ Connected to metrics WebSocket');
      setIsConnected(true);
      socket.emit('subscribe_system_monitoring');
    });

    socket.on('disconnect', () => {
      console.log('❌ Disconnected from metrics WebSocket');
      setIsConnected(false);
    });

    // Real-time metric updates
    socket.on('system_status_update', (data: any) => {
      setCurrentCpu(data.cpu_percent);
      setCurrentMemory(data.memory_percent);
      setSystemUptime(data.uptime);
      
      // Update history
      setCpuHistory(prev => addDataPoint(prev, data.cpu_percent));
      setMemoryHistory(prev => addDataPoint(prev, data.memory_percent));
    });

    socket.on('network_update', (data: any) => {
      setNetworkHistory(prev => ({
        rx: addDataPoint(prev.rx, data.bytes_recv_per_sec / 1024), // Convert to KB/s
        tx: addDataPoint(prev.tx, data.bytes_sent_per_sec / 1024)
      }));
    });

    socket.on('services_update', (data: ServiceHealth[]) => {
      setServices(data);
    });

    socket.on('metrics_update', (data: any) => {
      setCurrentDisk(data.disk_percent);
      setActiveConnections(data.connections);
      setRequestsPerSecond(data.requests_per_second);
      
      if (data.latency) {
        setLatencyHistory(prev => addDataPoint(prev, data.latency));
      }
    });

    socket.on('alert', (alert: any) => {
      setAlerts(prev => [alert, ...prev].slice(0, 5)); // Keep last 5 alerts
    });
    
    // Data stream updates
    socket.on('data_streams_update', (streams: DataStream[]) => {
      setDataStreams(streams);
    });

    return () => {
      if (socket.connected) {
        socket.emit('unsubscribe_system_monitoring');
        socket.disconnect();
      }
    };
  }, []);

  // Format uptime
  const formatUptime = (seconds: number): string => {
    const days = Math.floor(seconds / 86400);
    const hours = Math.floor((seconds % 86400) / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    return `${days}d ${hours}h ${minutes}m`;
  };

  // Chart configurations
  const chartOptions = {
    responsive: true,
    maintainAspectRatio: false,
    plugins: {
      legend: {
        display: false
      },
      tooltip: {
        backgroundColor: 'rgba(0, 0, 0, 0.8)',
        titleColor: '#00d4ff',
        bodyColor: '#00d4ff',
        borderColor: '#00d4ff',
        borderWidth: 1
      }
    },
    scales: {
      x: {
        display: false
      },
      y: {
        display: true,
        grid: {
          color: 'rgba(0, 212, 255, 0.1)'
        },
        ticks: {
          color: '#00d4ff',
          font: {
            size: 10
          }
        }
      }
    }
  };

  const cpuChartData = {
    labels: cpuHistory.map(() => ''),
    datasets: [{
      data: cpuHistory.map(d => d.value),
      borderColor: currentCpu > 80 ? '#ff6b6b' : currentCpu > 60 ? '#ffd43b' : '#51cf66',
      backgroundColor: currentCpu > 80 ? 'rgba(255, 107, 107, 0.1)' : currentCpu > 60 ? 'rgba(255, 212, 59, 0.1)' : 'rgba(81, 207, 102, 0.1)',
      fill: true,
      tension: 0.4,
      borderWidth: 2
    }]
  };

  const memoryChartData = {
    labels: memoryHistory.map(() => ''),
    datasets: [{
      data: memoryHistory.map(d => d.value),
      borderColor: currentMemory > 85 ? '#ff6b6b' : currentMemory > 70 ? '#ffd43b' : '#339af0',
      backgroundColor: currentMemory > 85 ? 'rgba(255, 107, 107, 0.1)' : currentMemory > 70 ? 'rgba(255, 212, 59, 0.1)' : 'rgba(51, 154, 240, 0.1)',
      fill: true,
      tension: 0.4,
      borderWidth: 2
    }]
  };

  const networkChartData = {
    labels: networkHistory.rx.map(() => ''),
    datasets: [
      {
        label: 'RX',
        data: networkHistory.rx.map(d => d.value),
        borderColor: '#00d4ff',
        backgroundColor: 'rgba(0, 212, 255, 0.1)',
        fill: true,
        tension: 0.4,
        borderWidth: 2
      },
      {
        label: 'TX',
        data: networkHistory.tx.map(d => d.value),
        borderColor: '#f06292',
        backgroundColor: 'rgba(240, 98, 146, 0.1)',
        fill: true,
        tension: 0.4,
        borderWidth: 2
      }
    ]
  };

  return (
    <div className={`${styles.systemDashboard} ${className}`}>
      {/* Header Row */}
      <div className={styles.dashboardHeader}>
        <div className={styles.dashboardTitle}>
          <h1>System Metrics Dashboard</h1>
          <div className={styles.connectionIndicator}>
            <span className={`${styles.statusDot} ${isConnected ? styles.connected : styles.disconnected}`} />
            <span>{isConnected ? 'Live' : 'Offline'}</span>
          </div>
        </div>
        <div className={styles.dashboardStats}>
          <div className={styles.statItem}>
            <span className={styles.statLabel}>Uptime</span>
            <span className={styles.statValue}>{formatUptime(systemUptime)}</span>
          </div>
          <div className={styles.statItem}>
            <span className={styles.statLabel}>Connections</span>
            <span className={styles.statValue}>{activeConnections}</span>
          </div>
          <div className={styles.statItem}>
            <span className={styles.statLabel}>Requests/s</span>
            <span className={styles.statValue}>{requestsPerSecond.toFixed(1)}</span>
          </div>
        </div>
      </div>

      {/* Main Metrics Grid */}
      <div className={styles.metricsGrid}>
        {/* CPU Panel */}
        <div className={styles.metricPanel}>
          <div className={styles.panelHeader}>
            <h3>CPU Usage</h3>
            <span className={`${styles.metricValue} ${currentCpu > 80 ? styles.critical : currentCpu > 60 ? styles.warning : styles.normal}`}>
              {currentCpu.toFixed(1)}%
            </span>
          </div>
          <div className={styles.chartContainer}>
            <Line data={cpuChartData} options={chartOptions} />
          </div>
        </div>

        {/* Memory Panel */}
        <div className={styles.metricPanel}>
          <div className={styles.panelHeader}>
            <h3>Memory Usage</h3>
            <span className={`${styles.metricValue} ${currentMemory > 85 ? styles.critical : currentMemory > 70 ? styles.warning : styles.normal}`}>
              {currentMemory.toFixed(1)}%
            </span>
          </div>
          <div className={styles.chartContainer}>
            <Line data={memoryChartData} options={chartOptions} />
          </div>
        </div>

        {/* Network Panel */}
        <div className={styles.metricPanel}>
          <div className={styles.panelHeader}>
            <h3>Network I/O</h3>
            <span className={styles.metricValue}>
              ↓ {networkHistory.rx[networkHistory.rx.length - 1]?.value.toFixed(1) || 0} KB/s
            </span>
          </div>
          <div className={styles.chartContainer}>
            <Line data={networkChartData} options={{...chartOptions, plugins: { ...chartOptions.plugins, legend: { display: true, labels: { color: '#00d4ff' } } }}} />
          </div>
        </div>

        {/* Disk Usage Gauge */}
        <div className={styles.metricPanel}>
          <div className={styles.panelHeader}>
            <h3>Disk Usage</h3>
            <span className={`${styles.metricValue} ${currentDisk > 90 ? styles.critical : currentDisk > 75 ? styles.warning : styles.normal}`}>
              {currentDisk.toFixed(1)}%
            </span>
          </div>
          <div className={styles.gaugeContainer}>
            <div className={styles.circularGauge}>
              <svg viewBox="0 0 100 100" className={styles.gaugeSvg}>
                <circle
                  cx="50"
                  cy="50"
                  r="45"
                  fill="none"
                  stroke="rgba(0, 212, 255, 0.1)"
                  strokeWidth="8"
                />
                <circle
                  cx="50"
                  cy="50"
                  r="45"
                  fill="none"
                  stroke={currentDisk > 90 ? '#ff6b6b' : currentDisk > 75 ? '#ffd43b' : '#00d4ff'}
                  strokeWidth="8"
                  strokeDasharray={`${currentDisk * 2.83} 283`}
                  strokeDashoffset="0"
                  transform="rotate(-90 50 50)"
                  className={styles.gaugeProgress}
                />
              </svg>
              <div className={styles.gaugeCenter}>
                <span className={styles.gaugeValue}>{currentDisk.toFixed(0)}</span>
                <span className={styles.gaugeUnit}>%</span>
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* Services Status Grid */}
      <div className={styles.servicesSection}>
        <h2 className={styles.sectionTitle}>Service Health</h2>
        <div className={styles.servicesGrid}>
          {services.map(service => (
            <div key={service.name} className={`${styles.serviceCard} ${styles[service.status]}`}>
              <div className={styles.serviceHeader}>
                <span className={`${styles.serviceIndicator} ${styles[service.status]}`} />
                <span className={styles.serviceName}>{service.name}</span>
              </div>
              <div className={styles.serviceMetrics}>
                <div className={styles.serviceMetric}>
                  <span className={styles.metricLabel}>CPU</span>
                  <span className={styles.metricBar}>
                    <span 
                      className={styles.metricFill} 
                      style={{ 
                        width: `${service.cpu}%`,
                        backgroundColor: service.cpu > 80 ? '#ff6b6b' : service.cpu > 60 ? '#ffd43b' : '#51cf66'
                      }}
                    />
                  </span>
                  <span className={styles.metricPercent}>{service.cpu.toFixed(0)}%</span>
                </div>
                <div className={styles.serviceMetric}>
                  <span className={styles.metricLabel}>MEM</span>
                  <span className={styles.metricBar}>
                    <span 
                      className={styles.metricFill} 
                      style={{ 
                        width: `${service.memory}%`,
                        backgroundColor: service.memory > 80 ? '#ff6b6b' : service.memory > 60 ? '#ffd43b' : '#339af0'
                      }}
                    />
                  </span>
                  <span className={styles.metricPercent}>{service.memory.toFixed(0)}%</span>
                </div>
              </div>
              <div className={styles.serviceUptime}>
                Uptime: {formatUptime(service.uptime)}
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* TRUE Continuous Stream - Zero Batching */}
      <div className={styles.dataStreamsSection}>
        <h2 className={styles.sectionTitle}>Continuous Trade Stream (WebSocket)</h2>
        <ContinuousStreamChart />
      </div>
      
      {/* PostgreSQL Real-Time Monitor */}
      <div className={styles.dataStreamsSection}>
        <h2 className={styles.sectionTitle}>Live Trade Activity Monitor (1s Polling)</h2>
        <PostgresRealtimeChart />
      </div>
      
      {/* Live Grafana Ingestion Chart */}
      <div className={styles.dataStreamsSection}>
        <h2 className={styles.sectionTitle}>Real-Time Ingestion Rate (Grafana)</h2>
        <GrafanaIngestionChart />
      </div>

      {/* Data Streams Section */}
      <div className={styles.dataStreamsSection}>
        <h2 className={styles.sectionTitle}>Exchange Data Streams</h2>
        <div className={styles.streamsGrid}>
          {dataStreams.map(stream => (
            <div key={stream.id} className={`${styles.streamCard} ${styles[stream.status]}`}>
              <div className={styles.streamHeader}>
                <div className={styles.streamTitle}>
                  <span className={`${styles.streamIndicator} ${styles[stream.status]}`} />
                  <div>
                    <div className={styles.streamName}>{stream.name}</div>
                    <div className={styles.streamExchange}>{stream.exchange}</div>
                  </div>
                </div>
                <div className={styles.streamType}>{stream.type}</div>
              </div>
              
              <div className={styles.streamStats}>
                <div className={styles.streamStat}>
                  <span className={styles.statLabel}>Messages/sec</span>
                  <span className={styles.statValue}>{stream.messagesPerSecond.toLocaleString()}</span>
                </div>
                <div className={styles.streamStat}>
                  <span className={styles.statLabel}>Total Messages</span>
                  <span className={styles.statValue}>{(stream.totalMessages / 1000).toFixed(1)}k</span>
                </div>
                <div className={styles.streamStat}>
                  <span className={styles.statLabel}>Latency</span>
                  <span className={`${styles.statValue} ${stream.latency > 100 ? styles.warning : ''}`}>
                    {stream.latency}ms
                  </span>
                </div>
                <div className={styles.streamStat}>
                  <span className={styles.statLabel}>Disk Written</span>
                  <span className={styles.statValue}>{stream.dataSize.toFixed(1)} MB</span>
                </div>
              </div>
              
              <div className={styles.streamFooter}>
                <div className={styles.streamError}>
                  Error Rate: <span className={stream.errorRate > 1 ? styles.error : ''}>
                    {stream.errorRate.toFixed(2)}%
                  </span>
                </div>
                {stream.lastMessage && (
                  <div className={styles.streamLastMessage}>
                    Last: {new Date(stream.lastMessage).toLocaleTimeString()}
                  </div>
                )}
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Alerts Section */}
      {alerts.length > 0 && (
        <div className={styles.alertsSection}>
          <h2 className={styles.sectionTitle}>Recent Alerts</h2>
          <div className={styles.alertsList}>
            {alerts.map((alert, index) => (
              <div key={index} className={`${styles.alertItem} ${styles[alert.type]}`}>
                <span className={styles.alertTime}>{new Date(alert.timestamp).toLocaleTimeString()}</span>
                <span className={styles.alertMessage}>{alert.message}</span>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
};

export default SystemDashboard;