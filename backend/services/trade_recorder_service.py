#!/usr/bin/env python3
"""
Trade Recorder Service Manager
Manages the WebSocket trade recorder as a background service
"""

import os
import sys
import time
import subprocess
import signal
import json
import duckdb
import pandas as pd
from datetime import datetime, timedelta
from pathlib import Path

class TradeRecorderManager:
    """Manages the trade recorder service"""
    
    def __init__(self):
        self.service_dir = Path(__file__).parent
        self.pid_file = self.service_dir / 'trade_recorder.pid'
        self.log_file = self.service_dir / 'trade_recorder.log'
        self.db_path = self.service_dir.parent / 'market_data' / 'market_data.duckdb'
    
    def start(self):
        """Start the trade recorder service"""
        if self.is_running():
            print("❌ Trade recorder is already running")
            return False
        
        print("Starting trade recorder service...")
        
        # Start the recorder in background
        process = subprocess.Popen(
            [sys.executable, 'websocket_recorder.py'],
            cwd=self.service_dir,
            stdout=open(self.log_file, 'a'),
            stderr=subprocess.STDOUT,
            preexec_fn=os.setsid if sys.platform != 'win32' else None
        )
        
        # Save PID
        with open(self.pid_file, 'w') as f:
            f.write(str(process.pid))
        
        time.sleep(2)  # Give it time to start
        
        if self.is_running():
            print("✅ Trade recorder started successfully")
            print(f"   PID: {process.pid}")
            print(f"   Log: {self.log_file}")
            return True
        else:
            print("❌ Failed to start trade recorder")
            return False
    
    def stop(self):
        """Stop the trade recorder service"""
        if not self.is_running():
            print("❌ Trade recorder is not running")
            return False
        
        try:
            with open(self.pid_file, 'r') as f:
                pid = int(f.read())
            
            print(f"Stopping trade recorder (PID: {pid})...")
            
            # Send SIGTERM for graceful shutdown
            os.kill(pid, signal.SIGTERM)
            
            # Wait for process to stop
            for _ in range(10):
                if not self.is_running():
                    break
                time.sleep(1)
            
            # Force kill if still running
            if self.is_running():
                os.kill(pid, signal.SIGKILL)
            
            # Clean up PID file
            if self.pid_file.exists():
                self.pid_file.unlink()
            
            print("✅ Trade recorder stopped")
            return True
            
        except Exception as e:
            print(f"❌ Error stopping trade recorder: {e}")
            return False
    
    def restart(self):
        """Restart the trade recorder service"""
        print("Restarting trade recorder...")
        self.stop()
        time.sleep(2)
        return self.start()
    
    def is_running(self):
        """Check if the service is running"""
        if not self.pid_file.exists():
            return False
        
        try:
            with open(self.pid_file, 'r') as f:
                pid = int(f.read())
            
            # Check if process exists
            os.kill(pid, 0)
            return True
        except (OSError, ValueError):
            # Process doesn't exist or invalid PID
            if self.pid_file.exists():
                self.pid_file.unlink()
            return False
    
    def status(self):
        """Get service status and statistics"""
        print("\n" + "=" * 60)
        print("TRADE RECORDER STATUS")
        print("=" * 60)
        
        # Check if running
        if self.is_running():
            with open(self.pid_file, 'r') as f:
                pid = f.read()
            print(f"Status: ✅ RUNNING (PID: {pid})")
        else:
            print("Status: ❌ STOPPED")
        
        # Get database statistics
        try:
            conn = duckdb.connect(str(self.db_path), read_only=True)
            
            # Get trade counts
            stats = conn.execute("""
                SELECT 
                    exchange,
                    COUNT(*) as total_trades,
                    COUNT(DISTINCT DATE_TRUNC('day', datetime)) as days_of_data,
                    MIN(datetime) as first_trade,
                    MAX(datetime) as last_trade
                FROM trades
                WHERE datetime >= CURRENT_TIMESTAMP - INTERVAL '7 days'
                GROUP BY exchange
                ORDER BY exchange
            """).fetchall()
            
            print("\nRecent Trade Statistics (Last 7 Days):")
            print("-" * 40)
            
            for row in stats:
                exchange, count, days, first, last = row
                print(f"\n{exchange.upper()}:")
                print(f"  Total trades: {count:,}")
                print(f"  Days of data: {days}")
                print(f"  First trade: {first}")
                print(f"  Last trade: {last}")
            
            # Get recent activity
            recent = conn.execute("""
                SELECT 
                    exchange,
                    COUNT(*) as trades_last_hour
                FROM trades
                WHERE datetime >= CURRENT_TIMESTAMP - INTERVAL '1 hour'
                GROUP BY exchange
            """).fetchall()
            
            print("\nLast Hour Activity:")
            print("-" * 40)
            for exchange, count in recent:
                print(f"  {exchange}: {count:,} trades")
            
            conn.close()
            
        except Exception as e:
            print(f"\n⚠️  Could not get database statistics: {e}")
        
        # Show log tail
        if self.log_file.exists():
            print("\nRecent Log Entries:")
            print("-" * 40)
            try:
                with open(self.log_file, 'r') as f:
                    lines = f.readlines()
                    for line in lines[-10:]:  # Last 10 lines
                        print(f"  {line.strip()}")
            except Exception as e:
                print(f"  Could not read log: {e}")
    
    def monitor(self):
        """Live monitoring of trade recording"""
        print("Starting live monitor (Press Ctrl+C to stop)...")
        print("-" * 60)
        
        try:
            while True:
                os.system('clear' if sys.platform != 'win32' else 'cls')
                
                print("=" * 60)
                print(f"TRADE RECORDER MONITOR - {datetime.now().strftime('%H:%M:%S')}")
                print("=" * 60)
                
                # Check service status
                if not self.is_running():
                    print("⚠️  SERVICE IS NOT RUNNING")
                    print("\nRun 'python trade_recorder_service.py start' to begin recording")
                else:
                    print("✅ SERVICE IS RUNNING")
                    
                    # Get live stats
                    try:
                        conn = duckdb.connect(str(self.db_path), read_only=True)
                        
                        # Get counts for last minute
                        result = conn.execute("""
                            SELECT 
                                exchange,
                                COUNT(*) as trades,
                                ROUND(AVG(price), 2) as avg_price,
                                ROUND(MIN(price), 2) as min_price,
                                ROUND(MAX(price), 2) as max_price
                            FROM trades
                            WHERE datetime >= CURRENT_TIMESTAMP - INTERVAL '1 minute'
                                AND symbol IN ('BTC/USD', 'BTC-USD')
                            GROUP BY exchange
                        """).fetchall()
                        
                        print("\nLast Minute (BTC/USD):")
                        print("-" * 40)
                        
                        for exchange, count, avg_p, min_p, max_p in result:
                            print(f"{exchange.upper():10} | Trades: {count:4} | "
                                  f"Avg: ${avg_p:,.2f} | "
                                  f"Range: ${min_p:,.2f}-${max_p:,.2f}")
                        
                        # Get total counts
                        totals = conn.execute("""
                            SELECT 
                                exchange,
                                COUNT(*) as total
                            FROM trades
                            WHERE datetime >= CURRENT_DATE
                            GROUP BY exchange
                        """).fetchall()
                        
                        print("\nToday's Totals:")
                        print("-" * 40)
                        
                        for exchange, total in totals:
                            print(f"{exchange.upper():10} | {total:,} trades")
                        
                        conn.close()
                        
                    except Exception as e:
                        print(f"\nError getting stats: {e}")
                
                # Wait 5 seconds before refresh
                time.sleep(5)
                
        except KeyboardInterrupt:
            print("\n\nMonitor stopped")


def main():
    """Main entry point"""
    manager = TradeRecorderManager()
    
    if len(sys.argv) < 2:
        print("Usage: python trade_recorder_service.py [start|stop|restart|status|monitor]")
        sys.exit(1)
    
    command = sys.argv[1].lower()
    
    if command == 'start':
        manager.start()
    elif command == 'stop':
        manager.stop()
    elif command == 'restart':
        manager.restart()
    elif command == 'status':
        manager.status()
    elif command == 'monitor':
        manager.monitor()
    else:
        print(f"Unknown command: {command}")
        print("Available commands: start, stop, restart, status, monitor")
        sys.exit(1)


if __name__ == "__main__":
    main()