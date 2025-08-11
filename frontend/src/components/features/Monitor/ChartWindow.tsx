/**
 * ChartWindow Component
 * Individual chart window with embedded controls and metadata
 */

import React, { useEffect, useRef, useState } from 'react';
import { createChart } from 'lightweight-charts';
import { ChartTileControls } from './ChartTileControls';
import { exchangeManager } from '../../../services/exchanges';
import type { MarketData, ExchangeType } from '../../../services/exchanges';
import styles from '../../MonitorPage/MonitorPage.module.css';

interface ChartConfig {
  symbol: string;
  exchange: ExchangeType;
  timeframe: string;
  marketData: MarketData[];
  isLoadingData: boolean;
  livePrice: number | null;
}

interface ChartWindowProps {
  windowId: string;
  config: ChartConfig;
  onSymbolChange: (symbol: string) => void;
  onExchangeChange: (exchange: ExchangeType) => void;
  onTimeframeChange: (timeframe: string) => void;
  onSplit: (orientation: 'horizontal' | 'vertical') => void;
  onClose?: () => void;
  playbackControls?: {
    isPlaying: boolean;
    currentBar: number;
    playbackSpeed: number;
  };
}

export const ChartWindow: React.FC<ChartWindowProps> = ({
  windowId,
  config,
  onSymbolChange,
  onExchangeChange,
  onTimeframeChange,
  onSplit,
  onClose,
  playbackControls
}) => {
  const chartContainerRef = useRef<HTMLDivElement>(null);
  const chartRef = useRef<any>(null);
  const candleSeriesRef = useRef<any>(null);
  const [symbolDropdownOpen, setSymbolDropdownOpen] = useState(false);
  const [exchangeDropdownOpen, setExchangeDropdownOpen] = useState(false);
  const [timeframeDropdownOpen, setTimeframeDropdownOpen] = useState(false);
  const dropdownTimeoutRef = useRef<NodeJS.Timeout | null>(null);

  const timeframes = ['1m', '5m', '15m', '1h', '4h', '1d'];
  const exchanges = exchangeManager.getAvailableExchanges();
  const symbols = exchangeManager.getSupportedSymbols();

  // Initialize chart
  useEffect(() => {
    if (!chartContainerRef.current || chartRef.current) return;

    const containerWidth = chartContainerRef.current.clientWidth;
    const containerHeight = chartContainerRef.current.clientHeight;
    
    if (!containerWidth || !containerHeight) return;

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
      rightPriceScale: {
        borderColor: isDark ? '#2d3139' : '#e0e3eb',
        textColor: isDark ? '#d1d4dc' : '#131722',
        mode: 0,
        autoScale: true,
        alignLabels: true,
        borderVisible: true,
        scaleMargins: {
          top: 0.1,
          bottom: 0.1
        }
      },
      timeScale: {
        borderColor: isDark ? '#2d3139' : '#e0e3eb',
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

    // Cleanup
    return () => {
      if (chartRef.current) {
        chartRef.current.remove();
        chartRef.current = null;
        candleSeriesRef.current = null;
      }
    };
  }, [windowId]); // Recreate chart if window ID changes

  // Update chart data
  useEffect(() => {
    if (!candleSeriesRef.current || !config.marketData.length) return;

    // Handle playback mode
    if (playbackControls?.isPlaying) {
      const visibleData = config.marketData.slice(0, playbackControls.currentBar);
      candleSeriesRef.current.setData(visibleData as any);
    } else {
      candleSeriesRef.current.setData(config.marketData as any);
    }
  }, [config.marketData, playbackControls]);

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

    // Use ResizeObserver for better performance
    const resizeObserver = new ResizeObserver(handleResize);
    if (chartContainerRef.current) {
      resizeObserver.observe(chartContainerRef.current);
    }

    return () => {
      resizeObserver.disconnect();
    };
  }, []);

  return (
    <div className={styles.chartWindow}>
      {/* Chart Container */}
      <div ref={chartContainerRef} className={styles.chart} />
      
      {/* Loading overlay */}
      {config.isLoadingData && (
        <div className={styles.loadingOverlay}>
          <div className={styles.loadingSpinner}></div>
          <div className={styles.loadingText}>Loading {config.symbol}...</div>
        </div>
      )}

      {/* Chart Info Overlay with embedded selectors */}
      <div className={styles.chartInfoOverlay}>
        {/* Symbol Row with Live Price */}
        <div className={styles.symbolRow}>
          {/* Symbol Selector */}
          <div 
            className={styles.symbolDropdown}
            onMouseEnter={() => {
              if (dropdownTimeoutRef.current) clearTimeout(dropdownTimeoutRef.current);
              setSymbolDropdownOpen(true);
            }}
            onMouseLeave={() => {
              dropdownTimeoutRef.current = setTimeout(() => setSymbolDropdownOpen(false), 200);
            }}
          >
            <button className={styles.symbolButton}>
              {config.symbol}
              <span className={styles.dropdownArrow}>▼</span>
            </button>
            {symbolDropdownOpen && (
              <div className={styles.dropdownMenu}>
                {symbols.map((sym) => (
                  <button
                    key={sym}
                    className={`${styles.dropdownOption} ${config.symbol === sym ? styles.active : ''}`}
                    onClick={() => {
                      onSymbolChange(sym);
                      setSymbolDropdownOpen(false);
                    }}
                  >
                    {sym}
                  </button>
                ))}
              </div>
            )}
          </div>

          {/* Live Price Indicator */}
          {config.livePrice && (
            <span className={styles.liveIndicator}>
              <span className={styles.liveDot}></span>
              ${config.livePrice.toFixed(2)}
            </span>
          )}
        </div>

        {/* Exchange and Timeframe Row */}
        <div className={styles.metadataRow}>
          {/* Exchange Selector */}
          <div 
            className={styles.inlineDropdown}
            onMouseEnter={() => {
              if (dropdownTimeoutRef.current) clearTimeout(dropdownTimeoutRef.current);
              setExchangeDropdownOpen(true);
            }}
            onMouseLeave={() => {
              dropdownTimeoutRef.current = setTimeout(() => setExchangeDropdownOpen(false), 200);
            }}
          >
            <button className={styles.compactButton}>
              {exchanges.find(e => e.value === config.exchange)?.label || config.exchange}
              <span className={styles.dropdownArrow}>▼</span>
            </button>
            {exchangeDropdownOpen && (
              <div className={styles.dropdownMenu}>
                {exchanges.map((ex) => (
                  <button
                    key={ex.value}
                    className={`${styles.dropdownOption} ${config.exchange === ex.value ? styles.active : ''}`}
                    onClick={() => {
                      onExchangeChange(ex.value as ExchangeType);
                      setExchangeDropdownOpen(false);
                    }}
                  >
                    {ex.label}
                  </button>
                ))}
              </div>
            )}
          </div>

          {/* Separator */}
          <span className={styles.separator}>•</span>

          {/* Timeframe Selector */}
          <div 
            className={styles.inlineDropdown}
            onMouseEnter={() => {
              if (dropdownTimeoutRef.current) clearTimeout(dropdownTimeoutRef.current);
              setTimeframeDropdownOpen(true);
            }}
            onMouseLeave={() => {
              dropdownTimeoutRef.current = setTimeout(() => setTimeframeDropdownOpen(false), 200);
            }}
          >
            <button className={styles.compactButton}>
              {config.timeframe}
              <span className={styles.dropdownArrow}>▼</span>
            </button>
            {timeframeDropdownOpen && (
              <div className={styles.dropdownMenu}>
                {timeframes.map((tf) => (
                  <button
                    key={tf}
                    className={`${styles.dropdownOption} ${config.timeframe === tf ? styles.active : ''}`}
                    onClick={() => {
                      onTimeframeChange(tf);
                      setTimeframeDropdownOpen(false);
                    }}
                  >
                    {tf}
                  </button>
                ))}
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Floating Tile Controls */}
      <ChartTileControls
        onSplitHorizontal={() => onSplit('horizontal')}
        onSplitVertical={() => onSplit('vertical')}
        onClose={onClose}
        canClose={!!onClose}
      />
    </div>
  );
};