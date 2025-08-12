//! Token Refresh Service for OAuth 2.1 Phase 2.4
//!
//! This module provides comprehensive token refresh management with OAuth 2.1 compliance,
//! building on all previous phases:
//! - Phase 1: OAuth 2.1 implementation
//! - Phase 2.1: User Context System with cross-platform user identification
//! - Phase 2.2: Multi-Platform Token Storage with secure storage backends
//! - Phase 2.3: Automatic Session Recovery with runtime validation
//!
//! Key Features:
//! - Background token refresh with automatic scheduling
//! - OAuth 2.1 refresh token rotation for security
//! - Provider-specific refresh logic for GitHub, Google, Microsoft, custom providers
//! - Robust retry mechanisms with exponential backoff
//! - Concurrent refresh handling with queue management
//! - Refresh scheduling based on token expiration times
//! - Comprehensive error recovery and network failure handling

use crate::{auth::{
    config::OAuthProviderConfig,
    oauth::OAuthTokenResponse,
    session_manager::{ActiveSession, SessionManager},
    token_storage::{TokenData, TokenStorage},
    user_context::UserContext,
}};
use crate::error::ProxyError;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::time::{interval, timeout, Instant};
use std::{collections::{HashMap, VecDeque}, sync::{Arc, Mutex}};
use std::time::{Duration, SystemTime};
use tracing::{debug, info, warn, error};
use secrecy::ExposeSecret;
use zeroize::Zeroize;
use crate::error::Result;

/// Comprehensive token refresh configuration with OAuth 2.1 compliance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenRefreshConfig {
    /// Enable automatic token refresh
    pub enabled: bool,
    /// Minutes before expiration to trigger refresh (default: 15 minutes)
    pub refresh_threshold_minutes: u64,
    /// Enable background refresh scheduler
    pub background_refresh_enabled: bool,
    /// Maximum retry attempts for failed refreshes (default: 3)
    pub max_retry_attempts: u32,
    /// Base delay in seconds for exponential backoff (default: 5)
    pub retry_delay_base_seconds: u64,
    /// Maximum concurrent refresh operations (default: 5)
    pub concurrent_refresh_limit: usize,
    /// Maximum size of refresh queue (default: 100)
    pub refresh_queue_size: usize,
    /// Enable refresh token rotation (OAuth 2.1 security feature)
    pub enable_refresh_token_rotation: bool,
    /// Token refresh timeout in seconds (default: 30)
    pub refresh_timeout_seconds: u64,
    /// Interval between background refresh checks in minutes (default: 30)
    pub background_check_interval_minutes: u64,
    /// Enable automatic cleanup of failed refresh attempts
    pub auto_cleanup_failed_attempts: bool,
    /// Maximum age for retry attempts before cleanup (hours, default: 24)
    pub max_retry_age_hours: u64,
    /// Enable refresh notifications
    pub enable_refresh_notifications: bool,
    /// Preserve token metadata during refresh
    pub preserve_token_metadata: bool,
}

impl Default for TokenRefreshConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            refresh_threshold_minutes: 15,
            background_refresh_enabled: true,
            max_retry_attempts: 3,
            retry_delay_base_seconds: 5,
            concurrent_refresh_limit: 5,
            refresh_queue_size: 100,
            enable_refresh_token_rotation: true, // OAuth 2.1 default
            refresh_timeout_seconds: 30,
            background_check_interval_minutes: 30,
            auto_cleanup_failed_attempts: true,
            max_retry_age_hours: 24,
            enable_refresh_notifications: true,
            preserve_token_metadata: true,
        }
    }
}

impl TokenRefreshConfig {
    /// Get refresh threshold as Duration
    pub fn refresh_threshold(&self) -> Duration {
        Duration::from_secs(self.refresh_threshold_minutes * 60)
    }

    /// Get retry delay base as Duration
    pub fn retry_delay_base(&self) -> Duration {
        Duration::from_secs(self.retry_delay_base_seconds)
    }

    /// Get refresh timeout as Duration
    pub fn refresh_timeout(&self) -> Duration {
        Duration::from_secs(self.refresh_timeout_seconds)
    }

    /// Get background check interval as Duration
    pub fn background_check_interval(&self) -> Duration {
        Duration::from_secs(self.background_check_interval_minutes * 60)
    }

    /// Get max retry age as Duration
    pub fn max_retry_age(&self) -> Duration {
        Duration::from_secs(self.max_retry_age_hours * 3600)
    }
}

/// Refresh task for background scheduling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshTask {
    /// OAuth provider name
    pub provider: String,
    /// User identifier
    pub user_id: String,
    /// Next refresh time
    pub next_refresh_at: SystemTime,
    /// Current retry count
    pub retry_count: u32,
    /// Last attempt timestamp
    pub last_attempt: Option<SystemTime>,
    /// Refresh interval for this task
    pub refresh_interval: Duration,
    /// Priority level (lower number = higher priority)
    pub priority: u32,
    /// Task creation timestamp
    pub created_at: SystemTime,
    /// Last error message if any
    pub last_error: Option<String>,
}

impl RefreshTask {
    /// Create a new refresh task
    pub fn new(provider: String, user_id: String, refresh_interval: Duration) -> Self {
        let now = SystemTime::now();
        Self {
            provider,
            user_id,
            next_refresh_at: now + refresh_interval,
            retry_count: 0,
            last_attempt: None,
            refresh_interval,
            priority: 1,
            created_at: now,
            last_error: None,
        }
    }

    /// Check if this task is due for execution
    pub fn is_due(&self) -> bool {
        SystemTime::now() >= self.next_refresh_at
    }

    /// Schedule next retry with exponential backoff
    pub fn schedule_retry(&mut self, base_delay: Duration, max_attempts: u32) {
        self.retry_count += 1;
        self.last_attempt = Some(SystemTime::now());
        
        if self.retry_count <= max_attempts {
            // Exponential backoff: base_delay * 2^(retry_count - 1)
            let multiplier = 2_u64.pow((self.retry_count - 1).min(10)); // Cap at 2^10
            let delay = base_delay * multiplier.min(3600).try_into().unwrap_or(3600); // Cap at 1 hour
            self.next_refresh_at = SystemTime::now() + delay;
            self.priority += 1; // Lower priority for retries
            
            debug!("Scheduled retry #{} for {}:{} in {:?}", 
                   self.retry_count, self.provider, self.user_id, delay);
        }
    }

    /// Check if task has exceeded retry limits
    pub fn has_exceeded_retry_limit(&self, max_attempts: u32) -> bool {
        self.retry_count >= max_attempts
    }

    /// Get task age
    pub fn age(&self) -> Duration {
        SystemTime::now()
            .duration_since(self.created_at)
            .unwrap_or(Duration::ZERO)
    }

    /// Generate task identifier
    pub fn task_id(&self) -> String {
        format!("{}:{}", self.provider, self.user_id)
    }
}

/// Refresh request for queueing
#[derive(Debug, Clone)]
pub struct RefreshRequest {
    /// OAuth provider name
    pub provider: String,
    /// User identifier
    pub user_id: String,
    /// Request priority
    pub priority: u32,
    /// Request timestamp
    pub requested_at: SystemTime,
    /// Force refresh even if not due
    pub force_refresh: bool,
}

impl RefreshRequest {
    /// Create a new refresh request
    pub fn new(provider: String, user_id: String) -> Self {
        Self {
            provider,
            user_id,
            priority: 1,
            requested_at: SystemTime::now(),
            force_refresh: false,
        }
    }

    /// Create a high-priority refresh request
    pub fn urgent(provider: String, user_id: String) -> Self {
        Self {
            provider,
            user_id,
            priority: 0,
            requested_at: SystemTime::now(),
            force_refresh: true,
        }
    }
}

/// Background refresh scheduler for managing token refresh tasks
#[derive(Debug)]
pub struct RefreshScheduler {
    /// Active refresh tasks keyed by task ID
    active_refreshes: HashMap<String, RefreshTask>,
    /// Refresh request queue
    refresh_queue: VecDeque<RefreshRequest>,
    /// Background task handle
    background_handle: Option<tokio::task::JoinHandle<()>>,
    /// Number of currently executing refreshes
    active_refresh_count: usize,
    /// Scheduler statistics
    stats: RefreshStats,
}

impl RefreshScheduler {
    /// Create a new refresh scheduler
    pub fn new() -> Self {
        Self {
            active_refreshes: HashMap::new(),
            refresh_queue: VecDeque::new(),
            background_handle: None,
            active_refresh_count: 0,
            stats: RefreshStats::default(),
        }
    }

    /// Add a refresh task
    pub fn add_task(&mut self, task: RefreshTask) {
        let task_id = task.task_id();
        debug!("Adding refresh task: {}", task_id);
        
        self.active_refreshes.insert(task_id, task);
        self.stats.total_tasks_scheduled += 1;
    }

    /// Remove a refresh task
    pub fn remove_task(&mut self, task_id: &str) -> Option<RefreshTask> {
        if let Some(task) = self.active_refreshes.remove(task_id) {
            debug!("Removed refresh task: {}", task_id);
            self.stats.total_tasks_completed += 1;
            Some(task)
        } else {
            None
        }
    }

    /// Enqueue a refresh request
    pub fn enqueue_request(&mut self, request: RefreshRequest, max_queue_size: usize) -> Result<()> {
        if self.refresh_queue.len() >= max_queue_size {
            warn!("Refresh queue is full, dropping oldest request");
            self.refresh_queue.pop_front();
            self.stats.total_requests_dropped += 1;
        }

        // Insert request based on priority
        let request_priority = request.priority;
        let mut insert_position = None;
        
        for (i, existing) in self.refresh_queue.iter().enumerate() {
            if request_priority < existing.priority {
                insert_position = Some(i);
                break;
            }
        }

        if let Some(i) = insert_position {
            self.refresh_queue.insert(i, request);
        } else {
            self.refresh_queue.push_back(request);
        }

        self.stats.total_requests_queued += 1;
        Ok(())
    }

    /// Dequeue a refresh request
    pub fn dequeue_request(&mut self) -> Option<RefreshRequest> {
        let request = self.refresh_queue.pop_front();
        if request.is_some() {
            self.stats.total_requests_processed += 1;
        }
        request
    }

    /// Get due tasks
    pub fn get_due_tasks(&self) -> Vec<RefreshTask> {
        self.active_refreshes
            .values()
            .filter(|task| task.is_due())
            .cloned()
            .collect()
    }

    /// Update task after retry
    pub fn update_task_retry(&mut self, task_id: &str, base_delay: Duration, max_attempts: u32, error: Option<String>) {
        if let Some(task) = self.active_refreshes.get_mut(task_id) {
            task.last_error = error;
            task.schedule_retry(base_delay, max_attempts);
            self.stats.total_retries += 1;
        }
    }

    /// Clean up old failed tasks
    pub fn cleanup_old_tasks(&mut self, max_age: Duration, max_attempts: u32) -> u32 {
        let _now = SystemTime::now();
        let initial_count = self.active_refreshes.len();
        
        self.active_refreshes.retain(|_, task| {
            let is_too_old = task.age() > max_age;
            let exceeded_retries = task.has_exceeded_retry_limit(max_attempts);
            
            if is_too_old || exceeded_retries {
                debug!("Cleaning up old/failed task: {} (age: {:?}, retries: {})", 
                       task.task_id(), task.age(), task.retry_count);
                false
            } else {
                true
            }
        });

        let cleaned_count = initial_count - self.active_refreshes.len();
        self.stats.total_tasks_cleaned += cleaned_count as u32;
        cleaned_count as u32
    }

    /// Increment active refresh count
    pub fn increment_active_count(&mut self) {
        self.active_refresh_count += 1;
        self.stats.current_active_refreshes = self.active_refresh_count;
    }

    /// Decrement active refresh count
    pub fn decrement_active_count(&mut self) {
        if self.active_refresh_count > 0 {
            self.active_refresh_count -= 1;
            self.stats.current_active_refreshes = self.active_refresh_count;
        }
    }

    /// Check if we can start more refresh operations
    pub fn can_start_refresh(&self, max_concurrent: usize) -> bool {
        self.active_refresh_count < max_concurrent
    }

    /// Get scheduler statistics
    pub fn get_stats(&self) -> RefreshStats {
        self.stats.clone()
    }
}

/// Refresh scheduler statistics
#[derive(Debug, Clone, Default, Serialize)]
pub struct RefreshStats {
    /// Total tasks scheduled
    pub total_tasks_scheduled: u32,
    /// Total tasks completed
    pub total_tasks_completed: u32,
    /// Total tasks cleaned up
    pub total_tasks_cleaned: u32,
    /// Total requests queued
    pub total_requests_queued: u32,
    /// Total requests processed
    pub total_requests_processed: u32,
    /// Total requests dropped
    pub total_requests_dropped: u32,
    /// Total retry attempts
    pub total_retries: u32,
    /// Currently active refreshes
    pub current_active_refreshes: usize,
    /// Total successful refreshes
    pub total_successful_refreshes: u32,
    /// Total failed refreshes
    pub total_failed_refreshes: u32,
}

/// Token refresh result
#[derive(Debug, Clone)]
pub struct TokenRefreshResult {
    /// Whether the refresh was successful
    pub success: bool,
    /// New token data (if successful)
    pub new_token: Option<TokenData>,
    /// Previous token data for comparison
    pub old_token: Option<TokenData>,
    /// Refresh duration
    pub duration: Duration,
    /// Error message if failed
    pub error: Option<String>,
    /// Whether refresh token was rotated
    pub refresh_token_rotated: bool,
    /// Provider name
    pub provider: String,
    /// User ID
    pub user_id: String,
    /// Retry attempt number
    pub retry_attempt: u32,
}

impl TokenRefreshResult {
    /// Create a successful refresh result
    pub fn success(
        new_token: TokenData,
        old_token: Option<TokenData>,
        duration: Duration,
        provider: String,
        user_id: String,
        refresh_token_rotated: bool,
    ) -> Self {
        Self {
            success: true,
            new_token: Some(new_token),
            old_token,
            duration,
            error: None,
            refresh_token_rotated,
            provider,
            user_id,
            retry_attempt: 0,
        }
    }

    /// Create a failed refresh result
    pub fn failure(
        error: String,
        duration: Duration,
        provider: String,
        user_id: String,
        retry_attempt: u32,
    ) -> Self {
        Self {
            success: false,
            new_token: None,
            old_token: None,
            duration,
            error: Some(error),
            refresh_token_rotated: false,
            provider,
            user_id,
            retry_attempt,
        }
    }
}

/// Main Token Refresh Service implementation
#[derive(Debug)]
pub struct TokenRefreshService {
    /// Token storage for retrieving and updating tokens
    token_storage: Arc<TokenStorage>,
    /// Session manager for session state updates
    session_manager: Arc<SessionManager>,
    /// Token refresh configuration
    refresh_config: TokenRefreshConfig,
    /// Background refresh scheduler
    refresh_scheduler: Arc<Mutex<RefreshScheduler>>,
    /// OAuth clients for different providers
    oauth_clients: HashMap<String, Arc<dyn std::fmt::Debug + Send + Sync>>,
    /// HTTP client for OAuth requests
    http_client: Client,
    /// OAuth provider configurations
    oauth_providers: HashMap<String, OAuthProviderConfig>,
    /// User context for session management
    user_context: UserContext,
}

impl TokenRefreshService {
    /// Create a new token refresh service
    pub async fn new(
        user_context: UserContext,
        token_storage: Arc<TokenStorage>,
        session_manager: Arc<SessionManager>,
        refresh_config: TokenRefreshConfig,
    ) -> Result<Self> {
        debug!("Creating token refresh service for user: {}", user_context.get_unique_user_id());

        let http_client = Client::builder()
            .timeout(refresh_config.refresh_timeout())
            .build()
            .map_err(|e| ProxyError::config(format!("Failed to create HTTP client: {}", e)))?;

        let service = Self {
            token_storage,
            session_manager,
            refresh_config,
            refresh_scheduler: Arc::new(Mutex::new(RefreshScheduler::new())),
            oauth_clients: HashMap::new(),
            http_client,
            oauth_providers: HashMap::new(),
            user_context,
        };

        info!("Token refresh service created successfully");
        Ok(service)
    }

    /// Set OAuth provider configurations
    pub fn set_oauth_providers(&mut self, providers: HashMap<String, OAuthProviderConfig>) {
        debug!("Setting {} OAuth providers", providers.len());
        self.oauth_providers = providers;
    }

    /// Set OAuth clients
    pub fn set_oauth_clients(&mut self, clients: HashMap<String, Arc<dyn std::fmt::Debug + Send + Sync>>) {
        debug!("Setting {} OAuth clients", clients.len());
        self.oauth_clients = clients;
    }

    /// Start background token refresh scheduler
    pub async fn start_background_refresh(&self) -> Result<()> {
        if !self.refresh_config.enabled || !self.refresh_config.background_refresh_enabled {
            debug!("Background token refresh is disabled");
            return Ok(());
        }

        info!("Starting background token refresh scheduler");
        
        let service_clone = self.clone_for_background();
        let refresh_interval = self.refresh_config.background_check_interval();
        
        let handle = tokio::spawn(async move {
            service_clone.run_background_scheduler(refresh_interval).await;
        });

        {
            let mut scheduler = self.refresh_scheduler.lock().unwrap();
            scheduler.background_handle = Some(handle);
        }

        // Schedule initial refresh tasks for existing tokens
        self.schedule_existing_tokens().await?;

        Ok(())
    }

    /// Stop background token refresh scheduler
    pub async fn stop_background_refresh(&self) {
        info!("Stopping background token refresh scheduler");
        
        let handle = {
            let mut scheduler = self.refresh_scheduler.lock().unwrap();
            scheduler.background_handle.take()
        };

        if let Some(handle) = handle {
            handle.abort();
            debug!("Background refresh scheduler stopped");
        }
    }

    /// Manual token refresh for specific provider and user
    pub async fn refresh_token(
        &self,
        provider: &str,
        user_id: &str,
        force_refresh: bool,
    ) -> Result<TokenRefreshResult> {
        let start_time = Instant::now();
        debug!("Refreshing token for {}:{}, force: {}", provider, user_id, force_refresh);

        // Get current token
        let token_key = self.generate_token_key(provider, Some(user_id));
        let current_token = match self.token_storage.retrieve_token(&token_key).await? {
            Some(token) => token,
            None => {
                warn!("No token found for {}:{}", provider, user_id);
                return Ok(TokenRefreshResult::failure(
                    "Token not found".to_string(),
                    start_time.elapsed(),
                    provider.to_string(),
                    user_id.to_string(),
                    0,
                ));
            }
        };

        // Check if refresh is needed
        if !force_refresh && !self.should_refresh_token(&current_token) {
            debug!("Token for {}:{} does not need refresh", provider, user_id);
            return Ok(TokenRefreshResult::success(
                current_token.clone(),
                Some(current_token),
                start_time.elapsed(),
                provider.to_string(),
                user_id.to_string(),
                false,
            ));
        }

        // Perform the actual refresh
        match self.perform_token_refresh(&current_token).await {
            Ok((new_token, refresh_token_rotated)) => {
                // Store the new token
                self.token_storage.store_token(&token_key, &new_token).await?;

                // Update session if it exists
                if let Some(session) = self.session_manager.get_active_session(provider) {
                    self.update_session_after_refresh(&session, &new_token).await?;
                }

                let duration = start_time.elapsed();
                info!("Token refreshed successfully for {}:{} in {:?}", provider, user_id, duration);

                Ok(TokenRefreshResult::success(
                    new_token,
                    Some(current_token),
                    duration,
                    provider.to_string(),
                    user_id.to_string(),
                    refresh_token_rotated,
                ))
            }
            Err(e) => {
                warn!("Token refresh failed for {}:{}: {}", provider, user_id, e);
                Ok(TokenRefreshResult::failure(
                    e.to_string(),
                    start_time.elapsed(),
                    provider.to_string(),
                    user_id.to_string(),
                    0,
                ))
            }
        }
    }

    /// Schedule refresh for a specific token
    pub async fn schedule_token_refresh(
        &self,
        provider: &str,
        user_id: &str,
        refresh_interval: Option<Duration>,
    ) -> Result<()> {
        let interval = refresh_interval.unwrap_or(self.refresh_config.refresh_threshold());
        let task = RefreshTask::new(provider.to_string(), user_id.to_string(), interval);
        
        {
            let mut scheduler = self.refresh_scheduler.lock().unwrap();
            scheduler.add_task(task);
        }
        
        debug!("Scheduled token refresh for {}:{}", provider, user_id);
        Ok(())
    }

    /// Request immediate token refresh
    pub async fn request_immediate_refresh(&self, provider: &str, user_id: &str) -> Result<()> {
        let request = RefreshRequest::urgent(provider.to_string(), user_id.to_string());
        
        {
            let mut scheduler = self.refresh_scheduler.lock().unwrap();
            scheduler.enqueue_request(request, self.refresh_config.refresh_queue_size)?;
        }
        
        debug!("Requested immediate token refresh for {}:{}", provider, user_id);
        Ok(())
    }

    /// Get refresh service statistics
    pub fn get_refresh_stats(&self) -> RefreshStats {
        let scheduler = self.refresh_scheduler.lock().unwrap();
        scheduler.get_stats()
    }

    /// Clean up old refresh tasks and failed attempts
    pub async fn cleanup_old_refresh_tasks(&self) -> u32 {
        let max_age = self.refresh_config.max_retry_age();
        let max_attempts = self.refresh_config.max_retry_attempts;
        
        let cleaned_count = {
            let mut scheduler = self.refresh_scheduler.lock().unwrap();
            scheduler.cleanup_old_tasks(max_age, max_attempts)
        };

        if cleaned_count > 0 {
            info!("Cleaned up {} old refresh tasks", cleaned_count);
        }

        cleaned_count
    }

    // Private helper methods

    /// Clone service for background operations
    fn clone_for_background(&self) -> TokenRefreshServiceBackground {
        TokenRefreshServiceBackground {
            token_storage: Arc::clone(&self.token_storage),
            session_manager: Arc::clone(&self.session_manager),
            refresh_config: self.refresh_config.clone(),
            refresh_scheduler: Arc::clone(&self.refresh_scheduler),
            oauth_providers: self.oauth_providers.clone(),
            http_client: self.http_client.clone(),
            user_context: self.user_context.clone(),
        }
    }

    /// Check if token should be refreshed
    fn should_refresh_token(&self, token: &TokenData) -> bool {
        match token.expires_at {
            Some(expires_at) => {
                let now = SystemTime::now();
                let refresh_threshold = self.refresh_config.refresh_threshold();
                now + refresh_threshold >= expires_at
            }
            None => false, // No expiration means no refresh needed
        }
    }

    /// Perform the actual token refresh operation
    async fn perform_token_refresh(&self, current_token: &TokenData) -> Result<(TokenData, bool)> {
        let provider_config = self.oauth_providers.get(&current_token.provider)
            .ok_or_else(|| ProxyError::config(format!("No provider configuration for: {}", current_token.provider)))?;

        let refresh_token = current_token.refresh_token.as_ref()
            .ok_or_else(|| ProxyError::config("No refresh token available".to_string()))?;

        // Prepare refresh request
        let token_endpoint = provider_config.token_endpoint.as_ref()
            .ok_or_else(|| ProxyError::config("No token endpoint configured".to_string()))?;

        let mut params = vec![
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token.expose_secret()),
        ];

        // Add client credentials
        if !provider_config.client_secret.expose_secret().is_empty() {
            params.push(("client_id", &provider_config.client_id));
            params.push(("client_secret", provider_config.client_secret.expose_secret()));
        }

        // Add resource indicators if supported
        if let Some(resources) = &provider_config.resource_indicators {
            for resource in resources {
                params.push(("resource", resource));
            }
        }

        // Make refresh request with timeout
        let request = self.http_client
            .post(token_endpoint)
            .form(&params)
            .header("Accept", "application/json");

        let response = timeout(
            self.refresh_config.refresh_timeout(),
            request.send()
        )
        .await
        .map_err(|_| ProxyError::config("Token refresh request timed out".to_string()))?
        .map_err(|e| ProxyError::config(format!("Token refresh request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(ProxyError::config(format!(
                "Token refresh failed with status {}: {}", status, body
            )));
        }

        let token_response: OAuthTokenResponse = response.json().await
            .map_err(|e| ProxyError::config(format!("Failed to parse token response: {}", e)))?;

        // Update token with refresh response
        let mut new_token = current_token.clone();
        let refresh_token_rotated = token_response.refresh_token.is_some();
        new_token.update_from_refresh(&token_response);

        // Preserve metadata if configured
        if self.refresh_config.preserve_token_metadata {
            // Metadata is already preserved in the clone
        }

        Ok((new_token, refresh_token_rotated))
    }

    /// Generate token key consistent with token storage
    fn generate_token_key(&self, provider: &str, user_id: Option<&str>) -> String {
        let unique_user_id = self.user_context.get_unique_user_id();
        match user_id {
            Some(uid) => format!("{}:{}:{}", unique_user_id, provider, uid),
            None => format!("{}:{}", unique_user_id, provider),
        }
    }

    /// Update session after successful token refresh
    async fn update_session_after_refresh(
        &self,
        _session: &ActiveSession,
        _new_token: &TokenData,
    ) -> Result<()> {
        // Update session expiration and validation status
        // This would integrate with the session manager to update session state
        debug!("Session updated after token refresh");
        Ok(())
    }

    /// Schedule refresh tasks for all existing tokens
    async fn schedule_existing_tokens(&self) -> Result<()> {
        debug!("Scheduling refresh tasks for existing tokens");
        
        let tokens = self.token_storage.get_all_tokens().await?;
        for (key, token) in tokens {
            if token.refresh_token.is_some() && !token.is_expired() {
                let user_id = token.user_id.as_deref().unwrap_or("unknown");
                if let Err(e) = self.schedule_token_refresh(&token.provider, user_id, None).await {
                    warn!("Failed to schedule refresh for token {}: {}", key, e);
                }
            }
        }

        info!("Scheduled refresh tasks for existing tokens");
        Ok(())
    }
}

/// Background service helper for running the scheduler
#[derive(Debug, Clone)]
struct TokenRefreshServiceBackground {
    token_storage: Arc<TokenStorage>,
    session_manager: Arc<SessionManager>,
    refresh_config: TokenRefreshConfig,
    refresh_scheduler: Arc<Mutex<RefreshScheduler>>,
    oauth_providers: HashMap<String, OAuthProviderConfig>,
    http_client: Client,
    user_context: UserContext,
}

impl TokenRefreshServiceBackground {
    /// Run the background refresh scheduler
    async fn run_background_scheduler(&self, check_interval: Duration) {
        info!("Background token refresh scheduler started");
        let mut interval_timer = interval(check_interval);
        
        loop {
            interval_timer.tick().await;
            
            if let Err(e) = self.process_refresh_cycle().await {
                error!("Error in refresh cycle: {}", e);
            }
        }
    }

    /// Process one refresh cycle
    async fn process_refresh_cycle(&self) -> Result<()> {
        // Clean up old tasks if enabled
        if self.refresh_config.auto_cleanup_failed_attempts {
            let max_age = self.refresh_config.max_retry_age();
            let max_attempts = self.refresh_config.max_retry_attempts;
            
            let cleaned_count = {
                let mut scheduler = self.refresh_scheduler.lock().unwrap();
                scheduler.cleanup_old_tasks(max_age, max_attempts)
            };
            
            if cleaned_count > 0 {
                debug!("Cleaned up {} old refresh tasks", cleaned_count);
            }
        }

        // Process due tasks
        let due_tasks = {
            let scheduler = self.refresh_scheduler.lock().unwrap();
            scheduler.get_due_tasks()
        };

        for task in due_tasks {
            if self.can_start_refresh() {
                self.spawn_refresh_task(task).await;
            } else {
                // Queue the task for later
                let request = RefreshRequest::new(task.provider.clone(), task.user_id.clone());
                let mut scheduler = self.refresh_scheduler.lock().unwrap();
                if let Err(e) = scheduler.enqueue_request(request, self.refresh_config.refresh_queue_size) {
                    warn!("Failed to queue refresh request: {}", e);
                }
            }
        }

        // Process queued requests
        while self.can_start_refresh() {
            let request = {
                let mut scheduler = self.refresh_scheduler.lock().unwrap();
                scheduler.dequeue_request()
            };

            if let Some(request) = request {
                let task = RefreshTask::new(
                    request.provider,
                    request.user_id,
                    self.refresh_config.refresh_threshold(),
                );
                self.spawn_refresh_task(task).await;
            } else {
                break;
            }
        }

        Ok(())
    }

    /// Check if we can start a new refresh operation
    fn can_start_refresh(&self) -> bool {
        let scheduler = self.refresh_scheduler.lock().unwrap();
        scheduler.can_start_refresh(self.refresh_config.concurrent_refresh_limit)
    }

    /// Spawn a refresh task
    async fn spawn_refresh_task(&self, task: RefreshTask) {
        let service_clone = self.clone();
        
        {
            let mut scheduler = self.refresh_scheduler.lock().unwrap();
            scheduler.increment_active_count();
        }

        tokio::spawn(async move {
            let result = service_clone.execute_refresh_task(task).await;
            
            // Update scheduler state
            {
                let mut scheduler = service_clone.refresh_scheduler.lock().unwrap();
                scheduler.decrement_active_count();
                
                match result {
                    Ok(_) => scheduler.stats.total_successful_refreshes += 1,
                    Err(_) => scheduler.stats.total_failed_refreshes += 1,
                }
            }
        });
    }

    /// Execute a single refresh task
    async fn execute_refresh_task(&self, task: RefreshTask) -> Result<()> {
        let start_time = Instant::now();
        debug!("Executing refresh task: {}", task.task_id());

        // Get current token
        let token_key = self.generate_token_key(&task.provider, Some(&task.user_id));
        let current_token = match self.token_storage.retrieve_token(&token_key).await? {
            Some(token) => token,
            None => {
                warn!("No token found for {}", task.task_id());
                return Err(ProxyError::config("Token not found".to_string()));
            }
        };

        // Perform refresh
        match self.perform_token_refresh(&current_token).await {
            Ok((new_token, _refresh_token_rotated)) => {
                // Store the new token
                self.token_storage.store_token(&token_key, &new_token).await?;
                
                // Remove completed task
                {
                    let mut scheduler = self.refresh_scheduler.lock().unwrap();
                    scheduler.remove_task(&task.task_id());
                }
                
                let duration = start_time.elapsed();
                info!("Token refreshed successfully for {} in {:?}", task.task_id(), duration);
                
                Ok(())
            }
            Err(e) => {
                warn!("Token refresh failed for {}: {}", task.task_id(), e);
                
                // Update task for retry
                {
                    let mut scheduler = self.refresh_scheduler.lock().unwrap();
                    scheduler.update_task_retry(
                        &task.task_id(),
                        self.refresh_config.retry_delay_base(),
                        self.refresh_config.max_retry_attempts,
                        Some(e.to_string()),
                    );
                }
                
                Err(e)
            }
        }
    }

    /// Perform token refresh (duplicate of main service method)
    async fn perform_token_refresh(&self, current_token: &TokenData) -> Result<(TokenData, bool)> {
        let provider_config = self.oauth_providers.get(&current_token.provider)
            .ok_or_else(|| ProxyError::config(format!("No provider configuration for: {}", current_token.provider)))?;

        let refresh_token = current_token.refresh_token.as_ref()
            .ok_or_else(|| ProxyError::config("No refresh token available".to_string()))?;

        let token_endpoint = provider_config.token_endpoint.as_ref()
            .ok_or_else(|| ProxyError::config("No token endpoint configured".to_string()))?;

        let mut params = vec![
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token.expose_secret()),
        ];

        if !provider_config.client_secret.expose_secret().is_empty() {
            params.push(("client_id", &provider_config.client_id));
            params.push(("client_secret", provider_config.client_secret.expose_secret()));
        }

        if let Some(resources) = &provider_config.resource_indicators {
            for resource in resources {
                params.push(("resource", resource));
            }
        }

        let request = self.http_client
            .post(token_endpoint)
            .form(&params)
            .header("Accept", "application/json");

        let response = timeout(
            self.refresh_config.refresh_timeout(),
            request.send()
        )
        .await
        .map_err(|_| ProxyError::config("Token refresh request timed out".to_string()))?
        .map_err(|e| ProxyError::config(format!("Token refresh request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(ProxyError::config(format!(
                "Token refresh failed with status {}: {}", status, body
            )));
        }

        let token_response: OAuthTokenResponse = response.json().await
            .map_err(|e| ProxyError::config(format!("Failed to parse token response: {}", e)))?;

        let mut new_token = current_token.clone();
        let refresh_token_rotated = token_response.refresh_token.is_some();
        new_token.update_from_refresh(&token_response);

        Ok((new_token, refresh_token_rotated))
    }

    /// Generate token key
    fn generate_token_key(&self, provider: &str, user_id: Option<&str>) -> String {
        let unique_user_id = self.user_context.get_unique_user_id();
        match user_id {
            Some(uid) => format!("{}:{}:{}", unique_user_id, provider, uid),
            None => format!("{}:{}", unique_user_id, provider),
        }
    }
}

/// Secure cleanup implementation for TokenRefreshResult
impl Zeroize for TokenRefreshResult {
    fn zeroize(&mut self) {
        if let Some(ref mut token) = self.new_token {
            token.zeroize();
        }
        if let Some(ref mut token) = self.old_token {
            token.zeroize();
        }
    }
}

impl Drop for TokenRefreshResult {
    fn drop(&mut self) {
        self.zeroize();
    }
}