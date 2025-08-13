import React, { useEffect, useRef, useState } from 'react';
import { createChart, IChartApi, ISeriesApi, LineData } from 'lightweight-charts';
import styles from './GrafanaEmbed.module.css';

interface IngestionData {
  exchange: string;
  current: number;
  avg_1s: number;
  avg_10s: number;
  avg_60s: number;
  history: number[];
}

export const RealTimeIngestionChart: React.FC = () => {
  const chartContainerRef = useRef<HTMLDivElement>(null);
  const chartRef = useRef<IChartApi | null>(null);
  const seriesRef = useRef<Map<string, ISeriesApi<"Line">>>(new Map());
  const [isConnected, setIsConnected] = useState(false);
  const [rates, setRates] = useState<Record<string, IngestionData>>({});
  const wsRef = useRef<WebSocket | null>(null);
  
  useEffect(() => {
    if (!chartContainerRef.current) return;
    
    // Create chart
    const chart = createChart(chartContainerRef.current, {
      width: chartContainerRef.current.clientWidth,
      height: 300,
      layout: {
        background: { color: '#1a1a1a' },
        textColor: '#d1d4dc',
      },
      grid: {
        vertLines: { color: '#2a2a2a' },
        horzLines: { color: '#2a2a2a' },
      },
      timeScale: {
        timeVisible: true,
        secondsVisible: true,
      },
    });
    chartRef.current = chart;
    
    // Create series for each exchange
    const coinbaseSeries = chart.addLineSeries({
      color: '#2962FF',
      title: 'Coinbase',
      lineWidth: 2,
    });
    seriesRef.current.set('coinbase', coinbaseSeries);
    
    const krakenSeries = chart.addLineSeries({
      color: '#FF6B00',
      title: 'Kraken',
      lineWidth: 2,
    });
    seriesRef.current.set('kraken', krakenSeries);
    
    // Connect to WebSocket
    const ws = new WebSocket('ws://localhost:8765/ws');
    wsRef.current = ws;
    
    ws.onopen = () => {
      console.log('Connected to real-time ingestion stream');
      setIsConnected(true);
    };
    
    ws.onmessage = (event) => {
      const message = JSON.parse(event.data);
      
      if (message.type === 'ingestion_rates') {
        const timestamp = Math.floor(Date.now() / 1000);
        
        // Update chart for each exchange
        Object.entries(message.data as Record<string, IngestionData>).forEach(([exchange, data]) => {
          const series = seriesRef.current.get(exchange.toLowerCase());
          if (series) {
            series.update({
              time: timestamp as any,
              value: data.current
            });
          }
        });
        
        setRates(message.data);
      }
    };
    
    ws.onerror = (error) => {
      console.error('WebSocket error:', error);
      setIsConnected(false);
    };
    
    ws.onclose = () => {
      console.log('Disconnected from real-time stream');
      setIsConnected(false);
    };
    
    // Handle resize
    const handleResize = () => {
      if (chartContainerRef.current && chart) {
        chart.applyOptions({
          width: chartContainerRef.current.clientWidth,
        });
      }
    };
    window.addEventListener('resize', handleResize);
    
    // Cleanup
    return () => {
      window.removeEventListener('resize', handleResize);
      if (ws.readyState === WebSocket.OPEN) {
        ws.close();
      }
      if (chart) {
        chart.remove();
      }
    };
  }, []);
  
  return (
    <div className={styles.grafanaContainer}>
      <div style={{ 
        display: 'flex', 
        justifyContent: 'space-between', 
        alignItems: 'center',
        padding: '10px 15px',
        borderBottom: '1px solid #2a2a2a'
      }}>
        <h3 style={{ margin: 0, fontSize: '14px' }}>Real-Time Ingestion Rate (trades/sec)</h3>
        <div style={{ display: 'flex', gap: '20px', fontSize: '12px' }}>
          {Object.entries(rates).map(([exchange, data]) => (
            <div key={exchange} style={{ display: 'flex', gap: '10px' }}>
              <span style={{ color: exchange === 'coinbase' ? '#2962FF' : '#FF6B00' }}>
                {exchange}: {data.current}/s
              </span>
              <span style={{ color: '#666' }}>
                (avg: {data.avg_10s.toFixed(1)}/s)
              </span>
            </div>
          ))}
          <div style={{ 
            width: '8px', 
            height: '8px', 
            borderRadius: '50%',
            backgroundColor: isConnected ? '#00ff00' : '#ff0000',
            alignSelf: 'center'
          }} />
        </div>
      </div>
      <div ref={chartContainerRef} style={{ height: '300px' }} />
    </div>
  );
};