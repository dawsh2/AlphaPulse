#!/usr/bin/env python3
"""
Mixin class to add signal recording to any strategy.
"""

import pandas as pd
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Any


class SignalRecorderMixin:
    """
    Mixin to add signal recording capabilities to any strategy.
    
    Usage:
        class MyStrategy(Strategy, SignalRecorderMixin):
            def on_bar(self, bar: Bar) -> None:
                # Your strategy logic
                signal = calculate_signal()
                
                # Record the signal
                self.record_signal(bar, {
                    'signal': signal,
                    'indicator_1': self.indicator_1.value,
                    'indicator_2': self.indicator_2.value,
                    'custom_metric': some_calculation,
                })
    """
    
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self._signal_records: List[Dict[str, Any]] = []
        
    def record_signal(self, bar, custom_data: Dict[str, Any] = None) -> None:
        """
        Record signal data for later analysis.
        
        Parameters
        ----------
        bar : Bar
            The current bar
        custom_data : Dict[str, Any]
            Custom data to record (signal values, indicators, etc.)
        """
        record = {
            # Standard bar data
            'timestamp': bar.ts_event,
            'datetime': datetime.fromtimestamp(bar.ts_event / 1e9),
            'symbol': str(self.config.instrument_id.symbol),
            'open': float(bar.open),
            'high': float(bar.high),
            'low': float(bar.low),
            'close': float(bar.close),
            'volume': int(bar.volume),
            
            # Strategy info
            'strategy_id': str(self.id),
        }
        
        # Add position info if available
        positions = self.cache.positions_open(
            venue=self.config.instrument_id.venue,
            instrument_id=self.config.instrument_id,
            strategy_id=self.id,
        )
        
        if positions:
            position = positions[0]
            record['position_size'] = float(position.quantity)
            record['position_side'] = position.side.name
            record['unrealized_pnl'] = float(position.unrealized_pnl(bar.close).as_double())
            record['position_id'] = str(position.id)
        else:
            record['position_size'] = 0.0
            record['position_side'] = 'FLAT'
            record['unrealized_pnl'] = 0.0
            record['position_id'] = None
            
        # Add custom data
        if custom_data:
            record.update(custom_data)
            
        self._signal_records.append(record)
        
    def save_signals(self, filepath: str = None, format: str = 'parquet') -> str:
        """
        Save recorded signals to file.
        
        Parameters
        ----------
        filepath : str, optional
            Output filepath. If None, auto-generates based on strategy ID
        format : str
            Output format: 'parquet', 'csv', or 'json'
            
        Returns
        -------
        str
            Path to saved file
        """
        if not self._signal_records:
            self.log.warning("No signals recorded to save")
            return None
            
        # Convert to DataFrame
        df = pd.DataFrame(self._signal_records)
        
        # Auto-generate filepath if needed
        if filepath is None:
            timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
            output_dir = Path("signal_traces")
            output_dir.mkdir(exist_ok=True)
            
            if format == 'parquet':
                filepath = output_dir / f"{self.id}_signals_{timestamp}.parquet"
            elif format == 'csv':
                filepath = output_dir / f"{self.id}_signals_{timestamp}.csv"
            elif format == 'json':
                filepath = output_dir / f"{self.id}_signals_{timestamp}.json"
            else:
                raise ValueError(f"Unknown format: {format}")
        
        # Save based on format
        if format == 'parquet':
            df.to_parquet(filepath, index=False, compression='snappy')
        elif format == 'csv':
            df.to_csv(filepath, index=False)
        elif format == 'json':
            df.to_json(filepath, orient='records', date_format='iso')
            
        self.log.info(f"Saved {len(df)} signal records to {filepath}")
        
        # Also save metadata
        metadata = {
            'strategy_id': str(self.id),
            'strategy_class': self.__class__.__name__,
            'instrument': str(self.config.instrument_id),
            'total_records': len(df),
            'date_range': f"{df['datetime'].min()} to {df['datetime'].max()}",
            'columns': list(df.columns),
        }
        
        # Add any config parameters
        if hasattr(self.config, '__dict__'):
            config_dict = {}
            for key, value in self.config.__dict__.items():
                if not key.startswith('_'):
                    try:
                        config_dict[key] = str(value)
                    except:
                        pass
            metadata['config'] = config_dict
        
        metadata_file = Path(filepath).with_suffix('.metadata.json')
        pd.Series(metadata).to_json(metadata_file, indent=2)
        
        return str(filepath)
        
    def get_signals_df(self) -> pd.DataFrame:
        """Get recorded signals as DataFrame."""
        if not self._signal_records:
            return pd.DataFrame()
        return pd.DataFrame(self._signal_records)
        
    def clear_signals(self) -> None:
        """Clear all recorded signals."""
        self._signal_records.clear()


# Example usage with existing strategy
def example_usage():
    """
    Example of how to use SignalRecorderMixin with existing strategy.
    """
    from nautilus_trader.trading.strategy import Strategy
    
    # Method 1: Create a wrapper class
    class EMACrossWithRecording(EMACross, SignalRecorderMixin):
        def on_bar(self, bar: Bar) -> None:
            # Call parent on_bar
            super().on_bar(bar)
            
            # Record signal data
            if self.fast_ema.initialized and self.slow_ema.initialized:
                self.record_signal(bar, {
                    'fast_ema': float(self.fast_ema.value),
                    'slow_ema': float(self.slow_ema.value),
                    'ema_diff': float(self.fast_ema.value - self.slow_ema.value),
                    'signal': 1 if self.fast_ema.value > self.slow_ema.value else -1,
                })
        
        def on_stop(self) -> None:
            # Save signals when strategy stops
            super().on_stop()
            self.save_signals(format='parquet')
    
    # Method 2: Monkey patch existing strategy
    def add_signal_recording(strategy_class):
        """Decorator to add signal recording to any strategy."""
        
        # Save original on_bar
        original_on_bar = strategy_class.on_bar
        
        def new_on_bar(self, bar):
            # Call original
            result = original_on_bar(bar)
            
            # Add your signal recording logic here
            # This is strategy-specific
            
            return result
        
        # Patch the class
        strategy_class.on_bar = new_on_bar
        
        # Add mixin methods
        for attr in dir(SignalRecorderMixin):
            if not attr.startswith('_'):
                setattr(strategy_class, attr, getattr(SignalRecorderMixin, attr))
        
        return strategy_class