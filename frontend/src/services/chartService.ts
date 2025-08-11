import { createChart, IChartApi, ISeriesApi, CandlestickData, UTCTimestamp } from 'lightweight-charts';
import type { MarketData } from './exchanges';

export interface ChartService {
  chart: IChartApi | null;
  candleSeries: ISeriesApi<'Candlestick'> | null;
  initChart: (container: HTMLDivElement) => void;
  updateData: (data: MarketData[]) => void;
  updatePlaybackData: (data: MarketData[], bars: number) => void;
  destroy: () => void;
}

export const createChartService = (): ChartService => {
  let chart: IChartApi | null = null;
  let candleSeries: ISeriesApi<'Candlestick'> | null = null;
  let wheelHandler: ((e: WheelEvent) => void) | null = null;
  let container: HTMLDivElement | null = null;
  let themeObserver: MutationObserver | null = null;

  const initChart = (chartContainer: HTMLDivElement) => {
    if (!chartContainer) {
      console.error('Chart container not found');
      return;
    }

    container = chartContainer;
    const containerWidth = chartContainer.clientWidth;
    const containerHeight = chartContainer.clientHeight;
    
    if (!containerWidth || !containerHeight) {
      console.error('Chart container has no dimensions', { containerWidth, containerHeight });
      return;
    }

    // Detect theme
    const isDark = document.documentElement.getAttribute('data-theme') === 'dark' ||
                   window.matchMedia('(prefers-color-scheme: dark)').matches;

    chart = createChart(chartContainer, {
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
        autoScale: true,
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

    candleSeries = chart.addCandlestickSeries({
      upColor: '#3fb950',
      downColor: '#f85149',
      borderUpColor: '#3fb950',
      borderDownColor: '#f85149',
      wickUpColor: '#3fb950',
      wickDownColor: '#f85149',
    });

    // Add custom wheel handler for Y-axis zoom
    wheelHandler = (e: WheelEvent) => {
      if (!chart) return;
      
      // Check if we're over the price scale area or holding shift
      const rect = chartContainer.getBoundingClientRect();
      if (!rect) return;
      
      const isOverPriceScale = e.clientX > rect.right - 60; // Price scale is ~60px wide
      
      if (e.shiftKey || isOverPriceScale) {
        e.preventDefault();
        e.stopPropagation();
        
        // Use the chart's time scale for zoom functionality
        const timeScale = chart.timeScale();
        const logicalRange = timeScale.getVisibleLogicalRange();
        
        if (logicalRange && candleSeries) {
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
    
    chartContainer.addEventListener('wheel', wheelHandler, { passive: false });
    
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
  };
  
  const updateChartTheme = () => {
    if (!chart) return;
    
    const isDark = document.documentElement.getAttribute('data-theme') === 'dark' ||
                   window.matchMedia('(prefers-color-scheme: dark)').matches;
    
    // Update chart colors based on theme
    chart.applyOptions({
      layout: {
        background: { color: 'transparent' },
        textColor: isDark ? '#f0f6fc' : '#33332d',
      },
      grid: {
        vertLines: { color: isDark ? '#383c45' : '#e5e0d5' },
        horzLines: { color: isDark ? '#383c45' : '#e5e0d5' },
      },
      crosshair: {
        mode: 0,
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
      },
    });
  };

  const updateData = (data: MarketData[]) => {
    if (!candleSeries || data.length === 0) return;
    
    const chartData = data.map(d => ({
      time: d.time as UTCTimestamp,
      open: d.open,
      high: d.high,
      low: d.low,
      close: d.close
    }));
    
    candleSeries.setData(chartData);
  };

  const updatePlaybackData = (data: MarketData[], bars: number) => {
    if (!candleSeries || data.length === 0) return;

    const visibleData = data.slice(0, bars);
    const chartData = visibleData.map(d => ({
      time: d.time as UTCTimestamp,
      open: d.open,
      high: d.high,
      low: d.low,
      close: d.close
    }));
    
    candleSeries.setData(chartData);
    
    // Add markers for signals
    const markers = visibleData
      .filter(d => d.signal)
      .map(d => ({
        time: d.time as UTCTimestamp,
        position: d.signal === 'buy' ? 'belowBar' as const : 'aboveBar' as const,
        color: d.signal === 'buy' ? '#3fb950' : '#f85149',
        shape: d.signal === 'buy' ? 'arrowUp' as const : 'arrowDown' as const,
        text: d.signal!.toUpperCase()
      }));

    candleSeries.setMarkers(markers);
  };

  const destroy = () => {
    if (container && wheelHandler) {
      container.removeEventListener('wheel', wheelHandler);
    }
    if (themeObserver) {
      themeObserver.disconnect();
      themeObserver = null;
    }
    if (chart) {
      chart.remove();
      chart = null;
      candleSeries = null;
    }
  };

  const resize = (width: number, height: number) => {
    if (chart) {
      chart.applyOptions({ width, height });
    }
  };

  return {
    get chart() { return chart; },
    get candleSeries() { return candleSeries; },
    initChart,
    updateData,
    updatePlaybackData,
    destroy
  };
};