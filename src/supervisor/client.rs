//! Supervisor Client
//! 
//! Client for communicating with the MagicTunnel supervisor process.

use crate::error::{ProxyError, Result};
use super::types::{SupervisorCommand, SupervisorResponse, CustomCommand, CommandType};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

/// Client for communicating with the supervisor server
pub struct SupervisorClient {
    port: u16,
    timeout_seconds: u64,
}

impl SupervisorClient {
    /// Create a new supervisor client
    pub fn new(port: u16) -> Self {
        Self {
            port,
            timeout_seconds: 10,
        }
    }

    /// Create a new supervisor client with custom timeout
    pub fn with_timeout(port: u16, timeout_seconds: u64) -> Self {
        Self {
            port,
            timeout_seconds,
        }
    }

    /// Check if supervisor is available
    pub async fn is_available(&self) -> bool {
        match timeout(
            Duration::from_secs(2),
            TcpStream::connect(format!("127.0.0.1:{}", self.port))
        ).await {
            Ok(Ok(_)) => true,
            _ => false,
        }
    }

    /// Send command to supervisor
    pub async fn send_command(&self, command: SupervisorCommand) -> Result<SupervisorResponse> {
        debug!("Sending supervisor command: {:?}", command);

        let connect_future = TcpStream::connect(format!("127.0.0.1:{}", self.port));
        let mut stream = timeout(Duration::from_secs(self.timeout_seconds), connect_future)
            .await
            .map_err(|_| ProxyError::timeout("Supervisor connection timeout".to_string()))?
            .map_err(|e| ProxyError::connection(format!("Failed to connect to supervisor: {}", e)))?;
        
        let command_json = serde_json::to_string(&command)
            .map_err(|e| ProxyError::mcp(format!("Failed to serialize command: {}", e)))?;
        
        // Send command
        let write_future = async {
            stream.write_all(command_json.as_bytes()).await?;
            stream.write_all(b"\n").await?;
            stream.flush().await?;
            Ok::<(), std::io::Error>(())
        };

        timeout(Duration::from_secs(self.timeout_seconds), write_future)
            .await
            .map_err(|_| ProxyError::timeout("Supervisor write timeout".to_string()))?
            .map_err(|e| ProxyError::connection(format!("Failed to send command to supervisor: {}", e)))?;

        // Read response
        let mut reader = BufReader::new(&mut stream);
        let mut response_line = String::new();
        
        let read_future = reader.read_line(&mut response_line);
        timeout(Duration::from_secs(self.timeout_seconds), read_future)
            .await
            .map_err(|_| ProxyError::timeout("Supervisor response timeout".to_string()))?
            .map_err(|e| ProxyError::connection(format!("Failed to read supervisor response: {}", e)))?;

        let response: SupervisorResponse = serde_json::from_str(&response_line)
            .map_err(|e| ProxyError::mcp(format!("Failed to deserialize supervisor response: {}", e)))?;

        debug!("Received supervisor response: {:?}", response);
        Ok(response)
    }

    /// Restart MagicTunnel via supervisor
    pub async fn restart_magictunnel(&self, args: Option<Vec<String>>) -> Result<SupervisorResponse> {
        info!("🔄 Requesting MagicTunnel restart via supervisor");
        let command = SupervisorCommand::Restart { args };
        self.send_command(command).await
    }

    /// Stop MagicTunnel via supervisor
    pub async fn stop_magictunnel(&self) -> Result<SupervisorResponse> {
        info!("⏹️ Requesting MagicTunnel stop via supervisor");
        let command = SupervisorCommand::Stop;
        self.send_command(command).await
    }

    /// Get MagicTunnel status via supervisor
    pub async fn get_status(&self) -> Result<SupervisorResponse> {
        debug!("📊 Requesting MagicTunnel status via supervisor");
        let command = SupervisorCommand::Status;
        self.send_command(command).await
    }

    /// Perform health check via supervisor
    pub async fn health_check(&self) -> Result<SupervisorResponse> {
        debug!("🏥 Requesting health check via supervisor");
        let command = SupervisorCommand::HealthCheck;
        self.send_command(command).await
    }

    /// Reload configuration via supervisor
    pub async fn reload_config(&self, config_path: Option<String>) -> Result<SupervisorResponse> {
        info!("📝 Requesting configuration reload via supervisor");
        let command = SupervisorCommand::ReloadConfig { config_path };
        self.send_command(command).await
    }

    /// Shutdown supervisor (this will also stop MagicTunnel)
    pub async fn shutdown_supervisor(&self) -> Result<SupervisorResponse> {
        warn!("⚠️ Requesting supervisor shutdown");
        let command = SupervisorCommand::Shutdown;
        self.send_command(command).await
    }

    /// Execute custom restart sequence with pre/post commands
    pub async fn custom_restart(&self, 
        pre_commands: Option<Vec<CustomCommand>>,
        start_args: Option<Vec<String>>,
        post_commands: Option<Vec<CustomCommand>>
    ) -> Result<SupervisorResponse> {
        info!("🔧 Requesting custom restart sequence via supervisor");
        let command = SupervisorCommand::CustomRestart {
            pre_commands,
            start_args,
            post_commands,
        };
        self.send_command(command).await
    }

    /// Execute a single custom command
    pub async fn execute_command(&self, command: CustomCommand, timeout_seconds: Option<u64>) -> Result<SupervisorResponse> {
        info!("⚡ Executing custom command: {:?}", command.command);
        let supervisor_command = SupervisorCommand::ExecuteCommand {
            command,
            timeout_seconds,
        };
        self.send_command(supervisor_command).await
    }

    /// Create a Makefile command
    pub fn create_make_command(target: &str, description: Option<String>) -> CustomCommand {
        CustomCommand {
            command_type: CommandType::Make,
            command: target.to_string(),
            args: None,
            working_dir: None,
            env: None,
            description,
            is_safe: Self::is_safe_make_target(target),
        }
    }

    /// Create a Cargo command  
    pub fn create_cargo_command(subcommand: &str, args: Option<Vec<String>>, description: Option<String>) -> CustomCommand {
        CustomCommand {
            command_type: CommandType::Cargo,
            command: subcommand.to_string(),
            args,
            working_dir: None,
            env: None,
            description,
            is_safe: Self::is_safe_cargo_command(subcommand),
        }
    }

    /// Create a shell command (use with caution)
    pub fn create_shell_command(command: &str, description: Option<String>) -> CustomCommand {
        CustomCommand {
            command_type: CommandType::Shell,
            command: command.to_string(),
            args: None,
            working_dir: None,
            env: None,
            description,
            is_safe: false, // Shell commands are inherently less safe
        }
    }

    /// Check if a make target is considered safe
    fn is_safe_make_target(target: &str) -> bool {
        const SAFE_TARGETS: &[&str] = &[
            "build", "build-release", "test", "check", "fmt", "clippy", 
            "clean", "run", "run-dev", "run-release", "docs", "help",
            "install-tools", "dev-check", "audit", "update"
        ];
        SAFE_TARGETS.contains(&target)
    }

    /// Check if a cargo command is considered safe
    fn is_safe_cargo_command(subcommand: &str) -> bool {
        const SAFE_SUBCOMMANDS: &[&str] = &[
            "build", "test", "check", "fmt", "clippy", "clean", 
            "run", "doc", "tree", "update", "audit"
        ];
        SAFE_SUBCOMMANDS.contains(&subcommand)
    }
}

impl Default for SupervisorClient {
    fn default() -> Self {
        Self::new(8081) // Default supervisor port
    }
}