#!/usr/bin/env python3
"""
Market Data Management Script
Handles collectors, monitoring, and data exports
"""

import sys
import signal
import subprocess
import time
from pathlib import Path
import argparse
import psutil

def check_postgres():
    """Check if PostgreSQL is running"""
    try:
        result = subprocess.run(
            ['/opt/homebrew/opt/postgresql@16/bin/pg_isready'],
            capture_output=True,
            text=True
        )
        return result.returncode == 0
    except:
        return False

def check_grafana():
    """Check if Grafana is running"""
    try:
        import requests
        response = requests.get('http://localhost:3000/api/health')
        return response.status_code == 200
    except:
        return False

def start_services():
    """Start required services"""
    print("🚀 Starting Market Data Services...")
    
    # Check PostgreSQL
    if not check_postgres():
        print("Starting PostgreSQL...")
        subprocess.run(['brew', 'services', 'start', 'postgresql@16'])
        time.sleep(3)
    else:
        print("✅ PostgreSQL is running")
    
    # Check Grafana
    if not check_grafana():
        print("Starting Grafana...")
        subprocess.run(['brew', 'services', 'start', 'grafana'])
        time.sleep(3)
    else:
        print("✅ Grafana is running")
    
    print("\n📊 Grafana Dashboard: http://localhost:3000")
    print("   Default login: admin/admin")
    print("   Dashboard: AlphaPulse Market Data")

def start_collector():
    """Start the PostgreSQL collector"""
    print("\n🔄 Starting Market Data Collector...")
    
    # Start collector
    collector_path = Path(__file__).parent.parent / 'services' / 'postgres_collector.py'
    
    if not collector_path.exists():
        print(f"❌ Collector not found at {collector_path}")
        return None
    
    process = subprocess.Popen(
        [sys.executable, str(collector_path)],
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        universal_newlines=True,
        bufsize=1
    )
    
    print(f"✅ Collector started (PID: {process.pid})")
    return process

def monitor_collector(process):
    """Monitor the collector output"""
    print("\n📈 Monitoring collector output (Ctrl+C to stop)...")
    
    try:
        for line in iter(process.stdout.readline, ''):
            if line:
                print(f"   {line.strip()}")
    except KeyboardInterrupt:
        print("\n⏹️  Stopping collector...")
        process.terminate()
        process.wait(timeout=5)
        print("✅ Collector stopped")

def stop_services():
    """Stop all services"""
    print("\n🛑 Stopping services...")
    
    # Kill any python processes running collectors
    for proc in psutil.process_iter(['pid', 'name', 'cmdline']):
        try:
            if 'python' in proc.info['name']:
                cmdline = proc.info.get('cmdline', [])
                if any('collector' in str(arg) for arg in cmdline):
                    print(f"Stopping collector process {proc.info['pid']}")
                    proc.terminate()
        except:
            pass

def export_to_parquet():
    """Export recent data to Parquet"""
    import pandas as pd
    import psycopg2
    from datetime import datetime, timedelta
    
    print("\n📦 Exporting data to Parquet...")
    
    conn = psycopg2.connect(
        host='localhost',
        port=5432,
        database='market_data',
        user='daws'
    )
    
    # Export last 24 hours
    yesterday = datetime.now() - timedelta(days=1)
    
    query = """
    SELECT * FROM trades 
    WHERE time >= %s
    ORDER BY time
    """
    
    df = pd.read_sql(query, conn, params=[yesterday])
    
    if len(df) > 0:
        # Create parquet directory
        parquet_dir = Path(__file__).parent.parent / 'market_data' / 'parquet' / 'exports'
        parquet_dir.mkdir(parents=True, exist_ok=True)
        
        # Save to parquet
        filename = parquet_dir / f"trades_{datetime.now().strftime('%Y%m%d_%H%M%S')}.parquet"
        df.to_parquet(filename, compression='snappy')
        
        print(f"✅ Exported {len(df)} trades to {filename}")
    else:
        print("❌ No data to export")
    
    conn.close()

def main():
    parser = argparse.ArgumentParser(description='Manage AlphaPulse Market Data Collection')
    parser.add_argument('command', choices=['start', 'stop', 'status', 'export', 'monitor'],
                       help='Command to execute')
    
    args = parser.parse_args()
    
    if args.command == 'start':
        start_services()
        process = start_collector()
        if process:
            monitor_collector(process)
    
    elif args.command == 'stop':
        stop_services()
    
    elif args.command == 'status':
        postgres_status = "✅ Running" if check_postgres() else "❌ Stopped"
        grafana_status = "✅ Running" if check_grafana() else "❌ Stopped"
        
        print("📊 Service Status:")
        print(f"   PostgreSQL: {postgres_status}")
        print(f"   Grafana: {grafana_status}")
        
        if check_grafana():
            print(f"\n🌐 Grafana: http://localhost:3000")
    
    elif args.command == 'export':
        export_to_parquet()
    
    elif args.command == 'monitor':
        start_services()
        print("\n📊 Monitoring Dashboard: http://localhost:3000")
        print("   The collector should be running separately")
        print("   Press Ctrl+C to exit")
        try:
            while True:
                time.sleep(1)
        except KeyboardInterrupt:
            print("\n👋 Exiting...")

if __name__ == '__main__':
    main()