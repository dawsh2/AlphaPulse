#!/usr/bin/env python3
"""
E2E Test Orchestrator
Main test runner that coordinates all validation components
"""

import asyncio
import json
import logging
import subprocess
import sys
import time
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Any, Optional, Tuple
import threading
from concurrent.futures import ThreadPoolExecutor

from protocol_validator import BinaryProtocolReader, ProtocolValidator
from ws_data_interceptor import WebSocketInterceptor
from comparison_engine import DataComparisonEngine

logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)


class ServiceManager:
    """Manages starting and stopping of services for testing"""
    
    def __init__(self):
        self.processes: Dict[str, subprocess.Popen] = {}
        self.service_order = [
            'relay_server',
            'ws_bridge',
            'exchange_collector'
        ]
        
    def start_service(self, name: str, command: List[str], env: Optional[Dict] = None) -> bool:
        """Start a service subprocess"""
        try:
            logger.info(f"Starting service: {name}")
            process = subprocess.Popen(
                command,
                env=env,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE
            )
            self.processes[name] = process
            
            # Give service time to start
            time.sleep(2)
            
            # Check if process is still running
            if process.poll() is None:
                logger.info(f"Service {name} started successfully (PID: {process.pid})")
                return True
            else:
                logger.error(f"Service {name} failed to start")
                return False
                
        except Exception as e:
            logger.error(f"Failed to start {name}: {e}")
            return False
    
    def stop_service(self, name: str):
        """Stop a service subprocess"""
        if name in self.processes:
            process = self.processes[name]
            logger.info(f"Stopping service: {name} (PID: {process.pid})")
            process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                logger.warning(f"Force killing {name}")
                process.kill()
            del self.processes[name]
    
    def stop_all(self):
        """Stop all services"""
        for name in list(self.processes.keys()):
            self.stop_service(name)
    
    def start_alphapulse_stack(self) -> bool:
        """Start the complete AlphaPulse service stack"""
        # Start relay server
        if not self.start_service(
            'relay_server',
            ['cargo', 'run', '--bin', 'relay_server'],
            env={**os.environ, 'RUST_LOG': 'info'}
        ):
            return False
        
        # Start WS bridge
        if not self.start_service(
            'ws_bridge',
            ['cargo', 'run', '--bin', 'ws_bridge'],
            env={**os.environ, 'RUST_LOG': 'info'}
        ):
            return False
        
        # Start exchange collector (using Alpaca as example)
        if not self.start_service(
            'exchange_collector',
            ['cargo', 'run', '--bin', 'exchange_collector'],
            env={**os.environ, 'EXCHANGE_NAME': 'alpaca', 'RUST_LOG': 'debug'}
        ):
            return False
        
        logger.info("All services started successfully")
        return True


class E2ETestOrchestrator:
    """Orchestrates end-to-end testing of the data pipeline"""
    
    def __init__(self, capture_duration: int = 60):
        self.capture_duration = capture_duration
        self.service_manager = ServiceManager()
        self.binary_reader = BinaryProtocolReader()
        self.ws_interceptor = WebSocketInterceptor()
        self.validator = ProtocolValidator()
        self.comparison_engine = DataComparisonEngine()
        self.results: Dict[str, Any] = {}
        
    async def capture_data_simultaneously(self) -> Tuple[List, List]:
        """Capture data from both Unix socket and WebSocket simultaneously"""
        logger.info(f"Starting simultaneous data capture for {self.capture_duration} seconds")
        
        # Create tasks for parallel capture
        with ThreadPoolExecutor(max_workers=2) as executor:
            # Binary capture (synchronous, runs in thread)
            binary_future = executor.submit(
                self.binary_reader.connect_and_capture,
                duration=self.capture_duration
            )
            
            # WebSocket capture (asynchronous)
            ws_task = asyncio.create_task(
                self.ws_interceptor.connect_and_capture(duration=self.capture_duration)
            )
            
            # Wait for both to complete
            await ws_task
            binary_messages = binary_future.result()
            
        logger.info(f"Captured {len(binary_messages)} binary messages")
        logger.info(f"Captured {len(self.ws_interceptor.messages)} WebSocket messages")
        
        return binary_messages, self.ws_interceptor.messages
    
    def validate_data_integrity(self) -> Dict[str, Any]:
        """Run all validation tests on captured data"""
        logger.info("Running data integrity validation")
        
        # Set up validator with captured data
        self.validator.binary_messages = self.binary_reader.messages
        self.validator.json_messages = [msg.raw_data for msg in self.ws_interceptor.messages]
        
        # Run protocol validation
        protocol_report = self.validator.generate_report()
        
        # Set up comparison engine with WebSocket data
        self.comparison_engine.ws_messages = [msg.raw_data for msg in self.ws_interceptor.messages]
        
        # Run comparison tests
        comparison_report = self.comparison_engine.run_all_comparisons()
        
        return {
            'protocol_validation': protocol_report,
            'data_comparison': comparison_report,
            'capture_stats': {
                'binary_messages': self.binary_reader.stats,
                'ws_messages': self.ws_interceptor.stats
            }
        }
    
    def validate_message_flow(self) -> Dict[str, Any]:
        """Validate message flow through the pipeline"""
        results = {
            'sequence_continuity': True,
            'symbol_consistency': True,
            'latency_tracking': True,
            'issues': []
        }
        
        # Check sequence numbers
        sequences = {}
        for msg in self.binary_reader.messages:
            if msg.sequence > 0:
                key = msg.msg_type
                if key not in sequences:
                    sequences[key] = []
                sequences[key].append(msg.sequence)
        
        for msg_type, seq_list in sequences.items():
            seq_list.sort()
            for i in range(1, len(seq_list)):
                if seq_list[i] != seq_list[i-1] + 1:
                    results['sequence_continuity'] = False
                    results['issues'].append({
                        'type': 'sequence_gap',
                        'msg_type': msg_type,
                        'expected': seq_list[i-1] + 1,
                        'actual': seq_list[i]
                    })
        
        # Check symbol hash consistency
        symbol_mappings = {}
        for msg in self.ws_interceptor.messages:
            if msg.msg_type == 'symbol_mapping':
                symbol_mappings[msg.symbol_hash] = msg.symbol
        
        for msg in self.ws_interceptor.messages:
            if msg.msg_type == 'trade' and msg.symbol_hash:
                if msg.symbol_hash in symbol_mappings:
                    expected_symbol = symbol_mappings[msg.symbol_hash]
                    if msg.symbol != expected_symbol:
                        results['symbol_consistency'] = False
                        results['issues'].append({
                            'type': 'symbol_mismatch',
                            'hash': msg.symbol_hash,
                            'expected': expected_symbol,
                            'actual': msg.symbol
                        })
        
        return results
    
    async def run_complete_test(self, start_services: bool = True) -> Dict[str, Any]:
        """Run complete E2E test"""
        start_time = time.time()
        
        try:
            # Start services if requested
            if start_services:
                logger.info("Starting AlphaPulse services")
                if not self.service_manager.start_alphapulse_stack():
                    return {'error': 'Failed to start services'}
                
                # Wait for services to stabilize
                logger.info("Waiting for services to stabilize...")
                await asyncio.sleep(5)
            
            # Capture data simultaneously
            binary_msgs, ws_msgs = await self.capture_data_simultaneously()
            
            # Validate data integrity
            integrity_results = self.validate_data_integrity()
            
            # Validate message flow
            flow_results = self.validate_message_flow()
            
            # Generate final report
            elapsed = time.time() - start_time
            
            report = {
                'test_info': {
                    'timestamp': datetime.now().isoformat(),
                    'duration_seconds': elapsed,
                    'capture_duration': self.capture_duration
                },
                'summary': {
                    'total_binary_messages': len(self.binary_reader.messages),
                    'total_ws_messages': len(self.ws_interceptor.messages),
                    'overall_status': 'PASS' if all([
                        integrity_results['protocol_validation']['summary']['pass_rate'] > 0.95,
                        flow_results['sequence_continuity'],
                        flow_results['symbol_consistency']
                    ]) else 'FAIL'
                },
                'integrity_validation': integrity_results,
                'message_flow_validation': flow_results
            }
            
            # Save report
            report_path = f"e2e_test_report_{datetime.now().strftime('%Y%m%d_%H%M%S')}.json"
            with open(report_path, 'w') as f:
                json.dump(report, f, indent=2, default=str)
            logger.info(f"Test report saved to {report_path}")
            
            # Save captured data for debugging
            self.binary_reader.save_to_file(f"binary_capture_{datetime.now().strftime('%Y%m%d_%H%M%S')}.json")
            self.ws_interceptor.save_to_file(f"ws_capture_{datetime.now().strftime('%Y%m%d_%H%M%S')}.json")
            
            return report
            
        except Exception as e:
            logger.error(f"Test execution failed: {e}")
            return {'error': str(e)}
            
        finally:
            if start_services:
                logger.info("Stopping services")
                self.service_manager.stop_all()
    
    def print_summary(self, report: Dict[str, Any]):
        """Print a human-readable summary of test results"""
        print("\n" + "="*60)
        print("E2E TEST SUMMARY")
        print("="*60)
        
        if 'error' in report:
            print(f"❌ Test failed: {report['error']}")
            return
        
        summary = report['summary']
        print(f"Overall Status: {'✅ PASS' if summary['overall_status'] == 'PASS' else '❌ FAIL'}")
        print(f"Binary Messages: {summary['total_binary_messages']}")
        print(f"WebSocket Messages: {summary['total_ws_messages']}")
        
        # Protocol validation
        protocol = report['integrity_validation']['protocol_validation']['summary']
        print(f"\nProtocol Validation:")
        print(f"  - Pass Rate: {protocol['pass_rate']:.1%}")
        print(f"  - Passed: {protocol['passed']}")
        print(f"  - Failed: {protocol['failed']}")
        
        # Message flow
        flow = report['message_flow_validation']
        print(f"\nMessage Flow:")
        print(f"  - Sequence Continuity: {'✅' if flow['sequence_continuity'] else '❌'}")
        print(f"  - Symbol Consistency: {'✅' if flow['symbol_consistency'] else '❌'}")
        
        if flow['issues']:
            print(f"  - Issues Found: {len(flow['issues'])}")
            for issue in flow['issues'][:5]:  # Show first 5 issues
                print(f"    • {issue['type']}: {issue}")
        
        # Latency stats
        if 'latency_stats' in report['integrity_validation']['data_comparison']:
            latency = report['integrity_validation']['data_comparison']['latency_stats']
            if 'total' in latency and latency['total']:
                stats = latency['total']
                print(f"\nLatency Statistics (ms):")
                print(f"  - Min: {stats['min_ms']:.2f}")
                print(f"  - Avg: {stats['avg_ms']:.2f}")
                print(f"  - Max: {stats['max_ms']:.2f}")
                print(f"  - P95: {stats['p95_ms']:.2f}")
        
        print("="*60 + "\n")


async def main():
    """Main entry point for E2E testing"""
    import argparse
    import os
    
    parser = argparse.ArgumentParser(description='AlphaPulse E2E Test Orchestrator')
    parser.add_argument('--duration', type=int, default=60, help='Capture duration in seconds')
    parser.add_argument('--no-services', action='store_true', help='Skip starting services (assumes they are already running)')
    parser.add_argument('--verbose', action='store_true', help='Enable verbose logging')
    
    args = parser.parse_args()
    
    if args.verbose:
        logging.getLogger().setLevel(logging.DEBUG)
    
    orchestrator = E2ETestOrchestrator(capture_duration=args.duration)
    
    logger.info("Starting E2E test orchestration")
    report = await orchestrator.run_complete_test(start_services=not args.no_services)
    
    orchestrator.print_summary(report)
    
    # Exit with appropriate code
    if report.get('summary', {}).get('overall_status') == 'PASS':
        sys.exit(0)
    else:
        sys.exit(1)


if __name__ == "__main__":
    import os
    asyncio.run(main())