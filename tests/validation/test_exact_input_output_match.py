#!/usr/bin/env python3
"""
TRUE Data Preservation Test

This test validates the ONLY thing that matters:
Frontend output === Pipeline input (EXACT)

Uses our own collector data as the source of truth:
1. Cache the first X data points from our collector
2. Use the same transformation code our pipeline uses  
3. Compare cached input to actual frontend output

No external APIs, no "deviation" tolerance.
Either the pipeline preserves data exactly, or it fails.
"""

import asyncio
import websockets
import json
import time
import socket
import struct
from typing import Dict, List, Any, Optional

class ExactInputOutputValidator:
    """Validates that output EXACTLY matches input using our own pipeline data"""
    
    def __init__(self):
        self.cached_inputs: List[Dict] = []
        self.frontend_outputs: List[Dict] = []
        
    def capture_pipeline_inputs(self, count: int = 10) -> List[Dict]:
        """Cache the first X data points from our collector via Unix socket"""
        inputs = []
        
        try:
            # Connect to the Unix socket where our collector sends data
            sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            sock.connect('/tmp/alphapulse/polygon.sock')
            
            print(f"📡 Connected to collector socket, capturing {count} inputs...")
            
            captured = 0
            while captured < count:
                # Read binary data from collector
                try:
                    # Read message header (8 bytes)
                    header = sock.recv(8)
                    if len(header) != 8:
                        break
                        
                    # Parse header to get message type and size
                    magic, msg_type, size, sequence = struct.unpack('<HHHH', header)
                    
                    if magic != 0x03FE:  # Our protocol magic
                        continue
                        
                    # Read message body
                    body = sock.recv(size)
                    if len(body) != size:
                        continue
                    
                    # Parse based on message type
                    if msg_type == 1:  # TRADE message
                        trade_data = self.parse_trade_message(body)
                        if trade_data:
                            inputs.append({
                                "source": "collector_unix_socket",
                                "type": "trade",
                                "data": trade_data,
                                "raw_bytes": body.hex(),
                                "timestamp": time.time(),
                                "sequence": sequence
                            })
                            captured += 1
                            print(f"   📊 Cached input #{captured}: {trade_data.get('symbol', 'Unknown')}")
                            
                except socket.timeout:
                    break
                except Exception as e:
                    print(f"   ⚠️ Socket read error: {e}")
                    break
            
            sock.close()
            
        except Exception as e:
            print(f"❌ Failed to capture pipeline inputs: {e}")
            
        return inputs
    
    def parse_trade_message(self, body: bytes) -> Optional[Dict]:
        """Parse binary trade message using our protocol format"""
        try:
            if len(body) < 64:  # Trade message should be 64 bytes
                return None
                
            # Unpack trade message (same format as our protocol)
            fields = struct.unpack('<QQQQQQQf', body)
            
            return {
                "symbol_id": fields[0],
                "price_fixed": fields[1],
                "volume_fixed": fields[2], 
                "liquidity_fixed": fields[3],
                "gas_cost_fixed": fields[4],
                "timestamp_ns": fields[5],
                "sequence": fields[6],
                "latency": fields[7],
                
                # Convert fixed-point back to floats (same as our pipeline)
                "price": fields[1] / 100000000.0,  # 8 decimal places
                "volume": fields[2] / 100000000.0,
                "liquidity": fields[3] / 100000000.0,
                "gas_cost": fields[4] / 100000000.0
            }
            
        except Exception as e:
            print(f"   ⚠️ Failed to parse trade message: {e}")
            return None
    
    async def capture_frontend_output(self, duration: int = 10) -> List[Dict]:
        """Capture what the frontend actually displays"""
        outputs = []
        
        try:
            uri = "ws://127.0.0.1:8765"
            async with websockets.connect(uri) as websocket:
                print(f"🎯 Capturing frontend outputs for {duration}s...")
                
                start_time = time.time()
                while time.time() - start_time < duration:
                    try:
                        message = await asyncio.wait_for(websocket.recv(), timeout=1.0)
                        data = json.loads(message)
                        
                        if data.get('msg_type') == 'trade':
                            outputs.append({
                                "source": "frontend_websocket",
                                "displayed_price": data.get('price'),
                                "displayed_volume": data.get('volume'),
                                "displayed_liquidity": data.get('liquidity'),
                                "symbol": data.get('symbol'),
                                "timestamp": time.time()
                            })
                            
                    except asyncio.TimeoutError:
                        continue
                        
        except Exception as e:
            print(f"❌ Frontend capture failed: {e}")
            
        return outputs
    
    def validate_exact_match(self, cached_input: Dict, frontend_output: Dict) -> Dict[str, Any]:
        """
        TRUE validation: Does frontend output EXACTLY equal cached pipeline input?
        No tolerance, no approximation, no "close enough"
        """
        
        validation = {
            "cached_input": cached_input,
            "frontend_output": frontend_output,
            "exact_match": False,
            "failures": []
        }
        
        # Extract values from cached input (our pipeline's processed data)
        input_data = cached_input.get("data", {})
        cached_price = input_data.get("price", 0)
        cached_volume = input_data.get("volume", 0) 
        cached_liquidity = input_data.get("liquidity", 0)
        cached_gas_cost = input_data.get("gas_cost", 0)
        
        # Extract values from frontend output
        frontend_price = frontend_output.get("displayed_price", 0)
        frontend_volume = frontend_output.get("displayed_volume", 0)
        frontend_liquidity = frontend_output.get("displayed_liquidity", 0)
        frontend_gas_cost = frontend_output.get("displayed_gas_cost", 0)
        
        # Check exact matches (only floating point precision tolerance)
        price_match = abs(cached_price - frontend_price) < 1e-10
        volume_match = abs(cached_volume - frontend_volume) < 1e-10
        liquidity_match = abs(cached_liquidity - frontend_liquidity) < 1e-10
        gas_match = abs(cached_gas_cost - frontend_gas_cost) < 1e-10
        
        if price_match and volume_match and liquidity_match and gas_match:
            validation["exact_match"] = True
        else:
            validation["exact_match"] = False
            
            if not price_match:
                validation["failures"].append(f"Price mismatch: cached {cached_price}, frontend {frontend_price}")
            if not volume_match:
                validation["failures"].append(f"Volume mismatch: cached {cached_volume}, frontend {frontend_volume}")
            if not liquidity_match:
                validation["failures"].append(f"Liquidity mismatch: cached {cached_liquidity}, frontend {frontend_liquidity}")
            if not gas_match:
                validation["failures"].append(f"Gas cost mismatch: cached {cached_gas_cost}, frontend {frontend_gas_cost}")
        
        return validation
    
    async def run_exact_validation(self) -> Dict[str, Any]:
        """Run true exact input/output validation using our own pipeline data"""
        print("=" * 80)
        print("EXACT INPUT/OUTPUT VALIDATION")
        print("No tolerance - frontend must EXACTLY match cached pipeline input")
        print("=" * 80)
        
        # Step 1: Cache pipeline inputs (data from our collector)
        print("\n1️⃣ Caching pipeline inputs from collector...")
        cached_inputs = self.capture_pipeline_inputs(count=5)
        
        if not cached_inputs:
            return {"status": "FAILED", "reason": "No pipeline inputs captured"}
        
        print(f"✅ Cached {len(cached_inputs)} pipeline inputs")
        
        # Step 2: Capture frontend outputs
        print("\n2️⃣ Capturing frontend outputs...")
        frontend_outputs = await self.capture_frontend_output(duration=10)
        
        if not frontend_outputs:
            return {"status": "FAILED", "reason": "No frontend outputs captured"}
        
        print(f"✅ Captured {len(frontend_outputs)} frontend outputs")
        
        # Step 3: Match cached inputs to frontend outputs by sequence/timing
        print("\n3️⃣ Matching cached inputs to frontend outputs...")
        
        validations = []
        exact_matches = 0
        
        for i, cached_input in enumerate(cached_inputs):
            # Find corresponding frontend output by sequence/timing
            # For now, match by index (in a real implementation, we'd match by symbol_id or sequence)
            if i < len(frontend_outputs):
                frontend_output = frontend_outputs[i]
                
                validation = self.validate_exact_match(cached_input, frontend_output)
                validations.append(validation)
                
                if validation["exact_match"]:
                    exact_matches += 1
                    print(f"   ✅ EXACT MATCH #{i+1}")
                else:
                    print(f"   ❌ MISMATCH #{i+1}")
                    for failure in validation["failures"]:
                        print(f"      • {failure}")
        
        # Step 4: Generate pass/fail result
        total_tests = len(validations)
        
        if exact_matches == total_tests:
            status = "PASSED"
            print(f"\n✅ ALL {total_tests} TESTS PASSED: Frontend exactly matches cached pipeline input")
        else:
            status = "FAILED" 
            print(f"\n❌ {total_tests - exact_matches}/{total_tests} TESTS FAILED: Frontend does not match cached pipeline input")
        
        return {
            "status": status,
            "total_tests": total_tests,
            "exact_matches": exact_matches,
            "failures": total_tests - exact_matches,
            "validations": validations,
            "timestamp": time.time()
        }

async def main():
    validator = ExactInputOutputValidator()
    
    print("🎯 EXACT INPUT/OUTPUT VALIDATION")
    print("This validates the ONLY thing that matters:")
    print("Frontend output === Raw API input (EXACTLY)")
    print()
    
    result = await validator.run_exact_validation()
    
    if result["status"] == "PASSED":
        print("\n🎉 SUCCESS: Pipeline preserves data exactly")
        return 0
    else:
        print("\n💥 FAILURE: Pipeline does not preserve data exactly")
        print("Frontend values do not match raw API inputs")
        return 1

if __name__ == "__main__":
    exit_code = asyncio.run(main())
    exit(exit_code)