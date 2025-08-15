#!/usr/bin/env python3
"""
Comprehensive Test Reporting and Monitoring

Aggregates all test results into a comprehensive report with monitoring
capabilities and actionable insights for the data validation pipeline.
"""

import json
import time
import os
import statistics
from typing import Dict, List, Any, Optional
from datetime import datetime, timezone
from dataclasses import dataclass, field
import subprocess

@dataclass
class TestSuiteResult:
    """Result from a test suite execution"""
    suite_name: str
    passed: bool
    total_tests: int
    passed_tests: int
    failed_tests: int
    execution_time_ms: float
    error_messages: List[str] = field(default_factory=list)
    metrics: Dict[str, Any] = field(default_factory=dict)

@dataclass
class ValidationPipelineHealth:
    """Overall health assessment of the validation pipeline"""
    overall_status: str  # "excellent", "good", "warning", "critical"
    confidence_score: float  # 0-100
    issues: List[str] = field(default_factory=list)
    recommendations: List[str] = field(default_factory=list)
    performance_grade: str = "A"  # A, B, C, D, F
    reliability_grade: str = "A"
    precision_grade: str = "A"

class ComprehensiveTestReporter:
    """Comprehensive test reporting and monitoring system"""
    
    def __init__(self):
        self.test_results: List[TestSuiteResult] = []
        self.start_time = time.time()
        self.report_timestamp = datetime.now(timezone.utc)
        
    def run_all_test_suites(self) -> List[TestSuiteResult]:
        """Run all test suites and collect results"""
        print("=" * 80)
        print("COMPREHENSIVE VALIDATION PIPELINE TEST EXECUTION")
        print("=" * 80)
        
        test_suites = [
            {
                "name": "Mock Data Validation",
                "script": "test_with_mock_data.py",
                "description": "Tests validation logic with simulated data"
            },
            {
                "name": "Live Exchange Validation", 
                "script": "test_live_exchange_validation.py",
                "description": "Tests with real exchange data feeds"
            },
            {
                "name": "Protocol Message Integrity",
                "script": "test_protocol_integrity.py", 
                "description": "Tests binary protocol message integrity"
            },
            {
                "name": "Performance and Stress",
                "script": "test_performance_stress.py",
                "description": "Performance testing under various loads"
            },
            {
                "name": "Decimal Precision",
                "script": "test_decimal_precision.py",
                "description": "Precision preservation validation"
            }
        ]
        
        for suite in test_suites:
            print(f"\n🧪 Running {suite['name']}...")
            print(f"   {suite['description']}")
            
            result = self._execute_test_suite(suite)
            self.test_results.append(result)
            
            status = "✅ PASS" if result.passed else "❌ FAIL"
            print(f"   {status} ({result.passed_tests}/{result.total_tests} tests, {result.execution_time_ms:.0f}ms)")
            
            if not result.passed:
                for error in result.error_messages[:3]:  # Show first 3 errors
                    print(f"      • {error}")
                if len(result.error_messages) > 3:
                    print(f"      • ... and {len(result.error_messages) - 3} more errors")
        
        return self.test_results
    
    def _execute_test_suite(self, suite: Dict[str, str]) -> TestSuiteResult:
        """Execute a single test suite and collect results"""
        start_time = time.time()
        
        try:
            # Execute the test script
            result = subprocess.run(
                ["python", suite["script"]], 
                capture_output=True, 
                text=True,
                timeout=300  # 5 minute timeout
            )
            
            execution_time = (time.time() - start_time) * 1000
            
            # Parse the results based on return code and output
            if result.returncode == 0:
                # Test passed
                metrics = self._parse_test_output(result.stdout, suite["name"])
                return TestSuiteResult(
                    suite_name=suite["name"],
                    passed=True,
                    total_tests=metrics.get("total_tests", 1),
                    passed_tests=metrics.get("passed_tests", 1),
                    failed_tests=metrics.get("failed_tests", 0),
                    execution_time_ms=execution_time,
                    metrics=metrics
                )
            else:
                # Test failed
                error_messages = self._extract_error_messages(result.stderr, result.stdout)
                return TestSuiteResult(
                    suite_name=suite["name"],
                    passed=False,
                    total_tests=1,
                    passed_tests=0,
                    failed_tests=1,
                    execution_time_ms=execution_time,
                    error_messages=error_messages
                )
                
        except subprocess.TimeoutExpired:
            return TestSuiteResult(
                suite_name=suite["name"],
                passed=False,
                total_tests=1,
                passed_tests=0,
                failed_tests=1,
                execution_time_ms=(time.time() - start_time) * 1000,
                error_messages=["Test suite timed out after 5 minutes"]
            )
        except Exception as e:
            return TestSuiteResult(
                suite_name=suite["name"],
                passed=False,
                total_tests=1,
                passed_tests=0,
                failed_tests=1,
                execution_time_ms=(time.time() - start_time) * 1000,
                error_messages=[f"Test execution failed: {e}"]
            )
    
    def _parse_test_output(self, stdout: str, suite_name: str) -> Dict[str, Any]:
        """Parse test output to extract metrics"""
        metrics = {}
        
        # Look for common patterns in test output
        lines = stdout.split('\n')
        
        for line in lines:
            # Parse test counts
            if "Total Tests:" in line or "📊 Total Tests:" in line:
                try:
                    metrics["total_tests"] = int(line.split(':')[-1].strip().replace(',', ''))
                except ValueError:
                    pass
            
            if "Passed:" in line or "✅ Passed:" in line:
                try:
                    metrics["passed_tests"] = int(line.split(':')[-1].strip().replace(',', ''))
                except ValueError:
                    pass
            
            if "Failed:" in line or "❌ Failed:" in line:
                try:
                    metrics["failed_tests"] = int(line.split(':')[-1].strip().replace(',', ''))
                except ValueError:
                    pass
            
            # Parse performance metrics
            if "Throughput:" in line or "📊 Throughput:" in line:
                try:
                    # Extract number from "X,XXX msg/sec" format
                    throughput_str = line.split(':')[-1].strip()
                    if "msg/sec" in throughput_str:
                        throughput = float(throughput_str.split()[0].replace(',', ''))
                        metrics["throughput_msg_per_sec"] = throughput
                except (ValueError, IndexError):
                    pass
            
            # Parse precision metrics
            if "Max Error:" in line or "max_error=" in line:
                try:
                    # Extract scientific notation numbers
                    parts = line.split()
                    for part in parts:
                        if 'e-' in part or 'e+' in part:
                            metrics["max_precision_error"] = float(part)
                            break
                except (ValueError, IndexError):
                    pass
            
            # Parse pass rates
            if "Pass Rate:" in line or "📈 Pass Rate:" in line:
                try:
                    rate_str = line.split(':')[-1].strip().replace('%', '')
                    metrics["pass_rate"] = float(rate_str)
                except ValueError:
                    pass
        
        return metrics
    
    def _extract_error_messages(self, stderr: str, stdout: str) -> List[str]:
        """Extract error messages from test output"""
        errors = []
        
        # Check stderr first
        if stderr.strip():
            errors.extend(stderr.strip().split('\n')[:5])  # First 5 lines
        
        # Look for error patterns in stdout
        for line in stdout.split('\n'):
            if any(pattern in line.lower() for pattern in ['error:', 'failed:', 'exception:', 'traceback']):
                errors.append(line.strip())
                if len(errors) >= 10:  # Limit to 10 errors
                    break
        
        return errors
    
    def assess_pipeline_health(self) -> ValidationPipelineHealth:
        """Assess overall health of the validation pipeline"""
        if not self.test_results:
            return ValidationPipelineHealth(
                overall_status="critical",
                confidence_score=0.0,
                issues=["No test results available"],
                recommendations=["Run test suites to assess pipeline health"]
            )
        
        # Calculate overall metrics
        total_tests = sum(r.total_tests for r in self.test_results)
        total_passed = sum(r.passed_tests for r in self.test_results)
        total_failed = sum(r.failed_tests for r in self.test_results)
        
        overall_pass_rate = (total_passed / total_tests * 100) if total_tests > 0 else 0
        suite_pass_rate = (sum(1 for r in self.test_results if r.passed) / len(self.test_results) * 100)
        
        # Collect performance metrics
        performance_metrics = []
        precision_metrics = []
        
        for result in self.test_results:
            if "throughput_msg_per_sec" in result.metrics:
                performance_metrics.append(result.metrics["throughput_msg_per_sec"])
            if "max_precision_error" in result.metrics:
                precision_metrics.append(result.metrics["max_precision_error"])
        
        # Assessment logic
        issues = []
        recommendations = []
        
        # Performance grading
        avg_throughput = statistics.mean(performance_metrics) if performance_metrics else 0
        if avg_throughput >= 500000:
            performance_grade = "A"
        elif avg_throughput >= 100000:
            performance_grade = "B"
        elif avg_throughput >= 50000:
            performance_grade = "C"
        elif avg_throughput >= 10000:
            performance_grade = "D"
        else:
            performance_grade = "F"
            issues.append(f"Low throughput: {avg_throughput:,.0f} msg/sec")
            recommendations.append("Optimize validation algorithms for better performance")
        
        # Reliability grading
        if suite_pass_rate >= 95:
            reliability_grade = "A"
        elif suite_pass_rate >= 90:
            reliability_grade = "B"
        elif suite_pass_rate >= 80:
            reliability_grade = "C"
        elif suite_pass_rate >= 70:
            reliability_grade = "D"
        else:
            reliability_grade = "F"
            issues.append(f"Low test pass rate: {suite_pass_rate:.1f}%")
            recommendations.append("Fix failing tests to improve pipeline reliability")
        
        # Precision grading
        max_precision_error = max(precision_metrics) if precision_metrics else 0
        if max_precision_error < 1e-10:
            precision_grade = "A"
        elif max_precision_error < 1e-8:
            precision_grade = "B"
        elif max_precision_error < 1e-6:
            precision_grade = "C"
        elif max_precision_error < 1e-4:
            precision_grade = "D"
        else:
            precision_grade = "F"
            issues.append(f"High precision loss: {max_precision_error:.2e}")
            recommendations.append("Review decimal conversion logic for precision issues")
        
        # Overall status determination
        confidence_score = (overall_pass_rate + suite_pass_rate) / 2
        
        if confidence_score >= 95 and not issues:
            overall_status = "excellent"
        elif confidence_score >= 90 and len(issues) <= 1:
            overall_status = "good"
        elif confidence_score >= 75 and len(issues) <= 3:
            overall_status = "warning"
        else:
            overall_status = "critical"
        
        # Additional recommendations based on patterns
        failed_suites = [r.suite_name for r in self.test_results if not r.passed]
        if failed_suites:
            recommendations.append(f"Focus on fixing: {', '.join(failed_suites)}")
        
        slow_suites = [r.suite_name for r in self.test_results if r.execution_time_ms > 30000]
        if slow_suites:
            recommendations.append(f"Optimize performance of: {', '.join(slow_suites)}")
        
        return ValidationPipelineHealth(
            overall_status=overall_status,
            confidence_score=confidence_score,
            issues=issues,
            recommendations=recommendations,
            performance_grade=performance_grade,
            reliability_grade=reliability_grade,
            precision_grade=precision_grade
        )
    
    def generate_comprehensive_report(self) -> Dict[str, Any]:
        """Generate comprehensive test report"""
        health = self.assess_pipeline_health()
        
        # Calculate summary statistics
        total_tests = sum(r.total_tests for r in self.test_results)
        total_passed = sum(r.passed_tests for r in self.test_results)
        total_failed = sum(r.failed_tests for r in self.test_results)
        total_execution_time = sum(r.execution_time_ms for r in self.test_results)
        
        # Collect all metrics
        all_metrics = {}
        for result in self.test_results:
            for key, value in result.metrics.items():
                if key not in all_metrics:
                    all_metrics[key] = []
                all_metrics[key].append(value)
        
        # Performance aggregation
        performance_summary = {}
        if "throughput_msg_per_sec" in all_metrics:
            throughputs = all_metrics["throughput_msg_per_sec"]
            performance_summary = {
                "max_throughput": max(throughputs),
                "min_throughput": min(throughputs),
                "avg_throughput": statistics.mean(throughputs),
                "total_messages_tested": sum(all_metrics.get("total_tests", []))
            }
        
        return {
            "metadata": {
                "report_timestamp": self.report_timestamp.isoformat(),
                "report_generation_time_ms": (time.time() - self.start_time) * 1000,
                "test_environment": {
                    "python_version": subprocess.run(["python", "--version"], capture_output=True, text=True).stdout.strip(),
                    "platform": os.name,
                    "test_directory": os.getcwd()
                }
            },
            "summary": {
                "total_test_suites": len(self.test_results),
                "successful_suites": sum(1 for r in self.test_results if r.passed),
                "failed_suites": sum(1 for r in self.test_results if not r.passed),
                "total_individual_tests": total_tests,
                "total_passed_tests": total_passed,
                "total_failed_tests": total_failed,
                "overall_pass_rate": (total_passed / total_tests * 100) if total_tests > 0 else 0,
                "total_execution_time_ms": total_execution_time
            },
            "health_assessment": {
                "overall_status": health.overall_status,
                "confidence_score": health.confidence_score,
                "performance_grade": health.performance_grade,
                "reliability_grade": health.reliability_grade,
                "precision_grade": health.precision_grade,
                "issues_identified": health.issues,
                "recommendations": health.recommendations
            },
            "performance_summary": performance_summary,
            "test_suite_results": [
                {
                    "suite_name": r.suite_name,
                    "passed": r.passed,
                    "total_tests": r.total_tests,
                    "passed_tests": r.passed_tests,
                    "failed_tests": r.failed_tests,
                    "execution_time_ms": r.execution_time_ms,
                    "error_count": len(r.error_messages),
                    "key_metrics": r.metrics
                }
                for r in self.test_results
            ],
            "detailed_errors": [
                {
                    "suite": r.suite_name,
                    "errors": r.error_messages
                }
                for r in self.test_results if r.error_messages
            ]
        }
    
    def save_reports(self, report: Dict[str, Any]) -> None:
        """Save comprehensive reports in multiple formats"""
        timestamp = self.report_timestamp.strftime("%Y%m%d_%H%M%S")
        
        # Save JSON report
        json_path = f"/Users/daws/alphapulse/backend/tests/e2e/comprehensive_report_{timestamp}.json"
        with open(json_path, "w") as f:
            json.dump(report, f, indent=2, default=str)
        
        # Save human-readable summary
        summary_path = f"/Users/daws/alphapulse/backend/tests/e2e/test_summary_{timestamp}.md"
        with open(summary_path, "w") as f:
            self._write_markdown_summary(f, report)
        
        print(f"\n📄 Reports saved:")
        print(f"   JSON: {json_path}")
        print(f"   Summary: {summary_path}")
    
    def _write_markdown_summary(self, file, report: Dict[str, Any]) -> None:
        """Write human-readable markdown summary"""
        file.write("# Data Validation Pipeline Test Report\n\n")
        file.write(f"**Generated:** {report['metadata']['report_timestamp']}\n")
        file.write(f"**Execution Time:** {report['metadata']['report_generation_time_ms']:.0f}ms\n\n")
        
        # Summary
        summary = report["summary"]
        file.write("## Summary\n\n")
        file.write(f"- **Test Suites:** {summary['successful_suites']}/{summary['total_test_suites']} passed\n")
        file.write(f"- **Individual Tests:** {summary['total_passed_tests']}/{summary['total_individual_tests']} passed\n")
        file.write(f"- **Pass Rate:** {summary['overall_pass_rate']:.1f}%\n")
        file.write(f"- **Total Execution Time:** {summary['total_execution_time_ms']:.0f}ms\n\n")
        
        # Health Assessment
        health = report["health_assessment"]
        status_emoji = {"excellent": "🟢", "good": "🟡", "warning": "🟠", "critical": "🔴"}
        file.write("## Health Assessment\n\n")
        file.write(f"**Overall Status:** {status_emoji.get(health['overall_status'], '⚪')} {health['overall_status'].upper()}\n")
        file.write(f"**Confidence Score:** {health['confidence_score']:.1f}%\n\n")
        file.write(f"**Grades:**\n")
        file.write(f"- Performance: {health['performance_grade']}\n")
        file.write(f"- Reliability: {health['reliability_grade']}\n")
        file.write(f"- Precision: {health['precision_grade']}\n\n")
        
        if health["issues_identified"]:
            file.write("### Issues Identified\n\n")
            for issue in health["issues_identified"]:
                file.write(f"- ⚠️ {issue}\n")
            file.write("\n")
        
        if health["recommendations"]:
            file.write("### Recommendations\n\n")
            for rec in health["recommendations"]:
                file.write(f"- 💡 {rec}\n")
            file.write("\n")
        
        # Performance Summary
        if report["performance_summary"]:
            perf = report["performance_summary"]
            file.write("## Performance Summary\n\n")
            file.write(f"- **Max Throughput:** {perf.get('max_throughput', 0):,.0f} msg/sec\n")
            file.write(f"- **Avg Throughput:** {perf.get('avg_throughput', 0):,.0f} msg/sec\n")
            file.write(f"- **Total Messages Tested:** {perf.get('total_messages_tested', 0):,}\n\n")
        
        # Test Suite Details
        file.write("## Test Suite Results\n\n")
        for suite in report["test_suite_results"]:
            status = "✅ PASS" if suite["passed"] else "❌ FAIL"
            file.write(f"### {suite['suite_name']} {status}\n\n")
            file.write(f"- Tests: {suite['passed_tests']}/{suite['total_tests']} passed\n")
            file.write(f"- Execution Time: {suite['execution_time_ms']:.0f}ms\n")
            
            if suite["key_metrics"]:
                file.write("- Key Metrics:\n")
                for key, value in suite["key_metrics"].items():
                    if isinstance(value, float):
                        file.write(f"  - {key}: {value:,.2f}\n")
                    else:
                        file.write(f"  - {key}: {value}\n")
            file.write("\n")

def run_comprehensive_validation_report():
    """Run comprehensive validation pipeline testing and reporting"""
    print("🚀 Starting comprehensive validation pipeline assessment...")
    
    reporter = ComprehensiveTestReporter()
    
    # Run all test suites
    test_results = reporter.run_all_test_suites()
    
    # Generate comprehensive report
    report = reporter.generate_comprehensive_report()
    
    # Display results
    print("\n" + "=" * 80)
    print("COMPREHENSIVE VALIDATION PIPELINE REPORT")
    print("=" * 80)
    
    summary = report["summary"]
    health = report["health_assessment"]
    
    print(f"📊 Test Execution Summary:")
    print(f"   Test Suites: {summary['successful_suites']}/{summary['total_test_suites']} passed")
    print(f"   Individual Tests: {summary['total_passed_tests']}/{summary['total_individual_tests']} passed")
    print(f"   Overall Pass Rate: {summary['overall_pass_rate']:.1f}%")
    print(f"   Total Execution Time: {summary['total_execution_time_ms']/1000:.1f}s")
    
    status_emoji = {"excellent": "🟢", "good": "🟡", "warning": "🟠", "critical": "🔴"}
    print(f"\n🏥 Pipeline Health Assessment:")
    print(f"   Overall Status: {status_emoji.get(health['overall_status'], '⚪')} {health['overall_status'].upper()}")
    print(f"   Confidence Score: {health['confidence_score']:.1f}%")
    print(f"   Performance Grade: {health['performance_grade']}")
    print(f"   Reliability Grade: {health['reliability_grade']}")
    print(f"   Precision Grade: {health['precision_grade']}")
    
    if health["issues_identified"]:
        print(f"\n⚠️  Issues Identified:")
        for issue in health["issues_identified"]:
            print(f"      • {issue}")
    
    if health["recommendations"]:
        print(f"\n💡 Recommendations:")
        for rec in health["recommendations"]:
            print(f"      • {rec}")
    
    if report["performance_summary"]:
        perf = report["performance_summary"]
        print(f"\n⚡ Performance Summary:")
        print(f"   Max Throughput: {perf.get('max_throughput', 0):,.0f} msg/sec")
        print(f"   Avg Throughput: {perf.get('avg_throughput', 0):,.0f} msg/sec")
    
    # Save reports
    reporter.save_reports(report)
    
    # Determine overall success
    success = (
        health["overall_status"] in ["excellent", "good"] and
        health["confidence_score"] >= 85.0 and
        summary["overall_pass_rate"] >= 90.0
    )
    
    print(f"\n🎯 FINAL ASSESSMENT:")
    if success:
        print("   ✅ VALIDATION PIPELINE IS READY FOR PRODUCTION")
        print("   The data validation system meets all quality and performance requirements.")
    else:
        print("   ⚠️ VALIDATION PIPELINE NEEDS ATTENTION")
        print("   Address the identified issues before production deployment.")
    
    return success

if __name__ == "__main__":
    success = run_comprehensive_validation_report()
    exit(0 if success else 1)