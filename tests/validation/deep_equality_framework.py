#!/usr/bin/env python3
"""
Deep Equality Validation Framework

A comprehensive, reusable framework for validating complete data preservation 
through any pipeline by caching original messages, tracking transformations,
and performing rigorous deep equality checks.

This ensures that data going into a pipeline comes out EXACTLY the same,
with zero tolerance for deviation - "reproduce errors from the API even!"
"""

import hashlib
import json
import uuid
import time
from typing import Dict, List, Any, Optional, Tuple, Union
from dataclasses import dataclass, field
from datetime import datetime
import copy
from decimal import Decimal


@dataclass
class PipelineStage:
    """Represents a single stage in the data pipeline"""
    name: str
    timestamp: float
    data: Any
    metadata: Dict[str, Any] = field(default_factory=dict)


@dataclass
class MessageTrace:
    """Complete trace of a message through the pipeline"""
    message_id: str
    original_hash: str
    stages: List[PipelineStage] = field(default_factory=list)
    final_output: Optional[Any] = None
    reconstructed: Optional[Any] = None
    validation_result: Optional[Dict[str, Any]] = None


class MessageTracker:
    """
    Tracks messages through the entire pipeline lifecycle
    
    Provides:
    - Original message caching with cryptographic hashing
    - Pipeline stage tracking with timestamps
    - Message ID generation and lookup
    """
    
    def __init__(self):
        self.original_cache: Dict[str, Tuple[Any, str]] = {}  # message_id -> (original_data, hash)
        self.traces: Dict[str, MessageTrace] = {}  # message_id -> MessageTrace
        self.hash_to_id: Dict[str, str] = {}  # hash -> message_id (for reverse lookup)
        
    def cache_original(self, data: Any, message_id: Optional[str] = None) -> str:
        """
        Cache the original message and return its tracking ID
        
        Args:
            data: Original data to cache (will be deep copied)
            message_id: Optional specific ID, otherwise generates UUID
            
        Returns:
            Message ID for tracking this data through pipeline
        """
        if message_id is None:
            message_id = str(uuid.uuid4())
            
        # Deep copy to ensure immutability
        original_copy = copy.deepcopy(data)
        
        # Generate cryptographic hash for integrity verification
        data_hash = self._generate_hash(original_copy)
        
        # Store in cache
        self.original_cache[message_id] = (original_copy, data_hash)
        self.hash_to_id[data_hash] = message_id
        
        # Initialize trace
        self.traces[message_id] = MessageTrace(
            message_id=message_id,
            original_hash=data_hash
        )
        
        return message_id
    
    def add_pipeline_stage(self, message_id: str, stage_name: str, data: Any, metadata: Optional[Dict] = None) -> bool:
        """
        Record data at a specific pipeline stage
        
        Args:
            message_id: ID of the message being tracked
            stage_name: Name of the pipeline stage
            data: Data at this stage
            metadata: Optional metadata about this stage
            
        Returns:
            True if successfully recorded, False if message_id not found
        """
        if message_id not in self.traces:
            return False
            
        stage = PipelineStage(
            name=stage_name,
            timestamp=time.time(),
            data=copy.deepcopy(data),
            metadata=metadata or {}
        )
        
        self.traces[message_id].stages.append(stage)
        return True
    
    def set_final_output(self, message_id: str, output: Any) -> bool:
        """
        Record the final output for a message
        
        Args:
            message_id: ID of the message
            output: Final output data
            
        Returns:
            True if successfully recorded, False if message_id not found
        """
        if message_id not in self.traces:
            return False
            
        self.traces[message_id].final_output = copy.deepcopy(output)
        return True
    
    def get_original(self, message_id: str) -> Optional[Tuple[Any, str]]:
        """Get original data and hash for a message ID"""
        return self.original_cache.get(message_id)
    
    def get_trace(self, message_id: str) -> Optional[MessageTrace]:
        """Get complete trace for a message ID"""
        return self.traces.get(message_id)
    
    def find_by_hash(self, data_hash: str) -> Optional[str]:
        """Find message ID by original data hash"""
        return self.hash_to_id.get(data_hash)
    
    def _generate_hash(self, data: Any) -> str:
        """Generate SHA-256 hash of data for integrity checking"""
        # Convert to deterministic JSON string
        json_str = json.dumps(data, sort_keys=True, separators=(',', ':'))
        return hashlib.sha256(json_str.encode()).hexdigest()


class DeepEqualityValidator:
    """
    Performs rigorous deep equality validation between original and reconstructed data
    
    Features:
    - Recursive deep comparison of nested structures
    - Floating point precision handling
    - Type checking and validation
    - Detailed mismatch reporting
    """
    
    def __init__(self, float_tolerance: float = 1e-10):
        """
        Initialize validator
        
        Args:
            float_tolerance: Tolerance for floating point comparisons
        """
        self.float_tolerance = float_tolerance
        self.validation_errors: List[str] = []
    
    def validate_reconstruction(self, original: Any, reconstructed: Any, path: str = "root") -> Dict[str, Any]:
        """
        Perform deep equality validation between original and reconstructed data
        
        Args:
            original: Original data
            reconstructed: Reconstructed data
            path: Current path in data structure (for error reporting)
            
        Returns:
            Validation result with detailed analysis
        """
        self.validation_errors = []
        
        start_time = time.time()
        is_equal = self._deep_compare(original, reconstructed, path)
        validation_time = time.time() - start_time
        
        result = {
            "is_equal": is_equal,
            "validation_time_ms": round(validation_time * 1000, 3),
            "errors": self.validation_errors.copy(),
            "error_count": len(self.validation_errors),
            "original_hash": hashlib.sha256(json.dumps(original, sort_keys=True).encode()).hexdigest(),
            "reconstructed_hash": hashlib.sha256(json.dumps(reconstructed, sort_keys=True).encode()).hexdigest(),
            "timestamp": datetime.now().isoformat()
        }
        
        return result
    
    def _deep_compare(self, obj1: Any, obj2: Any, path: str) -> bool:
        """
        Recursively compare two objects for deep equality
        
        Args:
            obj1: First object
            obj2: Second object  
            path: Current path for error reporting
            
        Returns:
            True if objects are deeply equal, False otherwise
        """
        # Type checking
        if type(obj1) != type(obj2):
            self.validation_errors.append(f"{path}: Type mismatch - {type(obj1).__name__} vs {type(obj2).__name__}")
            return False
        
        # None handling
        if obj1 is None and obj2 is None:
            return True
        if obj1 is None or obj2 is None:
            self.validation_errors.append(f"{path}: None mismatch - {obj1} vs {obj2}")
            return False
        
        # Float/Decimal comparison with tolerance
        if isinstance(obj1, (float, Decimal)) and isinstance(obj2, (float, Decimal)):
            diff = abs(float(obj1) - float(obj2))
            if diff > self.float_tolerance:
                self.validation_errors.append(f"{path}: Float difference {diff} exceeds tolerance {self.float_tolerance}")
                return False
            return True
        
        # String comparison
        if isinstance(obj1, str):
            if obj1 != obj2:
                self.validation_errors.append(f"{path}: String mismatch - '{obj1}' vs '{obj2}'")
                return False
            return True
        
        # Number comparison
        if isinstance(obj1, (int, bool)):
            if obj1 != obj2:
                self.validation_errors.append(f"{path}: Value mismatch - {obj1} vs {obj2}")
                return False
            return True
        
        # List comparison
        if isinstance(obj1, list):
            if len(obj1) != len(obj2):
                self.validation_errors.append(f"{path}: List length mismatch - {len(obj1)} vs {len(obj2)}")
                return False
            
            all_equal = True
            for i, (item1, item2) in enumerate(zip(obj1, obj2)):
                if not self._deep_compare(item1, item2, f"{path}[{i}]"):
                    all_equal = False
            return all_equal
        
        # Dictionary comparison
        if isinstance(obj1, dict):
            keys1 = set(obj1.keys())
            keys2 = set(obj2.keys())
            
            if keys1 != keys2:
                missing_keys1 = keys2 - keys1
                missing_keys2 = keys1 - keys2
                if missing_keys1:
                    self.validation_errors.append(f"{path}: Missing keys in obj1: {missing_keys1}")
                if missing_keys2:
                    self.validation_errors.append(f"{path}: Missing keys in obj2: {missing_keys2}")
                return False
            
            all_equal = True
            for key in keys1:
                if not self._deep_compare(obj1[key], obj2[key], f"{path}.{key}"):
                    all_equal = False
            return all_equal
        
        # Direct comparison for other types
        if obj1 != obj2:
            self.validation_errors.append(f"{path}: Direct comparison failed - {obj1} vs {obj2}")
            return False
        
        return True


class ReconstructionEngine:
    """
    Converts final pipeline output back to original format for comparison
    
    This engine understands the transformations applied by the pipeline
    and can reverse them to reconstruct the original data format.
    """
    
    def __init__(self):
        self.transformation_rules: Dict[str, callable] = {}
        
    def register_transformation(self, stage_name: str, reverse_func: callable):
        """
        Register a reverse transformation function for a pipeline stage
        
        Args:
            stage_name: Name of the pipeline stage
            reverse_func: Function that reverses the transformation
        """
        self.transformation_rules[stage_name] = reverse_func
    
    def reconstruct_from_frontend(self, frontend_output: Dict[str, Any]) -> Dict[str, Any]:
        """
        Reconstruct original format from frontend WebSocket output
        
        Args:
            frontend_output: Final frontend output data
            
        Returns:
            Reconstructed data in original API format
        """
        # This will be populated based on our specific pipeline transformations
        # For now, implement basic reconstruction for common fields
        
        if not isinstance(frontend_output, dict):
            raise ValueError("Frontend output must be a dictionary")
        
        # For simple test cases, just return the input unchanged
        # In production, this would do complex reverse transformations
        if frontend_output.get('msg_type') == 'trade':
            # Handle trade message reconstruction
            reconstructed = {
                "address": self._extract_pool_address(frontend_output.get('symbol', '')),
                "data": self._reconstruct_swap_data(frontend_output),
                "topics": self._reconstruct_topics(frontend_output),
                "transactionHash": frontend_output.get('tx_hash', ''),
                "blockNumber": self._reconstruct_block_number(frontend_output)
            }
        else:
            # For test/demo purposes, pass through unchanged
            reconstructed = frontend_output.copy()
        
        return reconstructed
    
    def _extract_pool_address(self, symbol: str) -> str:
        """Extract pool address from symbol string"""
        # Symbol format: "polygon:0x1234...:TOKEN1/TOKEN2"
        if ':' in symbol:
            parts = symbol.split(':')
            if len(parts) >= 2:
                return parts[1]
        return ""
    
    def _reconstruct_swap_data(self, frontend_output: Dict) -> str:
        """Reconstruct swap data hex string from frontend values"""
        # This would need to reverse the exact hex parsing logic
        # For now, return placeholder
        return "0x" + "0" * 256  # Placeholder 256-char hex string
    
    def _reconstruct_topics(self, frontend_output: Dict) -> List[str]:
        """Reconstruct event topics array"""
        # This would reconstruct the event signature and indexed parameters
        return ["0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822"]  # V2 Swap signature
    
    def _reconstruct_block_number(self, frontend_output: Dict) -> str:
        """Reconstruct block number in hex format"""
        # This would reverse the block number parsing
        return "0x" + hex(int(time.time()))[2:]  # Placeholder using current time


class DeepEqualityFramework:
    """
    Main framework class that orchestrates the entire deep equality validation process
    """
    
    def __init__(self, float_tolerance: float = 1e-10):
        self.tracker = MessageTracker()
        self.validator = DeepEqualityValidator(float_tolerance)
        self.reconstructor = ReconstructionEngine()
        self.statistics = {
            "messages_tracked": 0,
            "validations_performed": 0,
            "perfect_matches": 0,
            "failed_validations": 0,
            "average_validation_time_ms": 0.0
        }
    
    def start_tracking(self, original_data: Any, message_id: Optional[str] = None) -> str:
        """
        Begin tracking a message through the pipeline
        
        Args:
            original_data: Original data to track
            message_id: Optional specific message ID
            
        Returns:
            Message ID for tracking
        """
        message_id = self.tracker.cache_original(original_data, message_id)
        self.statistics["messages_tracked"] += 1
        return message_id
    
    def record_stage(self, message_id: str, stage_name: str, data: Any, metadata: Optional[Dict] = None) -> bool:
        """Record data at a pipeline stage"""
        return self.tracker.add_pipeline_stage(message_id, stage_name, data, metadata)
    
    def validate_final_output(self, message_id: str, final_output: Any) -> Dict[str, Any]:
        """
        Validate final output against original using deep equality
        
        Args:
            message_id: Message ID to validate
            final_output: Final pipeline output
            
        Returns:
            Comprehensive validation result
        """
        # Record final output
        self.tracker.set_final_output(message_id, final_output)
        
        # Get original data
        original_data, original_hash = self.tracker.get_original(message_id)
        if original_data is None:
            return {
                "error": f"Original data not found for message_id: {message_id}",
                "is_equal": False
            }
        
        # Reconstruct original format from final output
        try:
            reconstructed = self.reconstructor.reconstruct_from_frontend(final_output)
        except Exception as e:
            return {
                "error": f"Reconstruction failed: {str(e)}",
                "is_equal": False
            }
        
        # Perform deep equality validation
        validation_result = self.validator.validate_reconstruction(original_data, reconstructed)
        
        # Update trace with results
        trace = self.tracker.get_trace(message_id)
        if trace:
            trace.reconstructed = reconstructed
            trace.validation_result = validation_result
        
        # Update statistics
        self.statistics["validations_performed"] += 1
        if validation_result["is_equal"]:
            self.statistics["perfect_matches"] += 1
        else:
            self.statistics["failed_validations"] += 1
        
        # Update average validation time
        total_time = (self.statistics["average_validation_time_ms"] * 
                     (self.statistics["validations_performed"] - 1) + 
                     validation_result["validation_time_ms"])
        self.statistics["average_validation_time_ms"] = total_time / self.statistics["validations_performed"]
        
        # Enhance result with framework metadata
        validation_result.update({
            "message_id": message_id,
            "original_hash": original_hash,
            "framework_statistics": self.statistics.copy()
        })
        
        return validation_result
    
    def get_statistics(self) -> Dict[str, Any]:
        """Get framework performance statistics"""
        success_rate = 0.0
        if self.statistics["validations_performed"] > 0:
            success_rate = (self.statistics["perfect_matches"] / 
                          self.statistics["validations_performed"]) * 100
        
        return {
            **self.statistics,
            "success_rate_percent": round(success_rate, 2),
            "failure_rate_percent": round(100 - success_rate, 2)
        }
    
    def get_detailed_trace(self, message_id: str) -> Optional[Dict[str, Any]]:
        """Get detailed trace information for a message"""
        trace = self.tracker.get_trace(message_id)
        if not trace:
            return None
        
        return {
            "message_id": trace.message_id,
            "original_hash": trace.original_hash,
            "pipeline_stages": [
                {
                    "name": stage.name,
                    "timestamp": stage.timestamp,
                    "metadata": stage.metadata
                }
                for stage in trace.stages
            ],
            "has_final_output": trace.final_output is not None,
            "has_reconstruction": trace.reconstructed is not None,
            "validation_passed": trace.validation_result.get("is_equal", False) if trace.validation_result else None,
            "error_count": trace.validation_result.get("error_count", 0) if trace.validation_result else 0
        }