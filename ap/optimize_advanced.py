#!/usr/bin/env python3
"""
Advanced optimization with walk-forward analysis and Optuna integration.
"""

from decimal import Decimal
from pathlib import Path
from datetime import datetime, timedelta
import pandas as pd
import numpy as np

try:
    import optuna
    OPTUNA_AVAILABLE = True
except ImportError:
    OPTUNA_AVAILABLE = False
    print("Optuna not installed. Install with: pip install optuna")

from nautilus_trader.backtest.engine import BacktestEngine, BacktestEngineConfig
from nautilus_trader.config import LoggingConfig
from nautilus_trader.model.currencies import USD
from nautilus_trader.model.data import Bar, BarType
from nautilus_trader.model.enums import AccountType, OmsType
from nautilus_trader.model.identifiers import InstrumentId, Symbol, Venue
from nautilus_trader.model.instruments import Equity
from nautilus_trader.model.objects import Money, Price, Quantity
from nautilus_trader.persistence.catalog import ParquetDataCatalog

import sys
sys.path.insert(0, str(Path(__file__).parent / "nt_reference"))
from examples.strategies.ema_cross import EMACross, EMACrossConfig


def calculate_sharpe_ratio(returns, periods_per_year=252*390):  # 390 minutes per trading day
    """Calculate Sharpe ratio from returns."""
    if len(returns) < 2:
        return 0
    
    avg_return = np.mean(returns)
    std_return = np.std(returns)
    
    if std_return == 0:
        return 0
    
    return np.sqrt(periods_per_year) * avg_return / std_return


def run_backtest_period(bars, fast_period, slow_period, trade_size=100):
    """Run backtest for a specific period with given parameters."""
    
    # Setup
    venue = Venue("ALPACA")
    instrument_id = InstrumentId(Symbol("NVDA"), venue)
    bar_type = BarType.from_str("NVDA.ALPACA-1-MINUTE-LAST-EXTERNAL")
    
    # Configure engine
    config = BacktestEngineConfig(
        trader_id=f"OPT-{fast_period}-{slow_period}",
        logging=LoggingConfig(log_level="ERROR"),
    )
    
    engine = BacktestEngine(config=config)
    
    # Add venue
    engine.add_venue(
        venue=venue,
        oms_type=OmsType.HEDGING,
        account_type=AccountType.MARGIN,
        base_currency=USD,
        starting_balances=[Money(100_000, USD)],
    )
    
    # Create instrument
    instrument = Equity(
        instrument_id=instrument_id,
        raw_symbol=Symbol("NVDA"),
        currency=USD,
        price_precision=2,
        price_increment=Price(0.01, 2),
        lot_size=Quantity.from_int(1),
        isin=None,
        ts_event=0,
        ts_init=0,
    )
    
    engine.add_instrument(instrument)
    engine.add_data(bars)
    
    # Configure strategy
    strategy_config = EMACrossConfig(
        instrument_id=instrument_id,
        bar_type=bar_type,
        fast_ema_period=fast_period,
        slow_ema_period=slow_period,
        trade_size=Decimal(trade_size),
        request_bars=False,
        subscribe_trade_ticks=False,
        subscribe_quote_ticks=False,
    )
    
    strategy = EMACross(config=strategy_config)
    engine.add_strategy(strategy)
    
    # Run
    engine.run()
    
    # Calculate metrics
    account = engine.cache.accounts()[0]
    positions = engine.cache.positions_closed()
    
    final_balance = float(account.balance_total(USD))
    pnl = final_balance - 100_000
    
    # Calculate returns for Sharpe ratio
    returns = []
    if positions:
        for pos in positions:
            ret = pos.realized_return
            if ret:
                returns.append(ret)
    
    metrics = {
        'pnl': pnl,
        'pnl_pct': (pnl / 100_000) * 100,
        'num_trades': len(positions),
        'sharpe': calculate_sharpe_ratio(returns) if returns else 0,
        'win_rate': len([p for p in positions if p.realized_pnl.as_double() > 0]) / len(positions) * 100 if positions else 0
    }
    
    return metrics


def objective(trial, train_bars, test_bars):
    """Optuna objective function for hyperparameter optimization."""
    
    # Suggest parameters
    fast_period = trial.suggest_int('fast_period', 5, 30)
    slow_period = trial.suggest_int('slow_period', fast_period + 5, 60)
    
    # Run backtest on training data
    train_metrics = run_backtest_period(train_bars, fast_period, slow_period)
    
    # Early stopping if no trades
    if train_metrics['num_trades'] < 10:
        return -1000  # Penalty for too few trades
    
    # Optimize for Sharpe ratio (could also use pnl_pct)
    return train_metrics['sharpe']


def walk_forward_optimization(bars, train_days=20, test_days=5, step_days=5):
    """
    Walk-forward optimization to avoid overfitting.
    
    Parameters
    ----------
    bars : list
        All available bars
    train_days : int
        Number of days for training period
    test_days : int
        Number of days for testing period
    step_days : int
        Number of days to step forward each iteration
    """
    
    results = []
    
    # Convert bars to DataFrame for easier date filtering
    bar_data = []
    for bar in bars:
        bar_data.append({
            'timestamp': bar.ts_event,
            'date': datetime.fromtimestamp(bar.ts_event / 1e9).date(),
            'bar': bar
        })
    df_bars = pd.DataFrame(bar_data)
    
    # Get unique dates
    unique_dates = sorted(df_bars['date'].unique())
    
    # Walk forward
    start_idx = 0
    while start_idx + train_days + test_days <= len(unique_dates):
        train_start_date = unique_dates[start_idx]
        train_end_date = unique_dates[start_idx + train_days - 1]
        test_start_date = unique_dates[start_idx + train_days]
        test_end_date = unique_dates[start_idx + train_days + test_days - 1]
        
        print(f"\nWalk-forward period:")
        print(f"  Train: {train_start_date} to {train_end_date}")
        print(f"  Test:  {test_start_date} to {test_end_date}")
        
        # Get train and test bars
        train_bars = df_bars[
            (df_bars['date'] >= train_start_date) & 
            (df_bars['date'] <= train_end_date)
        ]['bar'].tolist()
        
        test_bars = df_bars[
            (df_bars['date'] >= test_start_date) & 
            (df_bars['date'] <= test_end_date)
        ]['bar'].tolist()
        
        if OPTUNA_AVAILABLE:
            # Use Optuna for optimization
            study = optuna.create_study(direction='maximize')
            study.optimize(
                lambda trial: objective(trial, train_bars, test_bars),
                n_trials=50,
                show_progress_bar=False
            )
            
            best_params = study.best_params
            print(f"  Best params: Fast={best_params['fast_period']}, Slow={best_params['slow_period']}")
            
            # Test on out-of-sample data
            test_metrics = run_backtest_period(
                test_bars,
                best_params['fast_period'],
                best_params['slow_period']
            )
        else:
            # Simple grid search if Optuna not available
            best_sharpe = -np.inf
            best_params = None
            
            for fast in [5, 10, 15, 20]:
                for slow in [20, 30, 40, 50]:
                    if fast >= slow:
                        continue
                    
                    train_metrics = run_backtest_period(train_bars, fast, slow)
                    if train_metrics['sharpe'] > best_sharpe:
                        best_sharpe = train_metrics['sharpe']
                        best_params = {'fast_period': fast, 'slow_period': slow}
            
            print(f"  Best params: Fast={best_params['fast_period']}, Slow={best_params['slow_period']}")
            
            # Test on out-of-sample data
            test_metrics = run_backtest_period(
                test_bars,
                best_params['fast_period'],
                best_params['slow_period']
            )
        
        results.append({
            'train_start': train_start_date,
            'train_end': train_end_date,
            'test_start': test_start_date,
            'test_end': test_end_date,
            'fast_period': best_params['fast_period'],
            'slow_period': best_params['slow_period'],
            'test_pnl_pct': test_metrics['pnl_pct'],
            'test_sharpe': test_metrics['sharpe'],
            'test_trades': test_metrics['num_trades']
        })
        
        print(f"  Test P&L: {test_metrics['pnl_pct']:.2f}%")
        
        start_idx += step_days
    
    return pd.DataFrame(results)


def main():
    """Run advanced optimization."""
    
    print("\n" + "="*60)
    print("ADVANCED EMA CROSS OPTIMIZATION")
    print("="*60)
    
    # Load data
    catalog_path = Path.cwd() / "catalog"
    catalog = ParquetDataCatalog(catalog_path)
    
    bar_type_str = "NVDA.ALPACA-1-MINUTE-LAST-EXTERNAL"
    bars = catalog.query(
        data_cls=Bar,
        identifiers=[bar_type_str],
    )
    
    print(f"\nLoaded {len(bars)} bars")
    
    # Option 1: Simple optimization on all data
    print("\n1. Full Period Optimization")
    print("-" * 30)
    
    if OPTUNA_AVAILABLE:
        study = optuna.create_study(direction='maximize')
        study.optimize(
            lambda trial: objective(trial, bars, bars),
            n_trials=100
        )
        
        print(f"\nBest parameters found:")
        print(f"  Fast EMA: {study.best_params['fast_period']}")
        print(f"  Slow EMA: {study.best_params['slow_period']}")
        print(f"  Best Sharpe: {study.best_value:.3f}")
    else:
        print("Skipping Optuna optimization (not installed)")
    
    # Option 2: Walk-forward optimization
    print("\n2. Walk-Forward Optimization")
    print("-" * 30)
    
    # Since we only have 5 days of data, do mini walk-forward
    wf_results = walk_forward_optimization(
        bars,
        train_days=3,
        test_days=1,
        step_days=1
    )
    
    if not wf_results.empty:
        print("\nWalk-Forward Results Summary:")
        print(wf_results.to_string())
        
        avg_test_pnl = wf_results['test_pnl_pct'].mean()
        print(f"\nAverage out-of-sample P&L: {avg_test_pnl:.2f}%")
        
        # Save results
        wf_results.to_csv("walk_forward_results.csv", index=False)
        print("\nResults saved to: walk_forward_results.csv")
    
    print("\n" + "="*60)
    print("RECOMMENDATIONS")
    print("="*60)
    print("\n1. Grid Search: Good for small parameter spaces")
    print("   - Use optimize_ema_cross.py for simple grid search")
    print("   - Fast with parallel processing")
    print("\n2. Optuna: Better for larger parameter spaces")
    print("   - Install with: pip install optuna")
    print("   - Uses Bayesian optimization")
    print("   - Can handle many parameters efficiently")
    print("\n3. Walk-Forward: Best for avoiding overfitting")
    print("   - Tests on out-of-sample data")
    print("   - More realistic performance estimates")
    print("\n4. For production:")
    print("   - Always use walk-forward or cross-validation")
    print("   - Test on multiple symbols")
    print("   - Include transaction costs")
    print("   - Consider regime changes")
    
    print("\n" + "="*60)


if __name__ == "__main__":
    main()