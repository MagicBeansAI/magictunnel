//! Service type definitions

use crate::config::RuntimeMode;

/// Service loading summary for startup logging
#[derive(Debug)]
pub struct ServiceLoadingSummary {
    pub runtime_mode: RuntimeMode,
    pub total_services: usize,
    pub proxy_services: Vec<String>,
    pub advanced_services: Option<Vec<String>>,
    pub loading_time_ms: u64,
}

/// Service status information
#[derive(Debug, Clone)]
pub struct ServiceStatus {
    pub name: String,
    pub status: ServiceState,
    pub message: Option<String>,
}

/// Service state enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum ServiceState {
    /// Service is initializing
    Initializing,
    /// Service is running normally
    Running,
    /// Service has warnings but is functional
    Warning,
    /// Service has failed
    Failed,
    /// Service is shutting down
    Stopping,
    /// Service has stopped
    Stopped,
}

impl std::fmt::Display for ServiceState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceState::Initializing => write!(f, "initializing"),
            ServiceState::Running => write!(f, "running"),
            ServiceState::Warning => write!(f, "warning"),
            ServiceState::Failed => write!(f, "failed"),
            ServiceState::Stopping => write!(f, "stopping"),
            ServiceState::Stopped => write!(f, "stopped"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_state_display() {
        assert_eq!(ServiceState::Running.to_string(), "running");
        assert_eq!(ServiceState::Failed.to_string(), "failed");
        assert_eq!(ServiceState::Initializing.to_string(), "initializing");
    }
}