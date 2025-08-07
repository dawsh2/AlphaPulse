import React, { useState, useEffect, useRef } from 'react';
import { createChart } from 'lightweight-charts';
import styles from './MonitorPage.module.css';

interface MarketData {
  time: number;
  open: number;
  high: number;
  low: number;
  close: number;
  volume: number;
  signal?: 'buy' | 'sell';
}

interface EventData {
  time: string;
  type: 'buy' | 'sell' | 'signal';
  description: string;
}

interface MetricData {
  totalPnL: number;
  winRate: number;
  sharpeRatio: number;
  maxDrawdown: number;
  totalTrades: number;
  avgTrade: number;
}

interface Strategy {
  id: string;
  name: string;
  winRate: number;
  pnl: number;
  active?: boolean;
}

type SidebarTab = 'metrics' | 'events' | 'strategies';
type PlaybackSpeed = 1 | 2 | 5 | 10;

interface DropdownOption {
  value: string;
  label: string;
}

const MonitorPage: React.FC = () => {
  // State management
  const [sidebarTab, setSidebarTab] = useState<SidebarTab>('metrics');
  const [isPlaying, setIsPlaying] = useState(false);
  const [currentBar, setCurrentBar] = useState(50);
  const [playbackSpeed, setPlaybackSpeed] = useState<PlaybackSpeed>(1);
  const [symbol, setSymbol] = useState('SPY');
  const [timeframe, setTimeframe] = useState('5m');
  const [selectedStrategy, setSelectedStrategy] = useState('Mean Reversion v2');
  const [timeframeDropdownOpen, setTimeframeDropdownOpen] = useState(false);
  const [strategyDropdownOpen, setStrategyDropdownOpen] = useState(false);
  const [speedDropdownOpen, setSpeedDropdownOpen] = useState(false);
  const [symbolDropdownOpen, setSymbolDropdownOpen] = useState(false);

  // Refs
  const chartContainerRef = useRef<HTMLDivElement>(null);
  const chartRef = useRef<any>(null);
  const candleSeriesRef = useRef<any>(null);
  const playbackIntervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  // Data
  const [marketData, setMarketData] = useState<MarketData[]>([]);
  const [eventData, setEventData] = useState<EventData[]>([]);

  // Mock data generators
  const generateMockData = (): MarketData[] => {
    const data: MarketData[] = [];
    const basePrice = 420;
    const now = new Date();
    const events: EventData[] = [];

    for (let i = 0; i < 390; i++) { // 390 minutes in trading day
      const time = new Date(now.getTime() - (390 - i) * 60000);
      const random = Math.random();
      const trend = Math.sin(i / 50) * 5;
      const noise = (random - 0.5) * 2;

      const open = basePrice + trend + noise + (i > 0 ? data[i-1].close - basePrice : 0) * 0.3;
      const close = open + (Math.random() - 0.5) * 1;
      const high = Math.max(open, close) + Math.random() * 0.5;
      const low = Math.min(open, close) - Math.random() * 0.5;
      const volume = Math.floor(1000000 + Math.random() * 2000000);

      const signal = i % 50 === 0 ? (Math.random() > 0.5 ? 'buy' : 'sell') : undefined;

      data.push({
        time: Math.floor(time.getTime() / 1000),
        open,
        high,
        low,
        close,
        volume,
        signal
      });

      // Add event data
      if (signal) {
        events.push({
          time: time.toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit' }),
          type: signal,
          description: `${signal.toUpperCase()} signal generated at $${close.toFixed(2)}`
        });
      }
    }

    setEventData(events);
    return data;
  };

  const mockMetrics: MetricData = {
    totalPnL: 2345.67,
    winRate: 68.4,
    sharpeRatio: 1.82,
    maxDrawdown: -8.3,
    totalTrades: 247,
    avgTrade: 9.49
  };

  const mockStrategies: Strategy[] = [
    { id: '1', name: 'Mean Reversion v2', winRate: 68, pnl: 12.3, active: true },
    { id: '2', name: 'Momentum Breakout', winRate: 52, pnl: 8.7 },
    { id: '3', name: 'Market Making', winRate: 71, pnl: 5.2 }
  ];

  // Chart initialization
  const initChart = () => {
    if (!chartContainerRef.current) return;

    // Detect theme
    const isDark = document.documentElement.getAttribute('data-theme') === 'dark' ||
                   window.matchMedia('(prefers-color-scheme: dark)').matches;

    const chart = createChart(chartContainerRef.current, {
      width: chartContainerRef.current.clientWidth,
      height: chartContainerRef.current.clientHeight,
      layout: {
        background: { color: 'transparent' },
        textColor: isDark ? '#f0f6fc' : '#33332d',
      },
      grid: {
        vertLines: { color: isDark ? '#383c45' : '#e5e0d5' },
        horzLines: { color: isDark ? '#383c45' : '#e5e0d5' },
      },
      crosshair: {
        mode: 0, // Normal
        vertLine: {
          color: isDark ? '#4d525b' : '#d8d2c4',
          width: 1,
          style: 3,
          labelBackgroundColor: isDark ? '#262931' : '#f5f2ea'
        },
        horzLine: {
          color: isDark ? '#4d525b' : '#d8d2c4',
          width: 1,
          style: 3,
          labelBackgroundColor: isDark ? '#262931' : '#f5f2ea'
        }
      },
      rightPriceScale: {
        borderColor: isDark ? '#383c45' : '#e5e0d5',
      },
      timeScale: {
        borderColor: isDark ? '#383c45' : '#e5e0d5',
        timeVisible: true,
        secondsVisible: false,
      },
    });

    const candleSeries = chart.addCandlestickSeries({
      upColor: '#3fb950',
      downColor: '#f85149',
      borderUpColor: '#3fb950',
      borderDownColor: '#f85149',
      wickUpColor: '#3fb950',
      wickDownColor: '#f85149',
    });

    chartRef.current = chart;
    candleSeriesRef.current = candleSeries;

    // Start with first 50 bars
    updateChart(50);
  };

  // Update chart data
  const updateChart = (bars: number) => {
    if (!candleSeriesRef.current || !marketData.length) return;

    const visibleData = marketData.slice(0, bars);
    candleSeriesRef.current.setData(visibleData);

    // Add markers for signals
    const markers = visibleData
      .filter(d => d.signal)
      .map(d => ({
        time: d.time,
        position: d.signal === 'buy' ? 'belowBar' : 'aboveBar',
        color: d.signal === 'buy' ? '#3fb950' : '#f85149',
        shape: d.signal === 'buy' ? 'arrowUp' : 'arrowDown',
        text: d.signal!.toUpperCase()
      }));

    candleSeriesRef.current.setMarkers(markers as any);
    setCurrentBar(bars);
  };

  // Playback controls
  const startPlayback = () => {
    if (playbackIntervalRef.current) {
      clearInterval(playbackIntervalRef.current);
    }

    playbackIntervalRef.current = setInterval(() => {
      setCurrentBar(prev => {
        if (prev < marketData.length) {
          const newBar = prev + 1;
          updateChart(newBar);
          return newBar;
        } else {
          // Reached end
          setIsPlaying(false);
          return marketData.length;
        }
      });
    }, 100 / playbackSpeed);
  };

  const stopPlayback = () => {
    if (playbackIntervalRef.current) {
      clearInterval(playbackIntervalRef.current);
      playbackIntervalRef.current = null;
    }
  };

  const togglePlay = () => {
    const newIsPlaying = !isPlaying;
    setIsPlaying(newIsPlaying);

    if (newIsPlaying) {
      startPlayback();
    } else {
      stopPlayback();
    }
  };

  const skipBackward = () => {
    const newBar = Math.max(1, currentBar - 50);
    setCurrentBar(newBar);
    updateChart(newBar);
  };

  const skipForward = () => {
    const newBar = Math.min(marketData.length, currentBar + 50);
    setCurrentBar(newBar);
    updateChart(newBar);
  };

  // Effects
  useEffect(() => {
    const data = generateMockData();
    setMarketData(data);
  }, []);

  useEffect(() => {
    if (marketData.length > 0) {
      initChart();
    }

    return () => {
      if (chartRef.current) {
        chartRef.current.remove();
      }
      if (playbackIntervalRef.current) {
        clearInterval(playbackIntervalRef.current);
      }
    };
  }, [marketData]);

  useEffect(() => {
    if (isPlaying) {
      stopPlayback();
      startPlayback();
    }
  }, [playbackSpeed]);


  // Handle window resize
  useEffect(() => {
    const handleResize = () => {
      if (chartRef.current && chartContainerRef.current) {
        chartRef.current.applyOptions({
          width: chartContainerRef.current.clientWidth,
          height: chartContainerRef.current.clientHeight
        });
      }
    };

    window.addEventListener('resize', handleResize);
    return () => window.removeEventListener('resize', handleResize);
  }, []);

  // Handle keyboard navigation (Shift + Arrow keys)
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (!e.shiftKey) return;
      
      if (e.key === 'ArrowLeft') {
        e.preventDefault();
        const newBar = Math.max(1, currentBar - 1);
        setCurrentBar(newBar);
        updateChart(newBar);
      } else if (e.key === 'ArrowRight') {
        e.preventDefault();
        const newBar = Math.min(marketData.length, currentBar + 1);
        setCurrentBar(newBar);
        updateChart(newBar);
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [currentBar, marketData.length]);

  // Get current bar data for price display
  const currentBarData = marketData[currentBar - 1];

  // Get visible events based on current bar
  const getVisibleEvents = (): EventData[] => {
    const eventsToShow = Math.floor(currentBar / 50);
    return eventData.slice(0, eventsToShow).reverse();
  };

  return (
    <div className={styles.monitorContainer}>
      {/* Content Area */}
      <div className={styles.contentArea}>
        {/* Sidebar */}
        <div className={styles.sidebar}>
          <div className={styles.sidebarHeader}>
            <div className={styles.sidebarTabs}>
              <button
                className={`${styles.sidebarTab} ${sidebarTab === 'metrics' ? styles.active : ''}`}
                onClick={() => setSidebarTab('metrics')}
              >
                Metrics
              </button>
              <button
                className={`${styles.sidebarTab} ${sidebarTab === 'events' ? styles.active : ''}`}
                onClick={() => setSidebarTab('events')}
              >
                Events
              </button>
              <button
                className={`${styles.sidebarTab} ${sidebarTab === 'strategies' ? styles.active : ''}`}
                onClick={() => setSidebarTab('strategies')}
              >
                Strategies
              </button>
            </div>
          </div>

          <div className={styles.sidebarContent}>
            {/* Metrics Tab */}
            {sidebarTab === 'metrics' && (
              <div className={styles.metricsGrid}>
                <div className={styles.metricCard}>
                  <div className={styles.metricLabel}>Total P&L</div>
                  <div className={`${styles.metricValue} ${mockMetrics.totalPnL > 0 ? styles.positive : styles.negative}`}>
                    {mockMetrics.totalPnL > 0 ? '+' : ''}${mockMetrics.totalPnL.toFixed(2)}
                  </div>
                </div>
                <div className={styles.metricCard}>
                  <div className={styles.metricLabel}>Win Rate</div>
                  <div className={styles.metricValue}>{mockMetrics.winRate.toFixed(1)}%</div>
                </div>
                <div className={styles.metricCard}>
                  <div className={styles.metricLabel}>Sharpe Ratio</div>
                  <div className={styles.metricValue}>{mockMetrics.sharpeRatio.toFixed(2)}</div>
                </div>
                <div className={styles.metricCard}>
                  <div className={styles.metricLabel}>Max Drawdown</div>
                  <div className={`${styles.metricValue} ${styles.negative}`}>
                    {mockMetrics.maxDrawdown.toFixed(1)}%
                  </div>
                </div>
                <div className={styles.metricCard}>
                  <div className={styles.metricLabel}>Total Trades</div>
                  <div className={styles.metricValue}>{mockMetrics.totalTrades}</div>
                </div>
                <div className={styles.metricCard}>
                  <div className={styles.metricLabel}>Avg Trade</div>
                  <div className={`${styles.metricValue} ${mockMetrics.avgTrade > 0 ? styles.positive : styles.negative}`}>
                    {mockMetrics.avgTrade > 0 ? '+' : ''}${mockMetrics.avgTrade.toFixed(2)}
                  </div>
                </div>
              </div>
            )}

            {/* Events Tab */}
            {sidebarTab === 'events' && (
              <div className={styles.eventLog}>
                {getVisibleEvents().map((event, index) => (
                  <div key={index} className={styles.eventItem}>
                    <span className={styles.eventTime}>{event.time}</span>
                    <span className={`${styles.eventType} ${event.type === 'buy' ? styles.buy : event.type === 'sell' ? styles.sell : ''}`}>
                      {event.type.toUpperCase()}
                    </span>
                    <span className={styles.eventMessage}>{event.description}</span>
                  </div>
                ))}
              </div>
            )}

            {/* Strategies Tab */}
            {sidebarTab === 'strategies' && (
              <div className={styles.strategyList}>
                {mockStrategies.map((strategy) => (
                  <div
                    key={strategy.id}
                    className={`${styles.strategyItem} ${strategy.active ? styles.active : ''}`}
                    onClick={() => setSelectedStrategy(strategy.name)}
                  >
                    <div className={styles.strategyName}>{strategy.name}</div>
                    <div className={styles.strategyStats}>
                      <span>Win: {strategy.winRate}%</span>
                      <span>P&L: +{strategy.pnl.toFixed(1)}%</span>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>

        <div className={styles.mainContent}>
          {/* Chart */}
          <div className={styles.chartContainer}>
            <div ref={chartContainerRef} className={styles.chart}></div>

            <div className={styles.chartInfo}>
              <div className={styles.symbolInfo}>
                <div className={styles.symbolDropdown}>
                  <button 
                    className={styles.symbolButton}
                    onClick={() => setSymbolDropdownOpen(!symbolDropdownOpen)}
                    onMouseEnter={() => setSymbolDropdownOpen(true)}
                    onMouseLeave={() => setTimeout(() => setSymbolDropdownOpen(false), 100)}
                  >
                    {symbol}
                    <span style={{ marginLeft: '8px', fontSize: '10px' }}>▼</span>
                  </button>
                  {symbolDropdownOpen && (
                    <div 
                      className={styles.symbolDropdownMenu}
                      onMouseEnter={() => setSymbolDropdownOpen(true)}
                      onMouseLeave={() => setSymbolDropdownOpen(false)}
                    >
                      <button
                        className={`${styles.dropdownOption} ${symbol === 'SPY' ? styles.active : ''}`}
                        onClick={() => { setSymbol('SPY'); setSymbolDropdownOpen(false); }}
                      >
                        SPY
                      </button>
                      <button
                        className={`${styles.dropdownOption} ${symbol === 'QQQ' ? styles.active : ''}`}
                        onClick={() => { setSymbol('QQQ'); setSymbolDropdownOpen(false); }}
                      >
                        QQQ
                      </button>
                      <button
                        className={`${styles.dropdownOption} ${symbol === 'IWM' ? styles.active : ''}`}
                        onClick={() => { setSymbol('IWM'); setSymbolDropdownOpen(false); }}
                      >
                        IWM
                      </button>
                      <button
                        className={`${styles.dropdownOption} ${symbol === 'AAPL' ? styles.active : ''}`}
                        onClick={() => { setSymbol('AAPL'); setSymbolDropdownOpen(false); }}
                      >
                        AAPL
                      </button>
                      <button
                        className={`${styles.dropdownOption} ${symbol === 'TSLA' ? styles.active : ''}`}
                        onClick={() => { setSymbol('TSLA'); setSymbolDropdownOpen(false); }}
                      >
                        TSLA
                      </button>
                    </div>
                  )}
                </div>
                <span className={styles.timeframe}>{timeframe}</span>
              </div>
              {currentBarData && (
                <div className={styles.priceInfo}>
                  <span className={styles.priceLabel}>O</span>
                  <span className={styles.priceValue}>{currentBarData.open.toFixed(2)}</span>
                  <span className={styles.priceLabel}>H</span>
                  <span className={styles.priceValue}>{currentBarData.high.toFixed(2)}</span>
                  <span className={styles.priceLabel}>L</span>
                  <span className={styles.priceValue}>{currentBarData.low.toFixed(2)}</span>
                  <span className={styles.priceLabel}>C</span>
                  <span className={styles.priceValue}>{currentBarData.close.toFixed(2)}</span>
                </div>
              )}
            </div>
          </div>

          {/* Controls Bar */}
          <div className={styles.controlsBar}>
            {/* Replay Controls */}
            <div className={styles.controlGroup}>
              <label className={styles.controlLabel}>Replay:</label>
              <div className={styles.replayControls}>
                <button className={styles.replayBtn} onClick={skipBackward}>
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                    <polygon points="11 19 2 12 11 5 11 19"></polygon>
                    <polygon points="22 19 13 12 22 5 22 19"></polygon>
                  </svg>
                </button>
                <button
                  className={`${styles.replayBtn} ${isPlaying ? styles.active : ''}`}
                  onClick={togglePlay}
                >
                  {isPlaying ? (
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                      <rect x="6" y="4" width="4" height="16"></rect>
                      <rect x="14" y="4" width="4" height="16"></rect>
                    </svg>
                  ) : (
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                      <polygon points="5 3 19 12 5 21 5 3"></polygon>
                    </svg>
                  )}
                </button>
                <button className={styles.replayBtn} onClick={skipForward}>
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                    <polygon points="13 19 22 12 13 5 13 19"></polygon>
                    <polygon points="2 19 11 12 2 5 2 19"></polygon>
                  </svg>
                </button>
                <div 
                  className={styles.dropdownWrapper}
                  onMouseEnter={() => setSpeedDropdownOpen(true)}
                  onMouseLeave={() => setSpeedDropdownOpen(false)}
                >
                  <button className={styles.dropdownButton}>
                    {playbackSpeed}x
                    <span style={{ marginLeft: '8px' }}>▼</span>
                  </button>
                  {speedDropdownOpen && (
                    <div className={styles.dropdownMenu}>
                      {[1, 2, 5, 10].map((speed) => (
                        <button
                          key={speed}
                          className={`${styles.dropdownOption} ${playbackSpeed === speed ? styles.active : ''}`}
                          onClick={() => setPlaybackSpeed(speed as PlaybackSpeed)}
                        >
                          {speed}x
                        </button>
                      ))}
                    </div>
                  )}
                </div>
              </div>
            </div>

            {/* Symbol & Timeframe */}
            <div className={styles.controlGroup}>
              <input
                type="text"
                className="form-input form-input-sm"
                placeholder="Symbol"
                value={symbol}
                onChange={(e) => setSymbol(e.target.value)}
                style={{ width: '80px' }}
              />
              <div 
                className={styles.dropdownWrapper}
                onMouseEnter={() => setTimeframeDropdownOpen(true)}
                onMouseLeave={() => setTimeframeDropdownOpen(false)}
              >
                <button className={styles.dropdownButton}>
                  {timeframe}
                  <span style={{ marginLeft: '8px' }}>▼</span>
                </button>
                {timeframeDropdownOpen && (
                  <div className={styles.dropdownMenu}>
                    {['1m', '5m', '15m', '1h', '1d'].map((tf) => (
                      <button
                        key={tf}
                        className={`${styles.dropdownOption} ${timeframe === tf ? styles.active : ''}`}
                        onClick={() => setTimeframe(tf)}
                      >
                        {tf}
                      </button>
                    ))}
                  </div>
                )}
              </div>
            </div>

            {/* Strategy Selector */}
            <div className={styles.controlGroup}>
              <label className={styles.controlLabel}>Strategy:</label>
              <div 
                className={styles.dropdownWrapper}
                onMouseEnter={() => setStrategyDropdownOpen(true)}
                onMouseLeave={() => setStrategyDropdownOpen(false)}
              >
                <button className={styles.dropdownButton} style={{ minWidth: '200px' }}>
                  {selectedStrategy}
                  <span style={{ marginLeft: '8px' }}>▼</span>
                </button>
                {strategyDropdownOpen && (
                  <div className={styles.dropdownMenu}>
                    {['Mean Reversion v2', 'Momentum Breakout', 'Market Making'].map((strat) => (
                      <button
                        key={strat}
                        className={`${styles.dropdownOption} ${selectedStrategy === strat ? styles.active : ''}`}
                        onClick={() => setSelectedStrategy(strat)}
                      >
                        {strat}
                      </button>
                    ))}
                  </div>
                )}
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

export default MonitorPage;