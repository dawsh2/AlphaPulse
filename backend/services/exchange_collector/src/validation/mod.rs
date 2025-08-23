/// PHASE 3: Deep equality validation system
/// 
/// This module provides comprehensive validation capabilities to ensure
/// data integrity through the entire pipeline from Polygon API to frontend.

pub mod reverse_transform;
pub mod continuous_monitor;

pub use reverse_transform::*;
pub use continuous_monitor::*;

/// Re-export validation types for easy access
pub use reverse_transform::{
    ReverseTransformEngine,
    FrontendTradeMessage,
    PolygonSwapEvent,
    ValidationResult,
    TransformationStep,
};