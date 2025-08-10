# MonitorPage Component

A comprehensive React component for monitoring trading strategies with real-time charts, replay functionality, and strategy performance metrics.

## Features

### Chart Visualization
- **TradingView Lightweight Charts Integration**: Professional-grade candlestick charts
- **Real-time Price Display**: OHLC prices with symbol and timeframe info
- **Trading Signals**: Visual markers for buy/sell signals
- **Responsive Design**: Auto-adjusts to container size

### Replay Mode
- **Playback Controls**: Play/pause, forward/backward navigation
- **Variable Speed**: 1x, 2x, 5x, 10x playback speeds
- **Timeline Scrubbing**: Navigate through historical data
- **Keyboard Controls**: Shift + Arrow keys for frame-by-frame navigation

### Deploy Mode
- **Live Trading View**: Real-time market data display
- **Strategy Monitoring**: Active strategy performance tracking
- **Account Management**: Paper/Live account switching

### Sidebar Panels
- **Metrics Panel**: P&L, Win Rate, Sharpe Ratio, Max Drawdown statistics
- **Events Panel**: Real-time trading event log (buy/sell signals)
- **Strategies Panel**: Strategy list with performance comparison

## Usage

### Installation

First, make sure you have the required dependency:

```bash
npm install lightweight-charts
```

### Basic Usage

```tsx
import MonitorPage from './components/MonitorPage';

function App() {
  return (
    <div>
      <MonitorPage />
    </div>
  );
}
```

### Integration with React Router

```tsx
import { MonitorPage } from './pages/MonitorPage';

// In your router configuration
<Route path="/monitor" element={<MonitorPage />} />
```

## Component Structure

```
MonitorPage/
├── MonitorPage.tsx           # Main component logic
├── MonitorPage.module.css    # Component styles
├── index.ts                  # Export file
└── README.md                # This documentation
```

## State Management

The component manages its own state using React hooks:

- **Mode**: Replay vs Deploy mode
- **Playback**: Play/pause, speed, current position
- **Chart Data**: Market data and trading signals  
- **UI State**: Active sidebar tab, selected strategy

## Mock Data

The component includes mock data generators for:
- **Market Data**: Realistic OHLC candlestick data
- **Trading Events**: Buy/sell signals with timestamps
- **Performance Metrics**: P&L, win rates, and risk metrics
- **Strategy Performance**: Multi-strategy comparison data

## Customization

### Styling
All styles are contained in `MonitorPage.module.css` and use CSS custom properties from the design system. Key customizable elements:

- Color scheme (supports light/dark themes)
- Layout proportions (chart vs sidebar sizing)
- Typography and spacing
- Animation speeds and transitions

### Data Integration
Replace mock data generators with real API calls:

```tsx
// Replace generateMockData() with:
const fetchMarketData = async (symbol: string) => {
  const response = await fetch(`/api/market-data/${symbol}`);
  return response.json();
};
```

### Chart Configuration
The TradingView chart is highly configurable:

```tsx
const chartOptions = {
  layout: { background: { color: 'transparent' } },
  grid: { vertLines: { color: '#e5e0d5' } },
  // Add custom indicators, overlays, etc.
};
```

## Browser Support

- Modern browsers with ES6+ support
- Canvas API support (for charts)
- CSS Grid support (for layout)

## Performance Notes

- Chart data is efficiently managed with incremental updates
- Large datasets are handled with data windowing
- Playback uses requestAnimationFrame for smooth animation
- Component unmounts properly clean up intervals and chart instances

## Dependencies

- `react` (^19.1.0)
- `lightweight-charts` (^4.1.3)
- CSS custom properties support
- Modern browser with Canvas support