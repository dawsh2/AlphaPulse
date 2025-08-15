import React, { useState, useEffect, useMemo } from 'react';
import './OptionsChain.css';

interface Option {
  strike: number;
  expiry: string;
  call: {
    bid: number;
    ask: number;
    volume: number;
    openInterest: number;
    iv: number;
    delta: number;
    gamma: number;
    theta: number;
    vega: number;
  };
  put: {
    bid: number;
    ask: number;
    volume: number;
    openInterest: number;
    iv: number;
    delta: number;
    gamma: number;
    theta: number;
    vega: number;
  };
}

interface OptionsChainProps {
  symbol: string;
  onClose: () => void;
}

// Mock option data generator with realistic OI patterns
const generateMockOptions = (symbol: string, stockPrice: number): Option[] => {
  const expiries = ['2024-02-16', '2024-03-15', '2024-04-19', '2024-06-21'];
  const strikes = [];
  
  // Generate strikes around current price
  const baseStrike = Math.round(stockPrice / 5) * 5;
  for (let i = -8; i <= 8; i++) {
    strikes.push(baseStrike + (i * 5));
  }

  const options: Option[] = [];
  
  expiries.forEach(expiry => {
    strikes.forEach(strike => {
      const daysToExpiry = Math.max(1, (new Date(expiry).getTime() - Date.now()) / (1000 * 60 * 60 * 24));
      const timeValue = Math.sqrt(daysToExpiry / 365);
      const moneyness = stockPrice / strike;
      
      // Mock Black-Scholes-ish calculations
      const baseIV = 0.2 + Math.random() * 0.3;
      const callIV = baseIV * (1 + (1 - moneyness) * 0.1);
      const putIV = baseIV * (1 + (moneyness - 1) * 0.1);
      
      // Simplified Greeks calculation
      const callDelta = Math.max(0.01, Math.min(0.99, 0.5 + (moneyness - 1) * 2));
      const putDelta = callDelta - 1;
      const gamma = Math.exp(-Math.pow(Math.log(moneyness), 2) / 2) * 0.1;
      const theta = -0.05 * timeValue;
      const vega = stockPrice * Math.sqrt(timeValue) * 0.01;
      
      const intrinsicCall = Math.max(0, stockPrice - strike);
      const intrinsicPut = Math.max(0, strike - stockPrice);
      
      const callPrice = intrinsicCall + (timeValue * callIV * stockPrice * 0.1);
      const putPrice = intrinsicPut + (timeValue * putIV * stockPrice * 0.1);
      
      // Realistic OI patterns - higher OI near ATM and round strikes
      const distanceFromATM = Math.abs(strike - stockPrice) / stockPrice;
      const roundStrike = strike % 5 === 0 ? 1.5 : (strike % 10 === 0 ? 2.0 : 1.0);
      const oiMultiplier = roundStrike * Math.exp(-distanceFromATM * 5) * (1 + Math.random() * 0.5);
      
      const baseCallOI = Math.floor(1000 + Math.random() * 10000) * oiMultiplier;
      const basePutOI = Math.floor(1000 + Math.random() * 8000) * oiMultiplier;
      
      // Volume is typically 5-20% of OI per day
      const callVolume = Math.floor(baseCallOI * (0.05 + Math.random() * 0.15));
      const putVolume = Math.floor(basePutOI * (0.05 + Math.random() * 0.15));
      
      options.push({
        strike,
        expiry,
        call: {
          bid: Math.max(0.01, callPrice - 0.05),
          ask: callPrice + 0.05,
          volume: callVolume,
          openInterest: Math.floor(baseCallOI),
          iv: callIV,
          delta: callDelta,
          gamma,
          theta,
          vega
        },
        put: {
          bid: Math.max(0.01, putPrice - 0.05),
          ask: putPrice + 0.05,
          volume: putVolume,
          openInterest: Math.floor(basePutOI),
          iv: putIV,
          delta: putDelta,
          gamma,
          theta,
          vega
        }
      });
    });
  });
  
  return options;
};

export const OptionsChain: React.FC<OptionsChainProps> = ({ symbol, onClose }) => {
  const [selectedExpiry, setSelectedExpiry] = useState<string>('2024-03-15');
  const [stockPrice] = useState<number>(150 + Math.random() * 100); // Mock stock price
  const [showGreeks, setShowGreeks] = useState<boolean>(false);

  const options = useMemo(() => {
    return generateMockOptions(symbol, stockPrice);
  }, [symbol, stockPrice]);

  const filteredOptions = useMemo(() => {
    return options
      .filter(opt => opt.expiry === selectedExpiry)
      .sort((a, b) => a.strike - b.strike);
  }, [options, selectedExpiry]);

  const expiries = useMemo(() => {
    return [...new Set(options.map(opt => opt.expiry))].sort();
  }, [options]);

  // Calculate comprehensive chain metrics including OI
  const chainMetrics = useMemo(() => {
    const calls = filteredOptions.map(opt => opt.call);
    const puts = filteredOptions.map(opt => opt.put);
    
    const totalCallOI = calls.reduce((sum, call) => sum + call.openInterest, 0);
    const totalPutOI = puts.reduce((sum, put) => sum + put.openInterest, 0);
    const totalCallVolume = calls.reduce((sum, call) => sum + call.volume, 0);
    const totalPutVolume = puts.reduce((sum, put) => sum + put.volume, 0);
    
    return {
      totalCallVolume,
      totalPutVolume,
      totalCallOI,
      totalPutOI,
      avgCallIV: calls.reduce((sum, call) => sum + call.iv, 0) / calls.length,
      avgPutIV: puts.reduce((sum, put) => sum + put.iv, 0) / puts.length,
      putCallVolumeRatio: totalPutVolume / totalCallVolume,
      putCallOIRatio: totalPutOI / totalCallOI,
      volumeToOIRatio: (totalCallVolume + totalPutVolume) / (totalCallOI + totalPutOI),
      atmStrike: filteredOptions.reduce((closest, opt) => 
        Math.abs(opt.strike - stockPrice) < Math.abs(closest.strike - stockPrice) ? opt : closest,
        filteredOptions[0]
      )?.strike || 0
    };
  }, [filteredOptions, stockPrice]);

  return (
    <div className="options-chain-overlay">
      <div className="options-chain">
        <div className="options-header">
          <div className="title-section">
            <h2>Options Chain - {symbol}</h2>
            <button className="close-btn" onClick={onClose}>×</button>
          </div>
          
          <div className="controls">
            <div className="stock-info">
              <span className="stock-price">Stock: ${stockPrice.toFixed(2)}</span>
              <span className="atm-strike">ATM: ${chainMetrics.atmStrike}</span>
            </div>
            
            <select 
              value={selectedExpiry} 
              onChange={(e) => setSelectedExpiry(e.target.value)}
              className="expiry-selector"
            >
              {expiries.map(expiry => (
                <option key={expiry} value={expiry}>{expiry}</option>
              ))}
            </select>
            
            <button 
              className={`greeks-toggle ${showGreeks ? 'active' : ''}`}
              onClick={() => setShowGreeks(!showGreeks)}
            >
              {showGreeks ? 'Hide Greeks' : 'Show Greeks'}
            </button>
          </div>

          <div className="chain-metrics">
            <span>Call Vol: {chainMetrics.totalCallVolume.toLocaleString()}</span>
            <span>Put Vol: {chainMetrics.totalPutVolume.toLocaleString()}</span>
            <span>Call OI: {chainMetrics.totalCallOI.toLocaleString()}</span>
            <span>Put OI: {chainMetrics.totalPutOI.toLocaleString()}</span>
            <span>P/C Vol: {chainMetrics.putCallVolumeRatio.toFixed(2)}</span>
            <span>P/C OI: {chainMetrics.putCallOIRatio.toFixed(2)}</span>
            <span>Vol/OI: {(chainMetrics.volumeToOIRatio * 100).toFixed(1)}%</span>
            <span>Call IV: {(chainMetrics.avgCallIV * 100).toFixed(1)}%</span>
            <span>Put IV: {(chainMetrics.avgPutIV * 100).toFixed(1)}%</span>
          </div>
        </div>

        <div className="options-table">
          <div className="table-header">
            <div className="calls-header">
              <span>CALLS</span>
            </div>
            <div className="strike-header">
              <span>STRIKE</span>
            </div>
            <div className="puts-header">
              <span>PUTS</span>
            </div>
          </div>

          <div className="options-data">
            {filteredOptions.map((option, idx) => (
              <div key={idx} className={`option-row ${option.strike === chainMetrics.atmStrike ? 'atm' : ''}`}>
                {/* Call side */}
                <div className="call-side">
                  <div className="option-prices">
                    <span className="bid">{option.call.bid.toFixed(2)}</span>
                    <span className="ask">{option.call.ask.toFixed(2)}</span>
                  </div>
                  <div className="option-info">
                    <span className="volume">Vol: {option.call.volume.toLocaleString()}</span>
                    <span className="oi">OI: {option.call.openInterest.toLocaleString()}</span>
                    <span className="iv">IV: {(option.call.iv * 100).toFixed(1)}%</span>
                    <span className="vol-oi-ratio">V/OI: {((option.call.volume / option.call.openInterest) * 100).toFixed(1)}%</span>
                  </div>
                  {showGreeks && (
                    <div className="greeks">
                      <span>Δ: {option.call.delta.toFixed(3)}</span>
                      <span>Γ: {option.call.gamma.toFixed(3)}</span>
                      <span>Θ: {option.call.theta.toFixed(3)}</span>
                      <span>ν: {option.call.vega.toFixed(3)}</span>
                    </div>
                  )}
                </div>

                {/* Strike */}
                <div className="strike">
                  <span className={option.strike < stockPrice ? 'itm' : option.strike === chainMetrics.atmStrike ? 'atm' : 'otm'}>
                    ${option.strike}
                  </span>
                </div>

                {/* Put side */}
                <div className="put-side">
                  <div className="option-prices">
                    <span className="bid">{option.put.bid.toFixed(2)}</span>
                    <span className="ask">{option.put.ask.toFixed(2)}</span>
                  </div>
                  <div className="option-info">
                    <span className="volume">Vol: {option.put.volume.toLocaleString()}</span>
                    <span className="oi">OI: {option.put.openInterest.toLocaleString()}</span>
                    <span className="iv">IV: {(option.put.iv * 100).toFixed(1)}%</span>
                    <span className="vol-oi-ratio">V/OI: {((option.put.volume / option.put.openInterest) * 100).toFixed(1)}%</span>
                  </div>
                  {showGreeks && (
                    <div className="greeks">
                      <span>Δ: {option.put.delta.toFixed(3)}</span>
                      <span>Γ: {option.put.gamma.toFixed(3)}</span>
                      <span>Θ: {option.put.theta.toFixed(3)}</span>
                      <span>ν: {option.put.vega.toFixed(3)}</span>
                    </div>
                  )}
                </div>
              </div>
            ))}
          </div>
        </div>

        <div className="options-footer">
          <div className="legend">
            <span className="legend-item itm">ITM: In The Money</span>
            <span className="legend-item atm">ATM: At The Money</span>
            <span className="legend-item otm">OTM: Out of The Money</span>
          </div>
          <div className="note">
            * Mock data with realistic OI patterns. Real-time OI is the closest to 'L2' for options without paying exchange fees.
            <br />* Vol/OI ratio shows daily turnover. P/C ratios indicate market sentiment.
          </div>
        </div>
      </div>
    </div>
  );
};