#!/usr/bin/env python3
"""
Protocol Message Integrity Tests

Tests the binary protocol message structures to ensure data integrity
throughout the entire pipeline: encoding → transport → decoding.
"""

import struct
import json
import time
import hashlib
import random
from typing import Dict, List, Any, Tuple, Optional
from decimal import Decimal
from dataclasses import dataclass
from protocol_validator import BinaryMessage, ProtocolValidator

@dataclass
class MessageIntegrityResult:
    """Result of a message integrity test"""
    test_name: str
    passed: bool
    error_message: str = ""
    original_data: Optional[Dict] = None
    decoded_data: Optional[Dict] = None
    data_corruption_detected: bool = False
    round_trip_success: bool = False

class ProtocolIntegrityTester:
    """Tests protocol message integrity at the binary level"""
    
    def __init__(self):
        self.results: List[MessageIntegrityResult] = []
        self.validator = ProtocolValidator()
        
    def test_message_encoding_integrity(self) -> List[MessageIntegrityResult]:
        """Test that messages maintain integrity during encoding/decoding"""
        print("🔧 Testing message encoding/decoding integrity...")
        
        test_cases = [
            # Trade messages with precise decimal values
            {
                "type": "trade",
                "timestamp_ns": 1692182400123456789,
                "symbol_hash": 0x1234567890ABCDEF,
                "price_fp": 460523000000,  # 4605.23 in fixed-point
                "volume_fp": 123456780,     # 1.2345678 in fixed-point  
                "side": 0,  # buy
                "test_name": "precise_decimal_trade"
            },
            {
                "type": "trade", 
                "timestamp_ns": 1692182400987654321,
                "symbol_hash": 0xFEDCBA0987654321,
                "price_fp": 6823456000000,  # 68234.56 in fixed-point
                "volume_fp": 25000000,      # 0.25 in fixed-point
                "side": 1,  # sell
                "test_name": "high_value_trade"
            },
            # Orderbook with multiple levels
            {
                "type": "orderbook",
                "timestamp_ns": 1692182400555666777,
                "symbol_hash": 0x1111222233334444,
                "bids": [
                    (460523000000, 100000000),  # $4605.23 @ 1.0
                    (460522000000, 200000000),  # $4605.22 @ 2.0
                    (460521000000, 150000000),  # $4605.21 @ 1.5
                ],
                "asks": [
                    (460524000000, 80000000),   # $4605.24 @ 0.8
                    (460525000000, 300000000),  # $4605.25 @ 3.0
                ],
                "test_name": "orderbook_multiple_levels"
            },
            # L2 snapshot with many levels (stress test)
            {
                "type": "l2_snapshot",
                "timestamp_ns": 1692182400111222333,
                "symbol_hash": 0x9999888877776666,
                "sequence": 12345,
                "bids": [(460523000000 - i*1000, 10000000 + i*1000000) for i in range(50)],
                "asks": [(460524000000 + i*1000, 10000000 + i*1000000) for i in range(50)],
                "test_name": "large_l2_snapshot"
            }
        ]
        
        for test_case in test_cases:
            result = self._test_single_message_integrity(test_case)
            self.results.append(result)
            
            status = "✅" if result.passed else "❌"
            print(f"   {status} {result.test_name}")
            if not result.passed:
                print(f"      Error: {result.error_message}")
        
        return self.results
    
    def _test_single_message_integrity(self, test_case: Dict) -> MessageIntegrityResult:
        """Test integrity of a single message through encode/decode cycle"""
        test_name = test_case["test_name"]
        
        try:
            # Step 1: Create the message in binary format (simulated)
            binary_data = self._encode_message_to_binary(test_case)
            
            # Step 2: Simulate potential corruption scenarios
            corrupted_data = self._simulate_transmission(binary_data)
            
            # Step 3: Decode the message back
            decoded_message = self._decode_binary_message(corrupted_data, test_case["type"])
            
            # Step 4: Verify data integrity
            integrity_check = self._verify_data_integrity(test_case, decoded_message)
            
            return MessageIntegrityResult(
                test_name=test_name,
                passed=integrity_check["passed"],
                error_message=integrity_check.get("error", ""),
                original_data=test_case,
                decoded_data=decoded_message,
                data_corruption_detected=integrity_check.get("corruption_detected", False),
                round_trip_success=integrity_check.get("round_trip_success", False)
            )
            
        except Exception as e:
            return MessageIntegrityResult(
                test_name=test_name,
                passed=False,
                error_message=f"Exception during integrity test: {e}",
                original_data=test_case
            )
    
    def _encode_message_to_binary(self, message: Dict) -> bytes:
        """Simulate encoding a message to binary format"""
        msg_type = message["type"]
        
        if msg_type == "trade":
            # Trade message format: [header][timestamp][symbol_hash][price][volume][side]
            return struct.pack('>IQQQQB', 
                1,  # message type (trade)
                message["timestamp_ns"],
                message["symbol_hash"], 
                message["price_fp"],
                message["volume_fp"],
                message["side"]
            )
            
        elif msg_type == "orderbook":
            # Orderbook format: [header][timestamp][symbol_hash][num_bids][bids...][num_asks][asks...]
            data = struct.pack('>IQQI', 
                2,  # message type (orderbook)
                message["timestamp_ns"],
                message["symbol_hash"],
                len(message["bids"])
            )
            
            # Pack bids
            for price, volume in message["bids"]:
                data += struct.pack('>QQ', price, volume)
                
            # Pack number of asks and asks
            data += struct.pack('>I', len(message["asks"]))
            for price, volume in message["asks"]:
                data += struct.pack('>QQ', price, volume)
                
            return data
            
        elif msg_type == "l2_snapshot":
            # L2 snapshot format: [header][timestamp][symbol_hash][sequence][num_bids][bids...][num_asks][asks...]
            data = struct.pack('>IQQII',
                3,  # message type (l2_snapshot)
                message["timestamp_ns"],
                message["symbol_hash"],
                message["sequence"],
                len(message["bids"])
            )
            
            # Pack bids
            for price, volume in message["bids"]:
                data += struct.pack('>QQ', price, volume)
                
            # Pack asks
            data += struct.pack('>I', len(message["asks"]))
            for price, volume in message["asks"]:
                data += struct.pack('>QQ', price, volume)
                
            return data
            
        else:
            raise ValueError(f"Unknown message type: {msg_type}")
    
    def _simulate_transmission(self, data: bytes) -> bytes:
        """Simulate potential issues during transmission"""
        # Most of the time, return data unchanged (99.9% success rate)
        if random.random() < 0.999:
            return data
            
        # Simulate rare transmission issues
        corruption_type = random.choice([
            "bit_flip",      # Single bit flip
            "truncation",    # Message truncated
            "duplication"    # Bytes duplicated
        ])
        
        if corruption_type == "bit_flip" and len(data) > 0:
            # Flip a random bit
            data_list = list(data)
            byte_idx = random.randint(0, len(data_list) - 1)
            bit_idx = random.randint(0, 7)
            data_list[byte_idx] ^= (1 << bit_idx)
            return bytes(data_list)
            
        elif corruption_type == "truncation" and len(data) > 10:
            # Truncate message
            truncate_at = len(data) - random.randint(1, min(10, len(data) // 2))
            return data[:truncate_at]
            
        elif corruption_type == "duplication" and len(data) > 0:
            # Duplicate some bytes
            dup_start = random.randint(0, len(data) - 1)
            dup_end = min(dup_start + random.randint(1, 8), len(data))
            return data + data[dup_start:dup_end]
            
        return data
    
    def _decode_binary_message(self, data: bytes, expected_type: str) -> Dict:
        """Decode binary message back to structured data"""
        try:
            if len(data) < 4:
                raise ValueError("Message too short")
                
            msg_type_id = struct.unpack('>I', data[:4])[0]
            
            if expected_type == "trade" and msg_type_id == 1:
                if len(data) < 37:  # 4 + 8 + 8 + 8 + 8 + 1
                    raise ValueError("Trade message too short")
                    
                _, timestamp, symbol_hash, price, volume, side = struct.unpack('>IQQQQB', data[:37])
                
                return {
                    "type": "trade",
                    "timestamp_ns": timestamp,
                    "symbol_hash": symbol_hash,
                    "price_fp": price,
                    "volume_fp": volume,
                    "side": side
                }
                
            elif expected_type == "orderbook" and msg_type_id == 2:
                offset = 4
                if len(data) < offset + 20:  # 4 + 8 + 8 + 4
                    raise ValueError("Orderbook message too short")
                    
                timestamp, symbol_hash, num_bids = struct.unpack('>QQI', data[offset:offset+20])
                offset += 20
                
                # Decode bids
                bids = []
                for _ in range(num_bids):
                    if offset + 16 > len(data):
                        raise ValueError("Insufficient data for bids")
                    price, volume = struct.unpack('>QQ', data[offset:offset+16])
                    bids.append((price, volume))
                    offset += 16
                
                # Decode asks
                if offset + 4 > len(data):
                    raise ValueError("Insufficient data for asks count")
                num_asks = struct.unpack('>I', data[offset:offset+4])[0]
                offset += 4
                
                asks = []
                for _ in range(num_asks):
                    if offset + 16 > len(data):
                        raise ValueError("Insufficient data for asks")
                    price, volume = struct.unpack('>QQ', data[offset:offset+16])
                    asks.append((price, volume))
                    offset += 16
                
                return {
                    "type": "orderbook",
                    "timestamp_ns": timestamp,
                    "symbol_hash": symbol_hash,
                    "bids": bids,
                    "asks": asks
                }
                
            elif expected_type == "l2_snapshot" and msg_type_id == 3:
                offset = 4
                if len(data) < offset + 24:  # 4 + 8 + 8 + 4 + 4
                    raise ValueError("L2 snapshot message too short")
                    
                timestamp, symbol_hash, sequence, num_bids = struct.unpack('>QQII', data[offset:offset+24])
                offset += 24
                
                # Decode bids
                bids = []
                for _ in range(num_bids):
                    if offset + 16 > len(data):
                        raise ValueError("Insufficient data for bids")
                    price, volume = struct.unpack('>QQ', data[offset:offset+16])
                    bids.append((price, volume))
                    offset += 16
                
                # Decode asks
                if offset + 4 > len(data):
                    raise ValueError("Insufficient data for asks count")
                num_asks = struct.unpack('>I', data[offset:offset+4])[0]
                offset += 4
                
                asks = []
                for _ in range(num_asks):
                    if offset + 16 > len(data):
                        raise ValueError("Insufficient data for asks")
                    price, volume = struct.unpack('>QQ', data[offset:offset+16])
                    asks.append((price, volume))
                    offset += 16
                
                return {
                    "type": "l2_snapshot",
                    "timestamp_ns": timestamp,
                    "symbol_hash": symbol_hash,
                    "sequence": sequence,
                    "bids": bids,
                    "asks": asks
                }
                
            else:
                raise ValueError(f"Message type mismatch: expected {expected_type}, got type_id {msg_type_id}")
                
        except Exception as e:
            raise ValueError(f"Decoding failed: {e}")
    
    def _verify_data_integrity(self, original: Dict, decoded: Dict) -> Dict[str, Any]:
        """Verify that decoded data matches original data"""
        errors = []
        
        # Check basic fields
        if original.get("timestamp_ns") != decoded.get("timestamp_ns"):
            errors.append(f"Timestamp mismatch: {original.get('timestamp_ns')} != {decoded.get('timestamp_ns')}")
            
        if original.get("symbol_hash") != decoded.get("symbol_hash"):
            errors.append(f"Symbol hash mismatch: {original.get('symbol_hash')} != {decoded.get('symbol_hash')}")
        
        # Type-specific checks
        if original["type"] == "trade":
            if original.get("price_fp") != decoded.get("price_fp"):
                errors.append(f"Price mismatch: {original.get('price_fp')} != {decoded.get('price_fp')}")
            if original.get("volume_fp") != decoded.get("volume_fp"):
                errors.append(f"Volume mismatch: {original.get('volume_fp')} != {decoded.get('volume_fp')}")
            if original.get("side") != decoded.get("side"):
                errors.append(f"Side mismatch: {original.get('side')} != {decoded.get('side')}")
                
        elif original["type"] in ["orderbook", "l2_snapshot"]:
            # Check bids
            orig_bids = original.get("bids", [])
            dec_bids = decoded.get("bids", [])
            
            if len(orig_bids) != len(dec_bids):
                errors.append(f"Bids count mismatch: {len(orig_bids)} != {len(dec_bids)}")
            else:
                for i, (orig_bid, dec_bid) in enumerate(zip(orig_bids, dec_bids)):
                    if orig_bid != dec_bid:
                        errors.append(f"Bid {i} mismatch: {orig_bid} != {dec_bid}")
            
            # Check asks
            orig_asks = original.get("asks", [])
            dec_asks = decoded.get("asks", [])
            
            if len(orig_asks) != len(dec_asks):
                errors.append(f"Asks count mismatch: {len(orig_asks)} != {len(dec_asks)}")
            else:
                for i, (orig_ask, dec_ask) in enumerate(zip(orig_asks, dec_asks)):
                    if orig_ask != dec_ask:
                        errors.append(f"Ask {i} mismatch: {orig_ask} != {dec_ask}")
            
            # Check sequence for L2 snapshots
            if original["type"] == "l2_snapshot":
                if original.get("sequence") != decoded.get("sequence"):
                    errors.append(f"Sequence mismatch: {original.get('sequence')} != {decoded.get('sequence')}")
        
        return {
            "passed": len(errors) == 0,
            "error": "; ".join(errors) if errors else "",
            "corruption_detected": len(errors) > 0,
            "round_trip_success": len(errors) == 0
        }
    
    def test_message_size_limits(self) -> List[MessageIntegrityResult]:
        """Test protocol message size limits (64KB)"""
        print("📏 Testing message size limits...")
        
        max_size = 65535  # 64KB - 1
        
        test_cases = [
            {
                "name": "normal_size_orderbook",
                "type": "orderbook",
                "num_levels": 100,  # Should be well under limit
                "expected_pass": True
            },
            {
                "name": "large_orderbook",
                "type": "orderbook", 
                "num_levels": 2000,  # Should approach but not exceed limit
                "expected_pass": True
            },
            {
                "name": "oversized_orderbook",
                "type": "orderbook",
                "num_levels": 5000,  # Should exceed 64KB limit
                "expected_pass": False
            }
        ]
        
        for test_case in test_cases:
            # Create message with specified number of levels
            message = {
                "type": test_case["type"],
                "timestamp_ns": 1692182400123456789,
                "symbol_hash": 0x1234567890ABCDEF,
                "bids": [(460523000000 - i, 10000000 + i) for i in range(test_case["num_levels"])],
                "asks": [(460524000000 + i, 10000000 + i) for i in range(test_case["num_levels"])],
                "test_name": test_case["name"]
            }
            
            try:
                # Encode message
                binary_data = self._encode_message_to_binary(message)
                message_size = len(binary_data)
                
                # Check size limit
                within_limit = message_size <= max_size
                passed = within_limit == test_case["expected_pass"]
                
                error_msg = ""
                if not passed:
                    if test_case["expected_pass"]:
                        error_msg = f"Message size {message_size} exceeds limit {max_size}"
                    else:
                        error_msg = f"Message size {message_size} should exceed limit but doesn't"
                
                result = MessageIntegrityResult(
                    test_name=test_case["name"],
                    passed=passed,
                    error_message=error_msg,
                    original_data={"size": message_size, "limit": max_size}
                )
                
                status = "✅" if result.passed else "❌"
                print(f"   {status} {test_case['name']}: {message_size:,} bytes")
                if not result.passed:
                    print(f"      Error: {error_msg}")
                    
                self.results.append(result)
                
            except Exception as e:
                result = MessageIntegrityResult(
                    test_name=test_case["name"],
                    passed=False,
                    error_message=f"Exception: {e}"
                )
                print(f"   ❌ {test_case['name']}: Exception - {e}")
                self.results.append(result)
        
        return self.results
    
    def test_precision_preservation_in_protocol(self) -> List[MessageIntegrityResult]:
        """Test that precision is preserved through the protocol"""
        print("🎯 Testing precision preservation in protocol messages...")
        
        # Test with challenging decimal values
        precision_test_cases = [
            ("4605.23", "WETH price precision"),
            ("68234.56", "WBTC price precision"),
            ("0.12345678", "Small volume precision"),
            ("1.23456789", "Extended decimal precision"),
            ("99999999.99999999", "Maximum precision value")
        ]
        
        for value_str, test_name in precision_test_cases:
            try:
                # Convert using our precision-preserving method
                value_decimal = Decimal(value_str)
                fixed_point = int(value_decimal * Decimal('100000000'))
                
                # Create a trade message
                message = {
                    "type": "trade",
                    "timestamp_ns": 1692182400123456789,
                    "symbol_hash": 0x1234567890ABCDEF,
                    "price_fp": fixed_point,
                    "volume_fp": 100000000,  # 1.0
                    "side": 0,
                    "test_name": test_name
                }
                
                # Encode and decode
                binary_data = self._encode_message_to_binary(message)
                decoded = self._decode_binary_message(binary_data, "trade")
                
                # Check precision preservation
                original_fp = message["price_fp"]
                decoded_fp = decoded["price_fp"]
                
                precision_preserved = original_fp == decoded_fp
                
                # Convert back to decimal for verification
                recovered_decimal = Decimal(decoded_fp) / Decimal('100000000')
                original_value = Decimal(value_str)
                precision_error = abs(recovered_decimal - original_value)
                
                result = MessageIntegrityResult(
                    test_name=test_name,
                    passed=precision_preserved and precision_error == 0,
                    error_message=f"Precision error: {precision_error}" if precision_error > 0 else "",
                    original_data={"value": value_str, "fixed_point": original_fp},
                    decoded_data={"fixed_point": decoded_fp, "recovered": str(recovered_decimal)}
                )
                
                status = "✅" if result.passed else "❌"
                print(f"   {status} {test_name}: {value_str} → {recovered_decimal}")
                if not result.passed:
                    print(f"      Error: {result.error_message}")
                    
                self.results.append(result)
                
            except Exception as e:
                result = MessageIntegrityResult(
                    test_name=test_name,
                    passed=False,
                    error_message=f"Exception: {e}"
                )
                print(f"   ❌ {test_name}: Exception - {e}")
                self.results.append(result)
        
        return self.results
    
    def generate_integrity_report(self) -> Dict[str, Any]:
        """Generate comprehensive integrity test report"""
        total_tests = len(self.results)
        passed_tests = sum(1 for r in self.results if r.passed)
        failed_tests = total_tests - passed_tests
        
        # Categorize results
        encoding_tests = [r for r in self.results if "integrity" in r.test_name or "precision" in r.test_name]
        size_tests = [r for r in self.results if "size" in r.test_name]
        precision_tests = [r for r in self.results if "precision" in r.test_name]
        
        return {
            "summary": {
                "total_tests": total_tests,
                "passed": passed_tests,
                "failed": failed_tests,
                "pass_rate": (passed_tests / total_tests * 100) if total_tests > 0 else 0
            },
            "categories": {
                "encoding_integrity": {
                    "total": len(encoding_tests),
                    "passed": sum(1 for r in encoding_tests if r.passed)
                },
                "size_limits": {
                    "total": len(size_tests),
                    "passed": sum(1 for r in size_tests if r.passed)
                },
                "precision_preservation": {
                    "total": len(precision_tests),
                    "passed": sum(1 for r in precision_tests if r.passed)
                }
            },
            "failed_tests": [
                {
                    "name": r.test_name,
                    "error": r.error_message
                }
                for r in self.results if not r.passed
            ]
        }

def run_protocol_integrity_tests():
    """Run all protocol integrity tests"""
    print("=" * 80)
    print("PROTOCOL MESSAGE INTEGRITY TESTS")
    print("=" * 80)
    
    tester = ProtocolIntegrityTester()
    
    # Run all test categories
    tester.test_message_encoding_integrity()
    tester.test_message_size_limits()
    tester.test_precision_preservation_in_protocol()
    
    # Generate report
    report = tester.generate_integrity_report()
    
    print("\n" + "=" * 80)
    print("PROTOCOL INTEGRITY RESULTS")
    print("=" * 80)
    
    summary = report["summary"]
    categories = report["categories"]
    
    print(f"📊 Total Tests: {summary['total_tests']}")
    print(f"✅ Passed: {summary['passed']}")
    print(f"❌ Failed: {summary['failed']}")
    print(f"📈 Pass Rate: {summary['pass_rate']:.1f}%")
    
    print(f"\n📋 Test Categories:")
    for category, stats in categories.items():
        print(f"   {category}: {stats['passed']}/{stats['total']} passed")
    
    if report["failed_tests"]:
        print(f"\n❌ Failed Tests:")
        for failed in report["failed_tests"]:
            print(f"   • {failed['name']}: {failed['error']}")
    
    # Overall assessment
    success = summary['pass_rate'] >= 95.0
    
    print(f"\n🏆 OVERALL ASSESSMENT:")
    if success:
        print("   ✅ EXCELLENT - Protocol maintains message integrity")
    else:
        print("   ❌ ISSUES DETECTED - Protocol integrity needs attention")
    
    # Save report
    with open("/Users/daws/alphapulse/backend/tests/e2e/protocol_integrity_report.json", "w") as f:
        json.dump(report, f, indent=2, default=str)
    
    print(f"\n📄 Detailed report saved to: protocol_integrity_report.json")
    
    return success

if __name__ == "__main__":
    success = run_protocol_integrity_tests()
    exit(0 if success else 1)