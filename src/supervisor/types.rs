//! Supervisor Communication Types
//! 
//! Shared types for communicating with the MagicTunnel supervisor process.

use serde::{Deserialize, Serialize};

/// Commands that can be sent to the supervisor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SupervisorCommand {
    /// Restart MagicTunnel with optional new arguments
    Restart { args: Option<Vec<String>> },
    /// Stop MagicTunnel gracefully
    Stop,
    /// Get current process status
    Status,
    /// Reload configuration
    ReloadConfig { config_path: Option<String> },
    /// Shutdown supervisor
    Shutdown,
    /// Health check
    HealthCheck,
    /// Execute custom restart sequence with pre/post commands
    CustomRestart { 
        pre_commands: Option<Vec<CustomCommand>>,
        start_args: Option<Vec<String>>,
        post_commands: Option<Vec<CustomCommand>>,
    },
    /// Execute arbitrary command (restricted for security)
    ExecuteCommand { 
        command: CustomCommand,
        timeout_seconds: Option<u64>,
    },
}

/// Custom command definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomCommand {
    /// Command type (make, shell, binary)
    pub command_type: CommandType,
    /// The actual command to execute
    pub command: String,
    /// Optional arguments
    pub args: Option<Vec<String>>,
    /// Working directory (optional)
    pub working_dir: Option<String>,
    /// Environment variables (optional)
    pub env: Option<std::collections::HashMap<String, String>>,
    /// Human-readable description
    pub description: Option<String>,
    /// Whether this command is considered safe for execution
    pub is_safe: bool,
}

/// Type of command to execute
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommandType {
    /// Makefile command (make <target>)
    Make,
    /// Shell command
    Shell,
    /// Direct binary execution
    Binary,
    /// Cargo command
    Cargo,
}

/// Response from supervisor commands
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisorResponse {
    pub success: bool,
    pub message: String,
    pub data: Option<serde_json::Value>,
    pub timestamp: String,
}

/// Command execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandExecutionResult {
    pub command: CustomCommand,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub execution_time_ms: u64,
    pub error_message: Option<String>,
}

/// Custom restart sequence result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomRestartResult {
    pub pre_command_results: Vec<CommandExecutionResult>,
    pub restart_successful: bool,
    pub post_command_results: Vec<CommandExecutionResult>,
    pub total_execution_time_ms: u64,
    pub overall_success: bool,
}

/// Process status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessStatus {
    pub pid: Option<u32>,
    pub status: String,
    pub uptime_seconds: Option<u64>,
    pub restart_count: u32,
    pub last_restart: Option<String>,
    pub health_status: String,
    pub args: Vec<String>,
}