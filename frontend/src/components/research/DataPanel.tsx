/**
 * DataPanel Component - Displays market data and analysis tools
 * Extracted from ResearchPage for better separation of concerns
 */
import React, { useState, useEffect } from 'react';
import { useMarketData, useCorrelation } from '../../hooks/useMarketData';
import { analysisService } from '../../services/api/analysisService';
import { dataStorage } from '../../services/data';
import type { DatasetInfo } from '../../services/data';
import styles from '../../pages/ResearchPage.module.css';

interface DataPanelProps {
  collapsedCategories: Set<string>;
  toggleCategory: (category: string) => void;
}

export const DataPanel: React.FC<DataPanelProps> = ({
  collapsedCategories,
  toggleCategory,
}) => {
  const { datasets, loading: loadingDatasets, error, refreshDatasets } = useMarketData();
  const { correlation, getCorrelation, loading: correlationLoading } = useCorrelation();
  
  // Local state for analysis
  const [selectedSymbols, setSelectedSymbols] = useState<string[]>([]);
  const [analysisResults, setAnalysisResults] = useState<any>(null);
  const [analysisLoading, setAnalysisLoading] = useState(false);

  // Handle dataset export
  const handleDatasetExport = async (dataset: any) => {
    try {
      const json = await dataStorage.exportToJSON({
        symbol: dataset.symbol,
        exchange: dataset.exchange,
        interval: dataset.interval
      });
      
      const blob = new Blob([json], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `${dataset.symbol}_${dataset.exchange}_${dataset.interval}.json`;
      a.click();
      URL.revokeObjectURL(url);
    } catch (error) {
      console.error('Failed to export dataset:', error);
    }
  };

  // Handle correlation analysis
  const handleCorrelationAnalysis = async () => {
    if (selectedSymbols.length !== 2) {
      alert('Please select exactly 2 symbols for correlation analysis');
      return;
    }

    await getCorrelation(selectedSymbols[0], selectedSymbols[1]);
  };

  // Handle statistics analysis
  const handleStatisticsAnalysis = async () => {
    if (selectedSymbols.length === 0) {
      alert('Please select at least one symbol for analysis');
      return;
    }

    setAnalysisLoading(true);
    try {
      const results = await Promise.all(
        selectedSymbols.map(async (symbol) => {
          const stats = await analysisService.getStatistics(symbol);
          return { symbol, stats };
        })
      );
      setAnalysisResults(results);
    } catch (error) {
      console.error('Failed to get statistics:', error);
    } finally {
      setAnalysisLoading(false);
    }
  };

  const formatNumber = (num: number | null | undefined): string => {
    if (num === null || num === undefined) return 'N/A';
    if (Math.abs(num) < 0.01) return num.toExponential(2);
    return num.toFixed(4);
  };

  return (
    <div className={styles.tabContent}>
      {/* Cached Datasets */}
      <div className={styles.dataCategory}>
        <div 
          className={`${styles.categoryHeader} ${collapsedCategories.has('Cached Data') ? styles.collapsed : ''}`}
          onClick={() => toggleCategory('Cached Data')}
          style={{ display: 'flex', alignItems: 'center' }}
        >
          <span className={styles.categoryArrow}>▼</span>
          <span>Market Data (Parquet Files)</span>
          <button 
            className={styles.refreshBtn}
            onClick={async (e) => {
              e.stopPropagation();
              await refreshDatasets();
            }}
            style={{ 
              marginLeft: 'auto',
              padding: '2px 8px',
              fontSize: '12px',
              background: 'transparent',
              border: '1px solid var(--border)',
              borderRadius: '4px',
              cursor: 'pointer'
            }}
          >
            {loadingDatasets ? 'Loading...' : 'Refresh'}
          </button>
        </div>
        
        {!collapsedCategories.has('Cached Data') && (
          <div className={styles.datasetList}>
            {loadingDatasets ? (
              <div className={styles.datasetItem}>
                <div className={styles.datasetName}>Loading datasets...</div>
              </div>
            ) : error ? (
              <div className={styles.datasetItem}>
                <div className={styles.datasetName} style={{ color: 'var(--color-error)' }}>
                  Error: {error}
                </div>
              </div>
            ) : datasets.length === 0 ? (
              <div className={styles.datasetItem}>
                <div className={styles.datasetName}>No cached data yet</div>
                <div className={styles.datasetInfo}>Open the Monitor page to fetch and cache market data</div>
              </div>
            ) : (
              datasets.map((dataset, index) => {
                const startDate = new Date(dataset.first_bar).toLocaleDateString();
                const endDate = new Date(dataset.last_bar).toLocaleDateString();
                const duration = Math.round((new Date(dataset.last_bar).getTime() - new Date(dataset.first_bar).getTime()) / (1000 * 60 * 60 * 24));
                
                return (
                  <div 
                    key={index} 
                    className={`${styles.datasetItem} ${selectedSymbols.includes(dataset.symbol) ? styles.selected : ''}`}
                    onClick={() => {
                      // Toggle symbol selection
                      setSelectedSymbols(prev => 
                        prev.includes(dataset.symbol)
                          ? prev.filter(s => s !== dataset.symbol)
                          : [...prev, dataset.symbol]
                      );
                    }}
                  >
                    <div className={styles.datasetName}>
                      {dataset.symbol} • {dataset.exchange.toUpperCase()} • 1m
                    </div>
                    <div className={styles.datasetInfo}>
                      {dataset.bar_count.toLocaleString()} candles • {duration} days
                    </div>
                    <div className={styles.datasetInfo} style={{ fontSize: '11px', opacity: 0.7 }}>
                      {startDate} → {endDate}
                    </div>
                    
                    <button
                      className={styles.exportBtn}
                      onClick={(e) => {
                        e.stopPropagation();
                        handleDatasetExport(dataset);
                      }}
                      style={{ 
                        marginTop: '4px',
                        padding: '2px 6px',
                        fontSize: '10px',
                        background: 'var(--color-bg-secondary)',
                        border: '1px solid var(--border)',
                        borderRadius: '2px',
                        cursor: 'pointer'
                      }}
                    >
                      Export JSON
                    </button>
                  </div>
                );
              })
            )}
          </div>
        )}
      </div>

      {/* Analysis Tools */}
      {selectedSymbols.length > 0 && (
        <div className={styles.dataCategory}>
          <div className={styles.categoryHeader}>
            <span>Analysis Tools ({selectedSymbols.length} selected)</span>
          </div>
          
          <div className={styles.analysisButtons} style={{ padding: '8px', display: 'flex', gap: '8px', flexWrap: 'wrap' }}>
            <button
              onClick={handleStatisticsAnalysis}
              disabled={analysisLoading}
              style={{
                padding: '4px 8px',
                fontSize: '12px',
                background: 'var(--color-primary)',
                color: 'white',
                border: 'none',
                borderRadius: '4px',
                cursor: 'pointer'
              }}
            >
              {analysisLoading ? 'Computing...' : 'Statistics'}
            </button>
            
            {selectedSymbols.length === 2 && (
              <button
                onClick={handleCorrelationAnalysis}
                disabled={correlationLoading}
                style={{
                  padding: '4px 8px',
                  fontSize: '12px',
                  background: 'var(--color-secondary)',
                  color: 'white',
                  border: 'none',
                  borderRadius: '4px',
                  cursor: 'pointer'
                }}
              >
                {correlationLoading ? 'Computing...' : 'Correlation'}
              </button>
            )}
          </div>

          <div className={styles.selectedSymbols} style={{ padding: '4px 8px', fontSize: '11px', opacity: 0.7 }}>
            Selected: {selectedSymbols.join(', ')}
          </div>
        </div>
      )}

      {/* Analysis Results */}
      {correlation !== null && (
        <div className={styles.dataCategory}>
          <div className={styles.categoryHeader}>
            <span>Correlation Results</span>
          </div>
          <div style={{ padding: '8px' }}>
            <div style={{ fontSize: '14px', fontWeight: 'bold' }}>
              {selectedSymbols[0]} ↔ {selectedSymbols[1]}: {formatNumber(correlation)}
            </div>
            <div style={{ fontSize: '12px', marginTop: '4px', opacity: 0.8 }}>
              {Math.abs(correlation) > 0.7 ? 'Strong' : Math.abs(correlation) > 0.3 ? 'Moderate' : 'Weak'} correlation
            </div>
          </div>
        </div>
      )}

      {analysisResults && (
        <div className={styles.dataCategory}>
          <div className={styles.categoryHeader}>
            <span>Statistics Results</span>
          </div>
          <div style={{ padding: '8px' }}>
            {analysisResults.map(({ symbol, stats }: any) => (
              <div key={symbol} style={{ marginBottom: '12px', fontSize: '12px' }}>
                <div style={{ fontWeight: 'bold', marginBottom: '4px' }}>{symbol}</div>
                <div>Sharpe Ratio: {formatNumber(stats.sharpe_ratio)}</div>
                <div>Volatility: {formatNumber(stats.volatility)}</div>
                <div>Mean Return: {formatNumber(stats.mean_return)}</div>
                <div>Total Bars: {stats.total_bars?.toLocaleString()}</div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
};