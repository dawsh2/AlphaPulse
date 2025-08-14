"""
Jupyter notebook utilities for AlphaPulse real-time market data visualization.

Provides interactive widgets and real-time plotting for market data analysis.
"""

import time
import asyncio
from typing import List, Dict, Any, Optional
from dataclasses import dataclass
from collections import deque
import logging

try:
    import matplotlib.pyplot as plt
    import matplotlib.animation as animation
    from matplotlib.dates import DateFormatter
    import pandas as pd
    import numpy as np
    HAS_MATPLOTLIB = True
except ImportError:
    HAS_MATPLOTLIB = False

try:
    import plotly.graph_objects as go
    from plotly.subplots import make_subplots
    import plotly.express as px
    from IPython.display import display, clear_output
    import ipywidgets as widgets
    HAS_PLOTLY = True
except ImportError:
    HAS_PLOTLY = False

from . import PyTrade, PyOrderBookDelta, PyOrderBook, DataStream

logger = logging.getLogger(__name__)

@dataclass
class DisplayConfig:
    """Configuration for Jupyter displays"""
    max_points: int = 1000
    update_interval_ms: int = 100
    auto_scroll: bool = True
    show_volume: bool = True
    show_spread: bool = True

class JupyterDisplay:
    """
    Interactive Jupyter notebook display for real-time market data.
    
    Provides live updating charts, orderbook visualization, and performance metrics.
    """
    
    def __init__(self, config: Optional[DisplayConfig] = None):
        if not HAS_PLOTLY:
            raise ImportError("plotly and ipywidgets are required for JupyterDisplay. "
                            "Install with: pip install plotly ipywidgets")
        
        self.config = config or DisplayConfig()
        self.trade_data = deque(maxlen=self.config.max_points)
        self.orderbook_data = {}
        self.metrics_data = deque(maxlen=100)
        self._widgets = {}
        self._figures = {}
        
    def create_trade_monitor(self, exchanges: List[str], symbols: List[str]) -> widgets.VBox:
        """
        Create an interactive trade monitoring widget.
        
        Args:
            exchanges: List of exchanges to monitor
            symbols: List of symbols to monitor
            
        Returns:
            IPython widget for trade monitoring
        """
        # Control widgets
        exchange_dropdown = widgets.Dropdown(
            options=exchanges,
            value=exchanges[0] if exchanges else None,
            description='Exchange:'
        )
        
        symbol_dropdown = widgets.Dropdown(
            options=symbols,
            value=symbols[0] if symbols else None,
            description='Symbol:'
        )
        
        start_button = widgets.Button(
            description='Start Monitoring',
            button_style='success'
        )
        
        stop_button = widgets.Button(
            description='Stop Monitoring',
            button_style='danger',
            disabled=True
        )
        
        # Metrics display
        metrics_html = widgets.HTML(value="<h3>Waiting for data...</h3>")
        
        # Trade plot
        trade_fig = go.FigureWidget()
        trade_fig.add_trace(go.Scatter(
            x=[], y=[], mode='markers+lines',
            name='Price', line=dict(color='blue')
        ))
        trade_fig.update_layout(
            title='Real-Time Trade Prices',
            xaxis_title='Time',
            yaxis_title='Price',
            height=400
        )
        
        # Volume plot
        volume_fig = go.FigureWidget()
        volume_fig.add_trace(go.Bar(
            x=[], y=[], name='Volume',
            marker_color='green'
        ))
        volume_fig.update_layout(
            title='Trade Volume',
            xaxis_title='Time',
            yaxis_title='Volume',
            height=300
        )
        
        self._widgets['trade_monitor'] = {
            'exchange_dropdown': exchange_dropdown,
            'symbol_dropdown': symbol_dropdown,
            'start_button': start_button,
            'stop_button': stop_button,
            'metrics_html': metrics_html,
            'trade_fig': trade_fig,
            'volume_fig': volume_fig,
            'monitoring': False
        }
        
        # Button callbacks
        def start_monitoring(button):
            self._start_trade_monitoring()
            
        def stop_monitoring(button):
            self._stop_trade_monitoring()
            
        start_button.on_click(start_monitoring)
        stop_button.on_click(stop_monitoring)
        
        # Layout
        controls = widgets.HBox([exchange_dropdown, symbol_dropdown, start_button, stop_button])
        plots = widgets.VBox([trade_fig, volume_fig])
        
        return widgets.VBox([
            widgets.HTML("<h2>📈 Real-Time Trade Monitor</h2>"),
            controls,
            metrics_html,
            plots
        ])
        
    def create_orderbook_viewer(self, exchange: str, symbol: str) -> widgets.VBox:
        """
        Create an interactive orderbook depth viewer.
        
        Args:
            exchange: Exchange to monitor
            symbol: Symbol to monitor
            
        Returns:
            IPython widget for orderbook visualization
        """
        # Orderbook plot
        orderbook_fig = go.FigureWidget()
        
        # Bid side (green)
        orderbook_fig.add_trace(go.Bar(
            x=[], y=[], orientation='h',
            name='Bids', marker_color='green',
            opacity=0.7
        ))
        
        # Ask side (red)  
        orderbook_fig.add_trace(go.Bar(
            x=[], y=[], orientation='h',
            name='Asks', marker_color='red',
            opacity=0.7
        ))
        
        orderbook_fig.update_layout(
            title=f'Order Book Depth - {exchange} {symbol}',
            xaxis_title='Cumulative Volume',
            yaxis_title='Price',
            height=500,
            barmode='relative'
        )
        
        # Spread and metrics
        spread_html = widgets.HTML(value="<p><b>Spread:</b> Loading...</p>")
        depth_html = widgets.HTML(value="<p><b>Depth:</b> Loading...</p>")
        
        self._widgets['orderbook_viewer'] = {
            'orderbook_fig': orderbook_fig,
            'spread_html': spread_html,
            'depth_html': depth_html,
            'exchange': exchange,
            'symbol': symbol
        }
        
        return widgets.VBox([
            widgets.HTML(f"<h2>📊 Order Book - {exchange} {symbol}</h2>"),
            widgets.HBox([spread_html, depth_html]),
            orderbook_fig
        ])
        
    def create_arbitrage_monitor(self, symbols: List[str]) -> widgets.VBox:
        """
        Create an arbitrage opportunity monitor.
        
        Args:
            symbols: List of symbols to monitor for arbitrage
            
        Returns:
            IPython widget for arbitrage monitoring
        """
        # Arbitrage opportunities table
        opportunities_html = widgets.HTML(
            value="<h3>No arbitrage opportunities detected</h3>"
        )
        
        # Profit chart
        profit_fig = go.FigureWidget()
        profit_fig.add_trace(go.Scatter(
            x=[], y=[], mode='markers',
            name='Arbitrage Profit (bps)',
            marker=dict(
                size=10,
                color=[],
                colorscale='viridis',
                showscale=True,
                colorbar=dict(title="Profit (bps)")
            )
        ))
        profit_fig.update_layout(
            title='Arbitrage Opportunities Over Time',
            xaxis_title='Time',
            yaxis_title='Profit (basis points)',
            height=400
        )
        
        # Symbol selector
        symbol_dropdown = widgets.Dropdown(
            options=symbols,
            value=symbols[0] if symbols else None,
            description='Symbol:'
        )
        
        # Min profit threshold
        min_profit_slider = widgets.FloatSlider(
            value=1.0,
            min=0.1,
            max=10.0,
            step=0.1,
            description='Min Profit (bps):',
            style={'description_width': 'initial'}
        )
        
        self._widgets['arbitrage_monitor'] = {
            'opportunities_html': opportunities_html,
            'profit_fig': profit_fig,
            'symbol_dropdown': symbol_dropdown,
            'min_profit_slider': min_profit_slider,
            'opportunities': []
        }
        
        # Layout
        controls = widgets.HBox([symbol_dropdown, min_profit_slider])
        
        return widgets.VBox([
            widgets.HTML("<h2>⚡ Arbitrage Opportunities</h2>"),
            controls,
            opportunities_html,
            profit_fig
        ])
        
    def create_performance_dashboard(self) -> widgets.VBox:
        """
        Create a performance monitoring dashboard.
        
        Returns:
            IPython widget for performance metrics
        """
        # Latency chart
        latency_fig = go.FigureWidget()
        latency_fig.add_trace(go.Scatter(
            x=[], y=[], mode='lines',
            name='Avg Latency (μs)',
            line=dict(color='orange')
        ))
        latency_fig.update_layout(
            title='System Latency Over Time',
            xaxis_title='Time',
            yaxis_title='Latency (microseconds)',
            height=300
        )
        
        # Throughput chart
        throughput_fig = go.FigureWidget()
        throughput_fig.add_trace(go.Scatter(
            x=[], y=[], mode='lines',
            name='Messages/sec',
            line=dict(color='purple')
        ))
        throughput_fig.update_layout(
            title='Message Throughput',
            xaxis_title='Time', 
            yaxis_title='Messages per Second',
            height=300
        )
        
        # Metrics summary
        metrics_html = widgets.HTML(value=self._format_metrics_html({}))
        
        self._widgets['performance_dashboard'] = {
            'latency_fig': latency_fig,
            'throughput_fig': throughput_fig,
            'metrics_html': metrics_html
        }
        
        return widgets.VBox([
            widgets.HTML("<h2>⚡ Performance Dashboard</h2>"),
            metrics_html,
            widgets.HBox([latency_fig, throughput_fig])
        ])
        
    def update_trades(self, trades: List[PyTrade]):
        """Update trade displays with new trade data"""
        if not trades:
            return
            
        current_time = time.time()
        
        for trade in trades:
            self.trade_data.append({
                'timestamp': current_time,
                'price': trade.price,
                'volume': trade.volume,
                'symbol': trade.symbol,
                'exchange': trade.exchange
            })
        
        # Update trade monitor if active
        if 'trade_monitor' in self._widgets:
            self._update_trade_monitor()
            
    def update_orderbook(self, orderbook: PyOrderBook):
        """Update orderbook displays with new orderbook data"""
        key = f"{orderbook.exchange}:{orderbook.symbol}"
        self.orderbook_data[key] = {
            'bids': orderbook.get_bids(),
            'asks': orderbook.get_asks(),
            'best_bid': orderbook.get_best_bid(),
            'best_ask': orderbook.get_best_ask(),
            'spread': orderbook.get_spread(),
            'timestamp': orderbook.timestamp
        }
        
        # Update orderbook viewer if active
        if 'orderbook_viewer' in self._widgets:
            self._update_orderbook_viewer()
            
    def update_arbitrage(self, opportunities: List[Dict[str, Any]]):
        """Update arbitrage displays with new opportunities"""
        if 'arbitrage_monitor' in self._widgets:
            widgets_dict = self._widgets['arbitrage_monitor']
            widgets_dict['opportunities'].extend(opportunities)
            
            # Keep only recent opportunities
            current_time = time.time()
            widgets_dict['opportunities'] = [
                opp for opp in widgets_dict['opportunities']
                if current_time - opp.get('timestamp', 0) < 300  # 5 minutes
            ]
            
            self._update_arbitrage_monitor()
            
    def update_metrics(self, metrics: Dict[str, Any]):
        """Update performance metrics displays"""
        self.metrics_data.append({
            'timestamp': time.time(),
            **metrics
        })
        
        if 'performance_dashboard' in self._widgets:
            self._update_performance_dashboard()
            
    def _update_trade_monitor(self):
        """Update trade monitor plots"""
        if not self.trade_data:
            return
            
        widgets_dict = self._widgets['trade_monitor']
        
        # Get recent trade data
        recent_trades = list(self.trade_data)[-100:]  # Last 100 trades
        
        timestamps = [t['timestamp'] for t in recent_trades]
        prices = [t['price'] for t in recent_trades]
        volumes = [t['volume'] for t in recent_trades]
        
        # Update price plot
        with widgets_dict['trade_fig'].batch_update():
            widgets_dict['trade_fig'].data[0].x = timestamps
            widgets_dict['trade_fig'].data[0].y = prices
            
        # Update volume plot
        with widgets_dict['volume_fig'].batch_update():
            widgets_dict['volume_fig'].data[0].x = timestamps
            widgets_dict['volume_fig'].data[0].y = volumes
            
        # Update metrics
        if recent_trades:
            latest = recent_trades[-1]
            avg_price = sum(prices) / len(prices)
            total_volume = sum(volumes)
            
            metrics_html = f"""
            <div style='display: flex; gap: 20px;'>
                <div><b>Latest Price:</b> ${latest['price']:.2f}</div>
                <div><b>Avg Price:</b> ${avg_price:.2f}</div>
                <div><b>Total Volume:</b> {total_volume:.4f}</div>
                <div><b>Trades Count:</b> {len(recent_trades)}</div>
            </div>
            """
            widgets_dict['metrics_html'].value = metrics_html
            
    def _update_orderbook_viewer(self):
        """Update orderbook viewer"""
        widgets_dict = self._widgets['orderbook_viewer']
        key = f"{widgets_dict['exchange']}:{widgets_dict['symbol']}"
        
        if key not in self.orderbook_data:
            return
            
        data = self.orderbook_data[key]
        
        # Update spread and depth info
        spread = data.get('spread', 0)
        best_bid = data.get('best_bid', 0)
        best_ask = data.get('best_ask', 0)
        
        widgets_dict['spread_html'].value = f"<p><b>Spread:</b> ${spread:.4f} ({(spread/best_ask*10000):.1f} bps)</p>"
        widgets_dict['depth_html'].value = f"<p><b>Best Bid:</b> ${best_bid:.2f} | <b>Best Ask:</b> ${best_ask:.2f}</p>"
        
    def _update_arbitrage_monitor(self):
        """Update arbitrage opportunity monitor"""
        widgets_dict = self._widgets['arbitrage_monitor']
        opportunities = widgets_dict['opportunities']
        
        if not opportunities:
            widgets_dict['opportunities_html'].value = "<h3>No arbitrage opportunities detected</h3>"
            return
            
        # Format opportunities table
        table_html = "<table border='1' style='border-collapse: collapse; width: 100%;'>"
        table_html += "<tr><th>Symbol</th><th>Buy Exchange</th><th>Sell Exchange</th><th>Profit (bps)</th><th>Time</th></tr>"
        
        for opp in opportunities[-10:]:  # Show last 10
            table_html += f"""
            <tr>
                <td>{opp.get('symbol', 'N/A')}</td>
                <td>{opp.get('buy_exchange', 'N/A')}</td>
                <td>{opp.get('sell_exchange', 'N/A')}</td>
                <td>{opp.get('profit_bps', 0):.2f}</td>
                <td>{time.strftime('%H:%M:%S', time.localtime(opp.get('timestamp', 0)))}</td>
            </tr>
            """
        table_html += "</table>"
        
        widgets_dict['opportunities_html'].value = table_html
        
    def _update_performance_dashboard(self):
        """Update performance dashboard"""
        if not self.metrics_data:
            return
            
        widgets_dict = self._widgets['performance_dashboard']
        
        # Get recent metrics
        recent_metrics = list(self.metrics_data)[-50:]  # Last 50 data points
        
        timestamps = [m['timestamp'] for m in recent_metrics]
        latencies = [m.get('avg_latency_us', 0) for m in recent_metrics]
        
        # Calculate throughput (messages per second)
        throughput = []
        for i, m in enumerate(recent_metrics):
            if i > 0:
                time_diff = m['timestamp'] - recent_metrics[i-1]['timestamp']
                trades_diff = m.get('trades_processed', 0) - recent_metrics[i-1].get('trades_processed', 0)
                if time_diff > 0:
                    throughput.append(trades_diff / time_diff)
                else:
                    throughput.append(0)
            else:
                throughput.append(0)
        
        # Update latency plot
        with widgets_dict['latency_fig'].batch_update():
            widgets_dict['latency_fig'].data[0].x = timestamps
            widgets_dict['latency_fig'].data[0].y = latencies
            
        # Update throughput plot
        with widgets_dict['throughput_fig'].batch_update():
            widgets_dict['throughput_fig'].data[0].x = timestamps
            widgets_dict['throughput_fig'].data[0].y = throughput
            
        # Update metrics summary
        if recent_metrics:
            latest = recent_metrics[-1]
            widgets_dict['metrics_html'].value = self._format_metrics_html(latest)
            
    def _format_metrics_html(self, metrics: Dict[str, Any]) -> str:
        """Format metrics as HTML"""
        if not metrics:
            return "<p>Waiting for metrics data...</p>"
            
        return f"""
        <div style='display: grid; grid-template-columns: repeat(4, 1fr); gap: 15px; padding: 10px; background: #f0f0f0; border-radius: 5px;'>
            <div><b>Avg Latency:</b> {metrics.get('avg_latency_us', 0):.1f} μs</div>
            <div><b>Trades Processed:</b> {metrics.get('trades_processed', 0):,}</div>
            <div><b>Deltas Processed:</b> {metrics.get('deltas_processed', 0):,}</div>
            <div><b>Arbitrage Opps:</b> {metrics.get('arbitrage_opportunities', 0)}</div>
        </div>
        """
        
    def _start_trade_monitoring(self):
        """Start trade monitoring"""
        widgets_dict = self._widgets['trade_monitor']
        widgets_dict['monitoring'] = True
        widgets_dict['start_button'].disabled = True
        widgets_dict['stop_button'].disabled = False
        
    def _stop_trade_monitoring(self):
        """Stop trade monitoring"""
        widgets_dict = self._widgets['trade_monitor']
        widgets_dict['monitoring'] = False
        widgets_dict['start_button'].disabled = False
        widgets_dict['stop_button'].disabled = True

# Convenience functions for quick setup

def quick_trade_monitor(exchanges: List[str], symbols: List[str]) -> widgets.VBox:
    """Quick setup for trade monitoring widget"""
    display = JupyterDisplay()
    return display.create_trade_monitor(exchanges, symbols)

def quick_orderbook_viewer(exchange: str, symbol: str) -> widgets.VBox:
    """Quick setup for orderbook viewer widget"""
    display = JupyterDisplay()
    return display.create_orderbook_viewer(exchange, symbol)

def quick_arbitrage_monitor(symbols: List[str]) -> widgets.VBox:
    """Quick setup for arbitrage monitoring widget"""
    display = JupyterDisplay()
    return display.create_arbitrage_monitor(symbols)

def quick_performance_dashboard() -> widgets.VBox:
    """Quick setup for performance dashboard widget"""
    display = JupyterDisplay()
    return display.create_performance_dashboard()