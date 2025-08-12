//! Automatic Session Recovery for OAuth 2.1 Phase 2.3
//!
//! This module provides automatic session restoration on application startup and during runtime,
//! building on the User Context System (Phase 2.1) and Multi-Platform Token Storage (Phase 2.2).
//!
//! Key Features:
//! - Startup session recovery with automatic token validation
//! - Runtime session validation with periodic checks
//! - Multi-provider support (GitHub, Google, Microsoft, custom)
//! - Graceful degradation for expired/invalid tokens
//! - Session state persistence across restarts
//! - Integration with existing authentication system

use crate::auth::{
    config::{AuthMethod, OAuthProviderConfig},
    token_storage::{TokenData, TokenStorage},
    user_context::UserContext,
    resolver::AuthResolver,
};
use crate::error::{Result, ProxyError};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, sync::{Arc, RwLock}};
use std::time::{Duration, SystemTime};
use tracing::{debug, error, info, trace, warn};
use std::collections::HashMap;
use tokio::{time::timeout};
use secrecy::ExposeSecret;

/// Authentication method for session tracking
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AuthMethodType {
    OAuth,
    DeviceCode,
    ApiKey,
    ServiceAccount,
}

impl From<&AuthMethod> for AuthMethodType {
    fn from(auth_method: &AuthMethod) -> Self {
        match auth_method {
            AuthMethod::OAuth { .. } => AuthMethodType::OAuth,
            AuthMethod::DeviceCode { .. } => AuthMethodType::DeviceCode,
            AuthMethod::ApiKey { .. } => AuthMethodType::ApiKey,
            AuthMethod::ServiceAccount { .. } => AuthMethodType::ServiceAccount,
        }
    }
}

/// Active session information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveSession {
    /// OAuth provider name (e.g., "github", "google", "microsoft")
    pub provider: String,
    /// User identifier associated with this session
    pub user_id: String,
    /// Token expiration timestamp
    pub expires_at: Option<SystemTime>,
    /// Last time this session was validated
    pub last_validated: SystemTime,
    /// Authentication method used for this session
    pub authentication_method: AuthMethodType,
    /// Session creation timestamp
    pub created_at: SystemTime,
    /// Number of validation attempts
    pub validation_attempts: u32,
    /// Whether this session is currently valid
    pub is_valid: bool,
    /// Error message if session validation failed
    pub last_error: Option<String>,
    /// Scopes associated with this session
    pub scopes: Vec<String>,
}

impl ActiveSession {
    /// Create a new active session
    pub fn new(
        provider: String,
        user_id: String,
        expires_at: Option<SystemTime>,
        authentication_method: AuthMethodType,
        scopes: Vec<String>,
    ) -> Self {
        let now = SystemTime::now();
        Self {
            provider,
            user_id,
            expires_at,
            last_validated: now,
            authentication_method,
            created_at: now,
            validation_attempts: 0,
            is_valid: true,
            last_error: None,
            scopes,
        }
    }

    /// Check if the session is expired
    pub fn is_expired(&self) -> bool {
        match self.expires_at {
            Some(expires_at) => SystemTime::now() > expires_at,
            None => false, // No expiration time means permanent session
        }
    }

    /// Check if the session needs validation (based on last validation time)
    pub fn needs_validation(&self, validation_interval: Duration) -> bool {
        SystemTime::now()
            .duration_since(self.last_validated)
            .unwrap_or(Duration::MAX)
            > validation_interval
    }

    /// Mark session as validated
    pub fn mark_validated(&mut self, is_valid: bool, error: Option<String>) {
        self.last_validated = SystemTime::now();
        self.validation_attempts += 1;
        self.is_valid = is_valid;
        self.last_error = error;
    }

    /// Get session age
    pub fn age(&self) -> Duration {
        SystemTime::now()
            .duration_since(self.created_at)
            .unwrap_or(Duration::ZERO)
    }

    /// Get time since last validation
    pub fn time_since_validation(&self) -> Duration {
        SystemTime::now()
            .duration_since(self.last_validated)
            .unwrap_or(Duration::ZERO)
    }
}

/// Session state for the entire system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    /// All active sessions keyed by session ID
    pub active_sessions: HashMap<String, ActiveSession>,
    /// Last time session recovery was performed
    pub last_recovery_check: SystemTime,
    /// Total number of recovery attempts
    pub recovery_attempts: u32,
    /// Providers that failed during last recovery attempt
    pub failed_providers: HashSet<String>,
    /// Session state version for migration compatibility
    pub version: u32,
    /// System identifier to detect environment changes
    pub system_id: String,
}

impl Default for SessionState {
    fn default() -> Self {
        Self {
            active_sessions: HashMap::new(),
            last_recovery_check: SystemTime::now(),
            recovery_attempts: 0,
            failed_providers: HashSet::new(),
            version: 1,
            system_id: Self::generate_system_id(),
        }
    }
}

impl SessionState {
    /// Generate a system identifier based on environment
    fn generate_system_id() -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        
        // Include hostname
        whoami::fallible::hostname().unwrap_or_else(|_| "unknown".to_string()).hash(&mut hasher);
        
        // Include username
        whoami::username().hash(&mut hasher);
        
        // Include platform info
        whoami::platform().to_string().hash(&mut hasher);
        
        format!("{:x}", hasher.finish())
    }

    /// Check if this session state is compatible with current system
    pub fn is_compatible_with_current_system(&self) -> bool {
        self.system_id == Self::generate_system_id()
    }

    /// Clean up expired sessions
    pub fn cleanup_expired_sessions(&mut self) -> u32 {
        let initial_count = self.active_sessions.len();
        self.active_sessions.retain(|_, session| !session.is_expired());
        let removed_count = initial_count - self.active_sessions.len();
        
        if removed_count > 0 {
            debug!("Cleaned up {} expired sessions", removed_count);
        }
        
        removed_count as u32
    }

    /// Get sessions that need validation
    pub fn get_sessions_needing_validation(&self, validation_interval: Duration) -> Vec<String> {
        self.active_sessions
            .iter()
            .filter(|(_, session)| session.needs_validation(validation_interval))
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Get statistics
    pub fn get_stats(&self) -> SessionStateStats {
        let total_sessions = self.active_sessions.len();
        let valid_sessions = self.active_sessions.values().filter(|s| s.is_valid).count();
        let expired_sessions = self.active_sessions.values().filter(|s| s.is_expired()).count();
        
        let mut providers = HashMap::new();
        for session in self.active_sessions.values() {
            *providers.entry(session.provider.clone()).or_insert(0) += 1;
        }

        SessionStateStats {
            total_sessions,
            valid_sessions,
            expired_sessions,
            failed_providers: self.failed_providers.len(),
            recovery_attempts: self.recovery_attempts,
            providers,
            last_recovery_check: self.last_recovery_check,
            system_compatible: self.is_compatible_with_current_system(),
        }
    }
}

/// Session state statistics
#[derive(Debug, Clone, Serialize)]
pub struct SessionStateStats {
    pub total_sessions: usize,
    pub valid_sessions: usize,
    pub expired_sessions: usize,
    pub failed_providers: usize,
    pub recovery_attempts: u32,
    pub providers: HashMap<String, u32>,
    pub last_recovery_check: SystemTime,
    pub system_compatible: bool,
}

/// Configuration for session recovery behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecoveryConfig {
    /// Enable automatic session recovery
    pub enabled: bool,
    /// Automatically recover sessions on application startup
    pub auto_recovery_on_startup: bool,
    /// Interval between validation checks in minutes
    pub validation_interval_minutes: u64,
    /// Maximum number of recovery attempts before giving up
    pub max_recovery_attempts: u32,
    /// Timeout for token validation requests in seconds
    pub token_validation_timeout_seconds: u64,
    /// Enable graceful degradation when tokens fail validation
    pub graceful_degradation: bool,
    /// Retry failed providers on next recovery cycle
    pub retry_failed_providers: bool,
    /// Clean up expired sessions during recovery
    pub cleanup_expired_sessions: bool,
    /// Maximum age for sessions before forced revalidation (hours)
    pub max_session_age_hours: u64,
    /// Enable persistence of session state
    pub persist_session_state: bool,
}

impl Default for SessionRecoveryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_recovery_on_startup: true,
            validation_interval_minutes: 60, // Validate every hour
            max_recovery_attempts: 3,
            token_validation_timeout_seconds: 30,
            graceful_degradation: true,
            retry_failed_providers: true,
            cleanup_expired_sessions: true,
            max_session_age_hours: 24, // Revalidate after 24 hours
            persist_session_state: true,
        }
    }
}

impl SessionRecoveryConfig {
    /// Get validation interval as Duration
    pub fn validation_interval(&self) -> Duration {
        Duration::from_secs(self.validation_interval_minutes * 60)
    }

    /// Get token validation timeout as Duration
    pub fn token_validation_timeout(&self) -> Duration {
        Duration::from_secs(self.token_validation_timeout_seconds)
    }

    /// Get maximum session age as Duration
    pub fn max_session_age(&self) -> Duration {
        Duration::from_secs(self.max_session_age_hours * 3600)
    }
}

/// Session recovery result
#[derive(Debug, Clone)]
pub struct SessionRecoveryResult {
    /// Number of sessions successfully recovered
    pub recovered_sessions: u32,
    /// Number of sessions that failed validation
    pub failed_validations: u32,
    /// Number of expired sessions cleaned up
    pub cleaned_up_sessions: u32,
    /// Providers that failed during recovery
    pub failed_providers: Vec<String>,
    /// Total time taken for recovery
    pub recovery_duration: Duration,
    /// Whether recovery was successful overall
    pub success: bool,
    /// Error messages if any
    pub errors: Vec<String>,
}

impl SessionRecoveryResult {
    /// Create a new successful recovery result
    pub fn success() -> Self {
        Self {
            recovered_sessions: 0,
            failed_validations: 0,
            cleaned_up_sessions: 0,
            failed_providers: Vec::new(),
            recovery_duration: Duration::ZERO,
            success: true,
            errors: Vec::new(),
        }
    }

    /// Create a failed recovery result
    pub fn failure(error: String) -> Self {
        Self {
            recovered_sessions: 0,
            failed_validations: 0,
            cleaned_up_sessions: 0,
            failed_providers: Vec::new(),
            recovery_duration: Duration::ZERO,
            success: false,
            errors: vec![error],
        }
    }

    /// Add an error to the result
    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
        self.success = false;
    }
}

/// Main session manager for automatic session recovery
#[derive(Debug)]
pub struct SessionManager {
    /// User context for session identification
    user_context: UserContext,
    /// Token storage for retrieving and managing tokens
    token_storage: Arc<TokenStorage>,
    /// Current session state (thread-safe)
    session_state: Arc<RwLock<SessionState>>,
    /// Session recovery configuration
    recovery_config: SessionRecoveryConfig,
    /// HTTP client for token validation
    http_client: Client,
    /// OAuth provider configurations
    oauth_providers: HashMap<String, OAuthProviderConfig>,
    /// Authentication resolver for method resolution
    auth_resolver: Option<Arc<AuthResolver>>,
}

impl SessionManager {
    /// Create a new session manager
    pub async fn new(
        user_context: UserContext,
        token_storage: Arc<TokenStorage>,
        recovery_config: SessionRecoveryConfig,
    ) -> Result<Self> {
        debug!("Creating session manager for user: {}", user_context.get_unique_user_id());
        
        let session_state = if recovery_config.persist_session_state {
            Self::load_session_state(&user_context).await.unwrap_or_default()
        } else {
            SessionState::default()
        };

        let http_client = Client::builder()
            .timeout(recovery_config.token_validation_timeout())
            .build()
            .map_err(|e| ProxyError::config(format!("Failed to create HTTP client: {}", e)))?;

        let manager = Self {
            user_context,
            token_storage,
            session_state: Arc::new(RwLock::new(session_state)),
            recovery_config: recovery_config.clone(),
            http_client,
            oauth_providers: HashMap::new(),
            auth_resolver: None,
        };

        // Load session state from storage if enabled
        if recovery_config.persist_session_state {
            manager.load_persisted_state().await?;
        }

        info!("Session manager created successfully");
        Ok(manager)
    }

    /// Set OAuth provider configurations
    pub fn set_oauth_providers(&mut self, providers: HashMap<String, OAuthProviderConfig>) {
        debug!("Setting {} OAuth providers", providers.len());
        self.oauth_providers = providers;
    }

    /// Set authentication resolver
    pub fn set_auth_resolver(&mut self, resolver: Arc<AuthResolver>) {
        debug!("Setting authentication resolver");
        self.auth_resolver = Some(resolver);
    }

    /// Perform automatic session recovery on startup
    pub async fn recover_sessions_on_startup(&self) -> Result<SessionRecoveryResult> {
        if !self.recovery_config.enabled || !self.recovery_config.auto_recovery_on_startup {
            debug!("Session recovery disabled or startup recovery disabled");
            return Ok(SessionRecoveryResult::success());
        }

        info!("Starting automatic session recovery on startup");
        let start_time = SystemTime::now();
        
        let result = self.perform_session_recovery().await;
        
        let duration = start_time.elapsed().unwrap_or(Duration::ZERO);
        info!("Startup session recovery completed in {:?}", duration);
        
        result
    }

    /// Perform runtime session validation
    pub async fn validate_sessions_runtime(&self) -> Result<SessionRecoveryResult> {
        if !self.recovery_config.enabled {
            debug!("Session recovery disabled");
            return Ok(SessionRecoveryResult::success());
        }

        let validation_interval = self.recovery_config.validation_interval();
        let sessions_to_validate = {
            let state = self.session_state.read().unwrap();
            state.get_sessions_needing_validation(validation_interval)
        };

        if sessions_to_validate.is_empty() {
            trace!("No sessions need validation");
            return Ok(SessionRecoveryResult::success());
        }

        debug!("Validating {} sessions", sessions_to_validate.len());
        let start_time = SystemTime::now();
        
        let result = self.validate_specific_sessions(&sessions_to_validate).await;
        
        let duration = start_time.elapsed().unwrap_or(Duration::ZERO);
        debug!("Runtime session validation completed in {:?}", duration);
        
        result
    }

    /// Perform full session recovery
    async fn perform_session_recovery(&self) -> Result<SessionRecoveryResult> {
        let start_time = SystemTime::now();
        let mut result = SessionRecoveryResult::success();

        // Update recovery attempt counter
        {
            let mut state = self.session_state.write().unwrap();
            state.recovery_attempts += 1;
            state.last_recovery_check = SystemTime::now();
        }

        // Check system compatibility
        {
            let state = self.session_state.read().unwrap();
            if !state.is_compatible_with_current_system() {
                warn!("Session state is not compatible with current system, resetting");
                drop(state);
                let mut state = self.session_state.write().unwrap();
                *state = SessionState::default();
                result.add_error("Session state reset due to system incompatibility".to_string());
            }
        }

        // Clean up expired sessions if enabled
        if self.recovery_config.cleanup_expired_sessions {
            let cleaned_up = {
                let mut state = self.session_state.write().unwrap();
                state.cleanup_expired_sessions()
            };
            result.cleaned_up_sessions = cleaned_up;
            if cleaned_up > 0 {
                debug!("Cleaned up {} expired sessions", cleaned_up);
            }
        }

        // Get all stored tokens and attempt to recover sessions
        match self.token_storage.get_all_tokens().await {
            Ok(tokens) => {
                for (key, token_data) in tokens {
                    match self.recover_session_from_token(&key, &token_data).await {
                        Ok(true) => {
                            result.recovered_sessions += 1;
                        }
                        Ok(false) => {
                            result.failed_validations += 1;
                            debug!("Token validation failed for key: {}", key);
                        }
                        Err(e) => {
                            result.failed_validations += 1;
                            result.add_error(format!("Recovery failed for token {}: {}", key, e));
                            warn!("Failed to recover session for token {}: {}", key, e);
                        }
                    }
                }
            }
            Err(e) => {
                result.add_error(format!("Failed to retrieve tokens: {}", e));
                error!("Failed to retrieve tokens for session recovery: {}", e);
            }
        }

        result.recovery_duration = start_time.elapsed().unwrap_or(Duration::ZERO);
        
        // Persist session state if enabled
        if self.recovery_config.persist_session_state {
            if let Err(e) = self.save_persisted_state().await {
                result.add_error(format!("Failed to persist session state: {}", e));
            }
        }

        info!(
            "Session recovery completed: {} recovered, {} failed, {} cleaned up",
            result.recovered_sessions, result.failed_validations, result.cleaned_up_sessions
        );

        Ok(result)
    }

    /// Validate specific sessions
    async fn validate_specific_sessions(&self, session_ids: &[String]) -> Result<SessionRecoveryResult> {
        let start_time = SystemTime::now();
        let mut result = SessionRecoveryResult::success();

        for session_id in session_ids {
            let session = {
                let state = self.session_state.read().unwrap();
                state.active_sessions.get(session_id).cloned()
            };

            if let Some(session) = session {
                match self.validate_session(&session).await {
                    Ok(is_valid) => {
                        self.update_session_validation_result(session_id, is_valid, None).await;
                        if is_valid {
                            result.recovered_sessions += 1;
                        } else {
                            result.failed_validations += 1;
                        }
                    }
                    Err(e) => {
                        self.update_session_validation_result(
                            session_id,
                            false,
                            Some(e.to_string())
                        ).await;
                        result.failed_validations += 1;
                        result.add_error(format!("Session {} validation failed: {}", session_id, e));
                    }
                }
            } else {
                result.add_error(format!("Session {} not found", session_id));
            }
        }

        result.recovery_duration = start_time.elapsed().unwrap_or(Duration::ZERO);
        
        // Persist session state if enabled
        if self.recovery_config.persist_session_state {
            if let Err(e) = self.save_persisted_state().await {
                result.add_error(format!("Failed to persist session state: {}", e));
            }
        }

        Ok(result)
    }

    /// Recover a session from a stored token
    async fn recover_session_from_token(&self, _key: &str, token_data: &TokenData) -> Result<bool> {
        // Check if token is expired
        if token_data.is_expired() {
            debug!("Token for provider {} is expired", token_data.provider);
            return Ok(false);
        }

        // Validate token with OAuth provider
        let is_valid = self.validate_token_with_provider(token_data).await?;
        
        if is_valid {
            // Create or update active session
            let user_id = token_data.user_id.as_deref().unwrap_or("unknown");
            let session_id = self.generate_session_id(&token_data.provider, user_id);
            let session = ActiveSession::new(
                token_data.provider.clone(),
                user_id.to_string(),
                token_data.expires_at,
                AuthMethodType::OAuth, // Assume OAuth for now
                token_data.scopes.clone(),
            );

            {
                let mut state = self.session_state.write().unwrap();
                state.active_sessions.insert(session_id.clone(), session);
                state.failed_providers.remove(&token_data.provider);
            }

            debug!("Recovered session for provider: {}", token_data.provider);
            Ok(true)
        } else {
            // Mark provider as failed
            {
                let mut state = self.session_state.write().unwrap();
                state.failed_providers.insert(token_data.provider.clone());
            }
            
            debug!("Token validation failed for provider: {}", token_data.provider);
            Ok(false)
        }
    }

    /// Validate a token with its OAuth provider
    async fn validate_token_with_provider(&self, token_data: &TokenData) -> Result<bool> {
        let provider_config = match self.oauth_providers.get(&token_data.provider) {
            Some(config) => config,
            None => {
                warn!("No provider configuration found for: {}", token_data.provider);
                return Ok(false);
            }
        };

        // Use userinfo endpoint to validate token
        let userinfo_url = match &provider_config.user_info_endpoint {
            Some(url) => url,
            None => {
                debug!("No userinfo endpoint configured for provider: {}", token_data.provider);
                return Ok(false);
            }
        };
        
        let request = self
            .http_client
            .get(userinfo_url)
            .header("Authorization", format!("Bearer {}", token_data.access_token.expose_secret()))
            .header("Accept", "application/json");

        match timeout(self.recovery_config.token_validation_timeout(), request.send()).await {
            Ok(Ok(response)) => {
                let is_success = response.status().is_success();
                if is_success {
                    trace!("Token validation successful for provider: {}", token_data.provider);
                } else {
                    debug!("Token validation failed with status: {} for provider: {}", 
                           response.status(), token_data.provider);
                }
                Ok(is_success)
            }
            Ok(Err(e)) => {
                debug!("Token validation request failed for provider {}: {}", token_data.provider, e);
                Ok(false)
            }
            Err(_) => {
                debug!("Token validation timed out for provider: {}", token_data.provider);
                Ok(false)
            }
        }
    }

    /// Validate an active session
    async fn validate_session(&self, session: &ActiveSession) -> Result<bool> {
        // Check if session is expired
        if session.is_expired() {
            debug!("Session for provider {} is expired", session.provider);
            return Ok(false);
        }

        // Get token data from storage
        let token_key = self.generate_token_key(&session.provider, Some(&session.user_id));
        match self.token_storage.retrieve_token(&token_key).await? {
            Some(token_data) => self.validate_token_with_provider(&token_data).await,
            None => {
                warn!("No token found for session: {}:{}", session.provider, session.user_id);
                Ok(false)
            }
        }
    }

    /// Update session validation result
    async fn update_session_validation_result(
        &self,
        session_id: &str,
        is_valid: bool,
        error: Option<String>,
    ) {
        let mut state = self.session_state.write().unwrap();
        if let Some(session) = state.active_sessions.get_mut(session_id) {
            session.mark_validated(is_valid, error);
        }
    }

    /// Generate a session ID
    fn generate_session_id(&self, provider: &str, user_id: &str) -> String {
        format!("{}:{}:{}", self.user_context.get_unique_user_id(), provider, user_id)
    }

    /// Generate a token key consistent with token storage
    fn generate_token_key(&self, provider: &str, user_id: Option<&str>) -> String {
        let unique_user_id = self.user_context.get_unique_user_id();
        match user_id {
            Some(uid) => format!("{}:{}:{}", unique_user_id, provider, uid),
            None => format!("{}:{}", unique_user_id, provider),
        }
    }

    /// Load session state from persistent storage
    async fn load_persisted_state(&self) -> Result<()> {
        let state_file = self.user_context.get_hostname_session_file_path("session_state.json");
        
        if state_file.exists() {
            match tokio::fs::read_to_string(&state_file).await {
                Ok(content) => {
                    match serde_json::from_str::<SessionState>(&content) {
                        Ok(loaded_state) => {
                            let mut state = self.session_state.write().unwrap();
                            *state = loaded_state;
                            debug!("Loaded session state from: {}", state_file.display());
                            return Ok(());
                        }
                        Err(e) => {
                            warn!("Failed to deserialize session state: {}", e);
                        }
                    }
                }
                Err(e) => {
                    debug!("Failed to read session state file: {}", e);
                }
            }
        }

        debug!("Using default session state");
        Ok(())
    }

    /// Save session state to persistent storage
    async fn save_persisted_state(&self) -> Result<()> {
        let state_file = self.user_context.get_hostname_session_file_path("session_state.json");
        
        let content = {
            let state = self.session_state.read().unwrap();
            serde_json::to_string_pretty(&*state)
                .map_err(|e| ProxyError::config(format!("Failed to serialize session state: {}", e)))?
        };

        tokio::fs::write(&state_file, content).await
            .map_err(|e| ProxyError::config(format!("Failed to write session state: {}", e)))?;

        trace!("Saved session state to: {}", state_file.display());
        Ok(())
    }

    /// Load session state from storage (static method for initialization)
    async fn load_session_state(user_context: &UserContext) -> Option<SessionState> {
        let state_file = user_context.get_hostname_session_file_path("session_state.json");
        
        if state_file.exists() {
            match tokio::fs::read_to_string(&state_file).await {
                Ok(content) => {
                    match serde_json::from_str::<SessionState>(&content) {
                        Ok(state) => {
                            debug!("Loaded session state from: {}", state_file.display());
                            return Some(state);
                        }
                        Err(e) => {
                            warn!("Failed to deserialize session state: {}", e);
                        }
                    }
                }
                Err(e) => {
                    debug!("Failed to read session state file: {}", e);
                }
            }
        }

        None
    }

    /// Get current session state statistics
    pub fn get_session_stats(&self) -> SessionStateStats {
        let state = self.session_state.read().unwrap();
        state.get_stats()
    }

    /// Get active sessions
    pub fn get_active_sessions(&self) -> HashMap<String, ActiveSession> {
        let state = self.session_state.read().unwrap();
        state.active_sessions.clone()
    }

    /// Check if a provider has an active valid session
    pub fn has_active_session(&self, provider: &str) -> bool {
        let state = self.session_state.read().unwrap();
        state.active_sessions
            .values()
            .any(|session| session.provider == provider && session.is_valid && !session.is_expired())
    }

    /// Get active session for provider
    pub fn get_active_session(&self, provider: &str) -> Option<ActiveSession> {
        let state = self.session_state.read().unwrap();
        state.active_sessions
            .values()
            .find(|session| session.provider == provider && session.is_valid && !session.is_expired())
            .cloned()
    }

    /// Remove a session
    pub async fn remove_session(&self, provider: &str, user_id: &str) -> Result<bool> {
        let session_id = self.generate_session_id(provider, user_id);
        let removed = {
            let mut state = self.session_state.write().unwrap();
            state.active_sessions.remove(&session_id).is_some()
        };

        if removed {
            debug!("Removed session: {}", session_id);
            
            // Persist state if enabled
            if self.recovery_config.persist_session_state {
                self.save_persisted_state().await?;
            }
        }

        Ok(removed)
    }

    /// Clear all sessions
    pub async fn clear_all_sessions(&self) -> Result<u32> {
        let count = {
            let mut state = self.session_state.write().unwrap();
            let count = state.active_sessions.len() as u32;
            state.active_sessions.clear();
            state.failed_providers.clear();
            count
        };

        info!("Cleared {} sessions", count);
        
        // Persist state if enabled
        if self.recovery_config.persist_session_state {
            self.save_persisted_state().await?;
        }

        Ok(count)
    }

    /// Get user context
    pub fn get_user_context(&self) -> &UserContext {
        &self.user_context
    }

    /// Get recovery configuration
    pub fn get_recovery_config(&self) -> &SessionRecoveryConfig {
        &self.recovery_config
    }
}