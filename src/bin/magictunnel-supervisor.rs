//! MagicTunnel Process Supervisor
//! 
//! A lightweight supervisor process that manages MagicTunnel lifecycle operations
//! including restart, stop, and configuration reload without UI interruption.

use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tokio::time::{interval, timeout};
use tracing::{debug, error, info, warn};
use std::collections::HashMap;

/// Supervisor configuration
#[derive(Debug, Clone, Deserialize)]
pub struct SupervisorConfig {
    /// Port for supervisor control interface
    pub control_port: u16,
    /// Path to magictunnel binary
    pub magictunnel_binary: PathBuf,
    /// Default arguments for magictunnel
    pub default_args: Vec<String>,
    /// Health check interval in seconds
    pub health_check_interval: u64,
    /// Maximum restart attempts before giving up
    pub max_restart_attempts: u32,
    /// Restart cooldown period in seconds
    pub restart_cooldown: u64,
}

impl Default for SupervisorConfig {
    fn default() -> Self {
        Self {
            control_port: 8081,
            magictunnel_binary: PathBuf::from("./magictunnel"),
            default_args: vec![
                "--config".to_string(),
                "magictunnel-config.yaml".to_string(),
                "--log-level".to_string(),
                "info".to_string(),
            ],
            health_check_interval: 30,
            max_restart_attempts: 5,
            restart_cooldown: 10,
        }
    }
}

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
    pub env: Option<HashMap<String, String>>,
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

/// Response from supervisor commands
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisorResponse {
    pub success: bool,
    pub message: String,
    pub data: Option<serde_json::Value>,
    pub timestamp: String,
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

/// MagicTunnel process manager
pub struct MagicTunnelProcess {
    config: SupervisorConfig,
    process: Option<Child>,
    start_time: Option<Instant>,
    restart_count: u32,
    last_restart: Option<Instant>,
    current_args: Vec<String>,
}

impl MagicTunnelProcess {
    pub fn new(config: SupervisorConfig) -> Self {
        let current_args = config.default_args.clone();
        Self {
            config,
            process: None,
            start_time: None,
            restart_count: 0,
            last_restart: None,
            current_args,
        }
    }

    /// Start MagicTunnel process
    pub async fn start(&mut self, args: Option<Vec<String>>) -> Result<(), Box<dyn std::error::Error>> {
        if self.is_running() {
            info!("MagicTunnel is already running, stopping first");
            self.stop().await?;
        }

        // Use provided args or current args
        if let Some(new_args) = args {
            self.current_args = new_args;
        }

        info!("Starting MagicTunnel with args: {:?}", self.current_args);

        let mut cmd = Command::new(&self.config.magictunnel_binary);
        cmd.args(&self.current_args)
            .env("MAGICTUNNEL_ENV", "development")
            .env("OLLAMA_BASE_URL", "http://localhost:11434")
            .env("MAGICTUNNEL_SEMANTIC_MODEL", "ollama:nomic-embed-text")
            .env("MAGICTUNNEL_DISABLE_SEMANTIC", "false")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        // Set working directory to the directory containing the binary
        if let Some(parent) = self.config.magictunnel_binary.parent() {
            cmd.current_dir(parent);
        }

        let mut child = cmd.spawn()?;
        let pid = child.id().unwrap_or(0);

        // Forward stdout and stderr to supervisor's output with prefixes
        if let Some(stdout) = child.stdout.take() {
            let stdout_reader = BufReader::new(stdout);
            tokio::spawn(async move {
                let mut lines = stdout_reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    info!("[MagicTunnel] {}", line);
                }
            });
        }

        if let Some(stderr) = child.stderr.take() {
            let stderr_reader = BufReader::new(stderr);
            tokio::spawn(async move {
                let mut lines = stderr_reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    warn!("[MagicTunnel] {}", line);
                }
            });
        }

        self.process = Some(child);
        self.start_time = Some(Instant::now());
        
        info!("✅ MagicTunnel started successfully with PID: {}", pid);
        Ok(())
    }

    /// Stop MagicTunnel process gracefully
    pub async fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(mut process) = self.process.take() {
            let pid = process.id().unwrap_or(0);
            info!("Stopping MagicTunnel process (PID: {})", pid);

            // Try graceful shutdown first
            if let Err(e) = process.kill().await {
                warn!("Failed to kill process {}: {}", pid, e);
            }

            // Wait for process to exit with timeout
            let wait_result = timeout(Duration::from_secs(10), process.wait()).await;

            match wait_result {
                Ok(Ok(status)) => {
                    info!("MagicTunnel exited with status: {}", status);
                }
                Ok(Err(e)) => {
                    error!("Error waiting for MagicTunnel to exit: {}", e);
                }
                Err(_) => {
                    warn!("MagicTunnel did not exit within timeout, may still be running");
                }
            }
        }

        self.start_time = None;
        Ok(())
    }

    /// Restart MagicTunnel process
    pub async fn restart(&mut self, args: Option<Vec<String>>) -> Result<(), Box<dyn std::error::Error>> {
        info!("🔄 Restarting MagicTunnel...");

        // Check restart cooldown
        if let Some(last_restart) = self.last_restart {
            let elapsed = last_restart.elapsed().as_secs();
            if elapsed < self.config.restart_cooldown {
                let remaining = self.config.restart_cooldown - elapsed;
                return Err(format!("Restart cooldown active, {} seconds remaining", remaining).into());
            }
        }

        // Check restart attempt limit
        if self.restart_count >= self.config.max_restart_attempts {
            return Err("Maximum restart attempts exceeded".into());
        }

        self.stop().await?;
        
        // Brief pause to ensure clean shutdown
        tokio::time::sleep(Duration::from_millis(500)).await;
        
        self.start(args).await?;
        self.restart_count += 1;
        self.last_restart = Some(Instant::now());

        info!("✅ MagicTunnel restarted successfully (attempt {})", self.restart_count);
        Ok(())
    }

    /// Check if process is running
    pub fn is_running(&mut self) -> bool {
        if let Some(process) = &mut self.process {
            match process.try_wait() {
                Ok(Some(_)) => {
                    // Process has exited
                    self.process = None;
                    self.start_time = None;
                    false
                }
                Ok(None) => {
                    // Process is still running
                    true
                }
                Err(_) => {
                    // Error checking process status
                    false
                }
            }
        } else {
            false
        }
    }

    /// Get process status
    pub fn get_status(&mut self) -> ProcessStatus {
        let is_running = self.is_running();
        let pid = if is_running {
            self.process.as_ref().and_then(|p| p.id())
        } else {
            None
        };

        let uptime_seconds = if is_running {
            self.start_time.map(|start| start.elapsed().as_secs())
        } else {
            None
        };

        let status = if is_running {
            "running".to_string()
        } else {
            "stopped".to_string()
        };

        let health_status = if is_running {
            "healthy".to_string()
        } else {
            "stopped".to_string()
        };

        ProcessStatus {
            pid,
            status,
            uptime_seconds,
            restart_count: self.restart_count,
            last_restart: self.last_restart.map(|t| {
                chrono::DateTime::from_timestamp(
                    (Instant::now() - t.elapsed()).elapsed().as_secs() as i64,
                    0
                ).unwrap_or_default().to_rfc3339()
            }),
            health_status,
            args: self.current_args.clone(),
        }
    }

    /// Get MagicTunnel's configured port from environment variables or default
    fn get_magictunnel_port() -> u16 {
        // Read port from MCP_PORT environment variable, with fallback to config default
        if let Ok(port_str) = std::env::var("MCP_PORT") {
            if let Ok(port) = port_str.parse::<u16>() {
                return port;
            }
        }
        
        // Default port from magictunnel-config.yaml is 3001
        3001
    }

    /// Perform health check by calling MagicTunnel's /health endpoint
    pub async fn health_check(&self) -> bool {
        let port = Self::get_magictunnel_port();
        let health_url = format!("http://127.0.0.1:{}/health", port);
        
        debug!("Health check attempting HTTP request to {}", health_url);
        
        // First try HTTP health endpoint
        if let Ok(client) = reqwest::Client::builder()
            .timeout(Duration::from_secs(3))
            .build() 
        {
            match client.get(&health_url).send().await {
                Ok(response) if response.status().is_success() => {
                    debug!("Health check successful - MagicTunnel /health endpoint responded OK on port {}", port);
                    return true;
                },
                Ok(response) => {
                    debug!("Health check failed - /health endpoint returned status {} on port {}", response.status(), port);
                },
                Err(e) => {
                    debug!("Health check failed - HTTP request error to port {}: {}", port, e);
                }
            }
        }
        
        // Fallback to TCP connection check
        debug!("Falling back to TCP connection check on port {}", port);
        let address = format!("127.0.0.1:{}", port);
        match timeout(Duration::from_secs(2), TcpStream::connect(&address)).await {
            Ok(Ok(_)) => {
                debug!("Health check successful - TCP connection to MagicTunnel on port {}", port);
                true
            },
            Ok(Err(e)) => {
                debug!("Health check failed - TCP connection error to port {}: {}", port, e);
                false
            },
            Err(_) => {
                debug!("Health check failed - TCP connection timeout to port {}", port);
                false
            }
        }
    }

    /// Execute a custom command
    pub async fn execute_custom_command(&self, command: &CustomCommand, timeout_secs: u64) -> CommandExecutionResult {
        let start_time = Instant::now();
        info!("Executing custom command: {:?} {}", command.command_type, command.command);

        // Security check
        if !command.is_safe {
            warn!("Attempting to execute unsafe command: {}", command.command);
            return CommandExecutionResult {
                command: command.clone(),
                success: false,
                exit_code: None,
                stdout: None,
                stderr: None,
                execution_time_ms: start_time.elapsed().as_millis() as u64,
                error_message: Some("Command marked as unsafe and execution was denied".to_string()),
            };
        }

        // Build the command
        let mut cmd = match command.command_type {
            CommandType::Make => {
                let mut cmd = Command::new("make");
                cmd.arg(&command.command);
                cmd
            }
            CommandType::Cargo => {
                let mut cmd = Command::new("cargo");
                cmd.arg(&command.command);
                cmd
            }
            CommandType::Shell => {
                let mut cmd = Command::new("sh");
                cmd.arg("-c").arg(&command.command);
                cmd
            }
            CommandType::Binary => {
                Command::new(&command.command)
            }
        };

        // Add arguments if provided
        if let Some(ref args) = command.args {
            cmd.args(args);
        }

        // Set working directory if provided
        if let Some(ref working_dir) = command.working_dir {
            cmd.current_dir(working_dir);
        } else {
            // Use the directory containing the magictunnel binary as default
            if let Some(parent) = self.config.magictunnel_binary.parent() {
                cmd.current_dir(parent);
            }
        }

        // Set environment variables if provided
        if let Some(ref env) = command.env {
            for (key, value) in env {
                cmd.env(key, value);
            }
        }

        // Configure stdio
        cmd.stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        // Execute the command with timeout
        let execution_future = async {
            match cmd.spawn() {
                Ok(mut child) => {
                    match timeout(Duration::from_secs(timeout_secs), child.wait()).await {
                        Ok(Ok(exit_status)) => {
                            // Collect output
                            let stdout = if let Some(stdout) = child.stdout.take() {
                                let mut output = Vec::new();
                                let mut reader = tokio::io::BufReader::new(stdout);
                                if let Ok(_) = tokio::io::AsyncReadExt::read_to_end(&mut reader, &mut output).await {
                                    Some(String::from_utf8_lossy(&output).to_string())
                                } else {
                                    None
                                }
                            } else {
                                None
                            };

                            let stderr = if let Some(stderr) = child.stderr.take() {
                                let mut output = Vec::new();
                                let mut reader = tokio::io::BufReader::new(stderr);
                                if let Ok(_) = tokio::io::AsyncReadExt::read_to_end(&mut reader, &mut output).await {
                                    Some(String::from_utf8_lossy(&output).to_string())
                                } else {
                                    None
                                }
                            } else {
                                None
                            };

                            (true, exit_status.code(), stdout, stderr, None)
                        }
                        Ok(Err(e)) => {
                            let _ = child.kill();
                            (false, None, None, None, Some(format!("Process error: {}", e)))
                        }
                        Err(_) => {
                            let _ = child.kill();
                            (false, None, None, None, Some("Command timed out".to_string()))
                        }
                    }
                }
                Err(e) => {
                    (false, None, None, None, Some(format!("Failed to spawn process: {}", e)))
                }
            }
        };

        let (success, exit_code, stdout, stderr, error_message) = execution_future.await;
        let execution_time_ms = start_time.elapsed().as_millis() as u64;

        let result = CommandExecutionResult {
            command: command.clone(),
            success,
            exit_code,
            stdout,
            stderr,
            execution_time_ms,
            error_message,
        };

        if success {
            info!("Custom command completed successfully in {}ms", execution_time_ms);
        } else {
            warn!("Custom command failed: {:?}", result.error_message);
        }

        result
    }

    /// Execute a custom restart sequence
    pub async fn execute_custom_restart(
        &mut self,
        pre_commands: Option<Vec<CustomCommand>>,
        start_args: Option<Vec<String>>,
        post_commands: Option<Vec<CustomCommand>>,
    ) -> CustomRestartResult {
        let start_time = Instant::now();
        info!("🔧 Starting custom restart sequence");

        let mut pre_command_results = Vec::new();
        let mut post_command_results = Vec::new();
        let mut overall_success = true;

        // Execute pre-commands
        if let Some(pre_cmds) = pre_commands {
            info!("Executing {} pre-restart commands", pre_cmds.len());
            for cmd in &pre_cmds {
                let result = self.execute_custom_command(cmd, 60).await; // 60 second timeout for pre-commands
                if !result.success {
                    warn!("Pre-command failed: {:?}", result.error_message);
                    overall_success = false;
                }
                pre_command_results.push(result);
            }
        }

        // Perform the restart
        let restart_successful = if overall_success {
            match self.restart(start_args).await {
                Ok(()) => {
                    info!("✅ MagicTunnel restart completed successfully");
                    true
                }
                Err(e) => {
                    error!("❌ MagicTunnel restart failed: {}", e);
                    overall_success = false;
                    false
                }
            }
        } else {
            warn!("Skipping restart due to pre-command failures");
            false
        };

        // Execute post-commands only if restart was successful
        if let Some(post_cmds) = post_commands {
            if restart_successful {
                info!("Executing {} post-restart commands", post_cmds.len());
                // Give MagicTunnel a moment to fully start before post-commands
                tokio::time::sleep(Duration::from_secs(2)).await;
                
                for cmd in &post_cmds {
                    let result = self.execute_custom_command(cmd, 60).await; // 60 second timeout for post-commands
                    if !result.success {
                        warn!("Post-command failed: {:?}", result.error_message);
                        overall_success = false;
                    }
                    post_command_results.push(result);
                }
            } else {
                info!("Skipping post-commands due to restart failure");
            }
        }

        let total_execution_time_ms = start_time.elapsed().as_millis() as u64;

        let result = CustomRestartResult {
            pre_command_results,
            restart_successful,
            post_command_results,
            total_execution_time_ms,
            overall_success,
        };

        if overall_success {
            info!("✅ Custom restart sequence completed successfully in {}ms", total_execution_time_ms);
        } else {
            warn!("⚠️ Custom restart sequence completed with errors in {}ms", total_execution_time_ms);
        }

        result
    }
}

/// Supervisor server that manages MagicTunnel process
pub struct SupervisorServer {
    config: SupervisorConfig,
    process: Arc<Mutex<MagicTunnelProcess>>,
}

impl SupervisorServer {
    pub fn new(config: SupervisorConfig) -> Self {
        let process = MagicTunnelProcess::new(config.clone());
        Self {
            config: config.clone(),
            process: Arc::new(Mutex::new(process)),
        }
    }

    /// Start the supervisor server
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("🚀 Starting MagicTunnel Supervisor on port {}", self.config.control_port);

        // Start MagicTunnel initially
        {
            let mut process = self.process.lock().await;
            if let Err(e) = process.start(None).await {
                error!("Failed to start MagicTunnel initially: {}", e);
            }
        }

        // Start health check task
        self.start_health_check_task().await;

        // Start control server
        let listener = TcpListener::bind(format!("127.0.0.1:{}", self.config.control_port)).await?;
        info!("✅ Supervisor control interface listening on port {}", self.config.control_port);

        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    debug!("Accepted connection from {}", addr);
                    let process = Arc::clone(&self.process);
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_connection(stream, process).await {
                            error!("Error handling connection: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                }
            }
        }
    }

    /// Start health check background task
    async fn start_health_check_task(&self) {
        let process = Arc::clone(&self.process);
        let interval_secs = self.config.health_check_interval;

        info!("🏥 Starting health check task (interval: {}s)", interval_secs);
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(interval_secs));
            
            loop {
                interval.tick().await;
                
                let mut process_guard = process.lock().await;
                if process_guard.is_running() {
                    let pid = process_guard.process.as_ref().and_then(|p| p.id()).unwrap_or(0);
                    if !process_guard.health_check().await {
                        warn!("❌ Health check failed for PID {} - MagicTunnel may be unresponsive", pid);
                        // Could implement auto-restart here if desired
                    } else {
                        info!("✅ Health check passed for PID {} - MagicTunnel is responsive", pid);
                    }
                }
            }
        });
    }

    /// Handle incoming control connection
    async fn handle_connection(
        mut stream: TcpStream,
        process: Arc<Mutex<MagicTunnelProcess>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut reader = BufReader::new(&mut stream);
        let mut line = String::new();

        match reader.read_line(&mut line).await {
            Ok(0) => return Ok(()), // Connection closed
            Ok(_) => {
                let command: SupervisorCommand = match serde_json::from_str(&line) {
                    Ok(cmd) => cmd,
                    Err(e) => {
                        let response = SupervisorResponse {
                            success: false,
                            message: format!("Invalid command format: {}", e),
                            data: None,
                            timestamp: chrono::Utc::now().to_rfc3339(),
                        };
                        Self::send_response(&mut stream, response).await?;
                        return Ok(());
                    }
                };

                let response = Self::handle_command(command, process).await;
                Self::send_response(&mut stream, response).await?;
            }
            Err(e) => {
                error!("Failed to read from connection: {}", e);
            }
        }

        Ok(())
    }

    /// Handle supervisor command
    async fn handle_command(
        command: SupervisorCommand,
        process: Arc<Mutex<MagicTunnelProcess>>,
    ) -> SupervisorResponse {
        let timestamp = chrono::Utc::now().to_rfc3339();

        match command {
            SupervisorCommand::Restart { args } => {
                let mut process_guard = process.lock().await;
                match process_guard.restart(args).await {
                    Ok(()) => SupervisorResponse {
                        success: true,
                        message: "MagicTunnel restarted successfully".to_string(),
                        data: Some(serde_json::to_value(process_guard.get_status()).unwrap()),
                        timestamp,
                    },
                    Err(e) => SupervisorResponse {
                        success: false,
                        message: format!("Failed to restart MagicTunnel: {}", e),
                        data: None,
                        timestamp,
                    },
                }
            }
            SupervisorCommand::Stop => {
                let mut process_guard = process.lock().await;
                match process_guard.stop().await {
                    Ok(()) => SupervisorResponse {
                        success: true,
                        message: "MagicTunnel stopped successfully".to_string(),
                        data: None,
                        timestamp,
                    },
                    Err(e) => SupervisorResponse {
                        success: false,
                        message: format!("Failed to stop MagicTunnel: {}", e),
                        data: None,
                        timestamp,
                    },
                }
            }
            SupervisorCommand::Status => {
                let mut process_guard = process.lock().await;
                let status = process_guard.get_status();
                SupervisorResponse {
                    success: true,
                    message: "Status retrieved successfully".to_string(),
                    data: Some(serde_json::to_value(status).unwrap()),
                    timestamp,
                }
            }
            SupervisorCommand::HealthCheck => {
                let process_guard = process.lock().await;
                let is_healthy = process_guard.health_check().await;
                SupervisorResponse {
                    success: true,
                    message: if is_healthy { "Healthy" } else { "Unhealthy" }.to_string(),
                    data: Some(serde_json::json!({ "healthy": is_healthy })),
                    timestamp,
                }
            }
            SupervisorCommand::ReloadConfig { config_path: _ } => {
                // For now, restart with same args to reload config
                let mut process_guard = process.lock().await;
                match process_guard.restart(None).await {
                    Ok(()) => SupervisorResponse {
                        success: true,
                        message: "Configuration reloaded successfully".to_string(),
                        data: Some(serde_json::to_value(process_guard.get_status()).unwrap()),
                        timestamp,
                    },
                    Err(e) => SupervisorResponse {
                        success: false,
                        message: format!("Failed to reload configuration: {}", e),
                        data: None,
                        timestamp,
                    },
                }
            }
            SupervisorCommand::Shutdown => {
                let mut process_guard = process.lock().await;
                let _ = process_guard.stop().await;
                SupervisorResponse {
                    success: true,
                    message: "Supervisor shutting down".to_string(),
                    data: None,
                    timestamp,
                }
            }
            SupervisorCommand::CustomRestart { pre_commands, start_args, post_commands } => {
                let mut process_guard = process.lock().await;
                let result = process_guard.execute_custom_restart(pre_commands, start_args, post_commands).await;
                SupervisorResponse {
                    success: result.overall_success,
                    message: if result.overall_success { 
                        "Custom restart sequence completed successfully".to_string() 
                    } else { 
                        "Custom restart sequence completed with errors".to_string() 
                    },
                    data: Some(serde_json::to_value(result).unwrap()),
                    timestamp,
                }
            }
            SupervisorCommand::ExecuteCommand { command, timeout_seconds } => {
                let process_guard = process.lock().await;
                let timeout = timeout_seconds.unwrap_or(30); // Default 30 second timeout
                let result = process_guard.execute_custom_command(&command, timeout).await;
                SupervisorResponse {
                    success: result.success,
                    message: if result.success { 
                        "Command executed successfully".to_string() 
                    } else { 
                        format!("Command execution failed: {}", result.error_message.as_ref().unwrap_or(&"Unknown error".to_string()))
                    },
                    data: Some(serde_json::to_value(result).unwrap()),
                    timestamp,
                }
            }
        }
    }

    /// Send response back to client
    async fn send_response(
        stream: &mut TcpStream,
        response: SupervisorResponse,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let response_json = serde_json::to_string(&response)?;
        stream.write_all(response_json.as_bytes()).await?;
        stream.write_all(b"\n").await?;
        stream.flush().await?;
        Ok(())
    }
}

/// Supervisor client for communicating with the supervisor server
pub struct SupervisorClient {
    port: u16,
}

impl SupervisorClient {
    pub fn new(port: u16) -> Self {
        Self { port }
    }

    /// Send command to supervisor
    pub async fn send_command(&self, command: SupervisorCommand) -> Result<SupervisorResponse, Box<dyn std::error::Error>> {
        let mut stream = TcpStream::connect(format!("127.0.0.1:{}", self.port)).await?;
        
        let command_json = serde_json::to_string(&command)?;
        stream.write_all(command_json.as_bytes()).await?;
        stream.write_all(b"\n").await?;
        stream.flush().await?;

        let mut reader = BufReader::new(&mut stream);
        let mut response_line = String::new();
        reader.read_line(&mut response_line).await?;

        let response: SupervisorResponse = serde_json::from_str(&response_line)?;
        Ok(response)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("magictunnel_supervisor=info".parse()?)
        )
        .init();

    let args: Vec<String> = env::args().collect();
    
    if args.len() > 1 && args[1] == "client" {
        // Run as client for testing
        let client = SupervisorClient::new(8081);
        
        if args.len() > 2 {
            let command = match args[2].as_str() {
                "restart" => SupervisorCommand::Restart { args: None },
                "stop" => SupervisorCommand::Stop,
                "status" => SupervisorCommand::Status,
                "health" => SupervisorCommand::HealthCheck,
                "custom-restart" => {
                    // Example custom restart with build command
                    let pre_commands = vec![
                        CustomCommand {
                            command_type: CommandType::Make,
                            command: "build".to_string(),
                            args: None,
                            working_dir: None,
                            env: None,
                            description: Some("Build before restart".to_string()),
                            is_safe: true,
                        }
                    ];
                    SupervisorCommand::CustomRestart {
                        pre_commands: Some(pre_commands),
                        start_args: None,
                        post_commands: None,
                    }
                }
                "execute-make" => {
                    if args.len() > 3 {
                        let make_target = &args[3];
                        let command = CustomCommand {
                            command_type: CommandType::Make,
                            command: make_target.to_string(),
                            args: None,
                            working_dir: None,
                            env: None,
                            description: Some(format!("Execute make {}", make_target)),
                            is_safe: true, // Assume make commands are safe
                        };
                        SupervisorCommand::ExecuteCommand {
                            command,
                            timeout_seconds: Some(120), // 2 minute timeout
                        }
                    } else {
                        eprintln!("Usage: {} client execute-make <target>", args[0]);
                        std::process::exit(1);
                    }
                }
                _ => {
                    eprintln!("Usage: {} client [restart|stop|status|health|custom-restart|execute-make <target>]", args[0]);
                    std::process::exit(1);
                }
            };
            
            match client.send_command(command).await {
                Ok(response) => {
                    println!("Response: {}", serde_json::to_string_pretty(&response)?);
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        
        return Ok(());
    }

    // Run as supervisor server
    let config = SupervisorConfig::default();
    let supervisor = SupervisorServer::new(config);
    
    info!("🎯 MagicTunnel Supervisor starting...");
    supervisor.start().await?;

    Ok(())
}