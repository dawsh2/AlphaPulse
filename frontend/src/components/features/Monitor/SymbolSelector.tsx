import React, { useState } from 'react';
import { exchangeManager } from '../../../services/exchanges';
import type { ExchangeType } from '../../../services/exchanges';
import styles from '../../MonitorPage/MonitorPage.module.css';

interface SymbolSelectorProps {
  symbol: string;
  exchange: ExchangeType;
  timeframe: string;
  selectedStrategy: string;
  onSymbolChange: (symbol: string) => void;
  onExchangeChange: (exchange: ExchangeType) => void;
  onTimeframeChange: (timeframe: string) => void;
  onStrategyChange: (strategy: string) => void;
  styles: Record<string, string>;
}

const SymbolSelector: React.FC<SymbolSelectorProps> = ({
  symbol,
  exchange,
  timeframe,
  selectedStrategy,
  onSymbolChange,
  onExchangeChange,
  onTimeframeChange,
  onStrategyChange,
  styles
}) => {
  const [timeframeDropdownOpen, setTimeframeDropdownOpen] = useState(false);
  const [strategyDropdownOpen, setStrategyDropdownOpen] = useState(false);
  const [symbolDropdownOpen, setSymbolDropdownOpen] = useState(false);

  const strategies = ['Mean Reversion v2', 'Momentum Breakout', 'Market Making'];
  const timeframes = ['tick', '1m', '5m', '15m', '1h', '1d'];

  return (
    <>
      {/* Exchange Selector */}
      <div className={styles.controlGroup}>
        <label className={styles.controlLabel}>Exchange:</label>
        <select 
          className="form-input form-input-sm"
          value={exchange}
          onChange={(e) => onExchangeChange(e.target.value as ExchangeType)}
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
          onChange={(e) => onSymbolChange(e.target.value)}
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
              {timeframes.map((tf) => (
                <button
                  key={tf}
                  className={`${styles.dropdownOption} ${timeframe === tf ? styles.active : ''}`}
                  onClick={() => onTimeframeChange(tf)}
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
              {strategies.map((strat) => (
                <button
                  key={strat}
                  className={`${styles.dropdownOption} ${selectedStrategy === strat ? styles.active : ''}`}
                  onClick={() => onStrategyChange(strat)}
                >
                  {strat}
                </button>
              ))}
            </div>
          )}
        </div>
      </div>
    </>
  );
};

export default SymbolSelector;