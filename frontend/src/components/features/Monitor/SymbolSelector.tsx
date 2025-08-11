import React, { useState, useRef } from 'react';
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
  const [exchangeDropdownOpen, setExchangeDropdownOpen] = useState(false);
  
  // Refs for timeout management
  const exchangeTimeoutRef = useRef<NodeJS.Timeout | null>(null);
  const timeframeTimeoutRef = useRef<NodeJS.Timeout | null>(null);
  const strategyTimeoutRef = useRef<NodeJS.Timeout | null>(null);

  const strategies = ['Mean Reversion v2', 'Momentum Breakout', 'Market Making'];
  const timeframes = ['tick', '1m', '5m', '15m', '1h', '1d'];
  const exchanges = exchangeManager.getAvailableExchanges();

  return (
    <>
      {/* Exchange Selector */}
      <div className={styles.controlGroup}>
        <label className={styles.controlLabel}>Exchange:</label>
        <div 
          className={styles.dropdownWrapper}
          onMouseEnter={() => {
            if (exchangeTimeoutRef.current) {
              clearTimeout(exchangeTimeoutRef.current);
              exchangeTimeoutRef.current = null;
            }
            setExchangeDropdownOpen(true);
          }}
          onMouseLeave={() => {
            exchangeTimeoutRef.current = setTimeout(() => {
              setExchangeDropdownOpen(false);
            }, 200);
          }}
        >
          <button className={styles.dropdownButton} style={{ minWidth: '120px' }}>
            {exchanges.find(ex => ex.value === exchange)?.label || exchange}
            <span style={{ marginLeft: '8px' }}>▼</span>
          </button>
          {exchangeDropdownOpen && (
            <div className={styles.dropdownMenu}>
              {exchanges.map((ex) => (
                <button
                  key={ex.value}
                  className={`${styles.dropdownOption} ${exchange === ex.value ? styles.active : ''}`}
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
          onMouseEnter={() => {
            if (timeframeTimeoutRef.current) {
              clearTimeout(timeframeTimeoutRef.current);
              timeframeTimeoutRef.current = null;
            }
            setTimeframeDropdownOpen(true);
          }}
          onMouseLeave={() => {
            timeframeTimeoutRef.current = setTimeout(() => {
              setTimeframeDropdownOpen(false);
            }, 200);
          }}
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

      {/* Strategy Selector */}
      <div className={styles.controlGroup}>
        <label className={styles.controlLabel}>Strategy:</label>
        <div 
          className={styles.dropdownWrapper}
          onMouseEnter={() => {
            if (strategyTimeoutRef.current) {
              clearTimeout(strategyTimeoutRef.current);
              strategyTimeoutRef.current = null;
            }
            setStrategyDropdownOpen(true);
          }}
          onMouseLeave={() => {
            strategyTimeoutRef.current = setTimeout(() => {
              setStrategyDropdownOpen(false);
            }, 200);
          }}
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
                  onClick={() => {
                    onStrategyChange(strat);
                    setStrategyDropdownOpen(false);
                  }}
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