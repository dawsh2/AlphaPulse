/**
 * Data Explorer Sidebar Component
 * Displays cached datasets, parquet files, signals, and backtests
 * Preserves exact styling from original ResearchPage
 */

import React from 'react';

interface Dataset {
  symbol: string;
  exchange: string;
  interval: string;
  candleCount: number;
  startTime: number;
  endTime: number;
}

interface DataExplorerSidebarProps {
  styles: any; // CSS module styles
  datasets: Dataset[];
  loadingDatasets: boolean;
  collapsedCategories: Set<string>;
  onToggleCategory: (category: string) => void;
  onExportDataset: (dataset: Dataset) => void;
}

export const DataExplorerSidebar: React.FC<DataExplorerSidebarProps> = ({
  styles,
  datasets,
  loadingDatasets,
  collapsedCategories,
  onToggleCategory,
  onExportDataset
}) => {
  return (
    <div className={styles.tabContent}>
      {/* Cached Datasets */}
      <div className={styles.dataCategory}>
        <div 
          className={`${styles.categoryHeader} ${collapsedCategories.has('Cached Data') ? styles.collapsed : ''}`}
          onClick={() => onToggleCategory('Cached Data')}
        >
          <span className={styles.categoryArrow}>▼</span>
          <span>Cached Market Data (IndexedDB)</span>
        </div>
        {!collapsedCategories.has('Cached Data') && (
          <div className={styles.datasetList}>
            {loadingDatasets ? (
              <div className={styles.datasetItem}>
                <div className={styles.datasetName}>Loading datasets...</div>
              </div>
            ) : datasets.length === 0 ? (
              <div className={styles.datasetItem}>
                <div className={styles.datasetName}>No cached data yet</div>
                <div className={styles.datasetInfo}>Open the Monitor page to fetch and cache market data</div>
              </div>
            ) : (
              datasets.map((dataset, index) => {
                const startDate = new Date(dataset.startTime * 1000).toLocaleDateString();
                const endDate = new Date(dataset.endTime * 1000).toLocaleDateString();
                const duration = Math.round((dataset.endTime - dataset.startTime) / (60 * 60 * 24));
                
                return (
                  <div 
                    key={index} 
                    className={styles.datasetItem} 
                    onClick={() => onExportDataset(dataset)}
                  >
                    <div className={styles.datasetName}>
                      {dataset.symbol} • {dataset.exchange.toUpperCase()} • {dataset.interval}
                    </div>
                    <div className={styles.datasetInfo}>
                      {dataset.candleCount.toLocaleString()} candles • {duration} days • {startDate} to {endDate}
                    </div>
                  </div>
                );
              })
            )}
          </div>
        )}
      </div>
      
      {/* Parquet Files (Backend) */}
      <div className={styles.dataCategory}>
        <div 
          className={`${styles.categoryHeader} ${collapsedCategories.has('Parquet Files') ? styles.collapsed : ''}`}
          onClick={() => onToggleCategory('Parquet Files')}
        >
          <span className={styles.categoryArrow}>▼</span>
          <span>Parquet Files (Backend Catalog)</span>
        </div>
        {!collapsedCategories.has('Parquet Files') && (
          <div className={styles.datasetList}>
            <div className={styles.datasetItem} onClick={() => {}}>
              <div className={styles.datasetName}>NVDA.ALPACA-1-MINUTE</div>
              <div className={styles.datasetInfo}>catalog/data/bar/ • OHLCV</div>
            </div>
          </div>
        )}
      </div>
      
      <div className={styles.dataCategory}>
        <div 
          className={`${styles.categoryHeader} ${collapsedCategories.has('Signals') ? styles.collapsed : ''}`}
          onClick={() => onToggleCategory('Signals')}
        >
          <span className={styles.categoryArrow}>▼</span>
          <span>Signals & Features</span>
        </div>
        {!collapsedCategories.has('Signals') && (
          <div className={styles.datasetList}>
            <div className={styles.datasetItem} onClick={() => {}}>
              <div className={styles.datasetName}>momentum_signals.parquet</div>
              <div className={styles.datasetInfo}>500K rows • 120MB • Features</div>
            </div>
            <div className={styles.datasetItem} onClick={() => {}}>
              <div className={styles.datasetName}>ml_features_v2.parquet</div>
              <div className={styles.datasetInfo}>2M rows • 380MB • ML features</div>
            </div>
          </div>
        )}
      </div>
      
      <div className={styles.dataCategory}>
        <div 
          className={`${styles.categoryHeader} ${collapsedCategories.has('Backtests') ? styles.collapsed : ''}`}
          onClick={() => onToggleCategory('Backtests')}
        >
          <span className={styles.categoryArrow}>▼</span>
          <span>Backtest Results</span>
        </div>
        {!collapsedCategories.has('Backtests') && (
          <div className={styles.datasetList}>
            <div className={styles.datasetItem} onClick={() => {}}>
              <div className={styles.datasetName}>ema_cross_results.parquet</div>
              <div className={styles.datasetInfo}>10K rows • 5MB • Performance</div>
            </div>
          </div>
        )}
      </div>
      
      {/* Quick Actions */}
      <div className={styles.dataActions}>
        <button className={styles.dataActionBtn}>
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <path d="M12 5v14M5 12h14"></path>
          </svg>
          Upload Dataset
        </button>
        <button className={styles.dataActionBtn}>
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect>
            <line x1="9" y1="9" x2="15" y2="9"></line>
            <line x1="9" y1="15" x2="15" y2="15"></line>
          </svg>
          SQL Query
        </button>
      </div>
    </div>
  );
};