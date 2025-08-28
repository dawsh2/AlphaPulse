//! Real-time Progress Tracking for Torq Development Workflow
//!
//! Provides comprehensive progress tracking with quality gates, task dependencies,
//! completion validation, and real-time status updates for the enhanced Torq
//! development workflow.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum ProgressError {
    #[error("Task not found: {id}")]
    TaskNotFound { id: String },
    
    #[error("Invalid transition from {from} to {to}")]
    InvalidTransition { from: String, to: String },
    
    #[error("Quality gate not passed: {gate}")]
    QualityGateNotPassed { gate: String },
    
    #[error("Dependency not satisfied: {dependency}")]
    DependencyNotSatisfied { dependency: String },
    
    #[error("Serialization error: {error}")]
    SerializationError { error: String },
}

/// Task status in the workflow
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TaskStatus {
    /// Task created but not started
    Pending,
    /// Task is actively being worked on
    InProgress,
    /// Task completed and all quality gates passed
    Completed,
    /// Task blocked by dependencies or issues
    Blocked,
    /// Task cancelled or discarded
    Cancelled,
}

impl fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaskStatus::Pending => write!(f, "pending"),
            TaskStatus::InProgress => write!(f, "in_progress"),
            TaskStatus::Completed => write!(f, "completed"),
            TaskStatus::Blocked => write!(f, "blocked"),
            TaskStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Quality gate status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum QualityGateStatus {
    /// Gate not yet executed
    Pending,
    /// Gate currently running
    InProgress,
    /// Gate passed successfully
    Passed,
    /// Gate failed with issues
    Failed,
    /// Gate failed but user overridden
    Overridden,
}

/// Individual quality gate for task validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityGate {
    /// Gate identifier
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Current status
    pub status: QualityGateStatus,
    /// When gate execution started
    pub started_at: Option<SystemTime>,
    /// When gate execution completed
    pub completed_at: Option<SystemTime>,
    /// Findings from gate execution
    pub findings: Vec<QualityFinding>,
    /// Summary of gate results
    pub summary: Option<QualitySummary>,
    /// Override reason if overridden
    pub override_reason: Option<String>,
}

/// Finding from quality gate execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityFinding {
    /// Unique finding ID
    pub id: String,
    /// Severity level
    pub severity: FindingSeverity,
    /// Category of finding
    pub category: String,
    /// Description of the issue
    pub description: String,
    /// Location where found
    pub location: String,
    /// Suggestion for fixing
    pub suggestion: String,
    /// Whether finding was overridden
    pub overridden: bool,
    /// Override reason if applicable
    pub override_reason: Option<String>,
}

/// Severity of quality findings
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FindingSeverity {
    Critical,
    Warning,
    Info,
}

/// Summary of quality gate execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualitySummary {
    /// Total number of findings
    pub total_findings: usize,
    /// Number of critical findings
    pub critical_count: usize,
    /// Number of warning findings
    pub warning_count: usize,
    /// Number of info findings
    pub info_count: usize,
    /// Files analyzed
    pub files_analyzed: usize,
    /// Lines of code analyzed
    pub lines_analyzed: usize,
}

/// Progress checkpoint for task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressCheckpoint {
    /// Unique checkpoint ID
    pub id: String,
    /// Checkpoint name/description
    pub name: String,
    /// When checkpoint was reached
    pub timestamp: SystemTime,
    /// Additional context/data
    pub context: HashMap<String, String>,
}

/// A task in the development workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique task identifier
    pub id: String,
    /// Task description/content
    pub content: String,
    /// Current status
    pub status: TaskStatus,
    /// When task was created
    pub created_at: SystemTime,
    /// When task was last updated
    pub updated_at: SystemTime,
    /// When task was completed (if completed)
    pub completed_at: Option<SystemTime>,
    
    /// Quality gates for this task
    pub quality_gates: HashMap<String, QualityGate>,
    
    /// Progress tracking
    pub progress: TaskProgress,
    
    /// Task dependencies
    pub dependencies: Vec<String>,
    /// Tasks that depend on this one
    pub dependents: Vec<String>,
    
    /// Override tracking
    pub overrides: Vec<Override>,
    
    /// Progress checkpoints
    pub checkpoints: Vec<ProgressCheckpoint>,
    
    /// Estimated completion percentage (0-100)
    pub completion_percentage: u8,
}

/// Progress state for a task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskProgress {
    /// Whether code implementation is complete
    pub code_complete: bool,
    /// Whether code review is complete
    pub review_complete: bool,
    /// Whether compilation check is complete
    pub compilation_complete: bool,
    /// Whether all quality gates have passed
    pub all_gates_passed: bool,
}

/// Override record for quality gate findings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Override {
    /// Finding ID that was overridden
    pub finding_id: String,
    /// Reason for override
    pub reason: String,
    /// Who approved the override
    pub approved_by: String,
    /// When override was applied
    pub timestamp: SystemTime,
}

/// Real-time progress tracker for Torq workflow
pub struct ProgressTracker {
    /// All tasks indexed by ID
    tasks: HashMap<String, Task>,
    /// Task dependencies graph
    dependency_graph: HashMap<String, HashSet<String>>,
    /// Execution history for auditing
    history: VecDeque<ProgressEvent>,
    /// Maximum history entries to keep
    max_history: usize,
}

/// Progress event for history tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressEvent {
    /// Event ID
    pub id: String,
    /// When event occurred
    pub timestamp: SystemTime,
    /// Task ID involved
    pub task_id: String,
    /// Type of event
    pub event_type: ProgressEventType,
    /// Event description
    pub description: String,
    /// Additional context
    pub context: HashMap<String, String>,
}

/// Types of progress events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProgressEventType {
    TaskCreated,
    TaskStarted,
    TaskCompleted,
    TaskBlocked,
    StatusChanged,
    QualityGateStarted,
    QualityGateCompleted,
    QualityGateFailed,
    QualityGateOverridden,
    CheckpointReached,
    DependencySatisfied,
    Override,
}

impl ProgressTracker {
    /// Create a new progress tracker
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
            dependency_graph: HashMap::new(),
            history: VecDeque::new(),
            max_history: 1000,
        }
    }
    
    /// Create a new task with quality gates
    pub fn create_task(
        &mut self,
        content: String,
        quality_gates: Vec<String>,
        dependencies: Vec<String>,
    ) -> Result<String, ProgressError> {
        let task_id = Uuid::new_v4().to_string();
        let now = SystemTime::now();
        
        // Create quality gates
        let mut gates = HashMap::new();
        for gate_name in &quality_gates {
            gates.insert(gate_name.clone(), QualityGate {
                id: Uuid::new_v4().to_string(),
                name: gate_name.clone(),
                status: QualityGateStatus::Pending,
                started_at: None,
                completed_at: None,
                findings: Vec::new(),
                summary: None,
                override_reason: None,
            });
        }
        
        let task = Task {
            id: task_id.clone(),
            content,
            status: TaskStatus::Pending,
            created_at: now,
            updated_at: now,
            completed_at: None,
            quality_gates: gates,
            progress: TaskProgress {
                code_complete: false,
                review_complete: false,
                compilation_complete: false,
                all_gates_passed: false,
            },
            dependencies: dependencies.clone(),
            dependents: Vec::new(),
            overrides: Vec::new(),
            checkpoints: Vec::new(),
            completion_percentage: 0,
        };
        
        // Update dependency graph
        if !dependencies.is_empty() {
            self.dependency_graph.insert(task_id.clone(), dependencies.into_iter().collect());
        }
        
        // Add to dependents lists
        for dep_id in &task.dependencies {
            if let Some(dep_task) = self.tasks.get_mut(dep_id) {
                dep_task.dependents.push(task_id.clone());
            }
        }
        
        // Record event
        self.add_event(task_id.clone(), ProgressEventType::TaskCreated, 
                      "Task created".to_string(), HashMap::new());
        
        self.tasks.insert(task_id.clone(), task);
        Ok(task_id)
    }
    
    /// Start working on a task
    pub fn start_task(&mut self, task_id: &str) -> Result<(), ProgressError> {
        let task = self.tasks.get_mut(task_id).ok_or(ProgressError::TaskNotFound {
            id: task_id.to_string(),
        })?;
        
        // Check dependencies
        for dep_id in &task.dependencies {
            if let Some(dep_task) = self.tasks.get(dep_id) {
                if dep_task.status != TaskStatus::Completed {
                    return Err(ProgressError::DependencyNotSatisfied {
                        dependency: dep_id.clone(),
                    });
                }
            }
        }
        
        // Validate transition
        if task.status != TaskStatus::Pending && task.status != TaskStatus::Blocked {
            return Err(ProgressError::InvalidTransition {
                from: task.status.to_string(),
                to: TaskStatus::InProgress.to_string(),
            });
        }
        
        task.status = TaskStatus::InProgress;
        task.updated_at = SystemTime::now();
        
        self.add_event(task_id.to_string(), ProgressEventType::TaskStarted,
                      "Task started".to_string(), HashMap::new());
        
        Ok(())
    }
    
    /// Update task progress
    pub fn update_progress(
        &mut self,
        task_id: &str,
        code_complete: Option<bool>,
        completion_percentage: Option<u8>,
    ) -> Result<(), ProgressError> {
        let task = self.tasks.get_mut(task_id).ok_or(ProgressError::TaskNotFound {
            id: task_id.to_string(),
        })?;
        
        if let Some(code_done) = code_complete {
            task.progress.code_complete = code_done;
        }
        
        if let Some(percentage) = completion_percentage {
            task.completion_percentage = percentage.min(100);
        }
        
        task.updated_at = SystemTime::now();
        
        // Add checkpoint if code is complete
        if task.progress.code_complete {
            let checkpoint = ProgressCheckpoint {
                id: Uuid::new_v4().to_string(),
                name: "Code Implementation Complete".to_string(),
                timestamp: SystemTime::now(),
                context: HashMap::new(),
            };
            task.checkpoints.push(checkpoint);
            
            self.add_event(task_id.to_string(), ProgressEventType::CheckpointReached,
                          "Code implementation complete".to_string(), HashMap::new());
        }
        
        Ok(())
    }
    
    /// Start a quality gate
    pub fn start_quality_gate(&mut self, task_id: &str, gate_name: &str) -> Result<(), ProgressError> {
        let task = self.tasks.get_mut(task_id).ok_or(ProgressError::TaskNotFound {
            id: task_id.to_string(),
        })?;
        
        let gate = task.quality_gates.get_mut(gate_name).ok_or(ProgressError::QualityGateNotPassed {
            gate: gate_name.to_string(),
        })?;
        
        gate.status = QualityGateStatus::InProgress;
        gate.started_at = Some(SystemTime::now());
        task.updated_at = SystemTime::now();
        
        self.add_event(task_id.to_string(), ProgressEventType::QualityGateStarted,
                      format!("Quality gate '{}' started", gate_name), HashMap::new());
        
        Ok(())
    }
    
    /// Complete a quality gate with results
    pub fn complete_quality_gate(
        &mut self,
        task_id: &str,
        gate_name: &str,
        passed: bool,
        findings: Vec<QualityFinding>,
        summary: QualitySummary,
    ) -> Result<(), ProgressError> {
        let task = self.tasks.get_mut(task_id).ok_or(ProgressError::TaskNotFound {
            id: task_id.to_string(),
        })?;
        
        let gate = task.quality_gates.get_mut(gate_name).ok_or(ProgressError::QualityGateNotPassed {
            gate: gate_name.to_string(),
        })?;
        
        gate.status = if passed { QualityGateStatus::Passed } else { QualityGateStatus::Failed };
        gate.completed_at = Some(SystemTime::now());
        gate.findings = findings;
        gate.summary = Some(summary);
        task.updated_at = SystemTime::now();
        
        // Update progress based on gate type
        match gate_name {
            "code_review" => task.progress.review_complete = passed,
            "compilation" => task.progress.compilation_complete = passed,
            _ => {}
        }
        
        let event_type = if passed { 
            ProgressEventType::QualityGateCompleted 
        } else { 
            ProgressEventType::QualityGateFailed 
        };
        
        self.add_event(task_id.to_string(), event_type,
                      format!("Quality gate '{}' {}", gate_name, 
                             if passed { "passed" } else { "failed" }),
                      HashMap::new());
        
        // Check if all gates passed
        self.check_all_gates_passed(task_id)?;
        
        Ok(())
    }
    
    /// Override a quality gate with reason
    pub fn override_quality_gate(
        &mut self,
        task_id: &str,
        gate_name: &str,
        reason: String,
        approved_by: String,
    ) -> Result<(), ProgressError> {
        let task = self.tasks.get_mut(task_id).ok_or(ProgressError::TaskNotFound {
            id: task_id.to_string(),
        })?;
        
        let gate = task.quality_gates.get_mut(gate_name).ok_or(ProgressError::QualityGateNotPassed {
            gate: gate_name.to_string(),
        })?;
        
        gate.status = QualityGateStatus::Overridden;
        gate.override_reason = Some(reason.clone());
        task.updated_at = SystemTime::now();
        
        // Add override record
        let override_record = Override {
            finding_id: gate.id.clone(),
            reason: reason.clone(),
            approved_by: approved_by.clone(),
            timestamp: SystemTime::now(),
        };
        task.overrides.push(override_record);
        
        self.add_event(task_id.to_string(), ProgressEventType::QualityGateOverridden,
                      format!("Quality gate '{}' overridden: {}", gate_name, reason),
                      [("approved_by".to_string(), approved_by)].into_iter().collect());
        
        // Check if all gates passed
        self.check_all_gates_passed(task_id)?;
        
        Ok(())
    }
    
    /// Complete a task (only if all quality gates pass)
    pub fn complete_task(&mut self, task_id: &str) -> Result<(), ProgressError> {
        let task = self.tasks.get_mut(task_id).ok_or(ProgressError::TaskNotFound {
            id: task_id.to_string(),
        })?;
        
        // Verify all quality gates are passed or overridden
        if !task.progress.all_gates_passed {
            return Err(ProgressError::QualityGateNotPassed {
                gate: "not all gates passed".to_string(),
            });
        }
        
        task.status = TaskStatus::Completed;
        task.completed_at = Some(SystemTime::now());
        task.updated_at = SystemTime::now();
        task.completion_percentage = 100;
        
        // Add completion checkpoint
        let checkpoint = ProgressCheckpoint {
            id: Uuid::new_v4().to_string(),
            name: "Task Completed".to_string(),
            timestamp: SystemTime::now(),
            context: HashMap::new(),
        };
        task.checkpoints.push(checkpoint);
        
        self.add_event(task_id.to_string(), ProgressEventType::TaskCompleted,
                      "Task completed".to_string(), HashMap::new());
        
        // Check if any dependent tasks can be unblocked
        let dependents = task.dependents.clone();
        for dependent_id in dependents {
            self.check_dependencies_satisfied(&dependent_id)?;
        }
        
        Ok(())
    }
    
    /// Get task status
    pub fn get_task(&self, task_id: &str) -> Option<&Task> {
        self.tasks.get(task_id)
    }
    
    /// Get all tasks
    pub fn get_all_tasks(&self) -> Vec<&Task> {
        self.tasks.values().collect()
    }
    
    /// Get tasks with specific status
    pub fn get_tasks_by_status(&self, status: TaskStatus) -> Vec<&Task> {
        self.tasks.values().filter(|task| task.status == status).collect()
    }
    
    /// Generate progress report
    pub fn generate_progress_report(&self) -> ProgressReport {
        let mut report = ProgressReport {
            total_tasks: self.tasks.len(),
            pending_tasks: 0,
            in_progress_tasks: 0,
            completed_tasks: 0,
            blocked_tasks: 0,
            cancelled_tasks: 0,
            total_quality_gates: 0,
            passed_quality_gates: 0,
            failed_quality_gates: 0,
            overridden_quality_gates: 0,
            recent_events: self.history.iter().rev().take(10).cloned().collect(),
        };
        
        for task in self.tasks.values() {
            match task.status {
                TaskStatus::Pending => report.pending_tasks += 1,
                TaskStatus::InProgress => report.in_progress_tasks += 1,
                TaskStatus::Completed => report.completed_tasks += 1,
                TaskStatus::Blocked => report.blocked_tasks += 1,
                TaskStatus::Cancelled => report.cancelled_tasks += 1,
            }
            
            for gate in task.quality_gates.values() {
                report.total_quality_gates += 1;
                match gate.status {
                    QualityGateStatus::Passed => report.passed_quality_gates += 1,
                    QualityGateStatus::Failed => report.failed_quality_gates += 1,
                    QualityGateStatus::Overridden => report.overridden_quality_gates += 1,
                    _ => {}
                }
            }
        }
        
        report
    }
    
    /// Check if all quality gates for a task are passed
    fn check_all_gates_passed(&mut self, task_id: &str) -> Result<(), ProgressError> {
        let task = self.tasks.get_mut(task_id).ok_or(ProgressError::TaskNotFound {
            id: task_id.to_string(),
        })?;
        
        let all_passed = task.quality_gates.values().all(|gate| {
            matches!(gate.status, QualityGateStatus::Passed | QualityGateStatus::Overridden)
        });
        
        if all_passed {
            task.progress.all_gates_passed = true;
        }
        
        Ok(())
    }
    
    /// Check if dependencies are satisfied for a task
    fn check_dependencies_satisfied(&mut self, task_id: &str) -> Result<(), ProgressError> {
        let dependencies = if let Some(deps) = self.dependency_graph.get(task_id) {
            deps.clone()
        } else {
            return Ok(()); // No dependencies
        };
        
        let all_satisfied = dependencies.iter().all(|dep_id| {
            self.tasks.get(dep_id)
                .map(|task| task.status == TaskStatus::Completed)
                .unwrap_or(false)
        });
        
        if all_satisfied {
            if let Some(task) = self.tasks.get_mut(task_id) {
                if task.status == TaskStatus::Blocked {
                    task.status = TaskStatus::Pending;
                    task.updated_at = SystemTime::now();
                    
                    self.add_event(task_id.to_string(), ProgressEventType::DependencySatisfied,
                                  "Dependencies satisfied, task unblocked".to_string(), HashMap::new());
                }
            }
        }
        
        Ok(())
    }
    
    /// Add event to history
    fn add_event(
        &mut self,
        task_id: String,
        event_type: ProgressEventType,
        description: String,
        context: HashMap<String, String>,
    ) {
        let event = ProgressEvent {
            id: Uuid::new_v4().to_string(),
            timestamp: SystemTime::now(),
            task_id,
            event_type,
            description,
            context,
        };
        
        self.history.push_back(event);
        
        // Keep history within limits
        while self.history.len() > self.max_history {
            self.history.pop_front();
        }
    }
}

/// Progress report summary
#[derive(Debug, Serialize, Deserialize)]
pub struct ProgressReport {
    pub total_tasks: usize,
    pub pending_tasks: usize,
    pub in_progress_tasks: usize,
    pub completed_tasks: usize,
    pub blocked_tasks: usize,
    pub cancelled_tasks: usize,
    pub total_quality_gates: usize,
    pub passed_quality_gates: usize,
    pub failed_quality_gates: usize,
    pub overridden_quality_gates: usize,
    pub recent_events: Vec<ProgressEvent>,
}

impl ProgressReport {
    /// Format report for display
    pub fn format_summary(&self) -> String {
        let completion_rate = if self.total_tasks > 0 {
            (self.completed_tasks as f64 / self.total_tasks as f64) * 100.0
        } else {
            0.0
        };
        
        format!(
            "📊 **Torq Development Progress**\n\
             🎯 **Tasks**: {} total ({:.1}% complete)\n\
             📋 Pending: {} | 🔄 In Progress: {} | ✅ Completed: {} | 🚫 Blocked: {}\n\n\
             🔍 **Quality Gates**: {} total\n\
             ✅ Passed: {} | ❌ Failed: {} | 🔄 Overridden: {}\n\n\
             📈 **Success Rate**: {:.1}%",
            self.total_tasks,
            completion_rate,
            self.pending_tasks,
            self.in_progress_tasks,
            self.completed_tasks,
            self.blocked_tasks,
            self.total_quality_gates,
            self.passed_quality_gates,
            self.failed_quality_gates,
            self.overridden_quality_gates,
            if self.total_quality_gates > 0 {
                (self.passed_quality_gates as f64 / self.total_quality_gates as f64) * 100.0
            } else {
                0.0
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_task_creation() {
        let mut tracker = ProgressTracker::new();
        
        let task_id = tracker.create_task(
            "Test task".to_string(),
            vec!["code_review".to_string(), "compilation".to_string()],
            vec![],
        ).unwrap();
        
        let task = tracker.get_task(&task_id).unwrap();
        assert_eq!(task.status, TaskStatus::Pending);
        assert_eq!(task.quality_gates.len(), 2);
    }
    
    #[test]
    fn test_task_workflow() {
        let mut tracker = ProgressTracker::new();
        
        let task_id = tracker.create_task(
            "Test task".to_string(),
            vec!["code_review".to_string()],
            vec![],
        ).unwrap();
        
        // Start task
        tracker.start_task(&task_id).unwrap();
        assert_eq!(tracker.get_task(&task_id).unwrap().status, TaskStatus::InProgress);
        
        // Update progress
        tracker.update_progress(&task_id, Some(true), Some(50)).unwrap();
        let task = tracker.get_task(&task_id).unwrap();
        assert!(task.progress.code_complete);
        assert_eq!(task.completion_percentage, 50);
        
        // Complete quality gate
        tracker.complete_quality_gate(
            &task_id,
            "code_review",
            true,
            vec![],
            QualitySummary {
                total_findings: 0,
                critical_count: 0,
                warning_count: 0,
                info_count: 0,
                files_analyzed: 1,
                lines_analyzed: 100,
            },
        ).unwrap();
        
        // Complete task
        tracker.complete_task(&task_id).unwrap();
        assert_eq!(tracker.get_task(&task_id).unwrap().status, TaskStatus::Completed);
    }
    
    #[test]
    fn test_dependencies() {
        let mut tracker = ProgressTracker::new();
        
        let task1_id = tracker.create_task(
            "Task 1".to_string(),
            vec![],
            vec![],
        ).unwrap();
        
        let task2_id = tracker.create_task(
            "Task 2".to_string(),
            vec![],
            vec![task1_id.clone()],
        ).unwrap();
        
        // Task 2 should not start while Task 1 is pending
        assert!(tracker.start_task(&task2_id).is_err());
        
        // Complete Task 1
        tracker.start_task(&task1_id).unwrap();
        tracker.complete_task(&task1_id).unwrap();
        
        // Now Task 2 should start
        assert!(tracker.start_task(&task2_id).is_ok());
    }
    
    #[test]
    fn test_progress_report() {
        let mut tracker = ProgressTracker::new();
        
        // Create some tasks
        let task1 = tracker.create_task("Task 1".to_string(), vec![], vec![]).unwrap();
        let task2 = tracker.create_task("Task 2".to_string(), vec![], vec![]).unwrap();
        
        tracker.start_task(&task1).unwrap();
        tracker.complete_task(&task1).unwrap();
        
        let report = tracker.generate_progress_report();
        
        assert_eq!(report.total_tasks, 2);
        assert_eq!(report.completed_tasks, 1);
        assert_eq!(report.pending_tasks, 1);
    }
}