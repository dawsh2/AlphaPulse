import React, { useState, useEffect, useMemo } from 'react';
import { DataFlowMonitor } from './components/DataFlowMonitor';
import { OrderbookVisualizer } from './components/OrderbookVisualizer';
import { TradeStream } from './components/TradeStream';
import { PrometheusMetrics } from './components/PrometheusMetrics';
import { SystemStatus } from './components/SystemStatus';
import { WebSocketFirehose } from './components/WebSocketFirehose';
import { TodoList } from './components/TodoList';
import { FileExplorer } from './components/FileExplorer';
import { AlpacaStreaming } from './components/AlpacaStreaming';
import { DeFiArbitrageTable } from './components/DeFiArbitrageTable';
import { useWebSocketFirehose } from './hooks/useWebSocketFirehose';
import './styles/dashboard.css';

type TabType = 'dashboard' | 'alpaca' | 'defi' | 'todos' | 'files' | 'metrics' | 'system';

export function App() {
  const { 
    trades, 
    orderbooks, 
    symbolMappings,
    metrics, 
    status, 
    isConnected 
  } = useWebSocketFirehose('/ws'); // Hash-based WebSocket

  const [selectedSymbol, setSelectedSymbol] = useState('BTC-USD');
  const [selectedExchange, setSelectedExchange] = useState('coinbase');
  const [activeTab, setActiveTab] = useState<TabType>('dashboard');

  // Memoize filtered trades to avoid recalculation on every render  
  const filteredTrades = useMemo(() => 
    trades.filter(t => {
      // Use direct symbol field from WebSocket messages
      return t.symbol === selectedSymbol;
    }),
    [trades, selectedSymbol]
  );

  // Hash-based orderbook lookup (no fallbacks)
  const selectedOrderbook = useMemo(() => {
    // Find the hash that corresponds to our selected symbol
    let targetHash: number | undefined;
    for (const [hash, symbol] of symbolMappings.entries()) {
      if (symbol === selectedSymbol) {
        targetHash = hash;
        break;
      }
    }
    
    // Return the orderbook directly via hash lookup (O(1))
    return targetHash !== undefined ? orderbooks[targetHash] : undefined;
  }, [orderbooks, selectedSymbol, symbolMappings]);

  const renderTabContent = () => {
    switch (activeTab) {
      case 'dashboard':
        return (
          <div className="dashboard-grid">
            {/* Top Row - System Overview */}
            <div className="grid-item span-2">
              <DataFlowMonitor 
                trades={trades}
                orderbooks={orderbooks}
                symbolMappings={symbolMappings}
                metrics={metrics}
              />
            </div>
            <div className="grid-item span-2">
              <SystemStatus status={status} />
            </div>

            {/* Middle Row - Market Data */}
            <div className="grid-item span-3">
              <OrderbookVisualizer 
                orderbook={selectedOrderbook}
                symbol={selectedSymbol}
                exchange={selectedExchange}
              />
            </div>
            <div className="grid-item">
              <TradeStream 
                trades={filteredTrades}
                symbol={selectedSymbol}
                exchange={selectedExchange}
              />
            </div>

            {/* Bottom Row - Metrics and Raw Data */}
            <div className="grid-item span-2">
              <PrometheusMetrics />
            </div>
            <div className="grid-item span-2">
              <WebSocketFirehose 
                trades={trades}
                orderbooks={orderbooks}
              />
            </div>
          </div>
        );
      case 'alpaca':
        return (
          <div className="tab-content alpaca-tab">
            <AlpacaStreaming 
              onSymbolSelect={(symbol) => {
                console.log('Selected symbol:', symbol);
                // Could update some global state or trigger other actions
              }}
            />
          </div>
        );
      case 'defi':
        return (
          <div className="tab-content defi-tab">
            <DeFiArbitrageTable />
          </div>
        );
      case 'todos':
        return null;
      case 'metrics':
        return (
          <div className="tab-content">
            <div className="grid-item span-4">
              <PrometheusMetrics />
            </div>
          </div>
        );
      case 'system':
        return (
          <div className="tab-content">
            <div className="grid-item span-4">
              <SystemStatus status={status} />
            </div>
            <div className="grid-item span-4">
              <WebSocketFirehose 
                trades={trades}
                orderbooks={orderbooks}
              />
            </div>
          </div>
        );
      default:
        return null;
    }
  };

  return (
    <div className="dashboard">
      <header className="dashboard-header">
        <div className="header-left">
          <h1>AlphaPulse Dev Dashboard</h1>
          <div className="status-indicator">
            <span className={`status-dot ${isConnected ? 'connected' : 'disconnected'}`} />
            <span>{isConnected ? 'Connected' : 'Disconnected'}</span>
          </div>
        </div>
        <div className="header-right">
          {activeTab === 'dashboard' && (
            <>
              <select 
                value={selectedExchange} 
                onChange={(e) => setSelectedExchange(e.target.value)}
                className="select-input"
              >
                <option value="coinbase">Coinbase</option>
                <option value="kraken">Kraken</option>
                <option value="binance">Binance US</option>
              </select>
              <select 
                value={selectedSymbol} 
                onChange={(e) => setSelectedSymbol(e.target.value)}
                className="select-input"
              >
                <option value="BTC-USD">BTC-USD</option>
                <option value="ETH-USD">ETH-USD</option>
                <option value="BTC-USDT">BTC-USDT</option>
                <option value="ETH-USDT">ETH-USDT</option>
              </select>
            </>
          )}
        </div>
      </header>

      <nav className="dashboard-tabs">
        <button 
          className={`tab-button ${activeTab === 'dashboard' ? 'active' : ''}`}
          onClick={() => setActiveTab('dashboard')}
        >
          Dashboard
        </button>
        <button 
          className={`tab-button ${activeTab === 'alpaca' ? 'active' : ''}`}
          onClick={() => setActiveTab('alpaca')}
        >
          Alpaca Stocks
        </button>
        <button 
          className={`tab-button ${activeTab === 'defi' ? 'active' : ''}`}
          onClick={() => setActiveTab('defi')}
        >
          DeFi Arbitrage
        </button>
        <button 
          className={`tab-button ${activeTab === 'todos' ? 'active' : ''}`}
          onClick={() => setActiveTab('todos')}
        >
          TODOs
        </button>
        <button 
          className={`tab-button ${activeTab === 'files' ? 'active' : ''}`}
          onClick={() => setActiveTab('files')}
        >
          Files
        </button>
        <button 
          className={`tab-button ${activeTab === 'metrics' ? 'active' : ''}`}
          onClick={() => setActiveTab('metrics')}
        >
          Metrics
        </button>
        <button 
          className={`tab-button ${activeTab === 'system' ? 'active' : ''}`}
          onClick={() => setActiveTab('system')}
        >
          System
        </button>
      </nav>

      <main className="dashboard-content">
        {renderTabContent()}
        <div className={`todo-overlay ${activeTab === 'todos' ? 'active' : ''}`}>
          <TodoList />
        </div>
        <div className={`file-overlay ${activeTab === 'files' ? 'active' : ''}`}>
          <FileExplorer />
        </div>
      </main>
    </div>
  );
}