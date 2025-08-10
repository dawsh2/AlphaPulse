/**
 * File Content Generator Service
 * Extracted from DevelopPage.tsx - handles content generation for different file types
 * PURE EXTRACTION - No fallback code
 */

export interface Tab {
  id: string;
  name: string;
  content: string;
  language?: string;
}

export interface FileContentOptions {
  tabs: Tab[];
  setTabs: (tabs: Tab[]) => void;
  setActiveTab: (tabId: string) => void;
  setEditorHidden: (hidden: boolean) => void;
}

export async function generateFileContent(
  filePath: string, 
  fileName: string,
  options: FileContentOptions
): Promise<void> {
  const { tabs, setTabs, setActiveTab, setEditorHidden } = options;

  // Open editor if it's hidden
  if (setEditorHidden) {
    setEditorHidden(false);
  }
  
  // Check if tab already exists
  const existingTab = tabs.find(tab => tab.id === filePath);
  if (existingTab) {
    setActiveTab(filePath);
    return;
  }
  
  // Generate content based on file type and location
  let content = '';
  
  // Handle README.md
  if (fileName === 'README.md') {
    content = `# AlphaPulse Development Environment

Welcome to the AlphaPulse integrated development environment for quantitative trading strategies.

## Getting Started

This environment provides everything you need to develop, test, and deploy trading strategies using NautilusTrader.

### Quick Start Guide

1. **Explore Examples**: Browse the \`examples/\` folder for sample strategies
2. **Use Snippets**: Access ready-to-use code snippets in the \`snippets/\` folder
3. **Run Backtests**: Use the terminal to execute strategy backtests

### Key Features

- **Monaco Editor**: Professional code editing with syntax highlighting
- **Integrated Terminal**: Run NautilusTrader commands directly
- **Code Snippets**: Pre-built functions for common trading operations
- **Live Preview**: Test strategies with real-time market data

### Project Structure

\`\`\`
├── README.md           # This file
├── snippets/           # Reusable code snippets
│   ├── data_loading/   # Data import utilities
│   ├── performance_metrics/ # Performance calculations
│   ├── visualizations/ # Charting functions
│   └── analysis_templates/ # Analysis templates
├── examples/           # Example strategies
├── config/            # Configuration files
└── docs/              # Documentation
\`\`\`

### Keyboard Shortcuts

- **Ctrl/Cmd + S**: Save current file
- **Ctrl/Cmd + Enter**: Run current code
- **Ctrl/Cmd + /**: Toggle comment
- **Ctrl/Cmd + D**: Duplicate line

### Resources

- [NautilusTrader Documentation](https://nautilustrader.io/docs/)
- [AlphaPulse Strategy Guide](docs/strategy_guide.md)
- [API Reference](docs/API.md)

---

*Happy Trading! 🚀*`;
  }
  // Snippet files get specialized content
  else if (filePath.includes('snippets/')) {
    if (filePath.includes('data_loading/')) {
      if (fileName === 'load_signals.py') {
        content = `# Load Signal Data from ADMF
import admf
import pandas as pd

def load_signals(strategy_id: str, limit: int = 100):
    """Load signal traces from the ADMF registry."""
    signals = admf.load_signals(
        strategy_type=strategy_id,
        limit=limit
    )
    
    # Convert to DataFrame for analysis
    df = pd.DataFrame(signals)
    print(f"Loaded {len(df)} signals for {strategy_id}")
    
    return df

# Example usage
if __name__ == "__main__":
    signals = load_signals('ema_cross', limit=50)
    print(signals.head())`;
      } else if (fileName === 'fetch_market_data.py') {
        content = `# Fetch Market Data
import pandas as pd
import numpy as np
from datetime import datetime, timedelta

def fetch_market_data(symbol: str, period: str = '1d', lookback: int = 30):
    """Fetch historical market data for analysis."""
    end_date = datetime.now()
    start_date = end_date - timedelta(days=lookback)
    
    # Mock data generation (replace with actual API call)
    dates = pd.date_range(start=start_date, end=end_date, freq=period)
    data = {
        'date': dates,
        'open': np.random.randn(len(dates)) * 2 + 100,
        'high': np.random.randn(len(dates)) * 2 + 102,
        'low': np.random.randn(len(dates)) * 2 + 98,
        'close': np.random.randn(len(dates)) * 2 + 100,
        'volume': np.random.randint(1000000, 5000000, len(dates))
    }
    
    return pd.DataFrame(data)

# Example usage
if __name__ == "__main__":
    data = fetch_market_data('SPY', '1d', 30)
    print(data.tail())`;
      } else {
        content = `# Import CSV Data
import pandas as pd
import os

def import_csv(filepath: str, parse_dates: bool = True):
    """Import data from CSV file."""
    if not os.path.exists(filepath):
        raise FileNotFoundError(f"File not found: {filepath}")
    
    df = pd.read_csv(
        filepath,
        parse_dates=['date'] if parse_dates else None,
        index_col='date' if parse_dates else None
    )
    
    print(f"Loaded {len(df)} rows from {filepath}")
    print(f"Columns: {', '.join(df.columns)}")
    
    return df`;
      }
    } else if (filePath.includes('performance_metrics/')) {
      if (fileName === 'sharpe_ratio.py') {
        content = `# Calculate Sharpe Ratio
import numpy as np
import pandas as pd

def calculate_sharpe_ratio(returns: pd.Series, risk_free_rate: float = 0.02):
    """
    Calculate the Sharpe ratio for a returns series.
    
    Args:
        returns: Series of returns
        risk_free_rate: Annual risk-free rate (default 2%)
    
    Returns:
        float: Sharpe ratio
    """
    excess_returns = returns - risk_free_rate / 252  # Daily risk-free rate
    
    if len(excess_returns) < 2:
        return 0.0
    
    sharpe = np.sqrt(252) * excess_returns.mean() / excess_returns.std()
    
    return sharpe

# Example usage
if __name__ == "__main__":
    # Generate sample returns
    returns = pd.Series(np.random.randn(252) * 0.01 + 0.0005)
    sharpe = calculate_sharpe_ratio(returns)
    print(f"Sharpe Ratio: {sharpe:.2f}")`;
      } else if (fileName === 'max_drawdown.py') {
        content = `# Calculate Maximum Drawdown
import pandas as pd
import numpy as np

def calculate_max_drawdown(equity_curve: pd.Series):
    """
    Calculate the maximum drawdown from an equity curve.
    
    Args:
        equity_curve: Series of portfolio values
    
    Returns:
        tuple: (max_drawdown, peak_date, trough_date)
    """
    # Calculate running maximum
    running_max = equity_curve.cummax()
    
    # Calculate drawdown
    drawdown = (equity_curve - running_max) / running_max
    
    # Find maximum drawdown
    max_dd = drawdown.min()
    max_dd_idx = drawdown.idxmin()
    
    # Find the peak before the max drawdown
    peak_idx = equity_curve[:max_dd_idx].idxmax()
    
    return max_dd, peak_idx, max_dd_idx

# Example usage
if __name__ == "__main__":
    # Generate sample equity curve
    dates = pd.date_range('2024-01-01', periods=252, freq='D')
    equity = pd.Series(np.cumprod(1 + np.random.randn(252) * 0.01), index=dates)
    
    max_dd, peak, trough = calculate_max_drawdown(equity)
    print(f"Max Drawdown: {max_dd:.2%}")
    print(f"Peak: {peak}, Trough: {trough}")`;
      } else {
        content = `# Calculate Win Rate
import pandas as pd
import numpy as np

def calculate_win_rate(trades: pd.DataFrame):
    """
    Calculate win rate from trade history.
    
    Args:
        trades: DataFrame with 'pnl' column
    
    Returns:
        dict: Win rate metrics
    """
    winning_trades = trades[trades['pnl'] > 0]
    losing_trades = trades[trades['pnl'] <= 0]
    
    metrics = {
        'win_rate': len(winning_trades) / len(trades) * 100,
        'total_trades': len(trades),
        'winning_trades': len(winning_trades),
        'losing_trades': len(losing_trades),
        'avg_win': winning_trades['pnl'].mean() if len(winning_trades) > 0 else 0,
        'avg_loss': losing_trades['pnl'].mean() if len(losing_trades) > 0 else 0
    }
    
    return metrics`;
      }
    } else if (filePath.includes('visualizations/')) {
      if (fileName === 'plot_pnl.py') {
        content = `# Plot P&L Curve
import matplotlib.pyplot as plt
import pandas as pd
import numpy as np

def plot_pnl_curve(pnl_series: pd.Series, title: str = "P&L Curve"):
    """
    Plot cumulative P&L curve with drawdown shading.
    
    Args:
        pnl_series: Series of P&L values
        title: Chart title
    """
    fig, (ax1, ax2) = plt.subplots(2, 1, figsize=(12, 8), height_ratios=[3, 1])
    
    # Cumulative P&L
    cum_pnl = pnl_series.cumsum()
    ax1.plot(cum_pnl.index, cum_pnl.values, 'b-', linewidth=2)
    ax1.fill_between(cum_pnl.index, 0, cum_pnl.values, alpha=0.3)
    ax1.set_title(title)
    ax1.set_ylabel('Cumulative P&L ($)')
    ax1.grid(True, alpha=0.3)
    
    # Drawdown
    running_max = cum_pnl.cummax()
    drawdown = cum_pnl - running_max
    ax2.fill_between(drawdown.index, 0, drawdown.values, color='red', alpha=0.3)
    ax2.set_ylabel('Drawdown ($)')
    ax2.set_xlabel('Date')
    ax2.grid(True, alpha=0.3)
    
    plt.tight_layout()
    return fig`;
      } else if (fileName === 'candlestick_chart.py') {
        content = `# Create Candlestick Chart
import plotly.graph_objects as go
import pandas as pd

def plot_candlestick(df: pd.DataFrame, title: str = "Price Chart"):
    """
    Create interactive candlestick chart.
    
    Args:
        df: DataFrame with OHLC data
        title: Chart title
    """
    fig = go.Figure(data=[go.Candlestick(
        x=df.index,
        open=df['open'],
        high=df['high'],
        low=df['low'],
        close=df['close'],
        name='OHLC'
    )])
    
    fig.update_layout(
        title=title,
        yaxis_title='Price',
        xaxis_title='Date',
        template='plotly_dark',
        xaxis_rangeslider_visible=False
    )
    
    return fig`;
      } else {
        content = `# Create Correlation Heatmap
import seaborn as sns
import matplotlib.pyplot as plt
import pandas as pd

def plot_correlation_heatmap(df: pd.DataFrame, title: str = "Correlation Matrix"):
    """
    Create correlation heatmap.
    
    Args:
        df: DataFrame with numeric columns
        title: Chart title
    """
    plt.figure(figsize=(10, 8))
    
    # Calculate correlation matrix
    corr_matrix = df.corr()
    
    # Create heatmap
    sns.heatmap(
        corr_matrix,
        annot=True,
        fmt='.2f',
        cmap='coolwarm',
        center=0,
        square=True,
        linewidths=1,
        cbar_kws={"shrink": 0.8}
    )
    
    plt.title(title)
    plt.tight_layout()
    return plt.gcf()`;
      }
    } else if (filePath.includes('analysis_templates/')) {
      content = `# ${fileName.replace('.py', '').replace('_', ' ').toUpperCase()} Template
import pandas as pd
import numpy as np
import matplotlib.pyplot as plt
from datetime import datetime

# Analysis configuration
CONFIG = {
    'lookback_period': 252,  # Trading days
    'confidence_level': 0.95,
    'initial_capital': 100000
}

def run_analysis(data: pd.DataFrame):
    """
    Run comprehensive ${fileName.replace('.py', '').replace('_', ' ')} analysis.
    
    Args:
        data: Input data for analysis
    
    Returns:
        dict: Analysis results
    """
    results = {}
    
    # Add your analysis logic here
    print(f"Running {fileName.replace('.py', '').replace('_', ' ')}...")
    
    return results

# Example usage
if __name__ == "__main__":
    # Load your data
    data = pd.DataFrame()  # Replace with actual data loading
    
    # Run analysis
    results = run_analysis(data)
    
    # Display results
    for key, value in results.items():
        print(f"{key}: {value}")`;
    } else if (filePath.includes('builder-ui/')) {
      if (fileName === 'signal_analysis.py') {
        content = `"""Signal Analysis UI Component for StrategyWorkbench

This module provides the signal analysis interface for the StrategyWorkbench.
Users can create custom UI components that integrate with Jupyter notebooks.
"""

import pandas as pd
import numpy as np
import plotly.graph_objects as go
from plotly.subplots import make_subplots
from datetime import datetime, timedelta
from typing import Dict, List, Optional, Tuple
import ipywidgets as widgets
from IPython.display import display, HTML

class SignalAnalysisUI:
    """Interactive signal analysis dashboard for strategy development."""
    
    def __init__(self, signals_df: pd.DataFrame, price_df: pd.DataFrame):
        self.signals = signals_df
        self.prices = price_df
        self.current_symbol = None
        self.widgets = {}
        self._setup_ui()
    
    def _setup_ui(self):
        """Initialize all UI components."""
        # Date range selector
        self.widgets['date_start'] = widgets.DatePicker(
            description='Start Date:',
            value=datetime.now() - timedelta(days=30)
        )
        self.widgets['date_end'] = widgets.DatePicker(
            description='End Date:',
            value=datetime.now()
        )
        
        # Symbol selector
        symbols = self.signals['symbol'].unique() if 'symbol' in self.signals.columns else ['SPY']
        self.widgets['symbol'] = widgets.Dropdown(
            options=symbols,
            value=symbols[0],
            description='Symbol:'
        )
        
        # Signal type filter
        self.widgets['signal_type'] = widgets.SelectMultiple(
            options=['BUY', 'SELL', 'HOLD'],
            value=['BUY', 'SELL'],
            description='Signals:'
        )
        
        # Confidence threshold
        self.widgets['confidence'] = widgets.FloatSlider(
            value=0.5,
            min=0.0,
            max=1.0,
            step=0.05,
            description='Min Confidence:'
        )
        
        # Analysis buttons
        self.widgets['analyze_btn'] = widgets.Button(
            description='Run Analysis',
            button_style='primary',
            icon='chart-line'
        )
        self.widgets['analyze_btn'].on_click(self._on_analyze)
        
        self.widgets['export_btn'] = widgets.Button(
            description='Export Results',
            button_style='success',
            icon='download'
        )
        self.widgets['export_btn'].on_click(self._on_export)
        
        # Output area
        self.widgets['output'] = widgets.Output()
    
    def _on_analyze(self, btn):
        """Handle analysis button click."""
        with self.widgets['output']:
            self.widgets['output'].clear_output()
            
            # Get filter parameters
            symbol = self.widgets['symbol'].value
            start_date = self.widgets['date_start'].value
            end_date = self.widgets['date_end'].value
            signal_types = self.widgets['signal_type'].value
            min_confidence = self.widgets['confidence'].value
            
            # Filter signals
            filtered_signals = self._filter_signals(
                symbol, start_date, end_date, signal_types, min_confidence
            )
            
            # Create visualizations
            fig = self._create_signal_chart(filtered_signals, symbol)
            fig.show()
            
            # Display statistics
            stats = self._calculate_statistics(filtered_signals)
            self._display_statistics(stats)
    
    def display(self):
        """Display the complete UI."""
        # Layout components
        controls = widgets.VBox([
            widgets.HBox([self.widgets['symbol'], self.widgets['confidence']]),
            widgets.HBox([self.widgets['date_start'], self.widgets['date_end']]),
            self.widgets['signal_type'],
            widgets.HBox([self.widgets['analyze_btn'], self.widgets['export_btn']])
        ])
        
        # Main dashboard
        dashboard = widgets.VBox([
            widgets.HTML("<h2>Signal Analysis Dashboard</h2>"),
            controls,
            self.widgets['output']
        ])
        
        display(dashboard)

# Example usage
if __name__ == "__main__":
    # Create sample data
    dates = pd.date_range(start='2024-01-01', periods=100, freq='D')
    signals_data = pd.DataFrame({
        'date': dates,
        'symbol': 'SPY',
        'signal': np.random.choice(['BUY', 'SELL', 'HOLD'], 100),
        'price': 400 + np.random.randn(100) * 10,
        'confidence': np.random.uniform(0.3, 1.0, 100),
        'returns': np.random.randn(100) * 0.02
    })
    
    price_data = pd.DataFrame(
        {'SPY': 400 + np.cumsum(np.random.randn(100) * 2)},
        index=dates
    )
    
    # Create and display UI
    ui = SignalAnalysisUI(signals_data, price_data)
    ui.display()
`;
      } else if (fileName === 'strategy_workbench.py') {
        content = `"""StrategyWorkbench - Main UI Framework

Button-driven Jupyter Notebook interface for strategy development.
"""

import ipywidgets as widgets
from IPython.display import display, clear_output
import pandas as pd
import numpy as np
from typing import Dict, Any, Callable

class StrategyWorkbench:
    """Main workbench UI for strategy development."""
    
    def __init__(self):
        self.current_view = 'home'
        self.strategy_data = {}
        self.widgets = {}
        self._initialize_ui()
    
    def _initialize_ui(self):
        """Initialize the main UI components."""
        # Navigation bar
        self.widgets['nav_home'] = widgets.Button(description='Home', button_style='info')
        self.widgets['nav_data'] = widgets.Button(description='Data', button_style='info')
        self.widgets['nav_strategy'] = widgets.Button(description='Strategy', button_style='info')
        self.widgets['nav_backtest'] = widgets.Button(description='Backtest', button_style='info')
        self.widgets['nav_deploy'] = widgets.Button(description='Deploy', button_style='info')
        
        # Bind navigation
        self.widgets['nav_home'].on_click(lambda b: self._switch_view('home'))
        self.widgets['nav_data'].on_click(lambda b: self._switch_view('data'))
        self.widgets['nav_strategy'].on_click(lambda b: self._switch_view('strategy'))
        self.widgets['nav_backtest'].on_click(lambda b: self._switch_view('backtest'))
        self.widgets['nav_deploy'].on_click(lambda b: self._switch_view('deploy'))
        
        # Main content area
        self.widgets['content'] = widgets.Output()
        
        # Status bar
        self.widgets['status'] = widgets.HTML(value='<b>Status:</b> Ready')
    
    def display(self):
        """Display the complete workbench."""
        navbar = widgets.HBox([
            self.widgets['nav_home'],
            self.widgets['nav_data'],
            self.widgets['nav_strategy'],
            self.widgets['nav_backtest'],
            self.widgets['nav_deploy']
        ])
        
        main_ui = widgets.VBox([
            navbar,
            self.widgets['content'],
            self.widgets['status']
        ])
        
        display(main_ui)

# Initialize workbench
workbench = StrategyWorkbench()
workbench.display()
`;
      } else if (fileName === 'components.py') {
        content = `"""Reusable UI Components for StrategyWorkbench"""

import ipywidgets as widgets
from typing import List, Dict, Any, Optional

class DataSelector(widgets.VBox):
    """Widget for selecting and loading data."""
    
    def __init__(self, data_sources: List[str]):
        self.source_dropdown = widgets.Dropdown(
            options=data_sources,
            description='Data Source:'
        )
        self.load_button = widgets.Button(
            description='Load Data',
            button_style='primary'
        )
        
        super().__init__([self.source_dropdown, self.load_button])

class StrategyBuilder(widgets.VBox):
    """Widget for building trading strategies."""
    
    def __init__(self):
        self.strategy_type = widgets.Dropdown(
            options=['Moving Average', 'Mean Reversion', 'Momentum', 'Custom'],
            description='Strategy Type:'
        )
        self.parameters = widgets.Textarea(
            value='{}',
            description='Parameters:',
            layout=widgets.Layout(width='100%', height='100px')
        )
        self.validate_button = widgets.Button(
            description='Validate',
            button_style='warning'
        )
        
        super().__init__([self.strategy_type, self.parameters, self.validate_button])
`;
      } else if (fileName === 'config.json') {
        content = `{
  "workbench": {
    "name": "StrategyWorkbench",
    "version": "1.0.0",
    "description": "Button-driven Jupyter Notebook interface for strategy development"
  },
  "modules": {
    "signal_analysis": {
      "enabled": true,
      "default_params": {
        "lookback_period": 30,
        "confidence_threshold": 0.5,
        "signal_types": ["BUY", "SELL"]
      }
    }
  }
}`;
      }
    } else if (filePath.includes('saved_notebooks/')) {
      // For notebook files, provide a different format
      content = `{
  "cells": [
    {
      "cell_type": "markdown",
      "metadata": {},
      "source": [
        "# ${fileName.replace('.ipynb', '').replace('_', ' ').toUpperCase()}\\n",
        "\\n",
        "Research notebook for strategy analysis and backtesting."
      ]
    },
    {
      "cell_type": "code",
      "execution_count": null,
      "metadata": {},
      "outputs": [],
      "source": [
        "# Import required libraries\\n",
        "import pandas as pd\\n",
        "import numpy as np\\n",
        "import matplotlib.pyplot as plt\\n",
        "import admf\\n",
        "from analysis_lib import *"
      ]
    }
  ],
  "metadata": {
    "kernelspec": {
      "display_name": "Python 3",
      "language": "python",
      "name": "python3"
    }
  },
  "nbformat": 4,
  "nbformat_minor": 4
}`;
    }
  } else {
    // Default content for other files
    content = `# ${fileName}
# This is a placeholder for the actual file content
# Content will be loaded from the backend

def main():
    print("AlphaPulse Trading Strategy")
    
if __name__ == "__main__":
    main()
`;
  }
  
  // Add new tab
  const newTab: Tab = {
    id: filePath,
    name: fileName,
    content,
    language: fileName.endsWith('.py') ? 'python' : 
              fileName.endsWith('.yaml') || fileName.endsWith('.yml') ? 'yaml' :
              fileName.endsWith('.json') ? 'json' :
              fileName.endsWith('.ipynb') ? 'json' :
              fileName.endsWith('.md') ? 'markdown' : 'text'
  };
  
  setTabs([...tabs, newTab]);
  setActiveTab(filePath);
}