import React, { useState, useEffect, useRef, useCallback } from 'react';
import { createChart } from 'lightweight-charts';
import MetricsSidebar from '../features/Monitor/MetricsSidebar';
import type { SidebarTab } from '../features/Monitor/MetricsSidebar';
import PlaybackControls from '../features/Monitor/PlaybackControls';
import type { PlaybackSpeed } from '../features/Monitor/PlaybackControls';
import SymbolSelector from '../features/Monitor/SymbolSelector';
import styles from './MonitorPage.module.css';
import { exchangeManager } from '../../services/exchanges';
import type { MarketData, ExchangeType } from '../../services/exchanges';
import { dataStorage, dataFetcher } from '../../services/data';
import { topOffStoredData } from '../../services/exchanges/dataTopOff';

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
  const [symbolDropdownOpen, setSymbolDropdownOpen] = useState(false);
  const [isLoadingData, setIsLoadingData] = useState(false);
  const dropdownTimeoutRef = useRef<NodeJS.Timeout | null>(null);

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

  const mockMetrics = {
    totalPnL: 2345.67,
    winRate: 68.4,
    sharpeRatio: 1.82,
    maxDrawdown: -8.3,
    totalTrades: 247,
    avgTrade: 9.49
  };

  const mockStrategies = [
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
        textColor: isDark ? '#f0f6fc' : '#1c1c1c',
      },
      grid: {
        vertLines: { color: isDark ? '#2d3139' : '#e0e3eb' },
        horzLines: { color: isDark ? '#2d3139' : '#e0e3eb' },
      },
      crosshair: {
        mode: 0, // Normal
        vertLine: {
          color: isDark ? '#505862' : '#9598a1',
          width: 1,
          style: 3,
          labelBackgroundColor: isDark ? '#2d3139' : '#ffffff'
        },
        horzLine: {
          color: isDark ? '#505862' : '#9598a1',
          width: 1,
          style: 3,
          labelBackgroundColor: isDark ? '#2d3139' : '#ffffff'
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
        borderColor: isDark ? '#2d3139' : '#e0e3eb',
        textColor: isDark ? '#d1d4dc' : '#131722',
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
        borderColor: isDark ? '#2d3139' : '#e0e3eb',
        textColor: isDark ? '#d1d4dc' : '#131722',
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
        
        // Use the chart's time scale for zoom functionality
        const timeScale = chartRef.current.timeScale();
        const logicalRange = timeScale.getVisibleLogicalRange();
        
        if (logicalRange) {
          // Zoom factor for price scale
          const scaleFactor = e.deltaY > 0 ? 1.05 : 0.95;
          
          // Get current auto scale mode and temporarily disable it
          const priceScale = candleSeries.priceScale();
          const autoScaleEnabled = priceScale.options().autoScale;
          
          // Apply scaling by adjusting the auto scale margins
          priceScale.applyOptions({
            autoScale: false,
            scaleMargins: {
              top: 0.1 * scaleFactor,
              bottom: 0.1 * scaleFactor
            }
          });
          
          // Re-enable auto scale if it was enabled
          setTimeout(() => {
            priceScale.applyOptions({
              autoScale: autoScaleEnabled
            });
          }, 100);
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
    // Show loading state when switching symbols
    setIsLoadingData(true);
    setMarketData([]); // Clear old data immediately
    
    // Set the exchange
    exchangeManager.setExchange(exchange);
    const service = exchangeManager.getService();
    
    if (!service) {
      console.error('No exchange service available');
      setIsLoadingData(false);
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
          let marketData: MarketData[] = cachedData
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
          
          // Check if we should fetch more historical data to build up our dataset
          const oldestCandle = marketData[0];
          const daysOfData = (Date.now() / 1000 - oldestCandle.time) / (60 * 60 * 24);
          
          // If we have less than 30 days of data, try to fetch more historical data
          if (daysOfData < 30 && exchange === 'coinbase') {
            console.log(`[${exchange}] Only have ${daysOfData.toFixed(1)} days of data, fetching more history...`);
            // Fetch another 7 days of older data
            const olderDataEndTime = oldestCandle.time;
            const olderDataStartTime = olderDataEndTime - (7 * 24 * 60 * 60);
            
            // TODO: Add method to fetch specific date range
            // For now, we'll just work with what we have
          }
          
          // Top off the stored data to fill any gaps
          const { data: toppedOffData, result } = await topOffStoredData(
            symbol,
            exchange,
            service,
            marketData
          );
          
          marketData = toppedOffData;
          
          setEventData(prev => [...prev, {
            time: new Date().toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit' }),
            type: 'signal',
            description: `[${exchange}] ${result.message}`
          }]);
          
          // Always set the market data
          setMarketData(marketData);
          setCurrentBar(marketData.length);
          setIsLoadingData(false);
        } else {
          // No cached data, fetch from exchange
          console.log(`[${exchange}] No cached data, fetching from API...`);
          
          // Fetch initial data - start with 7 days for good historical context
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
              
              let marketData: MarketData[] = newData
                .sort((a, b) => a.timestamp - b.timestamp)
                .map(candle => ({
                  time: candle.timestamp,
                  open: candle.open,
                  high: candle.high,
                  low: candle.low,
                  close: candle.close,
                  volume: candle.volume
                }));
              
              // Top off the newly fetched data
              if (marketData.length > 0) {
                const { data: toppedOffData, result } = await topOffStoredData(
                  symbol,
                  exchange,
                  service,
                  marketData
                );
                
                marketData = toppedOffData;
                console.log(`[${exchange}] Top-off result: ${result.message}`);
              }
              
              setMarketData(marketData);
              setCurrentBar(marketData.length);
              setIsLoadingData(false);
              
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
            setIsLoadingData(false);
            
            setEventData(prev => [...prev, {
              time: new Date().toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit' }),
              type: 'signal',
              description: `[${exchange}] Loaded ${historicalData.length} historical candles`
            }]);
          }
        }
      } catch (error) {
        console.error(`[${exchange}] Failed to backfill data:`, error);
        setIsLoadingData(false);
      }
    };

    // Connect to exchange WebSocket  
    const connectToExchange = async (currentMarketData: MarketData[]) => {
      let lastUpdateTime = 0;
      
      // Don't fetch recent candles here - let topOffStoredData handle gap filling
      // This was causing the gap issue by only fetching 3 minutes
      
      // Now connect WebSocket for live updates
      const ws = service.connect(symbol, (newCandle: MarketData) => {
        // Update live price immediately
        setLivePrice(newCandle.close);
        
        // Throttle chart updates to once per second to prevent spam
        const now = Date.now();
        if (now - lastUpdateTime < 1000) {
          return; // Skip this update if less than 1 second has passed
        }
        lastUpdateTime = now;
        
        // Update market data
        setMarketData(prev => {
          const updated = [...prev];
          const existingIndex = updated.findIndex(d => d.time === newCandle.time);
          
          if (existingIndex >= 0) {
            // For the current minute, be careful about replacing complete candles
            const existing = updated[existingIndex];
            
            // Check if the new candle has better data
            // A candle built from WebSocket might start with all prices the same
            const existingRange = existing.high - existing.low;
            const newRange = newCandle.high - newCandle.low;
            
            // Only update if the new candle has equal or better price range
            // OR if the prices have actually changed
            if (newRange >= existingRange ||
                existing.close !== newCandle.close ||
                (newCandle.high > existing.high) ||
                (newCandle.low < existing.low)) {
              updated[existingIndex] = newCandle;
            } else {
              return prev; // Keep the existing better candle
            }
          } else {
            // Add new candle only if it's newer
            const lastCandle = updated[updated.length - 1];
            if (!lastCandle || newCandle.time > lastCandle.time) {
              updated.push(newCandle);
              // Keep a reasonable amount of data in memory
              if (updated.length > 10000) {
                updated.shift();
              }
            }
          }
          
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
      if (cancelled) return;
      
      try {
        await backfillData();
        if (!cancelled) {
          // Small delay to avoid rapid connect/disconnect in dev
          // Note: We can't pass marketData here as it's in state and async
          // The connectToExchange will work with live updates only
          setTimeout(() => {
            if (!cancelled) {
              connectToExchange([]);
            }
          }, 100);
        }
      } catch (error) {
        console.error('Failed to initialize:', error);
      }
    };
    
    // Small delay to prevent double-fetch in React StrictMode
    const timeoutId = setTimeout(() => {
      if (!cancelled) {
        init();
      }
    }, 10);
    
    // Cleanup on unmount or symbol change
    return () => {
      cancelled = true;
      clearTimeout(timeoutId);
      abortController.abort();
      if (service) {
        service.disconnect();
      }
      if (wsRef.current) {
        if (wsRef.current.readyState === WebSocket.OPEN || 
            wsRef.current.readyState === WebSocket.CONNECTING) {
          wsRef.current.close();
        }
        wsRef.current = null;
      }
    };
  }, [symbol, exchange]);

  // Initialize chart when container is ready
  useEffect(() => {
    let retryCount = 0;
    const maxRetries = 5;
    let themeObserver: MutationObserver | null = null;
    
    const tryInitChart = () => {
      if (!chartRef.current && chartContainerRef.current) {
        const width = chartContainerRef.current.clientWidth;
        const height = chartContainerRef.current.clientHeight;
        
        console.log(`Chart init attempt ${retryCount + 1}:`, { width, height, hasContainer: !!chartContainerRef.current });
        
        if (width > 0 && height > 0) {
          initChart();
          
          // Set up theme observer to update chart colors when theme changes
          themeObserver = new MutationObserver((mutations) => {
            mutations.forEach((mutation) => {
              if (mutation.type === 'attributes' && mutation.attributeName === 'data-theme') {
                updateChartTheme();
              }
            });
          });
          
          themeObserver.observe(document.documentElement, {
            attributes: true,
            attributeFilter: ['data-theme']
          });
        } else if (retryCount < maxRetries) {
          retryCount++;
          setTimeout(tryInitChart, 200);
        } else {
          console.error('Failed to initialize chart after', maxRetries, 'attempts');
        }
      }
    };
    
    // Function to update chart theme
    const updateChartTheme = () => {
      if (!chartRef.current) return;
      
      const isDark = document.documentElement.getAttribute('data-theme') === 'dark' ||
                     window.matchMedia('(prefers-color-scheme: dark)').matches;
      
      // Update all chart colors
      chartRef.current.applyOptions({
        layout: {
          background: { color: 'transparent' },
          textColor: isDark ? '#f0f6fc' : '#1c1c1c',
        },
        grid: {
          vertLines: { color: isDark ? '#2d3139' : '#e0e3eb' },
          horzLines: { color: isDark ? '#2d3139' : '#e0e3eb' },
        },
        crosshair: {
          mode: 0,
          vertLine: {
            color: isDark ? '#505862' : '#9598a1',
            width: 1,
            style: 3,
            labelBackgroundColor: isDark ? '#2d3139' : '#ffffff'
          },
          horzLine: {
            color: isDark ? '#505862' : '#9598a1',
            width: 1,
            style: 3,
            labelBackgroundColor: isDark ? '#2d3139' : '#ffffff'
          }
        },
        rightPriceScale: {
          borderColor: isDark ? '#2d3139' : '#e0e3eb',
          textColor: isDark ? '#d1d4dc' : '#131722',
        },
        timeScale: {
          borderColor: isDark ? '#2d3139' : '#e0e3eb',
          textColor: isDark ? '#d1d4dc' : '#131722',
        },
      });
    };
    
    // Start initialization after a small delay
    const timer = setTimeout(tryInitChart, 100);

    return () => {
      clearTimeout(timer);
      // Disconnect theme observer
      if (themeObserver) {
        themeObserver.disconnect();
      }
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

  // Update chart when market data changes (debounced)
  useEffect(() => {
    if (!candleSeriesRef.current || marketData.length === 0) {
      if (!candleSeriesRef.current && marketData.length > 0) {
        console.log('Chart not ready yet, data available:', marketData.length);
      }
      return;
    }
    
    // Debounce chart updates to prevent excessive re-renders
    const timeoutId = setTimeout(() => {
      if (candleSeriesRef.current && marketData.length > 0) {
        console.log(`Updating chart with ${marketData.length} candles`);
        // Only log in debug mode or first update
        if (marketData.length <= 100 || !candleSeriesRef.current.options) {
          console.log('First candle:', marketData[0]);
          console.log('Last candles:', marketData.slice(-3));
        }
        
        candleSeriesRef.current.setData(marketData as any);
      }
    }, 100); // 100ms debounce
    
    return () => clearTimeout(timeoutId);
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


  return (
    <div className={styles.monitorContainer}>
      {/* Content Area */}
      <div className={styles.contentArea}>
        {/* Sidebar */}
        <MetricsSidebar
          sidebarTab={sidebarTab}
          setSidebarTab={setSidebarTab}
          mockMetrics={mockMetrics}
          eventData={eventData}
          mockStrategies={mockStrategies}
          currentBar={currentBar}
          selectedStrategy={selectedStrategy}
          setSelectedStrategy={setSelectedStrategy}
          styles={styles}
        />

        <div className={styles.mainContent}>
          {/* Chart */}
          <div className={styles.chartContainer}>
            <div ref={chartContainerRef} className={styles.chart}></div>
            
            {/* Loading overlay */}
            {isLoadingData && (
              <div className={styles.loadingOverlay}>
                <div className={styles.loadingSpinner}></div>
                <div className={styles.loadingText}>Updating market data...</div>
              </div>
            )}

            <div className={styles.chartInfo}>
              <div className={styles.symbolInfo}>
                <div className={styles.symbolDropdown}>
                  <button 
                    className={styles.symbolButton}
                    onClick={() => setSymbolDropdownOpen(!symbolDropdownOpen)}
                  >
                    {symbol}
                    <span style={{ marginLeft: '8px', fontSize: '10px' }}>▼</span>
                  </button>
                  {symbolDropdownOpen && (
                    <div 
                      className={styles.symbolDropdownMenu}
                      onMouseEnter={() => {
                        if (dropdownTimeoutRef.current) {
                          clearTimeout(dropdownTimeoutRef.current);
                          dropdownTimeoutRef.current = null;
                        }
                      }}
                      onMouseLeave={() => {
                        dropdownTimeoutRef.current = setTimeout(() => {
                          setSymbolDropdownOpen(false);
                        }, 300);
                      }}
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
            </div>
          </div>

          {/* Controls Bar */}
          <div className={styles.controlsBar}>
            {/* Replay Controls */}
            <PlaybackControls
              isPlaying={isPlaying}
              playbackSpeed={playbackSpeed}
              currentBar={currentBar}
              maxBars={marketData.length}
              onTogglePlay={togglePlay}
              onSkipBackward={skipBackward}
              onSkipForward={skipForward}
              onSpeedChange={setPlaybackSpeed}
              styles={styles}
            />

            <SymbolSelector
              symbol={symbol}
              exchange={exchange}
              timeframe={timeframe}
              selectedStrategy={selectedStrategy}
              onSymbolChange={setSymbol}
              onExchangeChange={setExchange}
              onTimeframeChange={setTimeframe}
              onStrategyChange={setSelectedStrategy}
              styles={styles}
            />
          </div>
        </div>
      </div>
    </div>
  );
};

export default MonitorPage;