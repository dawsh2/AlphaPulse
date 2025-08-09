import React, { useState, useEffect, useRef, useCallback } from 'react';
import { createChart } from 'lightweight-charts';
import styles from './MonitorPage.module.css';
import { exchangeManager } from '../../services/exchanges';
import type { MarketData, ExchangeType } from '../../services/exchanges';
import { dataStorage, dataFetcher } from '../../services/data';

interface KrakenOHLC {
  channel: string;
  type: string;
  data: {
    symbol: string;
    timestamp: string;
    open: string;
    high: string;
    low: string;
    close: string;
    vwap: string;
    volume: string;
    count: number;
    interval_begin: string;
  }[];
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
  const [symbol, setSymbol] = useState('BTC/USD');
  const [exchange, setExchange] = useState<ExchangeType>('coinbase');
  const [livePrice, setLivePrice] = useState<number | null>(null);
  const [timeframe, setTimeframe] = useState('1m');
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
  const wsRef = useRef<WebSocket | null>(null);

  // Data
  const [marketData, setMarketData] = useState<MarketData[]>([]);
  const [eventData, setEventData] = useState<EventData[]>([]);

  // Mock data and strategies remain unchanged

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
    if (!chartContainerRef.current) {
      console.error('Chart container not found');
      return;
    }

    const containerWidth = chartContainerRef.current.clientWidth;
    const containerHeight = chartContainerRef.current.clientHeight;
    
    if (!containerWidth || !containerHeight) {
      console.error('Chart container has no dimensions', { containerWidth, containerHeight });
      return;
    }

    // Detect theme
    const isDark = document.documentElement.getAttribute('data-theme') === 'dark' ||
                   window.matchMedia('(prefers-color-scheme: dark)').matches;

    const chart = createChart(chartContainerRef.current, {
      width: containerWidth,
      height: containerHeight,
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
      handleScroll: {
        mouseWheel: true,
        pressedMouseMove: true,
        horzTouchDrag: true,
        vertTouchDrag: true
      },
      handleScale: {
        axisPressedMouseMove: {
          time: true,
          price: true
        },
        mouseWheel: true,
        pinch: true
      },
      rightPriceScale: {
        borderColor: isDark ? '#383c45' : '#e5e0d5',
        mode: 0, // Normal mode (not logarithmic)
        autoScale: true, // Keep auto-scale for now
        invertScale: false,
        alignLabels: true,
        borderVisible: true,
        entireTextOnly: false,
        visible: true,
        scaleMargins: {
          top: 0.1,
          bottom: 0.1
        }
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

    // Show all available data
    if (marketData.length > 0) {
      candleSeries.setData(marketData as any);
      setCurrentBar(marketData.length);
    }
    
    // Add custom wheel handler for Y-axis zoom
    const wheelHandler = (e: WheelEvent) => {
      if (!chartRef.current) return;
      
      // Check if we're over the price scale area or holding shift
      const rect = chartContainerRef.current?.getBoundingClientRect();
      if (!rect) return;
      
      const isOverPriceScale = e.clientX > rect.right - 60; // Price scale is ~60px wide
      
      if (e.shiftKey || isOverPriceScale) {
        e.preventDefault();
        e.stopPropagation();
        
        const priceScale = candleSeries.priceScale();
        const logicalRange = priceScale.getVisibleLogicalRange();
        
        if (logicalRange) {
          // Zoom factor
          const scaleFactor = e.deltaY > 0 ? 1.05 : 0.95;
          
          // Get current visible price range
          const currentRange = candleSeries.priceScale().getVisiblePriceRange();
          if (currentRange) {
            const center = (currentRange.from + currentRange.to) / 2;
            const range = currentRange.to - currentRange.from;
            const newRange = range * scaleFactor;
            
            // Apply new range
            candleSeries.priceScale().setVisiblePriceRange({
              from: center - newRange / 2,
              to: center + newRange / 2
            });
          }
        }
      }
    };
    
    if (chartContainerRef.current) {
      chartContainerRef.current.addEventListener('wheel', wheelHandler, { passive: false });
      
      // Store handler for cleanup
      (chartContainerRef.current as any)._wheelHandler = wheelHandler;
    }
  };

  // Update chart data for playback mode
  const updateChart = (bars: number) => {
    if (!candleSeriesRef.current || !marketData.length) return;

    // For playback mode, show limited data
    if (isPlaying) {
      const visibleData = marketData.slice(0, bars);
      candleSeriesRef.current.setData(visibleData as any);
      
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
    }
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
    // Set the exchange
    exchangeManager.setExchange(exchange);
    const service = exchangeManager.getService();
    
    if (!service) {
      console.error('No exchange service available');
      return;
    }

    // Backfill historical data
    const backfillData = async () => {
      try {
        // First, try to load from local storage
        console.log(`[${exchange}] Checking local storage for ${symbol} data...`);
        
        const cachedData = await dataStorage.queryCandles({
          symbol,
          exchange,
          interval: '1m',
          limit: 10000 // Get up to ~7 days of data
        });
        
        if (cachedData.length > 0) {
          // Convert stored data to MarketData format
          const marketData: MarketData[] = cachedData
            .sort((a, b) => a.timestamp - b.timestamp)
            .map(candle => ({
              time: candle.timestamp,
              open: candle.open,
              high: candle.high,
              low: candle.low,
              close: candle.close,
              volume: candle.volume
            }));
          
          console.log(`[${exchange}] Loaded ${marketData.length} candles from cache`);
          setMarketData(marketData);
          setCurrentBar(marketData.length);
          
          // Add event
          setEventData(prev => [...prev, {
            time: new Date().toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit' }),
            type: 'signal',
            description: `[${exchange}] Loaded ${marketData.length} candles from cache`
          }]);
          
          // Check if we need to update in background
          dataFetcher.updateIfNeeded(symbol, exchange).catch(console.error);
        } else {
          // No cached data, fetch from exchange
          console.log(`[${exchange}] No cached data, fetching from API...`);
          
          // For initial load, fetch and store 7 days of data
          if (exchange === 'coinbase' && (symbol === 'BTC/USD' || symbol === 'ETH/USD')) {
            const result = await dataFetcher.fetchAndStoreHistoricalData(symbol, 7);
            if (result.success) {
              // Load the newly fetched data
              const newData = await dataStorage.queryCandles({
                symbol,
                exchange,
                interval: '1m',
                limit: 10000
              });
              
              const marketData: MarketData[] = newData
                .sort((a, b) => a.timestamp - b.timestamp)
                .map(candle => ({
                  time: candle.timestamp,
                  open: candle.open,
                  high: candle.high,
                  low: candle.low,
                  close: candle.close,
                  volume: candle.volume
                }));
              
              setMarketData(marketData);
              setCurrentBar(marketData.length);
              
              setEventData(prev => [...prev, {
                time: new Date().toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit' }),
                type: 'signal',
                description: `[${exchange}] Fetched and cached ${result.candleCount} candles`
              }]);
            }
          } else {
            // Fallback to regular API fetch for other exchanges
            const historicalData = await service.fetchHistoricalData(symbol, 30);
            console.log(`[${exchange}] Loaded ${historicalData.length} historical candles`);
            
            setMarketData(historicalData);
            setCurrentBar(historicalData.length);
            
            setEventData(prev => [...prev, {
              time: new Date().toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit' }),
              type: 'signal',
              description: `[${exchange}] Loaded ${historicalData.length} historical candles`
            }]);
          }
        }
      } catch (error) {
        console.error(`[${exchange}] Failed to backfill data:`, error);
      }
    };

    // Connect to exchange WebSocket
    const connectToExchange = () => {
      const ws = service.connect(symbol, (newCandle: MarketData) => {
        console.log(`[${exchange}] New candle:`, {
          time: new Date(newCandle.time * 1000).toISOString(),
          ohlc: `O:${newCandle.open.toFixed(2)} H:${newCandle.high.toFixed(2)} L:${newCandle.low.toFixed(2)} C:${newCandle.close.toFixed(2)}`
        });
        
        // Update live price
        setLivePrice(newCandle.close);
        
        // Update market data
        setMarketData(prev => {
          const updated = [...prev];
          const existingIndex = updated.findIndex(d => d.time === newCandle.time);
          
          if (existingIndex >= 0) {
            // Update existing candle
            updated[existingIndex] = newCandle;
          } else {
            // Add new candle only if it's newer
            const lastCandle = updated[updated.length - 1];
            if (!lastCandle || newCandle.time > lastCandle.time) {
              updated.push(newCandle);
              if (updated.length > 500) {
                updated.shift();
              }
            }
          }
          
          updated.sort((a, b) => a.time - b.time);
          return updated;
        });
      });
      
      if (ws) {
        wsRef.current = ws;
        
        // Add connection event
        setEventData(prev => [...prev, {
          time: new Date().toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit' }),
          type: 'signal',
          description: `Connected to ${exchange} live ${symbol} feed`
        }]);
      }
    };

    let cancelled = false;
    const abortController = new AbortController();
    
    // First backfill historical data, then connect WebSocket
    const init = async () => {
      try {
        await backfillData();
        if (!cancelled) {
          // Small delay to avoid rapid connect/disconnect in dev
          setTimeout(() => {
            if (!cancelled) {
              connectToExchange();
            }
          }, 100);
        }
      } catch (error) {
        console.error('Failed to initialize:', error);
      }
    };
    
    init();
    
    // Cleanup on unmount or symbol change
    return () => {
      cancelled = true;
      abortController.abort();
      if (service) {
        service.disconnect();
      }
      if (wsRef.current) {
        wsRef.current = null;
      }
    };
  }, [symbol, exchange]);

  // Initialize chart when container is ready
  useEffect(() => {
    let retryCount = 0;
    const maxRetries = 5;
    
    const tryInitChart = () => {
      if (!chartRef.current && chartContainerRef.current) {
        const width = chartContainerRef.current.clientWidth;
        const height = chartContainerRef.current.clientHeight;
        
        console.log(`Chart init attempt ${retryCount + 1}:`, { width, height, hasContainer: !!chartContainerRef.current });
        
        if (width > 0 && height > 0) {
          initChart();
        } else if (retryCount < maxRetries) {
          retryCount++;
          setTimeout(tryInitChart, 200);
        } else {
          console.error('Failed to initialize chart after', maxRetries, 'attempts');
        }
      }
    };
    
    // Start initialization after a small delay
    const timer = setTimeout(tryInitChart, 100);

    return () => {
      clearTimeout(timer);
      // Remove wheel handler
      if (chartContainerRef.current && (chartContainerRef.current as any)._wheelHandler) {
        chartContainerRef.current.removeEventListener('wheel', (chartContainerRef.current as any)._wheelHandler);
        delete (chartContainerRef.current as any)._wheelHandler;
      }
      if (chartRef.current) {
        chartRef.current.remove();
        chartRef.current = null;
        candleSeriesRef.current = null;
      }
      if (playbackIntervalRef.current) {
        clearInterval(playbackIntervalRef.current);
      }
    };
  }, []); // Empty dependency array - only init once

  // Update chart when market data changes
  useEffect(() => {
    if (candleSeriesRef.current && marketData.length > 0) {
      console.log(`Updating chart with ${marketData.length} candles`);
      // Log first and last few candles to debug
      console.log('First candle:', marketData[0]);
      console.log('Last candles:', marketData.slice(-3));
      
      candleSeriesRef.current.setData(marketData as any);
    } else if (!candleSeriesRef.current && marketData.length > 0) {
      console.log('Chart not ready yet, data available:', marketData.length);
    }
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
                      {exchangeManager.getSupportedSymbols().map((sym) => (
                        <button
                          key={sym}
                          className={`${styles.dropdownOption} ${symbol === sym ? styles.active : ''}`}
                          onClick={() => { setSymbol(sym); setSymbolDropdownOpen(false); }}
                        >
                          {sym}
                        </button>
                      ))}
                    </div>
                  )}
                </div>
                <span className={styles.timeframe}>{timeframe}</span>
                {livePrice && (
                  <span className={styles.liveIndicator}>
                    <span className={styles.liveDot}></span>
                    LIVE: ${livePrice.toFixed(2)}
                  </span>
                )}
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

            {/* Exchange Selector */}
            <div className={styles.controlGroup}>
              <label className={styles.controlLabel}>Exchange:</label>
              <select 
                className="form-input form-input-sm"
                value={exchange}
                onChange={(e) => setExchange(e.target.value as ExchangeType)}
                style={{ width: '100px' }}
              >
                {exchangeManager.getAvailableExchanges().map(ex => (
                  <option key={ex.value} value={ex.value}>{ex.label}</option>
                ))}
              </select>
            </div>

            {/* Symbol & Timeframe */}
            <div className={styles.controlGroup}>
              <input
                type="text"
                className="form-input form-input-sm"
                placeholder="Symbol"
                value={symbol}
                onChange={(e) => setSymbol(e.target.value)}
                style={{ width: '100px' }}
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
                    {['tick', '1m', '5m', '15m', '1h', '1d'].map((tf) => (
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