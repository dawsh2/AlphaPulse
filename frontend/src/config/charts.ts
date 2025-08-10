/**
 * Chart configuration
 */

export const chartConfig = {
  // Default chart settings
  defaults: {
    theme: 'light',
    timeframe: '1h',
    chartType: 'candlestick',
    indicators: ['volume'],
  },
  
  // TradingView Lightweight Charts options
  lightweightCharts: {
    layout: {
      backgroundColor: 'transparent',
      textColor: '#33332d',
      fontSize: 12,
    },
    grid: {
      vertLines: {
        color: 'rgba(70, 70, 70, 0.1)',
      },
      horzLines: {
        color: 'rgba(70, 70, 70, 0.1)',
      },
    },
    crosshair: {
      mode: 1, // Magnet mode
      vertLine: {
        width: 1,
        color: 'rgba(70, 70, 70, 0.3)',
        style: 0,
      },
      horzLine: {
        width: 1,
        color: 'rgba(70, 70, 70, 0.3)',
        style: 0,
      },
    },
    priceScale: {
      borderColor: 'rgba(70, 70, 70, 0.2)',
    },
    timeScale: {
      borderColor: 'rgba(70, 70, 70, 0.2)',
      timeVisible: true,
      secondsVisible: false,
    },
  },
  
  // Candlestick colors
  candlestick: {
    upColor: '#26a69a',
    downColor: '#ef5350',
    borderVisible: false,
    wickUpColor: '#26a69a',
    wickDownColor: '#ef5350',
  },
  
  // Volume histogram colors
  volume: {
    upColor: 'rgba(38, 166, 154, 0.3)',
    downColor: 'rgba(239, 83, 80, 0.3)',
  },
  
  // Line series colors for indicators
  indicators: {
    sma: { color: '#2962FF', lineWidth: 2 },
    ema: { color: '#FF6D00', lineWidth: 2 },
    bb_upper: { color: 'rgba(41, 98, 255, 0.5)', lineWidth: 1 },
    bb_middle: { color: 'rgba(41, 98, 255, 0.8)', lineWidth: 1 },
    bb_lower: { color: 'rgba(41, 98, 255, 0.5)', lineWidth: 1 },
    rsi: { color: '#9C27B0', lineWidth: 2 },
    macd: { color: '#2196F3', lineWidth: 2 },
    signal: { color: '#FF9800', lineWidth: 2 },
  },
  
  // Chart types
  chartTypes: [
    { id: 'candlestick', label: 'Candlestick' },
    { id: 'line', label: 'Line' },
    { id: 'area', label: 'Area' },
    { id: 'bar', label: 'Bar' },
    { id: 'heikinashi', label: 'Heikin Ashi' },
  ],
  
  // Drawing tools
  drawingTools: [
    { id: 'trendline', label: 'Trend Line', icon: '📈' },
    { id: 'horizontal', label: 'Horizontal Line', icon: '➖' },
    { id: 'vertical', label: 'Vertical Line', icon: '│' },
    { id: 'rectangle', label: 'Rectangle', icon: '▭' },
    { id: 'fib', label: 'Fibonacci', icon: '🔢' },
    { id: 'text', label: 'Text', icon: '📝' },
  ],
} as const;