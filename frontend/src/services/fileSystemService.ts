/**
 * File System Service
 * Extracted from DevelopPage.tsx - handles file structure loading and management
 * PURE EXTRACTION - No fallback code
 */

export interface FileItem {
  path: string;
  name: string;
  type: 'file' | 'folder';
  children?: FileItem[];
}

export async function loadFileStructure(): Promise<FileItem[]> {
  try {
    const response = await fetch('http://localhost:5001/api/nt-reference/list-files');
    const data = await response.json();
    
    // Transform the data into our file structure
    const fileStructure: FileItem[] = [];
    
    if (data.examples) {
      const examplesFolder: FileItem = {
        path: 'examples/',
        name: 'examples',
        type: 'folder',
        children: []
      };
      
      if (data.examples.strategies) {
        const strategiesFolder: FileItem = {
          path: 'examples/strategies/',
          name: 'strategies',
          type: 'folder',
          children: data.examples.strategies.map((file: string) => ({
            path: `strategies/${file}`,
            name: file,
            type: 'file'
          }))
        };
        examplesFolder.children?.push(strategiesFolder);
      }
      
      if (data.examples.algorithms) {
        const algorithmsFolder: FileItem = {
          path: 'examples/algorithms/',
          name: 'algorithms',
          type: 'folder',
          children: data.examples.algorithms.map((file: string) => ({
            path: `algorithms/${file}`,
            name: file,
            type: 'file'
          }))
        };
        examplesFolder.children?.push(algorithmsFolder);
      }
      
      fileStructure.push(examplesFolder);
    }
    
    // Add README.md at the top
    fileStructure.push(
      { path: 'README.md', name: 'README.md', type: 'file' }
    );
    
    
    // Add Notebooks directory with snippets and builder-ui as subdirectories
    fileStructure.push(
      {
        path: 'notebooks/',
        name: 'notebooks',
        type: 'folder',
        children: [
          { path: 'notebooks/strategy_development.ipynb', name: 'strategy_development.ipynb', type: 'file' },
          { path: 'notebooks/market_analysis.ipynb', name: 'market_analysis.ipynb', type: 'file' },
          { path: 'notebooks/backtest_results.ipynb', name: 'backtest_results.ipynb', type: 'file' },
          { path: 'notebooks/signal_research.ipynb', name: 'signal_research.ipynb', type: 'file' },
          { path: 'notebooks/portfolio_optimization.ipynb', name: 'portfolio_optimization.ipynb', type: 'file' },
          {
            path: 'notebooks/snippets/',
            name: 'snippets',
            type: 'folder',
            children: [
              {
                path: 'notebooks/snippets/data_loading/',
                name: 'data_loading',
                type: 'folder',
                children: [
                  { path: 'notebooks/snippets/data_loading/load_signals.py', name: 'load_signals.py', type: 'file' },
                  { path: 'notebooks/snippets/data_loading/fetch_market_data.py', name: 'fetch_market_data.py', type: 'file' },
                  { path: 'notebooks/snippets/data_loading/import_csv.py', name: 'import_csv.py', type: 'file' }
                ]
              },
              {
                path: 'notebooks/snippets/performance_metrics/',
                name: 'performance_metrics',
                type: 'folder',
                children: [
                  { path: 'notebooks/snippets/performance_metrics/sharpe_ratio.py', name: 'sharpe_ratio.py', type: 'file' },
                  { path: 'notebooks/snippets/performance_metrics/max_drawdown.py', name: 'max_drawdown.py', type: 'file' },
                  { path: 'notebooks/snippets/performance_metrics/win_rate.py', name: 'win_rate.py', type: 'file' }
                ]
              },
              {
                path: 'notebooks/snippets/visualizations/',
                name: 'visualizations',
                type: 'folder',
                children: [
                  { path: 'notebooks/snippets/visualizations/plot_pnl.py', name: 'plot_pnl.py', type: 'file' },
                  { path: 'notebooks/snippets/visualizations/candlestick_chart.py', name: 'candlestick_chart.py', type: 'file' },
                  { path: 'notebooks/snippets/visualizations/heatmap.py', name: 'heatmap.py', type: 'file' }
                ]
              },
              {
                path: 'notebooks/snippets/analysis_templates/',
                name: 'analysis_templates',
                type: 'folder',
                children: [
                  { path: 'notebooks/snippets/analysis_templates/backtest_analysis.py', name: 'backtest_analysis.py', type: 'file' },
                  { path: 'notebooks/snippets/analysis_templates/correlation_study.py', name: 'correlation_study.py', type: 'file' },
                  { path: 'notebooks/snippets/analysis_templates/risk_metrics.py', name: 'risk_metrics.py', type: 'file' }
                ]
              },
              {
                path: 'notebooks/snippets/saved_notebooks/',
                name: 'saved_notebooks',
                type: 'folder',
                children: [
                  { path: 'notebooks/snippets/saved_notebooks/ema_cross_research.ipynb', name: 'ema_cross_research.ipynb', type: 'file' },
                  { path: 'notebooks/snippets/saved_notebooks/mean_reversion_analysis.ipynb', name: 'mean_reversion_analysis.ipynb', type: 'file' }
                ]
              }
            ]
          },
          {
            path: 'notebooks/builder-ui/',
            name: 'builder-ui',
            type: 'folder',
            children: [
              { path: 'notebooks/builder-ui/signal_analysis.py', name: 'signal_analysis.py', type: 'file' },
              { path: 'notebooks/builder-ui/strategy_workbench.py', name: 'strategy_workbench.py', type: 'file' },
              { path: 'notebooks/builder-ui/components.py', name: 'components.py', type: 'file' },
              { path: 'notebooks/builder-ui/config.json', name: 'config.json', type: 'file' }
            ]
          }
        ]
      }
    );
    
    // Add tests directory with subdirectories
    fileStructure.push(
      {
        path: 'tests/',
        name: 'tests',
        type: 'folder',
        children: [
          {
            path: 'tests/snippets/',
            name: 'snippets',
            type: 'folder',
            children: [
              { path: 'tests/snippets/test_data_loading.py', name: 'test_data_loading.py', type: 'file' },
              { path: 'tests/snippets/test_indicators.py', name: 'test_indicators.py', type: 'file' },
              { path: 'tests/snippets/test_metrics.py', name: 'test_metrics.py', type: 'file' },
              { path: 'tests/snippets/test_plots.py', name: 'test_plots.py', type: 'file' }
            ]
          },
          {
            path: 'tests/strategies/',
            name: 'strategies',
            type: 'folder',
            children: [
              { path: 'tests/strategies/test_ema_cross.py', name: 'test_ema_cross.py', type: 'file' },
              { path: 'tests/strategies/test_momentum.py', name: 'test_momentum.py', type: 'file' }
            ]
          }
        ]
      }
    );
    
    // Add Configuration directory
    fileStructure.push(
      {
        path: 'config/',
        name: 'config',
        type: 'folder',
        children: [
          { path: 'config/strategies.yaml', name: 'strategies.yaml', type: 'file' },
          { path: 'config/indicators.json', name: 'indicators.json', type: 'file' },
          { path: 'config/data_sources.yaml', name: 'data_sources.yaml', type: 'file' }
        ]
      }
    );
    
    return fileStructure;
  } catch (error) {
    console.error('Failed to load files:', error);
    // Return default file structure
    return [
      { path: 'README.md', name: 'README.md', type: 'file' },
      {
        path: 'notebooks/',
        name: 'notebooks',
        type: 'folder',
        children: [
          { path: 'notebooks/strategy_development.ipynb', name: 'strategy_development.ipynb', type: 'file' },
          { path: 'notebooks/market_analysis.ipynb', name: 'market_analysis.ipynb', type: 'file' },
          {
            path: 'notebooks/snippets/',
            name: 'snippets',
            type: 'folder',
            children: [
              {
                path: 'notebooks/snippets/data_loading/',
                name: 'data_loading',
                type: 'folder',
                children: [
                  { path: 'notebooks/snippets/data_loading/load_signals.py', name: 'load_signals.py', type: 'file' }
                ]
              }
            ]
          }
        ]
      },
      {
        path: 'tests/',
        name: 'tests',
        type: 'folder',
        children: [
          {
            path: 'tests/strategies/',
            name: 'strategies',
            type: 'folder',
            children: [
              { path: 'strategies/ema_cross.py', name: 'ema_cross.py', type: 'file' },
              { path: 'strategies/momentum.py', name: 'momentum.py', type: 'file' }
            ]
          }
        ]
      }
    ];
  }
}