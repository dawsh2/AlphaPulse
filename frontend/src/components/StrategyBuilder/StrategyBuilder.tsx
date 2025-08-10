import React, { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import styles from './StrategyBuilder.module.css';
import { BacktestResults } from './components';

interface StrategyBuilderProps {
  isOpen: boolean;
  onClose: () => void;
  initialTemplate?: string;
}

type LogicType = 'AND' | 'OR' | 'CUSTOM';
type TemplateType = 'trend-following' | 'mean-reversion' | 'breakout' | 'custom';
type ComponentType = 'indicator' | 'strategy';

interface StrategyComponent {
  id: string;
  name: string;
  type: ComponentType;
  category: string;
  parameters?: Record<string, any>;
}

interface ParameterSweep {
  enabled: boolean;
  min: number;
  max: number;
  step: number;
  value: number;
}

const availableComponents: StrategyComponent[] = [
  // Existing Strategies
  { id: 'momentum_breakout', name: 'Momentum Breakout v2', type: 'strategy', category: 'Strategy' },
  { id: 'mean_reversion', name: 'Mean Reversion Pro', type: 'strategy', category: 'Strategy' },
  { id: 'trend_rider', name: 'Trend Rider XL', type: 'strategy', category: 'Strategy' },
  { id: 'volatility_harvest', name: 'Volatility Harvester', type: 'strategy', category: 'Strategy' },
  // Indicators
  { id: 'ema', name: 'EMA (Exponential Moving Average)', type: 'indicator', category: 'Trend' },
  { id: 'sma', name: 'SMA (Simple Moving Average)', type: 'indicator', category: 'Trend' },
  { id: 'rsi', name: 'RSI (Relative Strength Index)', type: 'indicator', category: 'Momentum' },
  { id: 'macd', name: 'MACD', type: 'indicator', category: 'Momentum' },
  { id: 'bb', name: 'Bollinger Bands', type: 'indicator', category: 'Volatility' },
  { id: 'atr', name: 'ATR (Average True Range)', type: 'indicator', category: 'Volatility' },
  { id: 'stoch', name: 'Stochastic Oscillator', type: 'indicator', category: 'Momentum' },
  { id: 'vwap', name: 'VWAP', type: 'indicator', category: 'Volume' },
  { id: 'obv', name: 'OBV (On-Balance Volume)', type: 'indicator', category: 'Volume' },
  { id: 'adx', name: 'ADX (Average Directional Index)', type: 'indicator', category: 'Trend' },
];

export const StrategyBuilder: React.FC<StrategyBuilderProps> = ({ isOpen, onClose, initialTemplate }) => {
  const navigate = useNavigate();
  const [currentStep, setCurrentStep] = useState(initialTemplate ? 2 : 1);
  const [selectedTemplate, setSelectedTemplate] = useState<TemplateType | null>(
    initialTemplate && ['trend-following', 'mean-reversion', 'breakout', 'custom'].includes(initialTemplate) 
      ? initialTemplate as TemplateType 
      : null
  );
  const [selectedComponents, setSelectedComponents] = useState<StrategyComponent[]>([]);
  const [logicType, setLogicType] = useState<LogicType>('AND');
  const [searchQuery, setSearchQuery] = useState('');
  const [showSearchResults, setShowSearchResults] = useState(false);
  const [parameterSweeps, setParameterSweeps] = useState<Record<string, ParameterSweep>>({});
  const [optimizationTarget, setOptimizationTarget] = useState<string>('sharpe');
  const [isRunningBacktest, setIsRunningBacktest] = useState(false);
  const [backtestProgress, setBacktestProgress] = useState(0);

  if (!isOpen) return null;

  const steps = [
    { number: 1, label: 'Template' },
    { number: 2, label: 'Strategy Logic' },
    { number: 3, label: 'Universe' },
    { number: 4, label: 'Backtest' },
    { number: 5, label: 'Results' },
  ];

  const handleTemplateSelect = (template: TemplateType) => {
    setSelectedTemplate(template);
    // Pre-populate components based on template
    if (template === 'trend-following') {
      setSelectedComponents([
        availableComponents.find(i => i.id === 'ema')!,
        availableComponents.find(i => i.id === 'macd')!,
      ]);
    } else if (template === 'mean-reversion') {
      setSelectedComponents([
        availableComponents.find(i => i.id === 'rsi')!,
        availableComponents.find(i => i.id === 'bb')!,
      ]);
    } else if (template === 'breakout') {
      setSelectedComponents([
        availableComponents.find(i => i.id === 'atr')!,
        availableComponents.find(i => i.id === 'adx')!,
      ]);
    }
    setCurrentStep(2);
  };

  // Initialize template and components based on initialTemplate prop
  useEffect(() => {
    if (initialTemplate) {
      if (['trend-following', 'mean-reversion', 'breakout', 'custom'].includes(initialTemplate)) {
        handleTemplateSelect(initialTemplate as TemplateType);
      }
    }
  }, [initialTemplate]);

  const handleIndicatorSearch = (query: string) => {
    setSearchQuery(query);
    setShowSearchResults(query.length > 0);
  };

  const filteredComponents = availableComponents.filter(component =>
    component.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
    component.id.toLowerCase().includes(searchQuery.toLowerCase()) ||
    component.category.toLowerCase().includes(searchQuery.toLowerCase())
  );

  const addComponent = (component: StrategyComponent) => {
    if (!selectedComponents.find(c => c.id === component.id)) {
      setSelectedComponents([...selectedComponents, component]);
    }
    setSearchQuery('');
    setShowSearchResults(false);
  };

  const removeComponent = (componentId: string) => {
    setSelectedComponents(selectedComponents.filter(c => c.id !== componentId));
  };

  const clearCanvas = () => {
    setSelectedComponents([]);
    setParameterSweeps({});
  };
  
  const calculateCombinations = () => {
    let combinations = 1;
    Object.values(parameterSweeps).forEach(sweep => {
      if (sweep?.enabled) {
        const count = Math.floor((sweep.max - sweep.min) / sweep.step) + 1;
        combinations *= count;
      }
    });
    return combinations;
  };

  const canGoToStep = (step: number) => {
    // Always allow going backward
    if (step < currentStep) return true;
    
    // Step 1 (Template): Always accessible
    if (step === 1) return true;
    
    // Step 2 (Strategy Logic): Need a template selected
    if (step === 2) return selectedTemplate !== null;
    
    // Step 3 (Universe): Need at least one component
    if (step === 3) return selectedComponents.length > 0;
    
    // Step 4 (Backtest): Can always access if reached step 3
    if (step === 4) return currentStep >= 3;
    
    // Step 5 (Results): Only after running backtest
    if (step === 5) return currentStep >= 5;
    
    return false;
  };

  const goToStep = (step: number) => {
    if (canGoToStep(step)) {
      setCurrentStep(step);
    }
  };

  const runBacktest = async () => {
    setIsRunningBacktest(true);
    setBacktestProgress(0);
    
    // Simulate backtest progress
    const hasOptimization = Object.values(parameterSweeps).some(s => s?.enabled);
    const totalSteps = hasOptimization ? calculateCombinations() : 10;
    const stepDuration = hasOptimization ? 50 : 200; // Optimization runs faster per step
    
    for (let i = 0; i <= totalSteps; i++) {
      await new Promise(resolve => setTimeout(resolve, stepDuration));
      setBacktestProgress((i / totalSteps) * 100);
    }
    
    // Complete and go to results
    setIsRunningBacktest(false);
    setCurrentStep(5);
  };

  const renderStepContent = () => {
    switch (currentStep) {
      case 1:
        return (
          <div className={styles.stepPanel}>
            <h2 className={styles.stepTitle}>Choose a Strategy Template</h2>
            <p className={styles.stepDescription}>Select a pre-built template or start from scratch</p>
            
            <div className={styles.templateOptions}>
              <button 
                className={`${styles.templateBtn} ${selectedTemplate === 'trend-following' ? styles.selected : ''}`}
                onClick={() => handleTemplateSelect('trend-following')}
              >
                <span className={styles.templateIcon}>📈</span>
                <div className={styles.templateText}>
                  <span className={styles.templateName}>Trend Following</span>
                  <span className={styles.templateDesc}>EMA crosses & momentum indicators</span>
                </div>
              </button>
              
              <button 
                className={`${styles.templateBtn} ${selectedTemplate === 'mean-reversion' ? styles.selected : ''}`}
                onClick={() => handleTemplateSelect('mean-reversion')}
              >
                <span className={styles.templateIcon}>🎯</span>
                <div className={styles.templateText}>
                  <span className={styles.templateName}>Mean Reversion</span>
                  <span className={styles.templateDesc}>RSI & Bollinger Bands</span>
                </div>
              </button>
              
              <button 
                className={`${styles.templateBtn} ${selectedTemplate === 'breakout' ? styles.selected : ''}`}
                onClick={() => handleTemplateSelect('breakout')}
              >
                <span className={styles.templateIcon}>💥</span>
                <div className={styles.templateText}>
                  <span className={styles.templateName}>Breakout Strategy</span>
                  <span className={styles.templateDesc}>Channel breakouts & volatility</span>
                </div>
              </button>
              
              <button 
                className={`${styles.templateBtn} ${selectedTemplate === 'custom' ? styles.selected : ''}`}
                onClick={() => handleTemplateSelect('custom')}
              >
                <span className={styles.templateIcon}>🔧</span>
                <div className={styles.templateText}>
                  <span className={styles.templateName}>Custom Strategy</span>
                  <span className={styles.templateDesc}>Build from scratch</span>
                </div>
              </button>
            </div>
            
            <div className={styles.actionsBar}>
              <div className={styles.leftActions}>
                <button className={styles.btn} onClick={onClose}>← Cancel</button>
              </div>
            </div>
          </div>
        );

      case 2:
        return (
          <div className={styles.stepPanel}>
            <h2 className={styles.stepTitle}>Configure Strategy & Parameters</h2>
            
            <div className={styles.strategyCanvas}>
              {selectedComponents.length === 0 ? (
                <div className={styles.canvasPlaceholder}>
                  <div>Add strategies or indicators</div>
                  <div>Search in the sidebar to find components</div>
                </div>
              ) : (
                <div className={styles.componentList}>
                  {selectedComponents.map(component => (
                    <div key={component.id} className={`${styles.componentCard} ${component.type === 'strategy' ? styles.strategyCard : styles.indicatorCard}`}>
                      <div className={styles.componentHeader}>
                        <div>
                          <span className={styles.componentType}>{component.type === 'strategy' ? '📊' : '📈'}</span>
                          <span>{component.name}</span>
                        </div>
                        <button 
                          className={styles.removeBtn}
                          onClick={() => removeComponent(component.id)}
                        >
                          ×
                        </button>
                      </div>
                      <div className={styles.componentParams}>
                        {component.type === 'strategy' ? (
                          <div className={styles.strategyConfig}>
                            <label>
                              Weight: <input type="number" defaultValue="1.0" step="0.1" min="0" max="2" className={styles.paramInput} />
                            </label>
                            <label>
                              Active: <input type="checkbox" defaultChecked className={styles.paramCheckbox} />
                            </label>
                          </div>
                        ) : (
                          <>
                        {component.id === 'ema' && (
                          <div className={styles.parameterControl}>
                            <div className={styles.parameterRow}>
                              <label className={styles.paramLabel}>Period</label>
                              <label className={styles.sweepToggle}>
                                <input 
                                  type="checkbox" 
                                  onChange={(e) => {
                                    const key = `${component.id}_period`;
                                    setParameterSweeps(prev => ({
                                      ...prev,
                                      [key]: {
                                        ...prev[key],
                                        enabled: e.target.checked,
                                        min: 10,
                                        max: 30,
                                        step: 5,
                                        value: 20
                                      }
                                    }));
                                  }}
                                />
                                <span className={styles.sweepLabel}>Sweep</span>
                              </label>
                            </div>
                            {parameterSweeps[`${component.id}_period`]?.enabled ? (
                              <div className={styles.sweepInputs}>
                                <input type="number" defaultValue="10" className={styles.sweepInput} placeholder="Min" />
                                <span className={styles.sweepTo}>to</span>
                                <input type="number" defaultValue="30" className={styles.sweepInput} placeholder="Max" />
                                <span className={styles.sweepStep}>step</span>
                                <input type="number" defaultValue="5" className={styles.sweepInput} placeholder="Step" />
                              </div>
                            ) : (
                              <input type="number" defaultValue="20" className={styles.paramInput} />
                            )}
                          </div>
                        )}
                        {component.id === 'rsi' && (
                          <>
                            <div className={styles.parameterControl}>
                              <div className={styles.parameterRow}>
                                <label className={styles.paramLabel}>Period</label>
                                <label className={styles.sweepToggle}>
                                  <input 
                                    type="checkbox" 
                                    onChange={(e) => {
                                      const key = `${component.id}_period`;
                                      setParameterSweeps(prev => ({
                                        ...prev,
                                        [key]: {
                                          ...prev[key],
                                          enabled: e.target.checked,
                                          min: 10,
                                          max: 20,
                                          step: 1,
                                          value: 14
                                        }
                                      }));
                                    }}
                                  />
                                  <span className={styles.sweepLabel}>Sweep</span>
                                </label>
                              </div>
                              {parameterSweeps[`${component.id}_period`]?.enabled ? (
                                <div className={styles.sweepInputs}>
                                  <input type="number" defaultValue="10" className={styles.sweepInput} placeholder="Min" />
                                  <span className={styles.sweepTo}>to</span>
                                  <input type="number" defaultValue="20" className={styles.sweepInput} placeholder="Max" />
                                  <span className={styles.sweepStep}>step</span>
                                  <input type="number" defaultValue="1" className={styles.sweepInput} placeholder="Step" />
                                </div>
                              ) : (
                                <input type="number" defaultValue="14" className={styles.paramInput} />
                              )}
                            </div>
                            <div className={styles.parameterControl}>
                              <div className={styles.parameterRow}>
                                <label className={styles.paramLabel}>Overbought</label>
                                <label className={styles.sweepToggle}>
                                  <input type="checkbox" />
                                  <span className={styles.sweepLabel}>Sweep</span>
                                </label>
                              </div>
                              <input type="number" defaultValue="70" className={styles.paramInput} />
                            </div>
                            <div className={styles.parameterControl}>
                              <div className={styles.parameterRow}>
                                <label className={styles.paramLabel}>Oversold</label>
                                <label className={styles.sweepToggle}>
                                  <input 
                                    type="checkbox" 
                                    onChange={(e) => {
                                      const key = `${component.id}_oversold`;
                                      setParameterSweeps(prev => ({
                                        ...prev,
                                        [key]: {
                                          ...prev[key],
                                          enabled: e.target.checked,
                                          min: 25,
                                          max: 35,
                                          step: 5,
                                          value: 30
                                        }
                                      }));
                                    }}
                                  />
                                  <span className={styles.sweepLabel}>Sweep</span>
                                </label>
                              </div>
                              {parameterSweeps[`${component.id}_oversold`]?.enabled ? (
                                <div className={styles.sweepInputs}>
                                  <input type="number" defaultValue="25" className={styles.sweepInput} placeholder="Min" />
                                  <span className={styles.sweepTo}>to</span>
                                  <input type="number" defaultValue="35" className={styles.sweepInput} placeholder="Max" />
                                  <span className={styles.sweepStep}>step</span>
                                  <input type="number" defaultValue="5" className={styles.sweepInput} placeholder="Step" />
                                </div>
                              ) : (
                                <input type="number" defaultValue="30" className={styles.paramInput} />
                              )}
                            </div>
                          </>
                        )}
                        {component.id === 'bb' && (
                          <>
                            <label>
                              Period: <input type="number" defaultValue="20" className={styles.paramInput} />
                            </label>
                            <label>
                              Std Dev: <input type="number" defaultValue="2" step="0.1" className={styles.paramInput} />
                            </label>
                          </>
                        )}
                        {component.id === 'macd' && (
                          <>
                            <label>
                              Fast: <input type="number" defaultValue="12" className={styles.paramInput} />
                            </label>
                            <label>
                              Slow: <input type="number" defaultValue="26" className={styles.paramInput} />
                            </label>
                            <label>
                              Signal: <input type="number" defaultValue="9" className={styles.paramInput} />
                            </label>
                          </>
                        )}
                          </>
                        )}
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </div>
            
            <div className={styles.logicBuilder}>
              <h3>Signal Combination Logic</h3>
              <div className={styles.logicOptions}>
                <button 
                  className={`${styles.logicOption} ${logicType === 'AND' ? styles.active : ''}`}
                  onClick={() => setLogicType('AND')}
                >
                  ALL signals must be true (AND)
                </button>
                <button 
                  className={`${styles.logicOption} ${logicType === 'OR' ? styles.active : ''}`}
                  onClick={() => setLogicType('OR')}
                >
                  ANY signal can be true (OR)
                </button>
                <button 
                  className={`${styles.logicOption} ${logicType === 'CUSTOM' ? styles.active : ''}`}
                  onClick={() => setLogicType('CUSTOM')}
                >
                  Custom logic builder
                </button>
              </div>
            </div>
            
            {/* Parameter Combinations Counter */}
            {Object.values(parameterSweeps).some(s => s?.enabled) && (
              <div className={styles.combinationsCounter}>
                <span className={styles.combinationsIcon}>🔬</span>
                <span className={styles.combinationsText}>
                  Testing {calculateCombinations()} parameter combinations
                </span>
              </div>
            )}
            
            <div className={styles.actionsBar}>
              <div className={styles.rightActions}>
                <button className={styles.btn} onClick={clearCanvas}>Clear All</button>
                <button className={styles.btn} onClick={() => goToStep(1)}>← Back</button>
                <button className={styles.btnPrimary} onClick={() => goToStep(3)}>Universe →</button>
              </div>
            </div>
          </div>
        );

      case 3:
        return (
          <div className={styles.stepPanel}>
            <h2 className={styles.stepTitle}>Select Trading Universe</h2>
            <div className={styles.universeGrid}>
              <div className={styles.universeSection}>
                <h3 className={styles.universeSectionTitle}>📈 Stocks</h3>
                <div className={styles.universeOptions}>
                  <label className={styles.checkboxLabel}>
                    <input type="checkbox" defaultChecked /> S&P 500 (503 stocks)
                  </label>
                  <label className={styles.checkboxLabel}>
                    <input type="checkbox" /> NASDAQ 100 (102 stocks)
                  </label>
                  <label className={styles.checkboxLabel}>
                    <input type="checkbox" /> Russell 2000 (2000 stocks)
                  </label>
                  <label className={styles.checkboxLabel}>
                    <input type="checkbox" /> Dow Jones (30 stocks)
                  </label>
                </div>
              </div>
              
              <div className={styles.universeSection}>
                <h3 className={styles.universeSectionTitle}>₿ Crypto</h3>
                <div className={styles.universeOptions}>
                  <label className={styles.checkboxLabel}>
                    <input type="checkbox" /> Top 10 by Market Cap
                  </label>
                  <label className={styles.checkboxLabel}>
                    <input type="checkbox" /> Top 50 by Market Cap
                  </label>
                  <label className={styles.checkboxLabel}>
                    <input type="checkbox" /> BTC/ETH Majors
                  </label>
                  <label className={styles.checkboxLabel}>
                    <input type="checkbox" /> DeFi Tokens
                  </label>
                </div>
              </div>
              
              <div className={styles.universeSection}>
                <h3 className={styles.universeSectionTitle}>💱 Forex</h3>
                <div className={styles.universeOptions}>
                  <label className={styles.checkboxLabel}>
                    <input type="checkbox" /> Major Pairs (8 pairs)
                  </label>
                  <label className={styles.checkboxLabel}>
                    <input type="checkbox" /> Minor Pairs (21 pairs)
                  </label>
                  <label className={styles.checkboxLabel}>
                    <input type="checkbox" /> Exotic Pairs (26 pairs)
                  </label>
                  <label className={styles.checkboxLabel}>
                    <input type="checkbox" /> USD Index Focus
                  </label>
                </div>
              </div>
              
              <div className={styles.universeSection}>
                <h3 className={styles.universeSectionTitle}>🥇 Commodities</h3>
                <div className={styles.universeOptions}>
                  <label className={styles.checkboxLabel}>
                    <input type="checkbox" /> Precious Metals (Gold, Silver)
                  </label>
                  <label className={styles.checkboxLabel}>
                    <input type="checkbox" /> Energy (Oil, Gas, Coal)
                  </label>
                  <label className={styles.checkboxLabel}>
                    <input type="checkbox" /> Agricultural (Corn, Wheat, Soy)
                  </label>
                  <label className={styles.checkboxLabel}>
                    <input type="checkbox" /> Industrial Metals (Copper)
                  </label>
                </div>
              </div>
            </div>
            
            <div className={styles.customSymbols}>
              <h3 className={styles.universeSectionTitle}>🎯 Custom Symbols</h3>
              <div className={styles.symbolInput}>
                <input 
                  type="text" 
                  placeholder="Enter symbols separated by commas (e.g., AAPL, MSFT, TSLA)"
                  className={styles.symbolInputField}
                />
                <button className={styles.addSymbolsBtn}>Add Symbols</button>
              </div>
            </div>
            
            <div className={styles.actionsBar}>
              <div className={styles.rightActions}>
                <button className={styles.btn} onClick={() => goToStep(2)}>← Back</button>
                <button className={styles.btnPrimary} onClick={() => goToStep(4)}>Backtest →</button>
              </div>
            </div>
          </div>
        );

      case 4:
        return (
          <div className={styles.stepPanel}>
            <h2 className={styles.stepTitle}>Backtest & Risk Management</h2>
            
            <div className={styles.configSections}>
              {/* Backtest Configuration */}
              <div className={styles.configSection}>
                <h3 className={styles.configSectionTitle}>📅 Backtest Period</h3>
                <div className={styles.backtestForm}>
                  <label>
                    Start Date
                    <input type="date" defaultValue="2022-01-01" className={styles.configInput} />
                  </label>
                  <label>
                    End Date
                    <input type="date" defaultValue="2024-01-01" className={styles.configInput} />
                  </label>
                  <label>
                    Initial Capital
                    <input type="number" defaultValue="100000" className={styles.configInput} />
                  </label>
                </div>
              </div>
              
              {/* Position Sizing */}
              <div className={styles.configSection}>
                <h3 className={styles.configSectionTitle}>💰 Position Sizing</h3>
                <div className={styles.positionSizingOptions}>
                  <label className={styles.radioLabel}>
                    <input type="radio" name="positionSizing" defaultChecked />
                    <span>Fixed Percentage: <input type="number" defaultValue="10" min="1" max="100" className={styles.inlineInput} />% of capital</span>
                  </label>
                  <label className={styles.radioLabel}>
                    <input type="radio" name="positionSizing" />
                    <span>Fixed Dollar Amount: $<input type="number" defaultValue="10000" className={styles.inlineInput} /> per position</span>
                  </label>
                  <label className={styles.radioLabel}>
                    <input type="radio" name="positionSizing" />
                    <span>Kelly Criterion (Risk-optimized)</span>
                  </label>
                  <label className={styles.radioLabel}>
                    <input type="radio" name="positionSizing" />
                    <span>ATR-based (Volatility-adjusted)</span>
                  </label>
                </div>
              </div>
              
              {/* Risk Management */}
              <div className={styles.configSection}>
                <h3 className={styles.configSectionTitle}>🛡️ Risk Management</h3>
                <div className={styles.riskManagementOptions}>
                  <div className={styles.riskRow}>
                    <label className={styles.checkboxLabel}>
                      <input type="checkbox" defaultChecked />
                      Stop Loss: <input type="number" defaultValue="2" step="0.1" className={styles.inlineInput} />% below entry
                    </label>
                  </div>
                  <div className={styles.riskRow}>
                    <label className={styles.checkboxLabel}>
                      <input type="checkbox" defaultChecked />
                      Take Profit: <input type="number" defaultValue="6" step="0.1" className={styles.inlineInput} />% above entry
                    </label>
                  </div>
                  <div className={styles.riskRow}>
                    <label className={styles.checkboxLabel}>
                      <input type="checkbox" />
                      Max Portfolio Risk: <input type="number" defaultValue="20" className={styles.inlineInput} />% total exposure
                    </label>
                  </div>
                  <div className={styles.riskRow}>
                    <label className={styles.checkboxLabel}>
                      <input type="checkbox" />
                      Max Positions: <input type="number" defaultValue="10" className={styles.inlineInput} /> concurrent trades
                    </label>
                  </div>
                  <div className={styles.riskRow}>
                    <label className={styles.checkboxLabel}>
                      <input type="checkbox" />
                      Trailing Stop: <input type="number" defaultValue="1" step="0.1" className={styles.inlineInput} />% trailing distance
                    </label>
                  </div>
                </div>
              </div>
              
              {/* Trading Costs */}
              <div className={styles.configSection}>
                <h3 className={styles.configSectionTitle}>💸 Trading Costs</h3>
                <div className={styles.tradingCostsForm}>
                  <label>
                    Commission per Trade
                    <input type="number" defaultValue="0.005" step="0.001" className={styles.configInput} />
                  </label>
                  <label>
                    Slippage (%)
                    <input type="number" defaultValue="0.05" step="0.01" className={styles.configInput} />
                  </label>
                </div>
              </div>
            </div>
            
            {/* Optimization Settings */}
            {Object.values(parameterSweeps).some(s => s?.enabled) && (
              <div className={styles.optimizationSettings}>
                <h3 className={styles.optimizationTitle}>🎯 Optimization Settings</h3>
                <div className={styles.optimizationForm}>
                  <label>
                    Optimization Target
                    <select 
                      value={optimizationTarget} 
                      onChange={(e) => setOptimizationTarget(e.target.value)}
                      className={styles.optimizationSelect}
                    >
                      <option value="sharpe">Sharpe Ratio</option>
                      <option value="returns">Total Returns</option>
                      <option value="winrate">Win Rate</option>
                      <option value="calmar">Calmar Ratio</option>
                    </select>
                  </label>
                  <label>
                    Max Iterations
                    <input type="number" defaultValue={calculateCombinations()} disabled className={styles.iterationsInput} />
                  </label>
                  <label className={styles.checkboxLabel}>
                    <input type="checkbox" defaultChecked />
                    Walk-Forward Analysis
                  </label>
                </div>
                <div className={styles.combinationsInfo}>
                  <span className={styles.infoIcon}>ℹ️</span>
                  <span>Will test {calculateCombinations()} parameter combinations to find optimal {optimizationTarget === 'sharpe' ? 'Sharpe Ratio' : optimizationTarget === 'returns' ? 'Returns' : optimizationTarget === 'winrate' ? 'Win Rate' : 'Calmar Ratio'}</span>
                </div>
              </div>
            )}
            <div className={styles.actionsBar}>
              <div className={styles.rightActions}>
                <button className={styles.btn} onClick={() => goToStep(3)} disabled={isRunningBacktest}>← Back</button>
                {isRunningBacktest ? (
                  <div className={styles.backtestProgress}>
                    <div className={styles.progressInfo}>
                      <span className={styles.progressIcon}>⚡</span>
                      <span className={styles.progressText}>
                        {Object.values(parameterSweeps).some(s => s?.enabled) 
                          ? `Optimizing... ${Math.round(backtestProgress)}%`
                          : `Backtesting... ${Math.round(backtestProgress)}%`
                        }
                      </span>
                    </div>
                    <div className={styles.progressBar}>
                      <div 
                        className={styles.progressFill} 
                        style={{ width: `${backtestProgress}%` }}
                      />
                    </div>
                  </div>
                ) : (
                  <button className={styles.btnPrimary} onClick={runBacktest}>
                    {Object.values(parameterSweeps).some(s => s?.enabled) ? '🎯 Optimize Strategy' : '🚀 Run Backtest'} →
                  </button>
                )}
              </div>
            </div>
          </div>
        );

      case 5:
        const hasOptimization = Object.values(parameterSweeps).some(s => s?.enabled);
        return (
          <BacktestResults
            metrics={{
              sharpeRatio: hasOptimization ? 2.35 : 1.82,
              annualReturn: hasOptimization ? 31.2 : 24.5,
              maxDrawdown: hasOptimization ? -6.7 : -8.3,
              winRate: hasOptimization ? 72 : 68,
              totalTrades: 247,
              profitFactor: 2.1
            }}
            isOptimization={hasOptimization}
            optimalParams={hasOptimization ? {
              'RSI Period': 14,
              'Oversold': 30
            } : {}}
            testedCombinations={hasOptimization ? calculateCombinations() : 0}
            onDeploy={() => {
              onClose();
              navigate('/monitor');
            }}
            onClose={onClose}
            onOpenNotebook={() => {
              onClose();
              navigate('/research');
            }}
          />
        );

      default:
        return null;
    }
  };

  return (
    <div className={styles.builderContainer}>      
      <div className={styles.builderContent}>
        {/* Progress Steps */}
        <div className={styles.progressSteps}>
          {steps.map(step => (
            <div 
              key={step.number}
              className={`${styles.step} ${currentStep === step.number ? styles.active : ''} ${currentStep > step.number ? styles.completed : ''} ${!canGoToStep(step.number) ? styles.disabled : ''}`}
              onClick={() => goToStep(step.number)}
            >
              <div className={styles.stepLabel}>{step.label}</div>
              {currentStep > step.number && <span className={styles.stepCheck}>✓</span>}
            </div>
          ))}
        </div>
        
        {/* Main Content Grid */}
        <div className={styles.contentGrid}>
          {/* Sidebar - Only show on step 2 */}
          {currentStep === 2 && (
            <aside className={styles.sidebar}>
              <h3>Strategy Components</h3>
              
              <div className={styles.searchContainer}>
                <input 
                  type="text" 
                  placeholder="Search strategies or indicators..." 
                  value={searchQuery}
                  onChange={(e) => handleIndicatorSearch(e.target.value)}
                  onFocus={() => setShowSearchResults(searchQuery.length > 0)}
                />
                {showSearchResults && (
                  <div className={styles.searchResults}>
                    {filteredComponents.map(component => (
                      <div 
                        key={component.id}
                        className={styles.searchResult}
                        onClick={() => addComponent(component)}
                      >
                        <span className={styles.componentName}>
                          {component.type === 'strategy' ? '📊 ' : '📈 '}
                          {component.name}
                        </span>
                        <span className={`${styles.componentCategory} ${component.type === 'strategy' ? styles.strategyTag : ''}`}>
                          {component.category}
                        </span>
                      </div>
                    ))}
                  </div>
                )}
              </div>
              
              <div className={styles.hint}>
                Try: 'momentum', 'rsi', 'trend', 'strategy'
              </div>
            </aside>
          )}
          
          {/* Main Panel */}
          <main className={`${styles.mainPanel} ${(currentStep !== 2 && currentStep !== 5) ? styles.fullWidth : ''} ${currentStep === 5 ? styles.resultsView : ''}`}>
            {renderStepContent()}
          </main>
        </div>
      </div>
    </div>
  );
};