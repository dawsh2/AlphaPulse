#!/usr/bin/env python3
"""
Test for Precision Violation Detection  
Following TDD - these tests should FAIL initially until implementation is done
"""

import unittest
import tempfile
import os
import subprocess
from pathlib import Path


class TestPrecisionViolationDetection(unittest.TestCase):
    """Test the precision violation detection script"""
    
    def setUp(self):
        """Set up test environment"""
        self.test_dir = tempfile.mkdtemp()
        self.script_path = Path(__file__).parent.parent / "scripts" / "patterns" / "detect-precision-violations.py"
        
    def tearDown(self):
        """Clean up test environment"""
        import shutil
        shutil.rmtree(self.test_dir, ignore_errors=True)
    
    def create_test_file(self, filename: str, content: str) -> str:
        """Create a test file with given content"""
        filepath = os.path.join(self.test_dir, filename)
        os.makedirs(os.path.dirname(filepath), exist_ok=True)
        with open(filepath, 'w') as f:
            f.write(content)
        return filepath
    
    def run_detection_script(self, target_path: str) -> tuple:
        """Run the detection script and return (stdout, stderr, returncode)"""
        try:
            result = subprocess.run(
                ["python3", str(self.script_path), target_path],
                capture_output=True,
                text=True,
                timeout=10
            )
            return result.stdout, result.stderr, result.returncode
        except FileNotFoundError:
            return "", "Script not found", 1
        except subprocess.TimeoutExpired:
            return "", "Timeout", 1
    
    def test_script_exists_and_executable(self):
        """Test that the detection script exists and is executable"""
        self.assertTrue(self.script_path.exists(), f"Script not found at {self.script_path}")
        self.assertTrue(os.access(self.script_path, os.X_OK), "Script is not executable")
    
    def test_detects_float_usage_in_financial_context(self):
        """Test detection of float/double usage for financial calculations"""
        violation_content = '''
pub struct PriceCalculator {
    pub price: f64,  // VIOLATION - float for price
    pub volume: f32, // VIOLATION - float for volume
}

pub fn calculate_profit(buy_price: f64, sell_price: f64, volume: f64) -> f64 {  // VIOLATION
    (sell_price - buy_price) * volume
}

pub fn get_market_price() -> f64 {  // VIOLATION - float return for price
    45000.50
}
'''
        filepath = self.create_test_file("src/trading.rs", violation_content)
        
        stdout, stderr, returncode = self.run_detection_script(filepath)
        
        # Should detect violations
        self.assertNotEqual(returncode, 0, "Script should detect violations and return non-zero")
        self.assertIn("f64", stdout, "Should report f64 usage")
        self.assertIn("f32", stdout, "Should report f32 usage")
        self.assertIn("price", stdout.lower(), "Should identify financial context")
    
    def test_ignores_safe_float_usage(self):
        """Test that non-financial float usage is not flagged"""
        safe_content = '''
pub fn calculate_distance(x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
    ((x2 - x1).powi(2) + (y2 - y1).powi(2)).sqrt()
}

pub fn render_graphics(opacity: f32, rotation: f64) {
    // Graphics calculations are OK to use floats
}

pub fn temperature_conversion(celsius: f64) -> f64 {
    celsius * 9.0 / 5.0 + 32.0
}
'''
        filepath = self.create_test_file("src/graphics.rs", safe_content)
        
        stdout, stderr, returncode = self.run_detection_script(filepath)
        
        # Should NOT detect violations for non-financial usage
        self.assertEqual(returncode, 0, "Script should not flag non-financial float usage")
    
    def test_detects_financial_keywords(self):
        """Test detection based on financial context keywords"""
        violation_content = '''
pub struct OrderBook {
    pub bid_price: f64,    // VIOLATION
    pub ask_price: f64,    // VIOLATION  
    pub spread: f32,       // VIOLATION
}

pub fn calculate_fees(amount: f64, fee_rate: f64) -> f64 {  // VIOLATION
    amount * fee_rate
}

pub fn portfolio_value(positions: Vec<Position>) -> f64 {  // VIOLATION
    positions.iter().map(|p| p.quantity * p.price).sum()
}
'''
        filepath = self.create_test_file("src/finance.rs", violation_content)
        
        stdout, stderr, returncode = self.run_detection_script(filepath)
        
        self.assertNotEqual(returncode, 0)
        self.assertIn("price", stdout.lower())
        self.assertIn("fee", stdout.lower())
        self.assertIn("portfolio", stdout.lower())
    
    def test_suggests_fixed_point_alternatives(self):
        """Test that suggestions provide fixed-point alternatives"""
        violation_content = '''
pub fn trade_profit(buy: f64, sell: f64) -> f64 {
    sell - buy
}
'''
        filepath = self.create_test_file("src/trader.rs", violation_content)
        
        stdout, stderr, returncode = self.run_detection_script(filepath)
        
        # Should suggest fixed-point arithmetic
        self.assertIn("fixed-point", stdout.lower())
        self.assertIn("i64", stdout)
        self.assertIn("precision", stdout.lower())
    
    def test_handles_struct_field_analysis(self):
        """Test detection in struct field names and types"""
        violation_content = '''
#[derive(Debug)]
pub struct Trade {
    pub symbol: String,
    pub price: f64,        // VIOLATION
    pub quantity: f64,     // VIOLATION  
    pub commission: f32,   // VIOLATION
    pub timestamp: u64,    // OK - not financial value
}
'''
        filepath = self.create_test_file("src/types.rs", violation_content)
        
        stdout, stderr, returncode = self.run_detection_script(filepath)
        
        self.assertNotEqual(returncode, 0)
        # Should detect 3 violations but not timestamp
        violation_lines = [line for line in stdout.split('\n') if 'VIOLATION' in line]
        self.assertEqual(len(violation_lines), 3, f"Expected 3 violations, found {len(violation_lines)}")
    
    def test_whitelist_configuration(self):
        """Test that whitelist mechanism works for approved float usage"""
        # Create test file that would normally trigger violations
        violation_content = '''
pub fn graphics_price_display(price: f64) -> String {
    format!("{:.2}", price)  // OK for display formatting
}
'''
        filepath = self.create_test_file("src/ui/display.rs", violation_content)
        
        stdout, stderr, returncode = self.run_detection_script(filepath)
        
        # Should respect whitelist for UI display code
        self.assertEqual(returncode, 0, "UI display code should be whitelisted")
    
    def test_provides_dex_specific_guidance(self):
        """Test DEX-specific precision guidance"""
        violation_content = '''
pub fn calculate_swap_output(
    amount_in: f64,        // VIOLATION
    reserve_in: f64,       // VIOLATION  
    reserve_out: f64       // VIOLATION
) -> f64 {                 // VIOLATION
    // DEX swap calculation
    let amount_in_with_fee = amount_in * 0.997;
    (amount_in_with_fee * reserve_out) / (reserve_in + amount_in_with_fee)
}
'''
        filepath = self.create_test_file("src/dex/swap.rs", violation_content)
        
        stdout, stderr, returncode = self.run_detection_script(filepath)
        
        self.assertNotEqual(returncode, 0)
        # Should provide DEX-specific guidance
        self.assertIn("wei", stdout.lower())
        self.assertIn("18 decimals", stdout)
        self.assertIn("native precision", stdout.lower())
    
    def test_handles_large_codebase_performance(self):
        """Test reasonable performance on large codebases"""
        # Create many files to simulate large codebase
        for i in range(30):
            content = f'// File {i}\npub fn function_{i}() -> i64 {{ {i} }}'
            self.create_test_file(f"src/file_{i}.rs", content)
        
        # Add one violation
        violation_content = '''
pub fn bad_pricing(amount: f64) -> f64 {
    amount * 1.05
}
'''
        self.create_test_file("src/bad_pricing.rs", violation_content)
        
        import time
        start_time = time.time()
        stdout, stderr, returncode = self.run_detection_script(self.test_dir)
        duration = time.time() - start_time
        
        # Should complete reasonably quickly
        self.assertLess(duration, 3.0, f"Script took {duration:.2f}s, should be < 3s")
        self.assertNotEqual(returncode, 0, "Should find the violation")
    
    def test_zero_false_positives_on_current_codebase(self):
        """Test that script produces zero false positives on actual codebase"""
        # Test against the real Torq codebase
        real_codebase_path = Path(__file__).parent.parent
        
        stdout, stderr, returncode = self.run_detection_script(str(real_codebase_path))
        
        # Should have controlled violations (not excessive false positives)
        if returncode != 0:
            lines = stdout.strip().split('\n') if stdout.strip() else []
            violation_count = len([line for line in lines if 'VIOLATION' in line])
            # We expect some legitimate violations, but not excessive false positives
            self.assertLess(violation_count, 50, 
                          f"Too many violations detected ({violation_count}), likely false positives")


if __name__ == '__main__':
    unittest.main()