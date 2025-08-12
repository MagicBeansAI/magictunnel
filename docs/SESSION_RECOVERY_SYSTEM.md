# OAuth 2.1 Phase 2.3: Automatic Session Recovery System

## Overview

The Automatic Session Recovery system is the third phase of MagicTunnel's OAuth 2.1 implementation, building on the **User Context System (Phase 2.1)** and **Multi-Platform Token Storage (Phase 2.2)** to provide seamless authentication persistence across application restarts and runtime sessions.

## Architecture

### Core Components

```
┌─────────────────────────────────────────────────────────────────┐
│                    Session Recovery System                      │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────────┐    ┌──────────────────┐                   │
│  │ SessionManager  │    │  SessionState    │                   │
│  │                 │    │                  │                   │
│  │ - Token         │◄───┤ - ActiveSessions │                   │
│  │   Validation    │    │ - RecoveryAttempts│                   │
│  │ - Recovery      │    │ - FailedProviders│                   │
│  │   Workflow      │    │ - SystemID       │                   │
│  │ - Persistence   │    │                  │                   │
│  └─────────────────┘    └──────────────────┘                   │
│           │                        │                           │
│           │              ┌──────────────────┐                  │
│           └──────────────┤ ActiveSession    │                  │
│                          │                  │                  │
│                          │ - Provider       │                  │
│                          │ - UserID         │                  │
│                          │ - ExpiresAt      │                  │
│                          │ - LastValidated  │                  │
│                          │ - AuthMethod     │                  │
│                          │ - ValidationData │                  │
│                          └──────────────────┘                  │
└─────────────────────────────────────────────────────────────────┘
                             │                 │
                             ▼                 ▼
    ┌─────────────────────────────────────────────────────────────┐
    │              Phase 2.1 & 2.2 Integration                   │
    ├─────────────────────────────────────────────────────────────┤
    │                                                             │
    │  User Context System          Multi-Platform Token Storage │
    │  ┌─────────────────┐          ┌─────────────────────────┐   │
    │  │ - Cross-platform│          │ - macOS Keychain       │   │
    │  │   User ID       │          │ - Windows Credential   │   │
    │  │ - Session Dir   │          │   Manager               │   │
    │  │ - Secure Storage│          │ - Linux Secret Service │   │
    │  │ - Hostname      │          │ - Encrypted Filesystem │   │
    │  │   Isolation     │          │   Fallback              │   │
    │  └─────────────────┘          └─────────────────────────┘   │
    └─────────────────────────────────────────────────────────────┘
```

### Key Features

#### 1. **Startup Session Recovery**
- Automatically restores valid authentication sessions on application start
- Validates stored tokens with OAuth providers using userinfo endpoints
- Creates active sessions for valid tokens
- Gracefully handles expired or invalid tokens

#### 2. **Runtime Session Validation**
- Periodic validation of active sessions based on configurable intervals
- Background token refresh and validation
- Proactive detection of token expiration or revocation
- Automatic cleanup of invalid sessions

#### 3. **Multi-Provider Support**
- GitHub, Google, Microsoft, and custom OAuth 2.1 providers
- Provider-specific userinfo endpoint validation
- Per-provider failure tracking and retry logic
- Configurable timeout and rate limiting

#### 4. **Session State Persistence**
- Cross-restart session state preservation
- System compatibility detection (hostname, user, platform)
- Atomic session state updates
- Secure session file storage with proper permissions

#### 5. **Graceful Degradation**
- Continues operation when token validation fails
- Marks sessions as invalid rather than crashing
- Provides detailed error information for debugging
- Fallback to manual re-authentication when needed

## Implementation Details

### Session Manager (`SessionManager`)

The central component that orchestrates session recovery and validation:

```rust
pub struct SessionManager {
    user_context: UserContext,              // Phase 2.1 integration
    token_storage: Arc<TokenStorage>,       // Phase 2.2 integration
    session_state: Arc<RwLock<SessionState>>, // Thread-safe state
    recovery_config: SessionRecoveryConfig,  // Behavior configuration
    http_client: Client,                    // For token validation
    oauth_providers: HashMap<String, OAuthProviderConfig>, // Provider configs
    auth_resolver: Option<Arc<AuthResolver>>, // Auth system integration
}
```

**Key Methods:**
- `recover_sessions_on_startup()` - Perform startup recovery
- `validate_sessions_runtime()` - Runtime validation cycle
- `has_active_session()` - Check session availability
- `get_active_session()` - Retrieve session data
- `clear_all_sessions()` - Clean session state

### Active Session (`ActiveSession`)

Represents an individual authenticated session:

```rust
pub struct ActiveSession {
    pub provider: String,                    // OAuth provider (e.g., "github")
    pub user_id: String,                     // User identifier
    pub expires_at: Option<SystemTime>,      // Token expiration
    pub last_validated: SystemTime,          // Last validation check
    pub authentication_method: AuthMethodType, // Auth method used
    pub created_at: SystemTime,              // Session creation time
    pub validation_attempts: u32,            // Validation attempt count
    pub is_valid: bool,                      // Current validity status
    pub last_error: Option<String>,          // Last validation error
    pub scopes: Vec<String>,                 // OAuth scopes
}
```

**Session Lifecycle:**
1. **Creation** - From valid token during recovery
2. **Validation** - Periodic checks with OAuth provider
3. **Expiration** - Automatic detection and cleanup
4. **Removal** - Manual or automatic session cleanup

### Session State (`SessionState`)

System-wide session management state:

```rust
pub struct SessionState {
    pub active_sessions: HashMap<String, ActiveSession>, // All active sessions
    pub last_recovery_check: SystemTime,                 // Last recovery time
    pub recovery_attempts: u32,                          // Total attempts
    pub failed_providers: HashSet<String>,               // Failed providers
    pub version: u32,                                    // State version
    pub system_id: String,                              // System identifier
}
```

**State Management:**
- **System Compatibility** - Detects environment changes
- **Cleanup Operations** - Removes expired sessions
- **Statistics** - Provides session metrics
- **Persistence** - Saves/loads from filesystem

### Configuration (`SessionRecoveryConfig`)

Comprehensive configuration for session recovery behavior:

```rust
pub struct SessionRecoveryConfig {
    pub enabled: bool,                          // Enable/disable recovery
    pub auto_recovery_on_startup: bool,         // Startup recovery
    pub validation_interval_minutes: u64,       // Validation frequency
    pub max_recovery_attempts: u32,             // Max retry attempts
    pub token_validation_timeout_seconds: u64,  // HTTP timeout
    pub graceful_degradation: bool,             // Error handling mode
    pub retry_failed_providers: bool,           // Retry failed providers
    pub cleanup_expired_sessions: bool,         // Auto cleanup
    pub max_session_age_hours: u64,            // Force revalidation
    pub persist_session_state: bool,           // State persistence
}
```

## Usage Examples

### Basic Setup

```rust
use magictunnel::auth::{
    SessionManager, SessionRecoveryConfig,
    UserContext, TokenStorage
};
use std::sync::Arc;

// Create user context (Phase 2.1)
let user_context = UserContext::new()?;

// Create token storage (Phase 2.2)
let token_storage = Arc::new(TokenStorage::new(user_context.clone()).await?);

// Configure session recovery
let config = SessionRecoveryConfig {
    enabled: true,
    auto_recovery_on_startup: true,
    validation_interval_minutes: 60,
    max_recovery_attempts: 3,
    token_validation_timeout_seconds: 30,
    graceful_degradation: true,
    ..Default::default()
};

// Create session manager
let session_manager = SessionManager::new(
    user_context,
    token_storage,
    config,
).await?;

// Set OAuth providers
let mut providers = HashMap::new();
providers.insert("github".to_string(), OAuthProviderConfig::github(
    "client_id".to_string(),
    "client_secret".to_string()
));
session_manager.set_oauth_providers(providers);
```

### Startup Recovery

```rust
// Perform automatic session recovery on application start
let recovery_result = session_manager.recover_sessions_on_startup().await?;

println!("Recovery completed:");
println!("  Recovered sessions: {}", recovery_result.recovered_sessions);
println!("  Failed validations: {}", recovery_result.failed_validations);
println!("  Cleaned up sessions: {}", recovery_result.cleaned_up_sessions);
println!("  Recovery duration: {:?}", recovery_result.recovery_duration);

if !recovery_result.success {
    for error in &recovery_result.errors {
        eprintln!("Recovery error: {}", error);
    }
}
```

### Runtime Validation

```rust
// Set up periodic runtime validation
use tokio::time::{interval, Duration};

let mut validation_interval = interval(Duration::from_secs(3600)); // Every hour

loop {
    validation_interval.tick().await;
    
    match session_manager.validate_sessions_runtime().await {
        Ok(result) => {
            if result.failed_validations > 0 {
                println!("Some sessions failed validation: {}", result.failed_validations);
            }
        }
        Err(e) => {
            eprintln!("Runtime validation error: {}", e);
        }
    }
}
```

### Session Management

```rust
// Check for active sessions
if session_manager.has_active_session("github") {
    if let Some(session) = session_manager.get_active_session("github") {
        println!("GitHub session active for user: {}", session.user_id);
        println!("Session age: {:?}", session.age());
        println!("Last validated: {:?}", session.time_since_validation());
    }
}

// Get session statistics
let stats = session_manager.get_session_stats();
println!("Session statistics:");
println!("  Total sessions: {}", stats.total_sessions);
println!("  Valid sessions: {}", stats.valid_sessions);
println!("  Expired sessions: {}", stats.expired_sessions);
println!("  Recovery attempts: {}", stats.recovery_attempts);

// Clear all sessions (for logout)
let cleared_count = session_manager.clear_all_sessions().await?;
println!("Cleared {} sessions", cleared_count);
```

### Custom Configuration

```rust
// Production configuration with conservative settings
let production_config = SessionRecoveryConfig {
    enabled: true,
    auto_recovery_on_startup: true,
    validation_interval_minutes: 30,        // Validate every 30 minutes
    max_recovery_attempts: 5,               // More retry attempts
    token_validation_timeout_seconds: 60,   // Longer timeout
    graceful_degradation: true,
    retry_failed_providers: true,
    cleanup_expired_sessions: true,
    max_session_age_hours: 12,              // Shorter max age
    persist_session_state: true,
};

// Development configuration with frequent validation
let development_config = SessionRecoveryConfig {
    enabled: true,
    auto_recovery_on_startup: true,
    validation_interval_minutes: 5,         // Validate every 5 minutes
    max_recovery_attempts: 1,               // Quick failure
    token_validation_timeout_seconds: 10,   // Short timeout
    graceful_degradation: false,            // Fail fast
    retry_failed_providers: false,
    cleanup_expired_sessions: true,
    max_session_age_hours: 1,               // Very short max age
    persist_session_state: false,           // No persistence
};
```

## Integration with Authentication System

### Auth Resolver Integration

The session manager integrates with the existing auth resolver to provide session-aware authentication:

```rust
use magictunnel::auth::{AuthResolver, MultiLevelAuthConfig};

// Create auth resolver with user context
let auth_config = MultiLevelAuthConfig::new();
let mut auth_resolver = AuthResolver::with_user_context(
    auth_config,
    user_context.clone()
)?;

// Set session manager in auth resolver (future enhancement)
// auth_resolver.set_session_manager(Arc::new(session_manager));

// Use resolver for authentication decisions
if let Some(auth_method) = auth_resolver.resolve_auth_for_tool("github_tool") {
    // Check if we have an active session before requiring re-auth
    match auth_method {
        AuthMethod::OAuth { provider, .. } => {
            if session_manager.has_active_session(&provider) {
                println!("Using existing session for {}", provider);
            } else {
                println!("Need to authenticate with {}", provider);
            }
        }
        _ => {
            // Handle other auth methods
        }
    }
}
```

### OAuth Provider Configuration

Session recovery works with OAuth providers configured in the system:

```rust
use magictunnel::auth::config::OAuthProviderConfig;

// GitHub configuration
let github_config = OAuthProviderConfig::github(
    std::env::var("GITHUB_CLIENT_ID").unwrap(),
    std::env::var("GITHUB_CLIENT_SECRET").unwrap()
);

// Google configuration  
let google_config = OAuthProviderConfig::google(
    std::env::var("GOOGLE_CLIENT_ID").unwrap(),
    std::env::var("GOOGLE_CLIENT_SECRET").unwrap()
);

// Custom provider configuration
let custom_config = OAuthProviderConfig {
    client_id: "custom_client_id".to_string(),
    client_secret: "custom_client_secret".to_string(),
    authorization_endpoint: "https://custom.com/oauth/authorize".to_string(),
    device_authorization_endpoint: None,
    token_endpoint: "https://custom.com/oauth/token".to_string(),
    user_info_endpoint: Some("https://custom.com/api/user".to_string()),
    scopes: vec!["read".to_string(), "write".to_string()],
    oauth_enabled: true,
    device_code_enabled: false,
    resource_indicators: None,
    extra_params: HashMap::new(),
};

let mut providers = HashMap::new();
providers.insert("github".to_string(), github_config);
providers.insert("google".to_string(), google_config);
providers.insert("custom".to_string(), custom_config);

session_manager.set_oauth_providers(providers);
```

## Security Considerations

### Token Validation

- **Userinfo Endpoint Validation** - Uses OAuth provider userinfo endpoints to validate tokens
- **Timeout Protection** - Configurable timeouts prevent hanging requests
- **Rate Limiting** - Respects OAuth provider rate limits
- **Error Handling** - Distinguishes between temporary and permanent failures

### Session Security

- **Secure Storage Integration** - Uses Phase 2.2 token storage for secure token persistence
- **System Isolation** - Sessions are isolated per user and hostname
- **Permission Controls** - Session files have restrictive permissions (0600)
- **Memory Safety** - Sensitive data is zeroized when dropped

### State Protection

- **Atomic Updates** - Session state changes are atomic
- **Corruption Recovery** - Handles corrupted session state gracefully
- **Version Compatibility** - Session state includes version information
- **System Compatibility** - Detects system changes and resets state when needed

## Error Handling

### Recovery Errors

The system provides detailed error reporting for recovery failures:

```rust
match session_manager.recover_sessions_on_startup().await {
    Ok(result) => {
        if !result.success {
            for error in &result.errors {
                match error.as_str() {
                    e if e.contains("Token validation failed") => {
                        // Handle token validation failure
                        eprintln!("Token invalid, need re-authentication");
                    }
                    e if e.contains("Provider timeout") => {
                        // Handle provider timeout
                        eprintln!("Provider unreachable, retrying later");
                    }
                    e if e.contains("Session state reset") => {
                        // Handle system compatibility issue
                        eprintln!("System changed, sessions reset");
                    }
                    _ => {
                        eprintln!("Unknown recovery error: {}", error);
                    }
                }
            }
        }
    }
    Err(e) => {
        eprintln!("Recovery system error: {}", e);
    }
}
```

### Validation Errors

Runtime validation provides specific error information:

```rust
let result = session_manager.validate_sessions_runtime().await?;

// Check for validation failures
if result.failed_validations > 0 {
    for provider in &result.failed_providers {
        println!("Provider {} validation failed", provider);
        
        // Get specific session error
        if let Some(session) = session_manager.get_active_session(provider) {
            if let Some(error) = &session.last_error {
                println!("  Error: {}", error);
                println!("  Attempts: {}", session.validation_attempts);
            }
        }
    }
}
```

## Performance Considerations

### Startup Performance

- **Parallel Validation** - Multiple tokens validated concurrently
- **Timeout Configuration** - Prevents slow providers from blocking startup
- **Background Recovery** - Can be moved to background thread if needed
- **Caching** - Session state caching reduces repeated validations

### Runtime Performance

- **Configurable Intervals** - Validation frequency can be tuned
- **Incremental Validation** - Only validates sessions that need checking
- **Connection Reuse** - HTTP client reuses connections
- **Efficient State Management** - Minimal memory overhead for session state

### Memory Management

- **Secure Cleanup** - Sensitive data is zeroized when dropped
- **Bounded State** - Session state size is bounded by active sessions
- **Efficient Storage** - Compact session representation
- **Garbage Collection** - Expired sessions are automatically cleaned up

## Monitoring and Observability

### Metrics

The session manager provides comprehensive metrics:

```rust
let stats = session_manager.get_session_stats();

println!("Session Recovery Metrics:");
println!("  Total sessions: {}", stats.total_sessions);
println!("  Valid sessions: {}", stats.valid_sessions);
println!("  Expired sessions: {}", stats.expired_sessions);
println!("  Failed providers: {}", stats.failed_providers);
println!("  Recovery attempts: {}", stats.recovery_attempts);
println!("  System compatible: {}", stats.system_compatible);

// Per-provider metrics
for (provider, count) in &stats.providers {
    println!("  {} sessions: {}", provider, count);
}
```

### Logging

The system provides structured logging at multiple levels:

```rust
// Enable debug logging for session recovery
RUST_LOG=magictunnel::auth::session_manager=debug cargo run

// Log levels:
// - ERROR: Critical failures, system errors
// - WARN:  Token validation failures, provider timeouts
// - INFO:  Recovery completion, session statistics
// - DEBUG: Detailed recovery process, validation attempts
// - TRACE: Individual session operations, HTTP requests
```

### Health Checks

```rust
// Check session manager health
let is_healthy = session_manager.get_session_stats().total_sessions > 0;

// Check token storage availability
let storage_available = token_storage.is_storage_available().await;

// Check OAuth providers
let mut provider_health = HashMap::new();
for provider in providers.keys() {
    let has_session = session_manager.has_active_session(provider);
    provider_health.insert(provider.clone(), has_session);
}
```

## Testing

The session recovery system includes comprehensive tests covering:

### Unit Tests
- Session manager creation and configuration
- Active session lifecycle and validation
- Session state management and persistence
- Recovery result handling and error cases
- Configuration validation and edge cases

### Integration Tests
- End-to-end recovery workflows
- Multi-provider session recovery
- Token validation with mock servers
- Error handling and graceful degradation
- Performance under load

### Example Test Run
```bash
# Run session recovery tests
cargo test session_recovery_test

# Run with logging
RUST_LOG=debug cargo test session_recovery_test

# Run specific test
cargo test test_session_manager_creation
```

## Future Enhancements

### Planned Features
1. **Token Refresh** - Automatic refresh of expired tokens
2. **Session Sharing** - Cross-process session sharing
3. **Analytics** - Detailed session usage analytics
4. **Webhooks** - Session event notifications
5. **Admin API** - Session management REST API

### Possible Improvements
1. **Database Backend** - Optional database storage for session state
2. **Distributed Sessions** - Multi-instance session synchronization
3. **Session Templates** - Pre-configured session policies
4. **Machine Learning** - Predictive session validation
5. **Federation** - Cross-system session federation

## Conclusion

The Automatic Session Recovery system provides a robust, secure, and scalable foundation for persistent authentication in MagicTunnel. By building on the User Context System and Multi-Platform Token Storage, it delivers seamless user experiences while maintaining strong security guarantees.

The system's design emphasizes:
- **Reliability** - Graceful handling of failures and edge cases
- **Security** - Secure token storage and validation
- **Performance** - Efficient validation and minimal overhead
- **Observability** - Comprehensive monitoring and debugging
- **Flexibility** - Configurable behavior for different deployment scenarios

This completes the OAuth 2.1 Phase 2.3 implementation, providing MagicTunnel with enterprise-grade authentication persistence capabilities.