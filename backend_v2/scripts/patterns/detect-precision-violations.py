#!/usr/bin/env python3
"""
AlphaPulse Precision Violation Detection
Detects float/double usage in financial calculation contexts
"""

import re
import os
import sys
import argparse
from pathlib import Path
from typing import List, Set, Tuple, Optional

# Financial context keywords that indicate problematic float usage
FINANCIAL_KEYWORDS = {
    # Price-related
    'price', 'bid', 'ask', 'spread', 'cost', 'value', 'worth',
    # Trading-related  
    'trade', 'order', 'position', 'quantity', 'amount', 'volume',
    # Financial calculations
    'profit', 'loss', 'fee', 'commission', 'interest', 'yield', 'return',
    # Portfolio and assets
    'portfolio', 'asset', 'balance', 'equity', 'capital', 'fund',
    # DEX-specific
    'reserve', 'liquidity', 'swap', 'mint', 'burn', 'slippage',
    # Currency and monetary
    'usd', 'eth', 'btc', 'token', 'coin', 'currency', 'money', 'wei'
}

# File patterns that are whitelisted for float usage
WHITELIST_PATTERNS = [
    r'.*graphics.*',
    r'.*ui/.*',
    r'.*display.*',
    r'.*render.*',
    r'.*math.*',  # For non-financial math
    r'.*geometry.*',
    r'.*physics.*',
    r'.*test.*',  # Test files are more lenient
]

# Float type patterns to detect
FLOAT_PATTERNS = [
    r'\bf32\b',
    r'\bf64\b',
    r'\bdouble\b',
    r'\bfloat\b',
]

class PrecisionViolation:
    def __init__(self, filename: str, line_number: int, line_content: str, 
                 float_type: str, context: str, suggestion: str):
        self.filename = filename
        self.line_number = line_number
        self.line_content = line_content.strip()
        self.float_type = float_type
        self.context = context
        self.suggestion = suggestion

class PrecisionDetector:
    def __init__(self, whitelist_patterns: Optional[List[str]] = None):
        self.whitelist_patterns = whitelist_patterns or WHITELIST_PATTERNS
        self.violations: List[PrecisionViolation] = []
        
    def is_whitelisted(self, filepath: str) -> bool:
        """Check if file is whitelisted for float usage"""
        normalized_path = filepath.lower()
        for pattern in self.whitelist_patterns:
            if re.search(pattern, normalized_path):
                return True
        return False
    
    def has_financial_context(self, line: str, surrounding_lines: List[str]) -> Tuple[bool, str]:
        """
        Check if the line containing float usage is in financial context
        Returns (is_financial, context_description)
        """
        # Combine current line with surrounding context
        context_text = ' '.join([line] + surrounding_lines).lower()
        
        # Check for financial keywords
        found_keywords = []
        for keyword in FINANCIAL_KEYWORDS:
            if keyword in context_text:
                found_keywords.append(keyword)
        
        if found_keywords:
            return True, f"Financial context: {', '.join(found_keywords[:3])}"
        
        # Check for financial patterns in variable names and function names
        financial_patterns = [
            r'\b\w*price\w*\b',
            r'\b\w*amount\w*\b', 
            r'\b\w*value\w*\b',
            r'\b\w*balance\w*\b',
            r'\b\w*fee\w*\b',
            r'\b\w*profit\w*\b',
        ]
        
        for pattern in financial_patterns:
            if re.search(pattern, context_text):
                return True, "Financial variable/function naming"
        
        return False, ""
    
    def get_suggestion(self, float_type: str, context: str, line: str) -> str:
        """Generate appropriate suggestion based on context"""
        suggestions = []
        
        if any(keyword in context.lower() for keyword in ['dex', 'swap', 'reserve', 'wei', 'eth']):
            suggestions.append("Use native token precision (18 decimals for WETH, 6 for USDC)")
            suggestions.append("Example: let amount_wei: i64 = 1_000_000_000_000_000_000; // 1 WETH")
        elif any(keyword in context.lower() for keyword in ['usd', 'price', 'value']):
            suggestions.append("Use 8-decimal fixed-point for USD values")
            suggestions.append("Example: let price_fixed: i64 = 4500000000000; // $45,000.00 (* 100_000_000)")
        else:
            suggestions.append("Use fixed-point arithmetic with i64/u64")
            suggestions.append("Example: Multiply by 10^8 for 8-decimal precision")
        
        return " | ".join(suggestions)
    
    def detect_in_line(self, line: str, line_number: int, filename: str, 
                      surrounding_lines: List[str]) -> List[PrecisionViolation]:
        """Detect precision violations in a single line"""
        violations = []
        
        # Skip comments
        if line.strip().startswith('//') or line.strip().startswith('*'):
            return violations
            
        # Skip string literals (basic detection)
        if '"' in line and any(pattern in line for pattern in ['f32', 'f64', 'float', 'double']):
            # Check if float usage is inside string literal
            in_string = False
            for i, char in enumerate(line):
                if char == '"' and (i == 0 or line[i-1] != '\\'):
                    in_string = not in_string
                if not in_string:
                    break
            else:
                # Everything after last quote is in string
                return violations
        
        # Look for float type usage
        for pattern in FLOAT_PATTERNS:
            matches = re.finditer(pattern, line)
            for match in matches:
                float_type = match.group()
                
                # Check if this is in financial context
                is_financial, context = self.has_financial_context(line, surrounding_lines)
                
                if is_financial:
                    suggestion = self.get_suggestion(float_type, context, line)
                    
                    violation = PrecisionViolation(
                        filename=filename,
                        line_number=line_number,
                        line_content=line,
                        float_type=float_type,
                        context=context,
                        suggestion=suggestion
                    )
                    violations.append(violation)
        
        return violations
    
    def detect_in_file(self, filepath: str) -> List[PrecisionViolation]:
        """Detect precision violations in a single file"""
        if not filepath.endswith('.rs'):
            return []
            
        if self.is_whitelisted(filepath):
            return []
        
        try:
            with open(filepath, 'r', encoding='utf-8') as f:
                lines = f.readlines()
        except (UnicodeDecodeError, IOError):
            return []
        
        violations = []
        
        for i, line in enumerate(lines):
            line_number = i + 1
            
            # Get surrounding context (3 lines before and after)
            start_idx = max(0, i - 3)
            end_idx = min(len(lines), i + 4)
            surrounding_lines = [lines[j].strip() for j in range(start_idx, end_idx) if j != i]
            
            line_violations = self.detect_in_line(line, line_number, filepath, surrounding_lines)
            violations.extend(line_violations)
        
        return violations
    
    def detect_in_directory(self, directory: str) -> List[PrecisionViolation]:
        """Detect precision violations in all Rust files in directory"""
        violations = []
        
        for root, dirs, files in os.walk(directory):
            for file in files:
                if file.endswith('.rs'):
                    filepath = os.path.join(root, file)
                    file_violations = self.detect_in_file(filepath)
                    violations.extend(file_violations)
        
        return violations
    
    def format_violation_report(self, violations: List[PrecisionViolation]) -> str:
        """Format violations into a readable report"""
        if not violations:
            return "✅ No precision violations found"
        
        report_lines = []
        report_lines.append("❌ Precision usage violations detected")
        report_lines.append("=" * 50)
        report_lines.append("")
        
        for violation in violations:
            report_lines.append(f"VIOLATION: Float usage in financial context")
            report_lines.append(f"  File: {violation.filename}:{violation.line_number}")
            report_lines.append(f"  Found: {violation.line_content}")
            report_lines.append(f"  Type: {violation.float_type}")
            report_lines.append(f"  Context: {violation.context}")
            report_lines.append(f"  Suggestion: {violation.suggestion}")
            report_lines.append("")
        
        report_lines.append("To fix these violations:")
        report_lines.append("1. Replace float types with fixed-point arithmetic using i64/u64")
        report_lines.append("2. For DEX tokens: preserve native precision (18 decimals WETH, 6 USDC)")
        report_lines.append("3. For USD prices: use 8-decimal fixed-point (* 100_000_000)")
        report_lines.append("4. If float usage is legitimate, add file to whitelist")
        
        return "\n".join(report_lines)

def main():
    parser = argparse.ArgumentParser(description='Detect precision violations in Rust code')
    parser.add_argument('target', help='File or directory to scan')
    parser.add_argument('--whitelist', help='Custom whitelist file')
    parser.add_argument('--quiet', action='store_true', help='Suppress output')
    
    args = parser.parse_args()
    
    # Load custom whitelist if provided
    whitelist_patterns = WHITELIST_PATTERNS
    if args.whitelist and os.path.exists(args.whitelist):
        with open(args.whitelist, 'r') as f:
            custom_patterns = [line.strip() for line in f if line.strip() and not line.startswith('#')]
            whitelist_patterns.extend(custom_patterns)
    
    detector = PrecisionDetector(whitelist_patterns)
    
    if not os.path.exists(args.target):
        print(f"Error: Path '{args.target}' does not exist", file=sys.stderr)
        sys.exit(1)
    
    # Detect violations
    if os.path.isfile(args.target):
        violations = detector.detect_in_file(args.target)
    elif os.path.isdir(args.target):
        violations = detector.detect_in_directory(args.target)
    else:
        print(f"Error: '{args.target}' is not a file or directory", file=sys.stderr)
        sys.exit(1)
    
    # Generate report
    if not args.quiet:
        print("AlphaPulse Precision Violation Detection")
        print("=" * 40)
        print("")
        if os.path.isfile(args.target):
            print(f"Scanning file: {args.target}")
        else:
            print(f"Scanning directory: {args.target}")
        print("")
        
        report = detector.format_violation_report(violations)
        print(report)
    
    # Exit with appropriate code
    sys.exit(1 if violations else 0)

if __name__ == '__main__':
    main()