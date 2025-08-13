"""
System Monitoring API Routes - Handle system status and monitoring requests

⚠️ DEPRECATED - This file will be moved to Rust/Tokio services
   See: /rust-migration.md Phase 2-3
   Do not add new features or migrate to FastAPI
   System monitoring will use Prometheus metrics in Rust
"""
from flask import Blueprint, jsonify, make_response
import psutil
import time
import os
import subprocess
import socket
from datetime import datetime

system_bp = Blueprint('system', __name__, url_prefix='/api/system')

def get_process_info(name_pattern):
    """Get process information by name pattern."""
    processes = []
    for proc in psutil.process_iter(['pid', 'name', 'cmdline', 'status', 'cpu_percent', 'memory_info', 'create_time']):
        try:
            if name_pattern.lower() in proc.info['name'].lower() or \
               any(name_pattern.lower() in cmd.lower() for cmd in (proc.info['cmdline'] or [])):
                
                # Calculate uptime
                uptime = int(time.time() - proc.info['create_time'])
                
                # Get memory in MB
                memory_mb = proc.info['memory_info'].rss / (1024 * 1024) if proc.info['memory_info'] else 0
                
                processes.append({
                    'pid': proc.info['pid'],
                    'name': proc.info['name'],
                    'status': proc.info['status'],
                    'cpu_percent': proc.info['cpu_percent'],
                    'memory_mb': memory_mb,
                    'uptime': uptime
                })
        except (psutil.NoSuchProcess, psutil.AccessDenied, psutil.ZombieProcess):
            pass
    
    return processes

def check_port_open(port):
    """Check if a port is open."""
    try:
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.settimeout(1)
        result = sock.connect_ex(('localhost', port))
        sock.close()
        return result == 0
    except:
        return False

@system_bp.route('/status', methods=['GET'])
def system_status():
    """Get overall system status."""
    try:
        # Get system uptime
        boot_time = psutil.boot_time()
        uptime = int(time.time() - boot_time)
        
        # Check critical services
        flask_running = any('flask' in proc.name().lower() or 'python' in proc.name().lower() 
                           for proc in psutil.process_iter(['name']))
        
        # Determine overall status
        overall_status = 'healthy'
        
        # Basic health checks
        cpu_percent = psutil.cpu_percent(interval=1)
        memory = psutil.virtual_memory()
        disk = psutil.disk_usage('/')
        
        if cpu_percent > 80 or memory.percent > 85 or disk.percent > 90:
            overall_status = 'warning'
        
        if not flask_running:
            overall_status = 'critical'
        
        return jsonify({
            'overall': overall_status,
            'uptime': uptime,
            'timestamp': datetime.utcnow().isoformat(),
            'system': {
                'cpu_percent': cpu_percent,
                'memory_percent': memory.percent,
                'disk_percent': disk.percent
            }
        })
        
    except Exception as e:
        return jsonify({
            'overall': 'error',
            'uptime': 0,
            'timestamp': datetime.utcnow().isoformat(),
            'error': str(e)
        }), 500

@system_bp.route('/services', methods=['GET'])
def system_services():
    """Get status of all system services."""
    try:
        services = []
        
        # Define services to monitor
        service_patterns = [
            {'name': 'Flask API Server', 'pattern': 'flask', 'port': 5001},
            {'name': 'Python Backend', 'pattern': 'python', 'port': None},
            {'name': 'Jupyter Kernel', 'pattern': 'jupyter', 'port': None},
            {'name': 'Node.js Frontend', 'pattern': 'node', 'port': 5173},
            {'name': 'Vite Dev Server', 'pattern': 'vite', 'port': 5173},
        ]
        
        for service_def in service_patterns:
            procs = get_process_info(service_def['pattern'])
            
            if procs:
                # Take the most relevant process (highest CPU or memory)
                main_proc = max(procs, key=lambda p: p['cpu_percent'] + p['memory_mb'])
                
                # Check port if specified
                port_status = True
                if service_def['port']:
                    port_status = check_port_open(service_def['port'])
                
                services.append({
                    'name': service_def['name'],
                    'status': 'running' if port_status else 'error',
                    'port': service_def['port'],
                    'pid': main_proc['pid'],
                    'uptime': main_proc['uptime'],
                    'memory': main_proc['memory_mb'],
                    'cpu': main_proc['cpu_percent'],
                    'lastCheck': datetime.utcnow().isoformat()
                })
            else:
                services.append({
                    'name': service_def['name'],
                    'status': 'stopped',
                    'port': service_def['port'],
                    'pid': None,
                    'uptime': None,
                    'memory': None,
                    'cpu': None,
                    'lastCheck': datetime.utcnow().isoformat()
                })
        
        return jsonify(services)
        
    except Exception as e:
        return jsonify({'error': str(e)}), 500

@system_bp.route('/streams', methods=['GET'])
def system_streams():
    """Get status of data streams and connections."""
    try:
        streams = []
        
        # Mock data streams - in a real implementation, this would check actual connections
        stream_definitions = [
            {
                'id': 'alpaca_market_data',
                'name': 'Alpaca Market Data',
                'source': 'Alpaca Markets',
                'status': 'connected',
                'messageCount': 1234,
                'latency': 45,
                'lastMessage': datetime.utcnow().isoformat()
            },
            {
                'id': 'alpaca_websocket',
                'name': 'Alpaca WebSocket',
                'source': 'Alpaca WebSocket',
                'status': 'connected',
                'messageCount': 856,
                'latency': 67,
                'lastMessage': datetime.utcnow().isoformat()
            },
            {
                'id': 'coinbase_websocket',
                'name': 'Coinbase Pro Feed',
                'source': 'Coinbase Pro',
                'status': 'disconnected',
                'messageCount': 0,
                'latency': None,
                'lastMessage': None
            },
            {
                'id': 'internal_events',
                'name': 'Internal Event Bus',
                'source': 'AlphaPulse Internal',
                'status': 'connected',
                'messageCount': 423,
                'latency': 12,
                'lastMessage': datetime.utcnow().isoformat()
            }
        ]
        
        # In a real implementation, you would:
        # 1. Check actual WebSocket connections
        # 2. Query message queues for counts
        # 3. Measure actual latency
        # 4. Get real connection status
        
        # For now, return mock data with some dynamic elements
        for stream_def in stream_definitions:
            # Add some randomness to make it feel live
            import random
            if stream_def['status'] == 'connected':
                stream_def['messageCount'] += random.randint(0, 10)
                if stream_def['latency']:
                    stream_def['latency'] += random.randint(-5, 5)
        
        return jsonify(stream_definitions)
        
    except Exception as e:
        return jsonify({'error': str(e)}), 500

@system_bp.route('/metrics', methods=['GET'])
def system_metrics():
    """Get real-time system metrics."""
    try:
        # CPU metrics
        cpu_percent = psutil.cpu_percent(interval=1)
        cpu_count = psutil.cpu_count()
        
        # Memory metrics
        memory = psutil.virtual_memory()
        
        # Disk metrics
        disk = psutil.disk_usage('/')
        
        # Network metrics
        network = psutil.net_io_counters()
        
        # Process count
        process_count = len(psutil.pids())
        
        return jsonify({
            'timestamp': datetime.utcnow().isoformat(),
            'cpu': {
                'percent': cpu_percent,
                'count': cpu_count
            },
            'memory': {
                'total': memory.total,
                'available': memory.available,
                'percent': memory.percent,
                'used': memory.used
            },
            'disk': {
                'total': disk.total,
                'free': disk.free,
                'percent': disk.percent,
                'used': disk.used
            },
            'network': {
                'bytes_sent': network.bytes_sent,
                'bytes_recv': network.bytes_recv,
                'packets_sent': network.packets_sent,
                'packets_recv': network.packets_recv
            },
            'processes': {
                'count': process_count
            }
        })
        
    except Exception as e:
        return jsonify({'error': str(e)}), 500

@system_bp.route('/data-streams-status', methods=['GET'])
def data_streams_status():
    """Get status of exchange data streams (Coinbase/Kraken L2)."""
    try:
        import duckdb
        from pathlib import Path
        import time
        
        # Get stats from DuckDB if available
        db_path = Path('/Users/daws/alphapulse/backend/market_data/market_data.duckdb')
        
        if not db_path.exists():
            return jsonify({
                'status': 'error',
                'message': 'Database not found',
                'kraken_l2': None,
                'coinbase_l2': None
            }), 404
        
        try:
            # Simple connection (same config as recorder)
            conn = duckdb.connect(str(db_path))
            
            # Get current time for filtering
            current_time = time.time()
            one_hour_ago = current_time - 3600
            
            # Get Kraken L2 stats (ALL TIME and recent)
            kraken_stats = conn.execute("""
                SELECT 
                    COUNT(*) as total_messages_all_time,
                    MAX(timestamp) as last_message,
                    COUNT(DISTINCT sequence_id) as unique_sequences
                FROM orderbook_updates
                WHERE exchange = 'kraken'
            """).fetchone()
            
            kraken_recent = conn.execute("""
                SELECT COUNT(*) as recent_messages
                FROM orderbook_updates
                WHERE exchange = 'kraken'
                AND timestamp > ?
            """, [one_hour_ago]).fetchone()
            
            # Get Coinbase L2 stats (ALL TIME and recent)
            coinbase_stats = conn.execute("""
                SELECT 
                    COUNT(*) as total_messages_all_time,
                    MAX(timestamp) as last_message,
                    COUNT(DISTINCT snapshot_id) as snapshots
                FROM orderbook_snapshots
                WHERE exchange = 'coinbase'
            """).fetchone()
            
            coinbase_recent = conn.execute("""
                SELECT COUNT(*) as recent_messages
                FROM orderbook_snapshots
                WHERE exchange = 'coinbase'
                AND timestamp > ?
            """, [one_hour_ago]).fetchone()
            
            conn.close()
            
            # Calculate messages per second (based on recent activity)
            kraken_mps = kraken_recent[0] / 3600 if kraken_recent[0] else 0
            coinbase_mps = coinbase_recent[0] / 3600 if coinbase_recent[0] else 0
            
            # Check if data is recent (within last minute)
            kraken_status = 'connected' if kraken_stats[1] and (current_time - kraken_stats[1] < 60) else 'disconnected'
            coinbase_status = 'connected' if coinbase_stats[1] and (current_time - coinbase_stats[1] < 60) else 'disconnected'
            
            return jsonify({
                'status': 'success',
                'kraken_l2': {
                    'id': 'kraken_l2',
                    'name': 'Kraken L2 Orderbook',
                    'exchange': 'Kraken',
                    'status': kraken_status,
                    'messages_per_second': round(kraken_mps, 2),
                    'total_messages': kraken_stats[0] if kraken_stats[0] else 0,  # ALL TIME total
                    'latency': 25,  # Mock latency
                    'last_message': datetime.fromtimestamp(kraken_stats[1]).isoformat() if kraken_stats[1] else None
                },
                'coinbase_l2': {
                    'id': 'coinbase_l2',
                    'name': 'Coinbase L2 Orderbook',
                    'exchange': 'Coinbase',
                    'status': coinbase_status,
                    'messages_per_second': round(coinbase_mps, 2),
                    'total_messages': coinbase_stats[0] if coinbase_stats[0] else 0,  # ALL TIME total
                    'latency': 20,  # Mock latency
                    'last_message': datetime.fromtimestamp(coinbase_stats[1]).isoformat() if coinbase_stats[1] else None
                }
            })
            
        except Exception as db_error:
            return jsonify({
                'status': 'error',
                'message': f'Database error: {str(db_error)}',
                'kraken_l2': None,
                'coinbase_l2': None
            }), 500
            
    except Exception as e:
        return jsonify({
            'status': 'error',
            'message': str(e),
            'kraken_l2': None,
            'coinbase_l2': None
        }), 500

# CORS handling
@system_bp.after_request
def after_request(response):
    response.headers.add('Access-Control-Allow-Origin', '*')
    response.headers.add('Access-Control-Allow-Headers', 'Content-Type,Authorization')
    response.headers.add('Access-Control-Allow-Methods', 'GET,PUT,POST,DELETE,OPTIONS')
    return response