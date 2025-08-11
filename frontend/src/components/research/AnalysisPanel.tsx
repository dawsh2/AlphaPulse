/**
 * AnalysisPanel Component - SQL queries and advanced analysis
 * Extracted from ResearchPage for better separation of concerns
 */
import React, { useState } from 'react';
import { useMarketData } from '../../hooks/useMarketData';
import styles from '../../pages/ResearchPage.module.css';

interface AnalysisPanelProps {
  collapsedCategories: Set<string>;
  toggleCategory: (category: string) => void;
}

export const AnalysisPanel: React.FC<AnalysisPanelProps> = ({
  collapsedCategories,
  toggleCategory,
}) => {
  const { queryData, queryLoading } = useMarketData();
  const [sqlQuery, setSqlQuery] = useState('SELECT symbol, AVG(close) as avg_price, COUNT(*) as bars FROM ohlcv GROUP BY symbol');
  const [queryResults, setQueryResults] = useState<any>(null);
  const [queryError, setQueryError] = useState<string | null>(null);

  // Handle SQL query execution
  const handleRunQuery = async () => {
    if (!sqlQuery.trim()) {
      alert('Please enter a SQL query');
      return;
    }

    if (!sqlQuery.trim().toUpperCase().startsWith('SELECT')) {
      alert('Only SELECT queries are allowed for security');
      return;
    }

    try {
      setQueryError(null);
      const results = await queryData(sqlQuery);
      setQueryResults(results);
    } catch (error) {
      setQueryError(error instanceof Error ? error.message : 'Query failed');
      setQueryResults(null);
    }
  };

  // Predefined query templates
  const queryTemplates = [
    {
      name: 'Symbol Summary',
      description: 'Average prices and bar counts by symbol',
      query: 'SELECT symbol, AVG(close) as avg_price, COUNT(*) as bars FROM ohlcv GROUP BY symbol'
    },
    {
      name: 'Daily OHLC',
      description: 'Daily aggregated OHLC data',
      query: `SELECT 
        symbol,
        DATE(datetime) as date,
        MIN(low) as daily_low,
        MAX(high) as daily_high,
        (SELECT open FROM ohlcv o2 WHERE o2.symbol = ohlcv.symbol AND DATE(o2.datetime) = DATE(ohlcv.datetime) ORDER BY datetime ASC LIMIT 1) as daily_open,
        (SELECT close FROM ohlcv o2 WHERE o2.symbol = ohlcv.symbol AND DATE(o2.datetime) = DATE(ohlcv.datetime) ORDER BY datetime DESC LIMIT 1) as daily_close,
        SUM(volume) as daily_volume
      FROM ohlcv 
      GROUP BY symbol, DATE(datetime) 
      ORDER BY symbol, date DESC 
      LIMIT 20`
    },
    {
      name: 'Price Changes',
      description: 'Hourly price changes and volatility',
      query: `SELECT 
        symbol,
        datetime,
        close,
        LAG(close) OVER (PARTITION BY symbol ORDER BY timestamp) as prev_close,
        (close - LAG(close) OVER (PARTITION BY symbol ORDER BY timestamp)) / LAG(close) OVER (PARTITION BY symbol ORDER BY timestamp) * 100 as pct_change
      FROM ohlcv 
      WHERE symbol = 'BTC/USD'
      ORDER BY timestamp DESC 
      LIMIT 50`
    },
    {
      name: 'Volume Analysis',
      description: 'Volume patterns and statistics',
      query: `SELECT 
        symbol,
        AVG(volume) as avg_volume,
        MAX(volume) as max_volume,
        MIN(volume) as min_volume,
        STDDEV(volume) as volume_std
      FROM ohlcv 
      GROUP BY symbol
      ORDER BY avg_volume DESC`
    }
  ];

  // Format results for display
  const formatCellValue = (value: any): string => {
    if (value === null || value === undefined) return 'null';
    if (typeof value === 'number') {
      if (Math.abs(value) < 0.01) return value.toExponential(2);
      return value.toLocaleString(undefined, { maximumFractionDigits: 4 });
    }
    if (typeof value === 'string' && value.length > 50) {
      return value.substring(0, 47) + '...';
    }
    return String(value);
  };

  return (
    <div className={styles.tabContent}>
      {/* SQL Query Interface */}
      <div className={styles.dataCategory}>
        <div 
          className={`${styles.categoryHeader} ${collapsedCategories.has('SQL Query') ? styles.collapsed : ''}`}
          onClick={() => toggleCategory('SQL Query')}
        >
          <span className={styles.categoryArrow}>▼</span>
          <span>SQL Query Interface</span>
        </div>
        
        {!collapsedCategories.has('SQL Query') && (
          <div style={{ padding: '12px' }}>
            <div style={{ marginBottom: '8px' }}>
              <textarea
                value={sqlQuery}
                onChange={(e) => setSqlQuery(e.target.value)}
                placeholder="-- Enter SQL query here (SELECT statements only)"
                style={{
                  width: '100%',
                  minHeight: '80px',
                  padding: '8px',
                  fontFamily: 'Monaco, "Lucida Console", monospace',
                  fontSize: '12px',
                  border: '1px solid var(--border)',
                  borderRadius: '4px',
                  background: 'var(--color-bg-primary)',
                  color: 'var(--color-text-primary)',
                  resize: 'vertical'
                }}
              />
            </div>
            
            <button 
              onClick={handleRunQuery}
              disabled={queryLoading}
              style={{
                padding: '6px 12px',
                background: 'var(--color-primary)',
                color: 'white',
                border: 'none',
                borderRadius: '4px',
                cursor: 'pointer',
                fontSize: '12px',
                display: 'flex',
                alignItems: 'center',
                gap: '6px'
              }}
            >
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <polygon points="5 3 19 12 5 21 5 3"></polygon>
              </svg>
              {queryLoading ? 'Running...' : 'Run Query'}
            </button>
          </div>
        )}
      </div>

      {/* Query Templates */}
      <div className={styles.dataCategory}>
        <div 
          className={`${styles.categoryHeader} ${collapsedCategories.has('Query Templates') ? styles.collapsed : ''}`}
          onClick={() => toggleCategory('Query Templates')}
        >
          <span className={styles.categoryArrow}>▼</span>
          <span>Query Templates</span>
        </div>
        
        {!collapsedCategories.has('Query Templates') && (
          <div className={styles.datasetList}>
            {queryTemplates.map((template, index) => (
              <div 
                key={index}
                className={styles.datasetItem}
                onClick={() => setSqlQuery(template.query)}
                style={{ cursor: 'pointer' }}
              >
                <div className={styles.datasetName}>{template.name}</div>
                <div className={styles.datasetInfo}>{template.description}</div>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Query Results */}
      {(queryResults || queryError) && (
        <div className={styles.dataCategory}>
          <div className={styles.categoryHeader}>
            <span>Query Results</span>
          </div>
          
          <div style={{ padding: '12px' }}>
            {queryError ? (
              <div style={{ 
                color: 'var(--color-error)', 
                background: 'var(--color-bg-secondary)',
                padding: '8px',
                borderRadius: '4px',
                fontSize: '12px'
              }}>
                Error: {queryError}
              </div>
            ) : queryResults ? (
              <div>
                <div style={{ 
                  fontSize: '12px', 
                  marginBottom: '8px',
                  color: 'var(--color-text-secondary)'
                }}>
                  {queryResults.rows} rows, {queryResults.columns?.length} columns
                </div>
                
                {queryResults.data && queryResults.data.length > 0 ? (
                  <div style={{ 
                    overflowX: 'auto',
                    border: '1px solid var(--border)',
                    borderRadius: '4px'
                  }}>
                    <table style={{ 
                      width: '100%',
                      fontSize: '11px',
                      borderCollapse: 'collapse'
                    }}>
                      <thead>
                        <tr style={{ background: 'var(--color-bg-secondary)' }}>
                          {queryResults.columns.map((col: string) => (
                            <th key={col} style={{ 
                              padding: '6px 8px',
                              textAlign: 'left',
                              borderBottom: '1px solid var(--border)',
                              fontWeight: 'bold'
                            }}>
                              {col}
                            </th>
                          ))}
                        </tr>
                      </thead>
                      <tbody>
                        {queryResults.data.slice(0, 100).map((row: any, index: number) => (
                          <tr key={index}>
                            {queryResults.columns.map((col: string) => (
                              <td key={col} style={{ 
                                padding: '4px 8px',
                                borderBottom: '1px solid var(--border)'
                              }}>
                                {formatCellValue(row[col])}
                              </td>
                            ))}
                          </tr>
                        ))}
                      </tbody>
                    </table>
                    
                    {queryResults.data.length > 100 && (
                      <div style={{ 
                        padding: '8px',
                        textAlign: 'center',
                        fontSize: '11px',
                        color: 'var(--color-text-secondary)',
                        background: 'var(--color-bg-secondary)'
                      }}>
                        Showing first 100 rows of {queryResults.rows}
                      </div>
                    )}
                  </div>
                ) : (
                  <div style={{ 
                    fontSize: '12px',
                    color: 'var(--color-text-secondary)',
                    fontStyle: 'italic'
                  }}>
                    Query executed successfully, no data returned
                  </div>
                )}
              </div>
            ) : null}
          </div>
        </div>
      )}

      {/* Data Schema Reference */}
      <div className={styles.dataCategory}>
        <div 
          className={`${styles.categoryHeader} ${collapsedCategories.has('Schema') ? styles.collapsed : ''}`}
          onClick={() => toggleCategory('Schema')}
        >
          <span className={styles.categoryArrow}>▼</span>
          <span>Database Schema</span>
        </div>
        
        {!collapsedCategories.has('Schema') && (
          <div style={{ padding: '12px', fontSize: '12px' }}>
            <div style={{ marginBottom: '8px' }}>
              <strong>ohlcv</strong> - Main market data table
            </div>
            <div style={{ marginLeft: '12px', color: 'var(--color-text-secondary)' }}>
              • symbol (VARCHAR) - Trading pair (e.g., 'BTC/USD')<br/>
              • exchange (VARCHAR) - Exchange name<br/>
              • timestamp (BIGINT) - Unix timestamp<br/>
              • datetime (TIMESTAMP) - Human readable date<br/>
              • open, high, low, close (DOUBLE) - OHLC prices<br/>
              • volume (DOUBLE) - Trading volume
            </div>
          </div>
        )}
      </div>
    </div>
  );
};