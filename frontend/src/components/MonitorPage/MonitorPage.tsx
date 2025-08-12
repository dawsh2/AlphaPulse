import React, { useState, useEffect, useRef, useCallback } from 'react';
import MetricsSidebar from '../features/Monitor/MetricsSidebar';
import type { SidebarTab } from '../features/Monitor/MetricsSidebar';
import PlaybackControls from '../features/Monitor/PlaybackControls';
import type { PlaybackSpeed } from '../features/Monitor/PlaybackControls';
import SymbolSelector from '../features/Monitor/SymbolSelector';
import { ChartLayoutManager } from '../features/Monitor/ChartLayoutManager';
import type { LayoutNode, ChartWindowNode } from '../features/Monitor/ChartLayoutManager';
import styles from './MonitorPage.module.css';
import { exchangeManager } from '../../services/exchanges';
import type { MarketData, ExchangeType } from '../../services/exchanges';
import { chartManager } from '../../services/charts/ChartManager';
import type { EventData } from '../../types/monitor.types';

const MonitorPage: React.FC = () => {
  // State management
  const [sidebarTab, setSidebarTab] = useState<SidebarTab>('metrics');
  const [isPlaying, setIsPlaying] = useState(false);
  const [currentBar, setCurrentBar] = useState(50);
  const [playbackSpeed, setPlaybackSpeed] = useState<PlaybackSpeed>(1);
  const [selectedStrategy, setSelectedStrategy] = useState('Mean Reversion v2');
  const [strategyDropdownOpen, setStrategyDropdownOpen] = useState(false);
  
  // Chart layout state
  const [layout, setLayout] = useState<LayoutNode>(() => {
    // Initialize with a single chart window
    const initialWindow: ChartWindowNode = {
      type: 'window',
      id: 'chart-main',
      config: {
        symbol: 'BTC/USD',
        exchange: 'coinbase' as ExchangeType,
        timeframe: '1m',
        marketData: [],
        isLoadingData: true, // Start with loading state
        livePrice: null
      }
    };
    return initialWindow;
  });

  // Event data for sidebar
  const [eventData, setEventData] = useState<EventData[]>([]);
  
  // Refs
  const playbackIntervalRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const chartDataRef = useRef<Map<string, MarketData[]>>(new Map());

  // Mock data for sidebar
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

  // Get the main chart for strategy selector (first window in the layout)
  const getMainChart = useCallback((node: LayoutNode): ChartWindowNode | null => {
    if (node.type === 'window') {
      return node;
    } else if (node.type === 'split' && node.children.length > 0) {
      return getMainChart(node.children[0]);
    }
    return null;
  }, []);

  // Handle symbol change for a specific window
  const handleSymbolChange = useCallback(async (windowId: string, symbol: string) => {
    console.log(`[MonitorPage] handleSymbolChange called for windowId: ${windowId}, symbol: ${symbol}`);
    // Update layout immediately
    setLayout(prev => {
      const updateWindow = (node: LayoutNode): LayoutNode => {
        if (node.type === 'window' && node.id === windowId) {
          return {
            ...node,
            config: {
              ...node.config,
              symbol,
              marketData: [],
              isLoadingData: true
            }
          };
        } else if (node.type === 'split') {
          return {
            ...node,
            children: node.children.map(updateWindow)
          };
        }
        return node;
      };
      return updateWindow(prev);
    });
    
    // Get the window config to load data
    const findWindow = (node: LayoutNode): ChartWindowNode | null => {
      if (node.type === 'window' && node.id === windowId) {
        return node;
      } else if (node.type === 'split') {
        for (const child of node.children) {
          const found = findWindow(child);
          if (found) return found;
        }
      }
      return null;
    };
    
    const targetWindow = findWindow(layout);
    if (targetWindow) {
      // Set up data callback BEFORE creating chart to avoid race condition
      chartManager.onDataUpdate(windowId, (data) => {
        setLayout(prev => {
          const updateWithData = (node: LayoutNode): LayoutNode => {
            if (node.type === 'window' && node.id === windowId) {
              return {
                ...node,
                config: {
                  ...node.config,
                  marketData: data,
                  isLoadingData: false
                }
              };
            } else if (node.type === 'split') {
              return {
                ...node,
                children: node.children.map(updateWithData)
              };
            }
            return node;
          };
          return updateWithData(prev);
        });
      });
      
      // Set up live price callback
      chartManager.onLivePriceUpdate(windowId, (price) => {
        setLayout(prev => {
          const updatePrice = (node: LayoutNode): LayoutNode => {
            if (node.type === 'window' && node.id === windowId) {
              return {
                ...node,
                config: {
                  ...node.config,
                  livePrice: price
                }
              };
            } else if (node.type === 'split') {
              return {
                ...node,
                children: node.children.map(updatePrice)
              };
            }
            return node;
          };
          return updatePrice(prev);
        });
      });
      
      // NOW create the chart after callbacks are registered
      const chart = await chartManager.createChart(
        windowId,
        symbol,
        targetWindow.config.exchange,
        targetWindow.config.timeframe
      );
    }
  }, [layout]);

  // Handle exchange change
  const handleExchangeChange = useCallback(async (windowId: string, exchange: ExchangeType) => {
    const updateWindow = (node: LayoutNode): LayoutNode => {
      if (node.type === 'window' && node.id === windowId) {
        return {
          ...node,
          config: {
            ...node.config,
            exchange,
            marketData: [],
            isLoadingData: true
          }
        };
      } else if (node.type === 'split') {
        return {
          ...node,
          children: node.children.map(updateWindow)
        };
      }
      return node;
    };
    
    setLayout(updateWindow(layout));
    
    // Reload data with new exchange
    await chartManager.updateChart(windowId, { exchange });
  }, [layout]);

  // Handle timeframe change
  const handleTimeframeChange = useCallback(async (windowId: string, timeframe: string) => {
    const updateWindow = (node: LayoutNode): LayoutNode => {
      if (node.type === 'window' && node.id === windowId) {
        return {
          ...node,
          config: {
            ...node.config,
            timeframe
          }
        };
      } else if (node.type === 'split') {
        return {
          ...node,
          children: node.children.map(updateWindow)
        };
      }
      return node;
    };
    
    setLayout(updateWindow(layout));
    
    // Note: For now we keep the same data, but in the future we might
    // want to resample the data based on the new timeframe
  }, [layout]);

  // Ref to prevent duplicate initialization 
  const chartInitializedRef = useRef(false);
  const cleanupRef = useRef(false);

  // Initialize first chart on mount
  useEffect(() => {
    const initializeMainChart = async () => {
      if (chartInitializedRef.current) return; // Prevent duplicate initialization
      chartInitializedRef.current = true;
      cleanupRef.current = false;
      
      const mainChart = getMainChart(layout);
      if (!mainChart) return;
      
      console.log(`[MonitorPage] Initializing main chart ${mainChart.id}`);
      
      // Mark as loading
      setLayout(prev => {
        const updateLoading = (node: LayoutNode): LayoutNode => {
          if (node.type === 'window' && node.id === mainChart.id) {
            return {
              ...node,
              config: {
                ...node.config,
                isLoadingData: true
              }
            };
          }
          return node;
        };
        return updateLoading(prev);
      });
      
      // Set up data listeners BEFORE creating chart to avoid race condition
      chartManager.onDataUpdate(mainChart.id, (data) => {
        console.log(`[MonitorPage] Data update for ${mainChart.id}: ${data.length} candles`);
        setLayout(prev => {
          const updateWithData = (node: LayoutNode): LayoutNode => {
            if (node.type === 'window' && node.id === mainChart.id) {
              return {
                ...node,
                config: {
                  ...node.config,
                  marketData: data,
                  isLoadingData: false
                }
              };
            }
            return node;
          };
          return updateWithData(prev);
        });
        
        // Store data for playback
        chartDataRef.current.set(mainChart.id, data);
        if (data.length > 0) {
          setCurrentBar(data.length);
        }
      });
      
      // Set up live price listener
      chartManager.onLivePriceUpdate(mainChart.id, (price) => {
        setLayout(prev => {
          const updatePrice = (node: LayoutNode): LayoutNode => {
            if (node.type === 'window' && node.id === mainChart.id) {
              return {
                ...node,
                config: {
                  ...node.config,
                  livePrice: price
                }
              };
            }
            return node;
          };
          return updatePrice(prev);
        });
      });
      
      // NOW create the chart after callbacks are registered
      const chart = chartManager.createChart(
        mainChart.id,
        mainChart.config.symbol,
        mainChart.config.exchange,
        mainChart.config.timeframe
      );
      
      // Add connection event
      setEventData(prev => [...prev, {
        time: new Date().toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit' }),
        type: 'signal',
        description: `Connected to ${mainChart.config.exchange} live ${mainChart.config.symbol} feed`
      }]);
    };
    
    initializeMainChart();
    
    // Cleanup - Skip cleanup in development mode due to React StrictMode
    return () => {
      console.log('[MonitorPage] useEffect cleanup called');
      
      // In development, React StrictMode calls cleanup immediately after effect
      // This breaks our async data loading, so skip cleanup in dev
      if (import.meta.env.DEV) {
        console.log('[MonitorPage] Skipping cleanup in development mode (React StrictMode)');
        return;
      }
      
      // Only clean up in production
      const mainChart = getMainChart(layout);
      if (mainChart) {
        console.log('[MonitorPage] Production cleanup - removing chart:', mainChart.id);
        chartManager.removeChart(mainChart.id);
      }
    };
  }, []); // Only run on mount

  // Playback controls
  const startPlayback = () => {
    if (playbackIntervalRef.current) {
      clearInterval(playbackIntervalRef.current);
    }

    const mainChart = getMainChart(layout);
    if (!mainChart) return;
    
    const marketData = chartDataRef.current.get(mainChart.id) || [];

    playbackIntervalRef.current = setInterval(() => {
      setCurrentBar(prev => {
        if (prev < marketData.length) {
          return prev + 1;
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
  };

  const skipForward = () => {
    const mainChart = getMainChart(layout);
    if (!mainChart) return;
    
    const marketData = chartDataRef.current.get(mainChart.id) || [];
    const newBar = Math.min(marketData.length, currentBar + 50);
    setCurrentBar(newBar);
  };

  // Update playback speed
  useEffect(() => {
    if (isPlaying) {
      stopPlayback();
      startPlayback();
    }
  }, [playbackSpeed]);

  // Get main chart data for controls bar
  const mainChart = getMainChart(layout);
  const mainMarketData = mainChart ? (chartDataRef.current.get(mainChart.id) || []) : [];

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
          {/* Chart Layout Manager */}
          <ChartLayoutManager
            layout={layout}
            onLayoutChange={setLayout}
            onSymbolChange={handleSymbolChange}
            onExchangeChange={handleExchangeChange}
            onTimeframeChange={handleTimeframeChange}
            playbackControls={
              isPlaying ? {
                isPlaying,
                currentBar,
                playbackSpeed
              } : undefined
            }
          />

          {/* Controls Bar - Only Replay and Strategy */}
          <div className={styles.controlsBar}>
            {/* Replay Controls */}
            <PlaybackControls
              isPlaying={isPlaying}
              playbackSpeed={playbackSpeed}
              currentBar={currentBar}
              maxBars={mainMarketData.length}
              onTogglePlay={togglePlay}
              onSkipBackward={skipBackward}
              onSkipForward={skipForward}
              onSpeedChange={setPlaybackSpeed}
              styles={styles}
            />

            {/* Strategy Selector */}
            <div className={styles.controlGroup}>
              <label className={styles.controlLabel}>Strategy:</label>
              <div className={styles.dropdownWrapper}>
                <button
                  className={styles.dropdownButton}
                  onClick={() => setStrategyDropdownOpen(!strategyDropdownOpen)}
                  onMouseEnter={() => setStrategyDropdownOpen(true)}
                  onMouseLeave={() => setTimeout(() => setStrategyDropdownOpen(false), 200)}
                >
                  {selectedStrategy}
                  <span style={{ marginLeft: '8px', fontSize: '10px' }}>▼</span>
                </button>
                {strategyDropdownOpen && (
                  <div 
                    className={styles.dropdownMenu}
                    onMouseEnter={() => setStrategyDropdownOpen(true)}
                    onMouseLeave={() => setStrategyDropdownOpen(false)}
                  >
                    {mockStrategies.map(strat => (
                      <button
                        key={strat.id}
                        className={`${styles.dropdownOption} ${selectedStrategy === strat.name ? styles.active : ''}`}
                        onClick={() => {
                          setSelectedStrategy(strat.name);
                          setStrategyDropdownOpen(false);
                        }}
                      >
                        {strat.name}
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