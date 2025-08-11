import React, { useEffect, useRef, useState } from 'react';
import { createChartService } from '../../../services/chartService';
import type { MarketData } from '../../../services/exchanges';
import styles from '../../MonitorPage/MonitorPage.module.css';

interface TradingChartProps {
  marketData: MarketData[];
  currentBar: number;
  isPlaying: boolean;
  symbol: string;
  timeframe: string;
  livePrice: number | null;
  isLoadingData: boolean;
  onCurrentBarChange: (bar: number) => void;
  onSymbolChange?: (symbol: string) => void;
  styles: Record<string, string>;
}

const TradingChart: React.FC<TradingChartProps> = ({
  marketData,
  currentBar,
  isPlaying,
  symbol,
  timeframe,
  livePrice,
  isLoadingData,
  onCurrentBarChange,
  onSymbolChange,
  styles: componentStyles
}) => {
  const chartContainerRef = useRef<HTMLDivElement>(null);
  const chartServiceRef = useRef(createChartService());
  const [symbolDropdownOpen, setSymbolDropdownOpen] = useState(false);
  const dropdownTimeoutRef = useRef<NodeJS.Timeout | null>(null);

  // Initialize chart
  useEffect(() => {
    let retryCount = 0;
    const maxRetries = 5;
    
    const tryInitChart = () => {
      if (!chartServiceRef.current.chart && chartContainerRef.current) {
        const width = chartContainerRef.current.clientWidth;
        const height = chartContainerRef.current.clientHeight;
        
        console.log(`Chart init attempt ${retryCount + 1}:`, { width, height, hasContainer: !!chartContainerRef.current });
        
        if (width > 0 && height > 0) {
          chartServiceRef.current.initChart(chartContainerRef.current);
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
      chartServiceRef.current.destroy();
    };
  }, []);

  // Update chart when market data changes
  useEffect(() => {
    if (!chartServiceRef.current.candleSeries || marketData.length === 0) {
      return;
    }
    
    // Debounce chart updates to prevent excessive re-renders
    const timeoutId = setTimeout(() => {
      if (chartServiceRef.current.candleSeries && marketData.length > 0) {
        console.log(`Updating chart with ${marketData.length} candles`);
        
        if (isPlaying) {
          chartServiceRef.current.updatePlaybackData(marketData, currentBar);
        } else {
          chartServiceRef.current.updateData(marketData);
        }
      }
    }, 100); // 100ms debounce
    
    return () => clearTimeout(timeoutId);
  }, [marketData, currentBar, isPlaying]);

  // Handle window resize
  useEffect(() => {
    const handleResize = () => {
      if (chartServiceRef.current.chart && chartContainerRef.current) {
        chartServiceRef.current.chart.applyOptions({
          width: chartContainerRef.current.clientWidth,
          height: chartContainerRef.current.clientHeight
        });
      }
    };

    window.addEventListener('resize', handleResize);
    return () => window.removeEventListener('resize', handleResize);
  }, []);

  // Handle keyboard navigation
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (!e.shiftKey) return;
      
      if (e.key === 'ArrowLeft') {
        e.preventDefault();
        const newBar = Math.max(1, currentBar - 1);
        onCurrentBarChange(newBar);
      } else if (e.key === 'ArrowRight') {
        e.preventDefault();
        const newBar = Math.min(marketData.length, currentBar + 1);
        onCurrentBarChange(newBar);
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [currentBar, marketData.length, onCurrentBarChange]);

  // Get current bar data for price display
  const currentBarData = marketData[currentBar - 1];

  // Get supported symbols (simplified for now)
  const supportedSymbols = ['BTC/USD', 'ETH/USD', 'SOL/USD'];

  return (
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
                {supportedSymbols.map((sym) => (
                  <button
                    key={sym}
                    className={`${styles.dropdownOption} ${symbol === sym ? styles.active : ''}`}
                    onClick={() => { 
                      if (onSymbolChange) {
                        onSymbolChange(sym);
                      }
                      setSymbolDropdownOpen(false); 
                    }}
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
  );
};

export default TradingChart;