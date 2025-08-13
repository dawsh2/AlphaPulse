import React, { useEffect, useRef, useState } from 'react';
import styles from './TrueRealtimeChart.module.css';

interface TradeStats {
  exchange: string;
  tradesPerSecond: number;
  total: number;
  latest: string;
}

interface Trade {
  time: string;
  exchange: string;
  symbol: string;
  price: number;
  size: number;
  side: string;
}

export const PostgresRealtimeChart: React.FC = () => {
  const [stats, setStats] = useState<TradeStats[]>([]);
  const [krakenTrades, setKrakenTrades] = useState<Trade[]>([]);
  const [isLive, setIsLive] = useState(true);
  const intervalRef = useRef<NodeJS.Timeout>();
  const consoleRef = useRef<HTMLDivElement>(null);
  
  useEffect(() => {
    const fetchStats = async () => {
      try {
        const response = await fetch('http://localhost:5001/api/market-data/stats');
        if (response.ok) {
          const data = await response.json();
          
          // Transform the data
          const transformedStats: TradeStats[] = data.exchanges?.map((ex: any) => ({
            exchange: ex.exchange,
            tradesPerSecond: ex.trades_per_second || 0,
            total: ex.total_trades || 0,
            latest: ex.last_trade_time || 'N/A'
          })) || [];
          
          setStats(transformedStats);
          setIsLive(true);
          
          // Filter and add Kraken trades
          if (data.recent_trades) {
            const newKrakenTrades = data.recent_trades
              .filter((t: Trade) => t.exchange === 'kraken')
              .slice(0, 5); // Get latest 5 Kraken trades
            
            setKrakenTrades(prevTrades => {
              // Combine new trades with existing, keep last 50
              const combined = [...newKrakenTrades, ...prevTrades];
              return combined.slice(0, 50);
            });
          }
        }
      } catch (error) {
        console.error('Failed to fetch stats:', error);
        setIsLive(false);
      }
    };
    
    // Fetch immediately
    fetchStats();
    
    // Then fetch every second for real-time updates
    intervalRef.current = setInterval(fetchStats, 1000);
    
    return () => {
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
      }
    };
  }, []);
  
  // Calculate max rate for visualization
  const maxRate = Math.max(...stats.map(s => s.tradesPerSecond), 1);
  
  return (
    <div className={styles.container}>
      <div className={styles.header}>
        <h3>📊 PostgreSQL Real-Time Trade Monitor</h3>
        <div className={styles.status}>
          <div className={`${styles.indicator} ${isLive ? styles.connected : styles.disconnected}`} />
          <span>{isLive ? 'LIVE DATA' : 'OFFLINE'}</span>
        </div>
      </div>
      
      {/* Real-time stats */}
      <div className={styles.stats}>
        {stats.map(stat => (
          <div key={stat.exchange} className={styles.statBox}>
            <div className={styles.exchange} style={{ 
              color: stat.exchange === 'coinbase' ? '#2962FF' : '#FF6B00' 
            }}>
              {stat.exchange.toUpperCase()}
            </div>
            <div className={styles.rate}>
              {stat.tradesPerSecond.toFixed(1)} trades/s
            </div>
            <div className={styles.total}>
              {stat.total.toLocaleString()} total today
            </div>
            
            {/* Activity bar */}
            <div style={{
              width: '100%',
              height: '4px',
              background: '#1a1a1a',
              borderRadius: '2px',
              marginTop: '8px',
              overflow: 'hidden'
            }}>
              <div style={{
                width: `${(stat.tradesPerSecond / maxRate) * 100}%`,
                height: '100%',
                background: stat.exchange === 'coinbase' ? '#2962FF' : '#FF6B00',
                transition: 'width 0.3s ease',
                boxShadow: `0 0 10px ${stat.exchange === 'coinbase' ? '#2962FF' : '#FF6B00'}`
              }} />
            </div>
          </div>
        ))}
      </div>
      
      {/* Kraken Trade Console */}
      <div style={{
        marginTop: '16px',
        background: '#0a0a0a',
        border: '1px solid #2a2a2a',
        borderRadius: '4px',
        overflow: 'hidden'
      }}>
        <div style={{
          padding: '8px 12px',
          background: '#141414',
          borderBottom: '1px solid #2a2a2a',
          fontSize: '12px',
          fontWeight: 'bold',
          color: '#FF6B00',
          fontFamily: 'monospace'
        }}>
          📡 KRAKEN LIVE TRADE STREAM
        </div>
        <div 
          ref={consoleRef}
          style={{
            height: '200px',
            overflowY: 'auto',
            padding: '8px',
            fontFamily: 'Monaco, Courier New, monospace',
            fontSize: '11px',
            lineHeight: '1.4'
          }}
        >
          {krakenTrades.length === 0 ? (
            <div style={{ color: '#666' }}>Waiting for Kraken trades...</div>
          ) : (
            krakenTrades.map((trade, i) => {
              const time = new Date(trade.time).toLocaleTimeString('en-US', { 
                hour12: false, 
                hour: '2-digit', 
                minute: '2-digit', 
                second: '2-digit' 
              });
              const sideColor = trade.side === 'buy' ? '#00ff88' : '#ff3366';
              
              return (
                <div 
                  key={`${trade.time}-${i}`}
                  style={{
                    color: '#999',
                    marginBottom: '2px',
                    opacity: 1 - (i * 0.015), // Fade older trades
                    animation: i === 0 ? 'slideIn 0.3s ease' : 'none'
                  }}
                >
                  <span style={{ color: '#666' }}>[{time}]</span>
                  {' '}
                  <span style={{ color: '#FF6B00' }}>KRAKEN</span>
                  {' '}
                  <span style={{ color: '#fff' }}>{trade.symbol}</span>
                  {' '}
                  <span style={{ color: sideColor, fontWeight: 'bold' }}>
                    {trade.side.toUpperCase()}
                  </span>
                  {' '}
                  <span style={{ color: '#fff' }}>${trade.price.toFixed(2)}</span>
                  {' '}
                  <span style={{ color: '#888' }}>× {trade.size.toFixed(6)}</span>
                </div>
              );
            })
          )}
        </div>
      </div>
      
      {/* Info */}
      <div style={{
        marginTop: '8px',
        padding: '8px',
        background: '#141414',
        borderRadius: '4px',
        fontSize: '10px',
        color: '#666'
      }}>
        <strong>Status:</strong> {stats.length > 0 ? 
          `Receiving ${stats.reduce((sum, s) => sum + s.tradesPerSecond, 0).toFixed(0)} trades/sec total` : 
          'Waiting for data...'
        } | <strong>Source:</strong> PostgreSQL | <strong>Update:</strong> 1s
      </div>
    </div>
  );
};