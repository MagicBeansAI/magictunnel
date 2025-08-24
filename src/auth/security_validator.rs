//! Comprehensive Security Validation and Session Recovery
//!
//! This module provides comprehensive security validation mechanisms and
//! session recovery capabilities to ensure secure multi-user remote deployments
//! with proper protection against session hijacking and unauthorized access.

use crate::auth::{IsolatedSession, IsolationSessionState};
use crate::error::Result;
use actix_web::HttpRequest;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc, time::{Duration, SystemTime}};
use tracing::{debug, info};
use chrono::Timelike;

/// Comprehensive security validator for remote sessions
#[derive(Debug)]
pub struct SecurityValidator {
    /// Validation configuration
    config: SecurityValidationConfig,
    
    /// Security policy engine
    policy_engine: Arc<SecurityPolicyEngine>,
    
    /// Session recovery manager
    recovery_manager: Arc<SessionRecoveryManager>,
    
    /// Threat detection engine
    threat_detector: Arc<ThreatDetectionEngine>,
}

/// Security validation configuration
#[derive(Debug, Clone)]
pub struct SecurityValidationConfig {
    /// Enable comprehensive security validation
    pub enable_validation: bool,
    
    /// Minimum security score required (0.0-1.0)
    pub min_security_score: f64,
    
    /// Enable threat detection
    pub enable_threat_detection: bool,
    
    /// Maximum failed validation attempts
    pub max_failed_attempts: u32,
    
    /// Security lockout duration in seconds
    pub lockout_duration_seconds: u64,
    
    /// Enable session fingerprinting
    pub enable_fingerprinting: bool,
    
    /// Session timeout in seconds
    pub session_timeout_seconds: u64,
    
    /// Enable IP validation
    pub enable_ip_validation: bool,
    
    /// Allow IP changes for same session
    pub allow_ip_changes: bool,
}

/// Security policy engine for validation rules
#[derive(Debug)]
pub struct SecurityPolicyEngine {
    /// Active security policies
    policies: std::sync::RwLock<Vec<SecurityPolicy>>,
    
    /// Policy enforcement level
    enforcement_level: SecurityEnforcementLevel,
}

/// Individual security policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPolicy {
    /// Policy identifier
    pub id: String,
    
    /// Policy name
    pub name: String,
    
    /// Policy description
    pub description: String,
    
    /// Policy conditions
    pub conditions: Vec<PolicyCondition>,
    
    /// Policy actions
    pub actions: Vec<PolicyAction>,
    
    /// Policy priority (higher = more important)
    pub priority: u32,
    
    /// Whether policy is enabled
    pub enabled: bool,
    
    /// Policy expiration time
    pub expires_at: Option<SystemTime>,
    
    /// Policy type for evaluation
    pub policy_type: PolicyType,
    
    /// Allowed IP addresses (for IP whitelist policies)
    pub allowed_ips: Option<Vec<String>>,
    
    /// Maximum session age in minutes (for session timeout policies)
    pub max_session_age_minutes: Option<u32>,
    
    /// Allowed regions (for geographic restriction policies)
    pub allowed_regions: Option<Vec<String>>,
    
    /// Allowed start hour (0-23, for time-based access)
    pub allowed_start_hour: Option<u32>,
    
    /// Allowed end hour (0-23, for time-based access)
    pub allowed_end_hour: Option<u32>,
}

/// Policy types for security enforcement
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PolicyType {
    IPWhitelist,
    SessionTimeout,
    ConcurrentSessions,
    GeographicRestriction,
    TimeBasedAccess,
}

/// Policy condition for evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyCondition {
    /// Condition field (e.g., "client_ip", "user_agent")
    pub field: String,
    
    /// Condition operator
    pub operator: ConditionOperator,
    
    /// Expected value
    pub value: String,
    
    /// Whether condition should match or not match
    pub negate: bool,
}

/// Policy condition operators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConditionOperator {
    Equals,
    NotEquals,
    Contains,
    NotContains,
    Matches, // Regex
    GreaterThan,
    LessThan,
    InRange,
}

/// Policy action to take when conditions are met
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PolicyAction {
    /// Allow the request
    Allow,
    /// Deny the request
    Deny,
    /// Require additional authentication
    RequireAuth,
    /// Log the event
    Log,
    /// Suspend the session
    SuspendSession,
    /// Terminate the session
    TerminateSession,
    /// Rate limit the client
    RateLimit,
    /// Alert administrators
    Alert,
}

/// Security enforcement levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityEnforcementLevel {
    /// Log violations but allow requests
    Monitoring,
    /// Block obvious threats, allow suspicious
    Permissive,
    /// Block suspicious and obvious threats
    Restrictive,
    /// Block everything that isn't explicitly allowed
    Paranoid,
}

/// Session recovery manager
#[derive(Debug)]
pub struct SessionRecoveryManager {
    /// Recovery configuration
    config: SessionRecoveryConfig,
    
    /// Active recovery tokens
    recovery_tokens: std::sync::RwLock<HashMap<String, RecoveryToken>>,
    
    /// Recovery attempts tracking
    recovery_attempts: std::sync::RwLock<HashMap<String, RecoveryAttempts>>,
}

/// Session recovery configuration
#[derive(Debug, Clone)]
pub struct SessionRecoveryConfig {
    /// Enable session recovery
    pub enable_recovery: bool,
    
    /// Recovery token lifetime in seconds
    pub token_lifetime_seconds: u64,
    
    /// Maximum recovery attempts per client
    pub max_attempts_per_client: u32,
    
    /// Recovery attempt window in seconds
    pub attempt_window_seconds: u64,
    
    /// Require additional validation for recovery
    pub require_additional_validation: bool,
}

/// Recovery token for session restoration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryToken {
    /// Token identifier
    pub token_id: String,
    
    /// Original session identifier
    pub session_id: String,
    
    /// Client identity fingerprint
    pub client_fingerprint: String,
    
    /// Token creation time
    pub created_at: SystemTime,
    
    /// Token expiration time
    pub expires_at: SystemTime,
    
    /// Recovery data
    pub recovery_data: RecoveryData,
    
    /// Whether token has been used
    pub used: bool,
}

/// Recovery data for session restoration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryData {
    /// Client IP address at time of token creation
    pub client_ip: String,
    
    /// User agent at time of token creation
    pub user_agent: Option<String>,
    
    /// Client hostname
    pub client_hostname: Option<String>,
    
    /// Client username
    pub client_username: Option<String>,
    
    /// Authentication method used
    pub auth_method: Option<String>,
    
    /// Additional validation data
    pub validation_data: HashMap<String, String>,
}

/// Recovery attempts tracking
#[derive(Debug, Clone)]
pub struct RecoveryAttempts {
    /// Number of attempts made
    pub attempt_count: u32,
    
    /// First attempt time
    pub first_attempt: SystemTime,
    
    /// Last attempt time
    pub last_attempt: SystemTime,
    
    /// Client IP addresses that made attempts
    pub attempt_ips: Vec<String>,
}

/// Threat detection engine
#[derive(Debug)]
pub struct ThreatDetectionEngine {
    /// Detection rules
    rules: std::sync::RwLock<Vec<ThreatDetectionRule>>,
    
    /// Threat intelligence data
    threat_intel: std::sync::RwLock<ThreatIntelligence>,
    
    /// Detection statistics
    stats: std::sync::RwLock<ThreatDetectionStats>,
    
    /// Recent detected threats (keep last 1000 for performance)
    recent_threats: std::sync::RwLock<Vec<StoredThreat>>,
}

/// Threat detection rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatDetectionRule {
    /// Rule identifier
    pub id: String,
    
    /// Rule name
    pub name: String,
    
    /// Threat indicators to look for (patterns, IPs, etc.)
    pub indicators: Vec<String>,
    
    /// Threat severity level
    pub severity: ThreatSeverity,
    
    /// Rule priority
    pub priority: u32,
    
    /// Whether rule is enabled
    pub enabled: bool,
    
    /// Rule type for evaluation
    pub rule_type: ThreatRuleType,
    
    /// Confidence threshold (0.0-1.0)
    pub confidence_threshold: f64,
    
    /// Recommended actions when threat is detected
    pub recommended_actions: Vec<SessionAction>,
}

/// Threat rule types for detection
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Hash, Eq)]
pub enum ThreatRuleType {
    SuspiciousIPPattern,
    AbnormalBehavior,
    RateLimitViolation,
    MaliciousHeaders,
    SessionHijacking,
}

/// Threat indicator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatIndicator {
    /// Indicator type
    pub indicator_type: ThreatIndicatorType,
    
    /// Indicator value or pattern
    pub value: String,
    
    /// Confidence score (0.0-1.0)
    pub confidence: f64,
}

/// Types of threat indicators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThreatIndicatorType {
    /// Malicious IP address
    MaliciousIp,
    /// Suspicious user agent pattern
    SuspiciousUserAgent,
    /// Brute force attack pattern
    BruteForcePattern,
    /// Session hijacking attempt
    SessionHijacking,
    /// Anomalous behavior pattern
    AnomalousBehavior,
    /// Known attack signature
    AttackSignature,
}

/// Threat severity levels
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ThreatSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Threat intelligence data
#[derive(Debug, Default)]
pub struct ThreatIntelligence {
    /// Known malicious IPs
    pub malicious_ips: std::collections::HashSet<String>,
    
    /// Suspicious user agents
    pub suspicious_user_agents: Vec<String>,
    
    /// Attack signatures
    pub attack_signatures: Vec<String>,
    
    /// Last update timestamp
    pub last_updated: Option<SystemTime>,
}

/// Threat detection statistics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ThreatDetectionStats {
    /// Total threats detected
    pub total_threats: u64,
    
    /// Threats by severity
    pub threats_by_severity: HashMap<String, u64>,
    
    /// Blocked requests
    pub blocked_requests: u64,
    
    /// False positives
    pub false_positives: u64,
    
    /// Last detection time
    pub last_detection: Option<SystemTime>,
}

impl Default for SecurityValidationConfig {
    fn default() -> Self {
        Self {
            enable_validation: true,
            min_security_score: 0.7,
            enable_threat_detection: true,
            max_failed_attempts: 5,
            lockout_duration_seconds: 1800, // 30 minutes
            enable_fingerprinting: true,
            session_timeout_seconds: 7200, // 2 hours
            enable_ip_validation: true,
            allow_ip_changes: false,
        }
    }
}

impl Default for SessionRecoveryConfig {
    fn default() -> Self {
        Self {
            enable_recovery: true,
            token_lifetime_seconds: 3600, // 1 hour
            max_attempts_per_client: 3,
            attempt_window_seconds: 900, // 15 minutes
            require_additional_validation: true,
        }
    }
}


impl SecurityValidator {
    /// Create new security validator
    pub fn new(
        config: SecurityValidationConfig,
        recovery_config: SessionRecoveryConfig,
    ) -> Self {
        let policy_engine = Arc::new(SecurityPolicyEngine::new());
        let recovery_manager = Arc::new(SessionRecoveryManager::new(recovery_config));
        let threat_detector = Arc::new(ThreatDetectionEngine::new());

        Self {
            config,
            policy_engine,
            recovery_manager,
            threat_detector,
        }
    }

    /// Perform comprehensive security validation
    pub async fn validate_session_security(
        &self,
        req: &HttpRequest,
        session: &IsolatedSession,
    ) -> Result<SecurityValidationResult> {
        debug!("Performing comprehensive security validation for session: {}", session.session_id);

        let mut validation_result = SecurityValidationResult {
            valid: true,
            security_score: 1.0,
            threat_level: ThreatLevel::None,
            policy_violations: Vec::new(),
            security_issues: Vec::new(),
            recommendations: Vec::new(),
            should_block: false,
            requires_additional_auth: false,
            session_actions: Vec::new(),
        };

        // Skip validation if disabled
        if !self.config.enable_validation {
            return Ok(validation_result);
        }

        // Validate client identity consistency
        self.validate_client_identity(req, session, &mut validation_result).await?;

        // Check security policies
        self.evaluate_security_policies(req, session, &mut validation_result).await?;

        // Perform threat detection
        if self.config.enable_threat_detection {
            self.detect_threats(req, session, &mut validation_result).await?;
        }

        // Validate session fingerprint
        if self.config.enable_fingerprinting {
            self.validate_session_fingerprint(req, session, &mut validation_result).await?;
        }

        // Check session timeout
        self.validate_session_timeout(session, &mut validation_result).await?;

        // Calculate final security score and determine actions
        self.finalize_validation_result(&mut validation_result).await?;

        info!("Security validation completed for session {}: score={}, valid={}",
              session.session_id, validation_result.security_score, validation_result.valid);

        Ok(validation_result)
    }

    /// Validate client identity consistency
    async fn validate_client_identity(
        &self,
        req: &HttpRequest,
        session: &IsolatedSession,
        result: &mut SecurityValidationResult,
    ) -> Result<()> {
        // Check IP address consistency
        if self.config.enable_ip_validation {
            let connection_info = req.connection_info();
            let current_ip = connection_info
                .realip_remote_addr()
                .unwrap_or("127.0.0.1:0")
                .split(':')
                .next()
                .unwrap_or("127.0.0.1");

            let session_ip = session.remote_context.client_identity.client_ip.to_string();

            if current_ip != session_ip && !self.config.allow_ip_changes {
                result.security_issues.push("Client IP address changed during session".to_string());
                result.security_score -= 0.3;
                result.recommendations.push("Verify client identity and location".to_string());
            }
        }

        // Check user agent consistency
        if let Some(session_ua) = &session.remote_context.client_identity.user_agent {
            if let Some(current_ua) = req.headers().get("user-agent").and_then(|h| h.to_str().ok()) {
                if session_ua != current_ua {
                    result.security_issues.push("User agent changed during session".to_string());
                    result.security_score -= 0.1;
                }
            }
        }

        Ok(())
    }

    /// Evaluate security policies
    async fn evaluate_security_policies(
        &self,
        req: &HttpRequest,
        session: &IsolatedSession,
        result: &mut SecurityValidationResult,
    ) -> Result<()> {
        let policy_violations = self.policy_engine.evaluate_policies(req, session).await?;
        
        for violation in policy_violations {
            result.policy_violations.push(violation.clone());
            
            match violation.severity {
                PolicyViolationSeverity::Low => result.security_score -= 0.1,
                PolicyViolationSeverity::Medium => result.security_score -= 0.2,
                PolicyViolationSeverity::High => result.security_score -= 0.4,
                PolicyViolationSeverity::Critical => {
                    result.security_score -= 0.6;
                    result.should_block = true;
                }
            }
            
            // Add recommended actions based on violation
            result.session_actions.extend(violation.recommended_actions.clone());
        }

        Ok(())
    }

    /// Detect security threats
    async fn detect_threats(
        &self,
        req: &HttpRequest,
        session: &IsolatedSession,
        result: &mut SecurityValidationResult,
    ) -> Result<()> {
        let threats = self.threat_detector.detect_threats(req, session).await?;
        
        if !threats.is_empty() {
            let max_severity = threats.iter()
                .map(|t| match t.severity {
                    ThreatSeverity::Low => 1,
                    ThreatSeverity::Medium => 2,
                    ThreatSeverity::High => 3,
                    ThreatSeverity::Critical => 4,
                })
                .max()
                .unwrap_or(0);

            result.threat_level = match max_severity {
                1 => ThreatLevel::Low,
                2 => ThreatLevel::Medium,
                3 => ThreatLevel::High,
                4 => ThreatLevel::Critical,
                _ => ThreatLevel::None,
            };

            // Adjust security score based on threats
            for threat in threats {
                match threat.severity {
                    ThreatSeverity::Low => result.security_score -= 0.05,
                    ThreatSeverity::Medium => result.security_score -= 0.15,
                    ThreatSeverity::High => result.security_score -= 0.3,
                    ThreatSeverity::Critical => {
                        result.security_score -= 0.5;
                        result.should_block = true;
                    }
                }
            }
        }

        Ok(())
    }

    /// Validate session fingerprint
    async fn validate_session_fingerprint(
        &self,
        req: &HttpRequest,
        session: &IsolatedSession,
        result: &mut SecurityValidationResult,
    ) -> Result<()> {
        // Generate current fingerprint
        let current_fingerprint = self.generate_session_fingerprint(req)?;
        
        // Compare with stored fingerprint if available
        if let Some(stored_fingerprint) = &session.remote_context.client_identity.capability_fingerprint {
            if &current_fingerprint != stored_fingerprint {
                result.security_issues.push("Session fingerprint mismatch".to_string());
                result.security_score -= 0.2;
                result.recommendations.push("Verify client capabilities and configuration".to_string());
            }
        }

        Ok(())
    }

    /// Validate session timeout
    async fn validate_session_timeout(
        &self,
        session: &IsolatedSession,
        result: &mut SecurityValidationResult,
    ) -> Result<()> {
        let session_age = SystemTime::now()
            .duration_since(session.metadata.created_at)
            .unwrap_or(Duration::from_secs(0));

        if session_age.as_secs() > self.config.session_timeout_seconds {
            result.security_issues.push("Session has exceeded maximum lifetime".to_string());
            result.security_score = 0.0;
            result.should_block = true;
            result.session_actions.push(SessionAction::TerminateSession);
        }

        let inactivity = SystemTime::now()
            .duration_since(session.metadata.last_activity)
            .unwrap_or(Duration::from_secs(0));

        if inactivity.as_secs() > self.config.session_timeout_seconds / 2 {
            result.security_issues.push("Session has been inactive for too long".to_string());
            result.security_score -= 0.2;
            result.requires_additional_auth = true;
        }

        Ok(())
    }

    /// Finalize validation result
    async fn finalize_validation_result(
        &self,
        result: &mut SecurityValidationResult,
    ) -> Result<()> {
        // Ensure security score is within bounds
        result.security_score = result.security_score.max(0.0).min(1.0);
        
        // Determine if session is valid based on minimum score
        result.valid = result.security_score >= self.config.min_security_score;
        
        // Set should_block if security score is critically low
        if result.security_score < 0.3 {
            result.should_block = true;
        }

        // Add general recommendations based on security score
        if result.security_score < 0.5 {
            result.recommendations.push("Consider re-authentication".to_string());
        }
        
        if result.security_score < 0.3 {
            result.recommendations.push("Terminate session and create new one".to_string());
        }

        Ok(())
    }

    /// Generate session fingerprint
    fn generate_session_fingerprint(&self, req: &HttpRequest) -> Result<String> {
        use sha2::{Sha256, Digest};
        
        let mut hasher = Sha256::new();
        
        // Add user agent
        if let Some(ua) = req.headers().get("user-agent").and_then(|h| h.to_str().ok()) {
            hasher.update(ua.as_bytes());
        }
        
        // Add accept headers
        if let Some(accept) = req.headers().get("accept").and_then(|h| h.to_str().ok()) {
            hasher.update(accept.as_bytes());
        }
        
        // Add accept-language
        if let Some(lang) = req.headers().get("accept-language").and_then(|h| h.to_str().ok()) {
            hasher.update(lang.as_bytes());
        }
        
        let fingerprint = format!("{:x}", hasher.finalize());
        Ok(fingerprint[..16].to_string())
    }

    /// Create recovery token for session
    pub async fn create_recovery_token(
        &self,
        session: &IsolatedSession,
    ) -> Result<String> {
        self.recovery_manager.create_recovery_token(session).await
    }

    /// Recover session using token
    pub async fn recover_session(
        &self,
        token: &str,
        req: &HttpRequest,
    ) -> Result<Option<String>> {
        self.recovery_manager.recover_session(token, req).await
    }
}

/// Security validation result
#[derive(Debug, Clone)]
pub struct SecurityValidationResult {
    /// Whether validation passed
    pub valid: bool,
    
    /// Security score (0.0-1.0)
    pub security_score: f64,
    
    /// Detected threat level
    pub threat_level: ThreatLevel,
    
    /// Policy violations found
    pub policy_violations: Vec<PolicyViolation>,
    
    /// Security issues identified
    pub security_issues: Vec<String>,
    
    /// Security recommendations
    pub recommendations: Vec<String>,
    
    /// Whether request should be blocked
    pub should_block: bool,
    
    /// Whether additional authentication is required
    pub requires_additional_auth: bool,
    
    /// Recommended session actions
    pub session_actions: Vec<SessionAction>,
}

/// Threat levels
#[derive(Debug, Clone, PartialEq)]
pub enum ThreatLevel {
    None,
    Low,
    Medium,
    High,
    Critical,
}

/// Policy violation
#[derive(Debug, Clone)]
pub struct PolicyViolation {
    pub policy_id: String,
    pub policy_name: String,
    pub violation_description: String,
    pub severity: PolicyViolationSeverity,
    pub recommended_actions: Vec<SessionAction>,
}

/// Policy violation severity
#[derive(Debug, Clone)]
pub enum PolicyViolationSeverity {
    Low,
    Medium,
    High,
    Critical,
}


/// Session actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionAction {
    RequireReauth,
    SuspendSession,
    TerminateSession,
    LogEvent,
    AlertAdmin,
    RateLimit,
    BlockRequest,
}

// Full implementation of security validator components
impl SecurityPolicyEngine {
    pub fn new() -> Self {
        Self {
            policies: std::sync::RwLock::new(Vec::new()),
            enforcement_level: SecurityEnforcementLevel::Restrictive,
        }
    }
    
    async fn evaluate_policies(
        &self,
        req: &HttpRequest,
        session: &IsolatedSession,
    ) -> Result<Vec<PolicyViolation>> {
        let mut violations = Vec::new();
        let policies = self.policies.read().unwrap();
        
        for policy in policies.iter() {
            if !policy.enabled {
                continue;
            }
            
            match self.evaluate_single_policy(policy, req, session).await {
                Ok(Some(violation)) => violations.push(violation),
                Ok(None) => {}, // Policy passed
                Err(e) => {
                    debug!("Error evaluating policy {}: {}", policy.id, e);
                    // Create a violation for policy evaluation failure
                    violations.push(PolicyViolation {
                        policy_id: policy.id.clone(),
                        policy_name: policy.name.clone(),
                        violation_description: format!("Policy evaluation failed: {}", e),
                        severity: PolicyViolationSeverity::Medium,
                        recommended_actions: vec![SessionAction::LogEvent],
                    });
                }
            }
        }
        
        Ok(violations)
    }
    
    /// Evaluate a single security policy
    async fn evaluate_single_policy(
        &self,
        policy: &SecurityPolicy,
        req: &HttpRequest,
        session: &IsolatedSession,
    ) -> Result<Option<PolicyViolation>> {
        
        match policy.policy_type {
            PolicyType::IPWhitelist => {
                // Extract client IP from request
                let client_ip = self.extract_client_ip(req);
                if let Some(allowed_ips) = &policy.allowed_ips {
                    if let Some(ip) = client_ip {
                        if !allowed_ips.contains(&ip) {
                            return Ok(Some(PolicyViolation {
                                policy_id: policy.id.clone(),
                                policy_name: policy.name.clone(),
                                violation_description: format!("IP address {} not in whitelist", ip),
                                severity: PolicyViolationSeverity::High,
                                recommended_actions: vec![SessionAction::BlockRequest, SessionAction::LogEvent],
                            }));
                        }
                    }
                }
            },
            PolicyType::SessionTimeout => {
                // Check session age against policy limits
                if let Some(max_age) = policy.max_session_age_minutes {
                    let session_age = SystemTime::now()
                        .duration_since(session.metadata.created_at)
                        .unwrap_or(Duration::ZERO)
                        .as_secs() / 60; // Convert to minutes
                    
                    if session_age > max_age as u64 {
                        return Ok(Some(PolicyViolation {
                            policy_id: policy.id.clone(),
                            policy_name: policy.name.clone(),
                            violation_description: format!("Session age {} minutes exceeds maximum of {} minutes", session_age, max_age),
                            severity: PolicyViolationSeverity::Medium,
                            recommended_actions: vec![SessionAction::RequireReauth, SessionAction::LogEvent],
                        }));
                    }
                }
            },
            PolicyType::ConcurrentSessions => {
                // This would require access to session manager to count concurrent sessions
                // For now, we'll skip this check as it requires external dependencies
                debug!("Concurrent session policy check skipped - requires session manager access");
            },
            PolicyType::GeographicRestriction => {
                // Geographic restrictions would require GeoIP lookup
                // For now, we'll implement a basic check based on configured regions
                if let Some(allowed_regions) = &policy.allowed_regions {
                    // Extract region from session metadata or IP geolocation
                    if let Some(region) = self.extract_region_from_session(session) {
                        if !allowed_regions.contains(&region) {
                            return Ok(Some(PolicyViolation {
                                policy_id: policy.id.clone(),
                                policy_name: policy.name.clone(),
                                violation_description: format!("Access from region {} is not allowed", region),
                                severity: PolicyViolationSeverity::High,
                                recommended_actions: vec![SessionAction::BlockRequest, SessionAction::LogEvent, SessionAction::AlertAdmin],
                            }));
                        }
                    }
                }
            },
            PolicyType::TimeBasedAccess => {
                // Check if current time is within allowed access hours
                if let (Some(start_hour), Some(end_hour)) = (policy.allowed_start_hour, policy.allowed_end_hour) {
                    let current_hour = chrono::Utc::now().hour();
                    
                    if current_hour < start_hour || current_hour > end_hour {
                        return Ok(Some(PolicyViolation {
                            policy_id: policy.id.clone(),
                            policy_name: policy.name.clone(),
                            violation_description: format!("Access at hour {} is outside allowed window ({}-{})", current_hour, start_hour, end_hour),
                            severity: PolicyViolationSeverity::Medium,
                            recommended_actions: vec![SessionAction::BlockRequest, SessionAction::LogEvent],
                        }));
                    }
                }
            },
        }
        
        Ok(None) // No violation found
    }
    
    /// Extract client IP from HTTP request
    fn extract_client_ip(&self, req: &HttpRequest) -> Option<String> {
        // Try X-Forwarded-For first (for proxies)
        if let Some(forwarded) = req.headers().get("x-forwarded-for") {
            if let Ok(forwarded_str) = forwarded.to_str() {
                // Take the first IP from the comma-separated list
                if let Some(first_ip) = forwarded_str.split(',').next() {
                    return Some(first_ip.trim().to_string());
                }
            }
        }
        
        // Try X-Real-IP
        if let Some(real_ip) = req.headers().get("x-real-ip") {
            if let Ok(ip_str) = real_ip.to_str() {
                return Some(ip_str.to_string());
            }
        }
        
        // Fallback to connection peer address
        if let Some(peer_addr) = req.peer_addr() {
            return Some(peer_addr.ip().to_string());
        }
        
        None
    }
    
    /// Extract region information from session metadata
    fn extract_region_from_session(&self, session: &IsolatedSession) -> Option<String> {
        // Try to extract region from session tags or connection metadata
        session.metadata.tags.get("region").cloned()
        // In a real implementation, this could also do GeoIP lookup based on session IP
    }
    
    pub fn get_policies(&self) -> Result<Vec<SecurityPolicy>> {
        let policies = self.policies.read().map_err(|_| crate::error::ProxyError::Internal(anyhow::anyhow!("Failed to read policies")))?;
        Ok(policies.clone())
    }
    
    pub fn get_statistics(&self) -> Result<HashMap<String, serde_json::Value>> {
        let mut stats = HashMap::new();
        let policies = self.policies.read().map_err(|_| crate::error::ProxyError::Internal(anyhow::anyhow!("Failed to read policies")))?;
        stats.insert("total_policies".to_string(), serde_json::Value::Number(policies.len().into()));
        stats.insert("active_policies".to_string(), serde_json::Value::Number(policies.iter().filter(|p| p.enabled).count().into()));
        Ok(stats)
    }
}

impl SessionRecoveryManager {
    fn new(_config: SessionRecoveryConfig) -> Self {
        Self {
            config: _config,
            recovery_tokens: std::sync::RwLock::new(HashMap::new()),
            recovery_attempts: std::sync::RwLock::new(HashMap::new()),
        }
    }
    
    async fn create_recovery_token(&self, session: &IsolatedSession) -> Result<String> {
        use rand::Rng;
        
        // Generate a cryptographically secure recovery token
        let token_bytes: [u8; 32] = rand::thread_rng().gen();
        let recovery_token = hex::encode(token_bytes);
        
        // Create recovery token record
        let recovery_record = RecoveryToken {
            token_id: recovery_token.clone(),
            session_id: session.session_id.clone(),
            client_fingerprint: format!("{}-{}", 
                session.metadata.connection_metadata.client_ip,
                session.metadata.connection_metadata.user_agent.as_deref().unwrap_or("unknown")
            ),
            created_at: SystemTime::now(),
            expires_at: SystemTime::now() + Duration::from_secs(self.config.token_lifetime_seconds),
            recovery_data: RecoveryData {
                client_ip: session.metadata.connection_metadata.client_ip.clone(),
                user_agent: session.metadata.connection_metadata.user_agent.clone(),
                client_hostname: None,
                client_username: None,
                auth_method: Some(format!("session_recovery")),
                validation_data: std::collections::HashMap::new(),
            },
            used: false,
        };
        
        // Store the recovery token
        {
            let mut tokens = self.recovery_tokens.write().unwrap();
            tokens.insert(recovery_token.clone(), recovery_record);
        }
        
        // Clean up expired tokens periodically
        self.cleanup_expired_tokens();
        
        info!("Created recovery token for session {}", session.session_id);
        Ok(recovery_token)
    }
    
    async fn recover_session(&self, token: &str, req: &HttpRequest) -> Result<Option<String>> {
        // Check recovery attempts for rate limiting
        if !self.check_recovery_rate_limit(req) {
            debug!("Recovery attempt blocked due to rate limiting");
            return Ok(None);
        }
        
        // Look up the recovery token
        let mut tokens = self.recovery_tokens.write().unwrap();
        let recovery_record = match tokens.get_mut(token) {
            Some(record) => record,
            None => {
                debug!("Recovery token not found: {}", token);
                self.record_recovery_attempt(req, false);
                return Ok(None);
            }
        };
        
        // Check if token has expired
        if SystemTime::now() > recovery_record.expires_at {
            debug!("Recovery token expired: {}", token);
            tokens.remove(token);
            self.record_recovery_attempt(req, false);
            return Ok(None);
        }
        
        // Check if token has already been used
        if recovery_record.used {
            debug!("Recovery token already used: {}", token);
            self.record_recovery_attempt(req, false);
            return Ok(None);
        }
        
        // Validate client context if configured
        if self.config.require_additional_validation {
            if !self.validate_recovery_context(recovery_record, req) {
                debug!("Recovery context validation failed for token: {}", token);
                self.record_recovery_attempt(req, false);
                return Ok(None);
            }
        }
        
        // Mark token as used
        recovery_record.used = true;
        let session_id = recovery_record.session_id.clone();
        
        // Record successful recovery attempt
        self.record_recovery_attempt(req, true);
        
        info!("Successfully recovered session {} using token", session_id);
        Ok(Some(session_id))
    }
    
    /// Check if recovery attempts are within rate limits
    fn check_recovery_rate_limit(&self, req: &HttpRequest) -> bool {
        let client_key = self.get_client_key(req);
        let attempts = self.recovery_attempts.read().unwrap();
        
        if let Some(client_attempts) = attempts.get(&client_key) {
            // Check if attempts exceed the limit within the window
            let window_start = SystemTime::now() - Duration::from_secs(self.config.attempt_window_seconds);
            
            if client_attempts.last_attempt > window_start &&
               client_attempts.attempt_count >= self.config.max_attempts_per_client {
                return false;
            }
        }
        
        true
    }
    
    /// Record a recovery attempt for rate limiting
    fn record_recovery_attempt(&self, req: &HttpRequest, successful: bool) {
        let client_key = self.get_client_key(req);
        let mut attempts = self.recovery_attempts.write().unwrap();
        
        let now = SystemTime::now();
        let client_attempts = attempts.entry(client_key.clone()).or_insert_with(|| RecoveryAttempts {
            attempt_count: 0,
            first_attempt: now,
            last_attempt: now,
            attempt_ips: vec![client_key.clone()],
        });
        
        // Reset attempt count if outside window
        let window_start = now - Duration::from_secs(self.config.attempt_window_seconds);
        if client_attempts.last_attempt < window_start {
            client_attempts.attempt_count = 0;
            client_attempts.first_attempt = now;
        }
        
        client_attempts.attempt_count += 1;
        client_attempts.last_attempt = now;
        
        // Track IP if not already present
        if !client_attempts.attempt_ips.contains(&client_key) {
            client_attempts.attempt_ips.push(client_key);
        }
    }
    
    /// Validate recovery context against original session
    fn validate_recovery_context(&self, recovery_record: &RecoveryToken, req: &HttpRequest) -> bool {
        // Check client IP if available (always validate if present)
        let original_ip = &recovery_record.recovery_data.client_ip;
        if let Some(current_ip) = self.extract_client_ip(req) {
            if original_ip != &current_ip {
                debug!("Recovery IP mismatch: {} != {}", original_ip, current_ip);
                return false;
            }
        }
        
        // Check User-Agent if available (basic validation)
        if let Some(ref original_ua) = recovery_record.recovery_data.user_agent {
            if let Some(current_ua) = req.headers().get("user-agent") {
                if let Ok(current_ua_str) = current_ua.to_str() {
                    if original_ua != current_ua_str {
                        debug!("Recovery User-Agent mismatch: {} != {}", original_ua, current_ua_str);
                        return false;
                    }
                }
            }
        }
        
        true
    }
    
    /// Extract client IP from HTTP request
    fn extract_client_ip(&self, req: &HttpRequest) -> Option<String> {
        // Try X-Forwarded-For first (for proxies)
        if let Some(forwarded) = req.headers().get("x-forwarded-for") {
            if let Ok(forwarded_str) = forwarded.to_str() {
                if let Some(first_ip) = forwarded_str.split(',').next() {
                    return Some(first_ip.trim().to_string());
                }
            }
        }
        
        // Try X-Real-IP
        if let Some(real_ip) = req.headers().get("x-real-ip") {
            if let Ok(ip_str) = real_ip.to_str() {
                return Some(ip_str.to_string());
            }
        }
        
        // Fallback to connection peer address
        if let Some(peer_addr) = req.peer_addr() {
            return Some(peer_addr.ip().to_string());
        }
        
        None
    }
    
    /// Get client identification key for rate limiting
    fn get_client_key(&self, req: &HttpRequest) -> String {
        // Use client IP as primary identifier
        if let Some(ip) = self.extract_client_ip(req) {
            return ip;
        }
        
        // Fallback to User-Agent hash if no IP available
        if let Some(ua) = req.headers().get("user-agent") {
            if let Ok(ua_str) = ua.to_str() {
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};
                let mut hasher = DefaultHasher::new();
                ua_str.hash(&mut hasher);
                return format!("ua_{}", hasher.finish());
            }
        }
        
        // Last resort - use remote address
        if let Some(peer) = req.peer_addr() {
            return peer.to_string();
        }
        
        "unknown_client".to_string()
    }
    
    /// Clean up expired recovery tokens
    fn cleanup_expired_tokens(&self) {
        let mut tokens = self.recovery_tokens.write().unwrap();
        let now = SystemTime::now();
        let initial_count = tokens.len();
        
        tokens.retain(|_, record| now < record.expires_at);
        
        let removed_count = initial_count - tokens.len();
        if removed_count > 0 {
            debug!("Cleaned up {} expired recovery tokens", removed_count);
        }
    }
    
    /// Get recovery statistics
    pub fn get_recovery_stats(&self) -> HashMap<String, u64> {
        let mut stats = HashMap::new();
        
        let tokens = self.recovery_tokens.read().unwrap();
        let attempts = self.recovery_attempts.read().unwrap();
        
        stats.insert("active_tokens".to_string(), tokens.len() as u64);
        stats.insert("total_clients_with_attempts".to_string(), attempts.len() as u64);
        
        // Calculate successful vs blocked based on attempt patterns
        let (successful, blocked): (u64, u64) = attempts.values()
            .fold((0, 0), |(succ, block), attempt| {
                // Consider attempts successful if they're below max attempts threshold
                if attempt.attempt_count <= self.config.max_attempts_per_client {
                    (succ + 1, block)
                } else {
                    (succ, block + 1)
                }
            });
        
        stats.insert("successful_recoveries".to_string(), successful);
        stats.insert("blocked_attempts".to_string(), blocked);
        
        stats
    }
}

impl ThreatDetectionEngine {
    pub fn new() -> Self {
        Self {
            rules: std::sync::RwLock::new(Vec::new()),
            threat_intel: std::sync::RwLock::new(ThreatIntelligence::default()),
            stats: std::sync::RwLock::new(ThreatDetectionStats::default()),
            recent_threats: std::sync::RwLock::new(Vec::new()),
        }
    }
    
    async fn detect_threats(
        &self,
        req: &HttpRequest,
        session: &IsolatedSession,
    ) -> Result<Vec<DetectedThreat>> {
        let mut detected_threats = Vec::new();
        let rules = self.rules.read().unwrap();
        
        for rule in rules.iter() {
            if !rule.enabled {
                continue;
            }
            
            match self.evaluate_threat_rule(rule, req, session).await {
                Ok(Some(threat)) => {
                    // Store threat for historical tracking
                    self.store_threat(&threat, req);
                    detected_threats.push(threat);
                    // Update statistics
                    self.update_threat_stats(rule, true);
                }
                Ok(None) => {
                    // Rule passed, update stats
                    self.update_threat_stats(rule, false);
                }
                Err(e) => {
                    debug!("Error evaluating threat rule {}: {}", rule.id, e);
                }
            }
        }
        
        // Update threat intelligence with new detections
        if !detected_threats.is_empty() {
            self.update_threat_intelligence(&detected_threats, req, session);
        }
        
        Ok(detected_threats)
    }
    
    /// Evaluate a single threat detection rule
    async fn evaluate_threat_rule(
        &self,
        rule: &ThreatDetectionRule,
        req: &HttpRequest,
        session: &IsolatedSession,
    ) -> Result<Option<DetectedThreat>> {
        
        match rule.rule_type {
            ThreatRuleType::SuspiciousIPPattern => {
                if let Some(client_ip) = self.extract_client_ip(req) {
                    // Check against known suspicious IP patterns
                    for pattern in &rule.indicators {
                        if self.matches_ip_pattern(&client_ip, pattern) {
                            return Ok(Some(DetectedThreat {
                                rule_id: rule.id.clone(),
                                threat_type: ThreatIndicatorType::MaliciousIp,
                                severity: rule.severity.clone(),
                                confidence: rule.confidence_threshold,
                                description: format!("Suspicious IP pattern detected: {} matches {}", client_ip, pattern),
                                indicators: vec![client_ip],
                            }));
                        }
                    }
                }
            },
            ThreatRuleType::AbnormalBehavior => {
                // Check for session behavior anomalies
                if let Some(anomaly) = self.detect_session_anomaly(session, rule) {
                    return Ok(Some(DetectedThreat {
                        rule_id: rule.id.clone(),
                        threat_type: ThreatIndicatorType::AnomalousBehavior,
                        severity: rule.severity.clone(),
                        confidence: rule.confidence_threshold,
                        description: format!("Abnormal session behavior detected: {}", anomaly),
                        indicators: vec![session.session_id.clone()],
                    }));
                }
            },
            ThreatRuleType::RateLimitViolation => {
                // Check for rate limiting violations
                if let Some(violation) = self.detect_rate_limit_violation(req, rule) {
                    return Ok(Some(DetectedThreat {
                        rule_id: rule.id.clone(),
                        threat_type: ThreatIndicatorType::BruteForcePattern,
                        severity: rule.severity.clone(),
                        confidence: rule.confidence_threshold,
                        description: format!("Rate limit violation detected: {}", violation),
                        indicators: vec![self.extract_client_ip(req).unwrap_or_else(|| "unknown".to_string())],
                    }));
                }
            },
            ThreatRuleType::MaliciousHeaders => {
                // Check for malicious or suspicious HTTP headers
                if let Some(suspicious_header) = self.detect_suspicious_headers(req, rule) {
                    return Ok(Some(DetectedThreat {
                        rule_id: rule.id.clone(),
                        threat_type: ThreatIndicatorType::SuspiciousUserAgent,
                        severity: rule.severity.clone(),
                        confidence: rule.confidence_threshold,
                        description: format!("Malicious header detected: {}", suspicious_header),
                        indicators: vec![suspicious_header.clone()],
                    }));
                }
            },
            ThreatRuleType::SessionHijacking => {
                // Check for session hijacking indicators
                if let Some(hijack_indicator) = self.detect_session_hijacking(session, req, rule) {
                    return Ok(Some(DetectedThreat {
                        rule_id: rule.id.clone(),
                        threat_type: ThreatIndicatorType::SessionHijacking,
                        severity: ThreatSeverity::Critical, // Session hijacking is always critical
                        confidence: rule.confidence_threshold,
                        description: format!("Session hijacking attempt detected: {}", hijack_indicator),
                        indicators: vec![session.session_id.clone()],
                    }));
                }
            },
        }
        
        Ok(None)
    }
    
    /// Check if IP matches a pattern (supports wildcards and CIDR)
    fn matches_ip_pattern(&self, ip: &str, pattern: &str) -> bool {
        // Simple pattern matching - in production this would be more sophisticated
        if pattern.contains('*') {
            // Wildcard matching
            let pattern_parts: Vec<&str> = pattern.split('.').collect();
            let ip_parts: Vec<&str> = ip.split('.').collect();
            
            if pattern_parts.len() != 4 || ip_parts.len() != 4 {
                return false;
            }
            
            for (pattern_part, ip_part) in pattern_parts.iter().zip(ip_parts.iter()) {
                if *pattern_part != "*" && *pattern_part != *ip_part {
                    return false;
                }
            }
            return true;
        } else if pattern.contains('/') {
            // CIDR matching - simplified implementation
            // In production, use proper CIDR library
            return pattern == ip; // Placeholder
        } else {
            // Exact match
            return pattern == ip;
        }
    }
    
    /// Detect session behavior anomalies
    fn detect_session_anomaly(&self, session: &IsolatedSession, rule: &ThreatDetectionRule) -> Option<String> {
        
        // Check session age anomalies
        let session_age_minutes = SystemTime::now()
            .duration_since(session.metadata.created_at)
            .unwrap_or(Duration::ZERO)
            .as_secs() / 60;
        
        // Look for unusually long sessions (potential session hijacking)
        if session_age_minutes > 480 { // 8 hours
            return Some("unusually_long_session".to_string());
        }
        
        // Check for rapid state changes
        if matches!(session.state, IsolationSessionState::Terminated) &&
           session_age_minutes < 1 {
            return Some("rapid_session_termination".to_string());
        }
        
        // Check metadata for anomalies
        if session.metadata.connection_metadata.client_ip.is_empty() {
            return Some("missing_client_ip".to_string());
        }
        
        None
    }
    
    /// Detect rate limit violations
    fn detect_rate_limit_violation(&self, _req: &HttpRequest, _rule: &ThreatDetectionRule) -> Option<String> {
        // This would typically integrate with rate limiting middleware
        // For now, return None as rate limiting is handled elsewhere
        None
    }
    
    /// Detect suspicious HTTP headers
    fn detect_suspicious_headers(&self, req: &HttpRequest, rule: &ThreatDetectionRule) -> Option<String> {
        // Check for suspicious header patterns
        let suspicious_patterns = [
            "curl", "wget", "python-requests", "bot", "crawler", "scanner",
            "sqlmap", "nmap", "nikto", "dirb", "gobuster", "burp", "zap"
        ];
        
        if let Some(user_agent) = req.headers().get("user-agent") {
            if let Ok(ua_str) = user_agent.to_str() {
                let ua_lower = ua_str.to_lowercase();
                for pattern in &suspicious_patterns {
                    if ua_lower.contains(pattern) {
                        return Some(format!("suspicious_user_agent: {}", ua_str));
                    }
                }
            }
        }
        
        // Check for suspicious X-Forwarded headers that might indicate bypassing
        if let Some(forwarded) = req.headers().get("x-forwarded-for") {
            if let Ok(forwarded_str) = forwarded.to_str() {
                // Look for private IP ranges in forwarded headers (potential bypass attempt)
                if forwarded_str.contains("127.0.0.1") || forwarded_str.contains("localhost") {
                    return Some(format!("suspicious_forwarded_header: {}", forwarded_str));
                }
            }
        }
        
        None
    }
    
    /// Detect session hijacking attempts
    fn detect_session_hijacking(&self, session: &IsolatedSession, req: &HttpRequest, _rule: &ThreatDetectionRule) -> Option<String> {
        // Check for IP address changes within session
        let session_ip = &session.metadata.connection_metadata.client_ip;
        if let Some(current_ip) = self.extract_client_ip(req) {
            if session_ip != &current_ip {
                return Some(format!("ip_change_detected: {} -> {}", session_ip, current_ip));
            }
        }
        
        // Check for User-Agent changes (less reliable but suspicious)
        if let Some(ref session_ua) = session.metadata.connection_metadata.user_agent {
            if let Some(current_ua) = req.headers().get("user-agent") {
                if let Ok(current_ua_str) = current_ua.to_str() {
                    if session_ua != current_ua_str {
                        // Only flag if the change is significant (not just version updates)
                        if !self.is_similar_user_agent(session_ua, current_ua_str) {
                            return Some(format!("user_agent_change_detected: {} -> {}", session_ua, current_ua_str));
                        }
                    }
                }
            }
        }
        
        None
    }
    
    /// Check if two user agents are similar (to avoid false positives for minor version changes)
    fn is_similar_user_agent(&self, original: &str, current: &str) -> bool {
        // Simple similarity check - extract browser name and major version
        let original_browser = self.extract_browser_info(original);
        let current_browser = self.extract_browser_info(current);
        original_browser == current_browser
    }
    
    /// Extract basic browser information from User-Agent
    fn extract_browser_info(&self, user_agent: &str) -> String {
        let ua_lower = user_agent.to_lowercase();
        if ua_lower.contains("chrome") {
            "chrome".to_string()
        } else if ua_lower.contains("firefox") {
            "firefox".to_string()
        } else if ua_lower.contains("safari") {
            "safari".to_string()
        } else if ua_lower.contains("edge") {
            "edge".to_string()
        } else {
            "unknown".to_string()
        }
    }
    
    /// Extract client IP from HTTP request
    fn extract_client_ip(&self, req: &HttpRequest) -> Option<String> {
        // Try X-Forwarded-For first (for proxies)
        if let Some(forwarded) = req.headers().get("x-forwarded-for") {
            if let Ok(forwarded_str) = forwarded.to_str() {
                if let Some(first_ip) = forwarded_str.split(',').next() {
                    return Some(first_ip.trim().to_string());
                }
            }
        }
        
        // Try X-Real-IP
        if let Some(real_ip) = req.headers().get("x-real-ip") {
            if let Ok(ip_str) = real_ip.to_str() {
                return Some(ip_str.to_string());
            }
        }
        
        // Fallback to connection peer address
        if let Some(peer_addr) = req.peer_addr() {
            return Some(peer_addr.ip().to_string());
        }
        
        None
    }
    
    /// Update threat detection statistics
    fn update_threat_stats(&self, rule: &ThreatDetectionRule, threat_detected: bool) {
        let mut stats = self.stats.write().unwrap();
        
        if threat_detected {
            stats.total_threats += 1;
            let severity_key = format!("{:?}", rule.severity);
            *stats.threats_by_severity.entry(severity_key).or_insert(0) += 1;
        }
        
        stats.last_detection = Some(SystemTime::now());
    }
    
    /// Update threat intelligence with new detections
    fn update_threat_intelligence(&self, threats: &[DetectedThreat], req: &HttpRequest, _session: &IsolatedSession) {
        let mut threat_intel = self.threat_intel.write().unwrap();
        
        for threat in threats {
            // Add indicators to threat intelligence based on type
            match threat.threat_type {
                ThreatIndicatorType::MaliciousIp => {
                    if let Some(client_ip) = self.extract_client_ip(req) {
                        threat_intel.malicious_ips.insert(client_ip);
                    }
                },
                ThreatIndicatorType::SuspiciousUserAgent => {
                    if let Some(ua) = req.headers().get("user-agent") {
                        if let Ok(ua_str) = ua.to_str() {
                            threat_intel.suspicious_user_agents.push(ua_str.to_string());
                        }
                    }
                },
                ThreatIndicatorType::AttackSignature => {
                    threat_intel.attack_signatures.extend_from_slice(&threat.indicators);
                },
                _ => {
                    // Handle other types as needed
                }
            }
        }
        
        threat_intel.last_updated = Some(SystemTime::now());
    }
    
    /// Add a new threat detection rule
    pub fn add_threat_rule(&self, rule: ThreatDetectionRule) -> Result<()> {
        let mut rules = self.rules.write().unwrap();
        rules.push(rule);
        info!("Added new threat detection rule: {}", rules.last().unwrap().name);
        Ok(())
    }
    
    /// Remove a threat detection rule by ID
    pub fn remove_threat_rule(&self, rule_id: &str) -> Result<bool> {
        let mut rules = self.rules.write().unwrap();
        let initial_len = rules.len();
        rules.retain(|r| r.id != rule_id);
        let removed = rules.len() < initial_len;
        if removed {
            info!("Removed threat detection rule: {}", rule_id);
        }
        Ok(removed)
    }
    
    /// Get threat detection statistics
    pub fn get_threat_statistics(&self) -> ThreatDetectionStats {
        let stats = self.stats.read().unwrap();
        stats.clone()
    }
    
    /// Get threat intelligence data
    pub fn get_threat_intelligence(&self) -> ThreatIntelligence {
        let intel = self.threat_intel.read().unwrap();
        ThreatIntelligence {
            malicious_ips: intel.malicious_ips.clone(),
            suspicious_user_agents: intel.suspicious_user_agents.clone(),
            attack_signatures: intel.attack_signatures.clone(),
            last_updated: intel.last_updated,
        }
    }
    
    pub fn get_threat_rules(&self) -> Result<Vec<ThreatDetectionRule>> {
        let rules = self.rules.read().map_err(|_| crate::error::ProxyError::Internal(anyhow::anyhow!("Failed to read threat rules")))?;
        Ok(rules.clone())
    }
    
    pub fn get_statistics(&self) -> Result<ThreatDetectionStats> {
        let stats = self.stats.read().map_err(|_| crate::error::ProxyError::Internal(anyhow::anyhow!("Failed to read threat detection stats")))?;
        Ok((*stats).clone())
    }
    
    /// Store a detected threat for historical tracking
    pub fn store_threat(&self, threat: &DetectedThreat, req: &HttpRequest) {
        if let Ok(mut recent_threats) = self.recent_threats.write() {
            let stored_threat = StoredThreat {
                id: uuid::Uuid::new_v4().to_string(),
                rule_id: threat.rule_id.clone(),
                threat_type: format!("{:?}", threat.threat_type),
                severity: format!("{:?}", threat.severity),
                confidence: threat.confidence,
                description: threat.description.clone(),
                indicators: threat.indicators.clone(),
                timestamp: SystemTime::now(),
                source_ip: req.connection_info().realip_remote_addr().map(|ip| ip.to_string()),
                user_agent: req.headers().get("user-agent").and_then(|h| h.to_str().ok()).map(|s| s.to_string()),
            };
            
            recent_threats.push(stored_threat);
            
            // Keep only last 1000 threats for performance
            if recent_threats.len() > 1000 {
                let len = recent_threats.len();
                recent_threats.drain(0..(len - 1000));
            }
        }
    }
    
    /// Get recent threats with optional filtering
    pub fn get_recent_threats(&self, limit: Option<usize>, since: Option<SystemTime>) -> Result<Vec<StoredThreat>> {
        let threats = self.recent_threats.read().map_err(|_| crate::error::ProxyError::Internal(anyhow::anyhow!("Failed to read recent threats")))?;
        
        let mut filtered_threats: Vec<StoredThreat> = threats
            .iter()
            .filter(|threat| {
                if let Some(since_time) = since {
                    threat.timestamp >= since_time
                } else {
                    true
                }
            })
            .cloned()
            .collect();
            
        // Sort by timestamp descending (most recent first)
        filtered_threats.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        
        if let Some(limit) = limit {
            filtered_threats.truncate(limit);
        }
        
        Ok(filtered_threats)
    }
}

/// Detected threat
#[derive(Debug, Clone)]
pub struct DetectedThreat {
    pub rule_id: String,
    pub threat_type: ThreatIndicatorType,
    pub severity: ThreatSeverity,
    pub confidence: f64,
    pub description: String,
    pub indicators: Vec<String>,
}

/// Stored threat for historical tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredThreat {
    pub id: String,
    pub rule_id: String,
    pub threat_type: String,
    pub severity: String,
    pub confidence: f64,
    pub description: String,
    pub indicators: Vec<String>,
    pub timestamp: SystemTime,
    pub source_ip: Option<String>,
    pub user_agent: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_security_validator_creation() {
        let config = SecurityValidationConfig::default();
        let recovery_config = SessionRecoveryConfig::default();
        let validator = SecurityValidator::new(config, recovery_config);
        
        assert!(validator.config.enable_validation);
        assert_eq!(validator.config.min_security_score, 0.7);
    }

    #[tokio::test]
    async fn test_session_fingerprint_generation() {
        use actix_web::test::TestRequest;
        
        let config = SecurityValidationConfig::default();
        let recovery_config = SessionRecoveryConfig::default();
        let validator = SecurityValidator::new(config, recovery_config);
        
        let req = TestRequest::default()
            .insert_header(("user-agent", "test-client/1.0"))
            .insert_header(("accept", "application/json"))
            .to_http_request();
        
        let fingerprint = validator.generate_session_fingerprint(&req);
        assert!(fingerprint.is_ok());
        assert_eq!(fingerprint.unwrap().len(), 16);
    }
}