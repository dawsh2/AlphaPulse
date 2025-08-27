#!/usr/bin/env python3
"""
Test for Transport Usage Violation Detection
Following TDD - these tests should FAIL initially until implementation is done
"""

import unittest
import tempfile
import os
import subprocess
from pathlib import Path


class TestTransportViolationDetection(unittest.TestCase):
    """Test the transport violation detection script"""
    
    def setUp(self):
        """Set up test environment"""
        self.test_dir = tempfile.mkdtemp()
        self.script_path = Path(__file__).parent.parent.parent / "scripts" / "patterns" / "detect-transport-violations.sh"
        
    def tearDown(self):
        """Clean up test environment"""
        # Clean up temp files
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
                ["bash", str(self.script_path), target_path],
                capture_output=True,
                text=True,
                timeout=10
            )
            return result.stdout, result.stderr, result.returncode
        except FileNotFoundError:
            # Script doesn't exist yet - expected in TDD RED phase
            return "", "Script not found", 1
        except subprocess.TimeoutExpired:
            return "", "Timeout", 1
    
    def test_script_exists_and_executable(self):
        """Test that the detection script exists and is executable"""
        # This should FAIL initially (RED phase)
        self.assertTrue(self.script_path.exists(), f"Script not found at {self.script_path}")
        self.assertTrue(os.access(self.script_path, os.X_OK), "Script is not executable")
    
    def test_detects_direct_transport_usage(self):
        """Test detection of direct UnixSocketTransport::new usage"""
        # Create test file with violation
        violation_content = '''
use alphapulse_transport::UnixSocketTransport;

pub fn create_connection() {
    let transport = UnixSocketTransport::new("/tmp/socket");  // VIOLATION
    transport.connect().unwrap();
}
'''
        filepath = self.create_test_file("src/bad_transport.rs", violation_content)
        
        stdout, stderr, returncode = self.run_detection_script(filepath)
        
        # Should detect the violation
        self.assertNotEqual(returncode, 0, "Script should detect violation and return non-zero")
        self.assertIn("UnixSocketTransport::new", stdout, "Should report the violation")
        self.assertIn("bad_transport.rs", stdout, "Should report the file name")
    
    def test_ignores_approved_factory_usage(self):
        """Test that approved factory patterns are not flagged"""
        # Create test file with approved usage
        approved_content = '''
// This file is whitelisted for direct transport usage
use alphapulse_transport::{TransportFactory, UnixSocketTransport};

pub fn create_factory() -> TransportFactory {
    // Factory implementations are allowed to use direct transport
    TransportFactory::new()
}

pub fn approved_function() {
    let transport = UnixSocketTransport::new("/tmp/socket");  // OK in factory
}
'''
        # Create in approved location (should be configurable)
        filepath = self.create_test_file("src/transport/factory.rs", approved_content)
        
        stdout, stderr, returncode = self.run_detection_script(filepath)
        
        # Should NOT detect violation in approved locations
        self.assertEqual(returncode, 0, "Script should not flag approved usage")
        self.assertNotIn("violation", stdout.lower(), "Should not report violations")
    
    def test_provides_helpful_error_messages(self):
        """Test that error messages are helpful with suggestions"""
        violation_content = '''
let transport = UnixSocketTransport::new("/tmp/socket");
'''
        filepath = self.create_test_file("src/service.rs", violation_content)
        
        stdout, stderr, returncode = self.run_detection_script(filepath)
        
        # Should provide helpful guidance
        self.assertIn("Use TransportFactory::create()", stdout, "Should suggest factory usage")
        self.assertIn("service.rs", stdout, "Should show file location")
    
    def test_handles_multiple_violations_in_file(self):
        """Test detection of multiple violations in single file"""
        multi_violation_content = '''
pub fn bad_function1() {
    let transport1 = UnixSocketTransport::new("/tmp/sock1");  // VIOLATION 1
}

pub fn bad_function2() {
    let transport2 = UnixSocketTransport::new("/tmp/sock2");  // VIOLATION 2
}
'''
        filepath = self.create_test_file("src/multi_bad.rs", multi_violation_content)
        
        stdout, stderr, returncode = self.run_detection_script(filepath)
        
        # Should detect both violations
        self.assertNotEqual(returncode, 0)
        violation_count = stdout.count("UnixSocketTransport::new")
        self.assertEqual(violation_count, 2, f"Should detect 2 violations, found {violation_count}")
    
    def test_handles_directory_scanning(self):
        """Test that script can scan entire directories"""
        # Create multiple files with violations
        self.create_test_file("src/file1.rs", 'let t = UnixSocketTransport::new("/tmp/1");')
        self.create_test_file("src/file2.rs", 'let t = UnixSocketTransport::new("/tmp/2");')
        self.create_test_file("src/file3.rs", 'let t = SomeOtherType::new();')  # No violation
        
        stdout, stderr, returncode = self.run_detection_script(self.test_dir)
        
        # Should find violations in both files
        self.assertNotEqual(returncode, 0)
        self.assertIn("file1.rs", stdout)
        self.assertIn("file2.rs", stdout)
        self.assertNotIn("file3.rs", stdout)  # Clean file should not be mentioned
    
    def test_whitelist_configuration(self):
        """Test that whitelist mechanism works"""
        # Create violation in file that should be whitelisted
        violation_content = 'let transport = UnixSocketTransport::new("/tmp/socket");'
        filepath = self.create_test_file("src/transport/factory.rs", violation_content)
        
        # Create whitelist config
        whitelist_config = '''
# Approved locations for direct transport usage
src/transport/factory.rs
src/transport/builder.rs
libs/transport/src/factory.rs
'''
        config_path = self.create_test_file("scripts/patterns/transport-whitelist.txt", whitelist_config)
        
        stdout, stderr, returncode = self.run_detection_script(filepath)
        
        # Should respect whitelist
        self.assertEqual(returncode, 0, "Whitelisted files should not trigger violations")
    
    def test_performance_on_large_codebase(self):
        """Test that script performs reasonably on large codebases"""
        # Create many files to simulate large codebase
        for i in range(50):
            content = f'// File {i}\npub fn function_{i}() {{ /* no violations */ }}'
            self.create_test_file(f"src/file_{i}.rs", content)
        
        # Add one violation
        self.create_test_file("src/file_violation.rs", 'let t = UnixSocketTransport::new("/tmp/test");')
        
        import time
        start_time = time.time()
        stdout, stderr, returncode = self.run_detection_script(self.test_dir)
        end_time = time.time()
        
        # Should complete in reasonable time (< 5 seconds for 50 files)
        duration = end_time - start_time
        self.assertLess(duration, 5.0, f"Script took {duration:.2f}s, should be < 5s")
        
        # Should still find the violation
        self.assertNotEqual(returncode, 0)
        self.assertIn("file_violation.rs", stdout)
    
    def test_handles_comments_and_strings(self):
        """Test that script doesn't flag usage in comments or strings"""
        safe_content = '''
// This comment mentions UnixSocketTransport::new but shouldn't be flagged
pub fn example() {
    let doc = "Example: UnixSocketTransport::new() creates a connection";
    println!("Don't use UnixSocketTransport::new directly");
    /* 
     * Block comment with UnixSocketTransport::new
     * should not be detected
     */
}
'''
        filepath = self.create_test_file("src/safe_usage.rs", safe_content)
        
        stdout, stderr, returncode = self.run_detection_script(filepath)
        
        # Should NOT detect violations in comments/strings
        self.assertEqual(returncode, 0, "Comments and strings should not trigger violations")
    
    def test_zero_false_positives_on_current_codebase(self):
        """Test that script produces zero false positives on actual codebase"""
        # This tests against the real AlphaPulse codebase
        real_codebase_path = Path(__file__).parent.parent.parent
        
        stdout, stderr, returncode = self.run_detection_script(str(real_codebase_path))
        
        # Parse output to check for expected violations vs false positives
        if returncode != 0:
            # If violations found, they should be legitimate
            # This test helps ensure we tune the detection correctly
            lines = stdout.strip().split('\n')
            for line in lines:
                if 'UnixSocketTransport::new' in line:
                    # Each violation should be in non-whitelisted location
                    # This assertion will help us refine the whitelist
                    self.assertNotIn('/transport/factory.rs', line, 
                                   f"Factory files should be whitelisted: {line}")


if __name__ == '__main__':
    unittest.main()