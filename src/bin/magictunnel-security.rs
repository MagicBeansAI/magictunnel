//! Security management CLI for MagicTunnel
//!
//! Provides command-line interface for managing security configurations,
//! viewing audit logs, and testing security policies.

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use anyhow::Result;
use tracing::{info, error, warn};
use magictunnel::config::Config;
use magictunnel::security::{
    SecurityConfig, AllowlistConfig, SanitizationConfig, RbacConfig, AuditConfig,
    AllowlistService, RbacService, AuditService,
    AllowlistContext, PermissionContext, AuditEventType, AuditOutcome
};
use magictunnel::security::audit::AuditQueryFilters;
use serde_json;
use std::collections::HashMap;
use chrono::Utc;

#[derive(Parser)]
#[command(name = "magictunnel-security")]
#[command(about = "Security management CLI for MagicTunnel")]
#[command(version)]
struct Cli {
    /// Configuration file path
    #[arg(short, long, default_value = "config.yaml")]
    config: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show security status and configuration
    Status,
    /// Test security policies
    Test {
        /// Tool name to test
        #[arg(short, long)]
        tool: String,
        /// User ID for testing
        #[arg(short, long)]
        user: Option<String>,
        /// User roles (comma-separated)
        #[arg(short, long)]
        roles: Option<String>,
        /// Tool parameters (JSON)
        #[arg(short, long)]
        parameters: Option<String>,
    },
    /// Manage roles and permissions
    Rbac {
        #[command(subcommand)]
        action: RbacCommands,
    },
    /// View audit logs
    Audit {
        #[command(subcommand)]
        action: AuditCommands,
    },
    /// Initialize security configuration
    Init {
        /// Output file path
        #[arg(short, long, default_value = "security-config.yaml")]
        output: PathBuf,
        /// Security level (basic, standard, strict)
        #[arg(short, long, default_value = "standard")]
        level: String,
    },
}

#[derive(Subcommand)]
enum RbacCommands {
    /// List all roles
    ListRoles,
    /// Show role details
    ShowRole {
        /// Role name
        name: String,
    },
    /// Add a new role
    AddRole {
        /// Role name
        name: String,
        /// Role description
        #[arg(short, long)]
        description: Option<String>,
        /// Permissions (comma-separated)
        #[arg(short, long)]
        permissions: String,
        /// Parent roles (comma-separated)
        #[arg(long)]
        parents: Option<String>,
    },
    /// Assign role to user
    AssignUser {
        /// User ID
        user: String,
        /// Role name
        role: String,
    },
    /// Remove role from user
    RemoveUser {
        /// User ID
        user: String,
        /// Role name
        role: String,
    },
    /// Check user permissions
    CheckUser {
        /// User ID
        user: String,
        /// Permission to check
        permission: String,
    },
}

#[derive(Subcommand)]
enum AuditCommands {
    /// Show recent audit entries
    Recent {
        /// Number of entries to show
        #[arg(short, long, default_value = "10")]
        count: usize,
    },
    /// Search audit logs
    Search {
        /// User ID filter
        #[arg(short, long)]
        user: Option<String>,
        /// Tool name filter
        #[arg(short, long)]
        tool: Option<String>,
        /// Hours to look back
        #[arg(long, default_value = "24")]
        hours: u64,
    },
    /// Show security violations
    Violations {
        /// Hours to look back
        #[arg(long, default_value = "24")]
        hours: u64,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    let cli = Cli::parse();
    
    // Load configuration
    let config = Config::load(&cli.config, None, None)?;
    
    match cli.command {
        Commands::Status => show_status(&config).await,
        Commands::Test { tool, user, roles, parameters } => {
            test_security(&config, &tool, user.as_deref(), roles.as_deref(), parameters.as_deref()).await
        }
        Commands::Rbac { action } => handle_rbac(&config, action).await,
        Commands::Audit { action } => handle_audit(&config, action).await,
        Commands::Init { output, level } => init_security_config(&output, &level).await,
    }
}

async fn show_status(config: &Config) -> Result<()> {
    println!("🔒 MagicTunnel Security Status");
    println!("=============================");
    
    if let Some(security_config) = &config.security {
        println!("Security: {}", if security_config.enabled { "✅ ENABLED" } else { "❌ DISABLED" });
        
        if let Some(allowlist_config) = &security_config.allowlist {
            println!("Tool Allowlisting: {}", if allowlist_config.enabled { "✅ ENABLED" } else { "❌ DISABLED" });
            if allowlist_config.enabled {
                println!("  - Default Action: {:?}", allowlist_config.default_action);
                println!("  - Tool Rules: {}", allowlist_config.tools.len());
                println!("  - Capability Patterns: {}", allowlist_config.capability_patterns.len());
                println!("  - Global Patterns: {}", allowlist_config.global_patterns.len());
            }
        } else {
            println!("Tool Allowlisting: ❌ NOT CONFIGURED");
        }
        
        if let Some(sanitization_config) = &security_config.sanitization {
            println!("Request Sanitization: {}", if sanitization_config.enabled { "✅ ENABLED" } else { "❌ DISABLED" });
            if sanitization_config.enabled {
                println!("  - Policies: {}", sanitization_config.policies.len());
                println!("  - Default Action: {:?}", sanitization_config.default_action);
            }
        } else {
            println!("Request Sanitization: ❌ NOT CONFIGURED");
        }
        
        if let Some(rbac_config) = &security_config.rbac {
            println!("RBAC: {}", if rbac_config.enabled { "✅ ENABLED" } else { "❌ DISABLED" });
            if rbac_config.enabled {
                println!("  - Roles: {}", rbac_config.roles.len());
                println!("  - User Assignments: {}", rbac_config.user_roles.len());
                println!("  - API Key Assignments: {}", rbac_config.api_key_roles.len());
                println!("  - Default Roles: {:?}", rbac_config.default_roles);
            }
        } else {
            println!("RBAC: ❌ NOT CONFIGURED");
        }
        
        
        if let Some(audit_config) = &security_config.audit {
            println!("Audit Logging: {}", if audit_config.enabled { "✅ ENABLED" } else { "❌ DISABLED" });
            if audit_config.enabled {
                println!("  - Events: {:?}", audit_config.events);
                println!("  - Storage: {:?}", audit_config.storage);
                println!("  - Retention: {} days", audit_config.retention_days);
            }
        } else {
            println!("Audit Logging: ❌ NOT CONFIGURED");
        }
        
    } else {
        println!("Security: ❌ NOT CONFIGURED");
        println!("To initialize security, run: magictunnel-security init");
    }
    
    Ok(())
}

async fn test_security(
    config: &Config,
    tool_name: &str,
    user_id: Option<&str>,
    roles: Option<&str>,
    parameters: Option<&str>,
) -> Result<()> {
    println!("🧪 Testing Security for Tool: {}", tool_name);
    println!("=====================================");
    
    let security_config = config.security.as_ref()
        .ok_or_else(|| anyhow::anyhow!("Security not configured"))?;
    
    // Test individual security components
    println!("Testing individual security components...\n");
    
    // Test allowlist if configured
    if let Some(allowlist_config) = &security_config.allowlist {
        if allowlist_config.enabled {
            println!("🔍 Testing Tool Allowlist:");
            let allowlist_service = AllowlistService::new(allowlist_config.clone())
                .map_err(|e| anyhow::anyhow!("Failed to create allowlist service: {}", e))?;
            
            let tool_params: HashMap<String, serde_json::Value> = if let Some(params) = parameters {
                serde_json::from_str(params).unwrap_or_default()
            } else {
                HashMap::new()
            };
            
            let context = AllowlistContext {
                user_id: user_id.map(|s| s.to_string()),
                user_roles: roles.unwrap_or("user").split(',').map(|s| s.trim().to_string()).collect(),
                api_key_name: None,
                permissions: vec!["read".to_string(), "write".to_string()],
                source: Some("test".to_string()),
                client_ip: Some("127.0.0.1".to_string()),
            };
            
            let result = allowlist_service.check_tool_access(tool_name, &tool_params, &context);
            println!("  - Allowed: {}", if result.allowed { "✅ YES" } else { "❌ NO" });
            println!("  - Action: {:?}", result.action);
            println!("  - Reason: {}", result.reason);
            if let Some(rule) = &result.matched_rule {
                println!("  - Matched Rule: {}", rule);
            }
            println!();
        }
    }
    
    // Test RBAC if configured
    if let Some(rbac_config) = &security_config.rbac {
        if rbac_config.enabled {
            println!("🔍 Testing RBAC:");
            let rbac_service = RbacService::new(rbac_config.clone())
                .map_err(|e| anyhow::anyhow!("Failed to create RBAC service: {}", e))?;
            
            let context = PermissionContext {
                user_id: user_id.map(|s| s.to_string()),
                user_roles: roles.unwrap_or("user").split(',').map(|s| s.trim().to_string()).collect(),
                api_key_name: None,
                resource: Some(format!("tool:{}", tool_name)),
                action: Some("execute".to_string()),
                client_ip: Some("127.0.0.1".to_string()),
                timestamp: Utc::now(),
                metadata: HashMap::new(),
            };
            
            let result = rbac_service.check_permission("tool:execute", &context);
            println!("  - Permission Granted: {}", if result.granted { "✅ YES" } else { "❌ NO" });
            println!("  - Reason: {}", result.reason);
            if !result.granting_roles.is_empty() {
                println!("  - Granting Roles: {:?}", result.granting_roles);
            }
            println!();
        }
    }
    
    println!("✅ Security test completed");
    Ok(())
}

async fn handle_rbac(config: &Config, action: RbacCommands) -> Result<()> {
    let security_config = config.security.as_ref()
        .ok_or_else(|| anyhow::anyhow!("Security not configured"))?;
    
    let rbac_config = security_config.rbac.as_ref()
        .ok_or_else(|| anyhow::anyhow!("RBAC not configured"))?;
    
    let rbac_service = RbacService::new(rbac_config.clone())
        .map_err(|e| anyhow::anyhow!("Failed to create RBAC service: {}", e))?;
    
    match action {
        RbacCommands::ListRoles => {
            println!("👥 RBAC Roles");
            println!("============");
            for (name, role) in rbac_service.get_roles() {
                println!("  - {}: {}", name, role.description.as_deref().unwrap_or("No description"));
                println!("    Permissions: {:?}", role.permissions);
                if !role.parent_roles.is_empty() {
                    println!("    Parents: {:?}", role.parent_roles);
                }
                println!("    Active: {}", role.active);
                println!();
            }
        }
        RbacCommands::ShowRole { name } => {
            if let Some(role) = rbac_service.get_role(&name) {
                println!("👤 Role: {}", name);
                println!("===========");
                println!("Description: {}", role.description.as_deref().unwrap_or("No description"));
                println!("Permissions: {:?}", role.permissions);
                println!("Parent Roles: {:?}", role.parent_roles);
                println!("Active: {}", role.active);
                if let Some(created) = role.created_at {
                    println!("Created: {}", created);
                }
                if let Some(modified) = role.modified_at {
                    println!("Modified: {}", modified);
                }
            } else {
                error!("Role '{}' not found", name);
            }
        }
        RbacCommands::CheckUser { user, permission } => {
            let context = PermissionContext {
                user_id: Some(user.clone()),
                user_roles: rbac_service.get_user_roles(&user),
                api_key_name: None,
                resource: None,
                action: None,
                client_ip: None,
                timestamp: Utc::now(),
                metadata: HashMap::new(),
            };
            
            let result = rbac_service.check_permission(&permission, &context);
            
            println!("🔍 Permission Check: {} for user {}", permission, user);
            println!("==================================");
            println!("Granted: {}", if result.granted { "✅ YES" } else { "❌ NO" });
            println!("Reason: {}", result.reason);
            if !result.granting_roles.is_empty() {
                println!("Granting Roles: {:?}", result.granting_roles);
            }
        }
        _ => {
            warn!("RBAC modification commands not implemented in CLI yet");
            println!("This command requires direct configuration file editing.");
        }
    }
    
    Ok(())
}

async fn handle_audit(config: &Config, action: AuditCommands) -> Result<()> {
    let security_config = config.security.as_ref()
        .ok_or_else(|| anyhow::anyhow!("Security not configured"))?;
    
    let audit_config = security_config.audit.as_ref()
        .ok_or_else(|| anyhow::anyhow!("Audit logging not configured"))?;
    
    let audit_service = AuditService::new(audit_config.clone()).await
        .map_err(|e| anyhow::anyhow!("Failed to create audit service: {}", e))?;
    
    match action {
        AuditCommands::Recent { count } => {
            println!("📋 Recent Audit Entries ({})", count);
            println!("========================");
            
            let filters = AuditQueryFilters {
                start_time: None,
                end_time: None,
                event_types: None,
                user_id: None,
                tool_name: None,
                outcome: None,
                limit: Some(count),
                offset: None,
            };
            
            let entries = audit_service.query(&filters).await
                .map_err(|e| anyhow::anyhow!("Failed to query audit entries: {}", e))?;
            
            for (i, entry) in entries.iter().enumerate() {
                println!("{}. {} - {} - {}", 
                    i + 1, 
                    entry.timestamp.format("%Y-%m-%d %H:%M:%S"),
                    entry.event_type_string(),
                    entry.summary()
                );
                println!("   User: {:?}", entry.user.as_ref().map(|u| &u.id));
                println!("   Outcome: {:?}", entry.outcome);
                if let Some(error) = &entry.error {
                    println!("   Error: {}", error.message);
                }
                println!();
            }
        }
        AuditCommands::Search { user, tool, hours } => {
            println!("🔍 Audit Search Results (last {} hours)", hours);
            println!("===============================");
            
            let start_time = Utc::now() - chrono::Duration::hours(hours as i64);
            
            let filters = AuditQueryFilters {
                start_time: Some(start_time),
                end_time: None,
                event_types: None,
                user_id: user,
                tool_name: tool,
                outcome: None,
                limit: Some(100),
                offset: None,
            };
            
            let entries = audit_service.query(&filters).await
                .map_err(|e| anyhow::anyhow!("Failed to search audit entries: {}", e))?;
            
            println!("Found {} entries", entries.len());
            
            for (i, entry) in entries.iter().enumerate() {
                println!("{}. {} - {} - {}", 
                    i + 1, 
                    entry.timestamp.format("%Y-%m-%d %H:%M:%S"),
                    entry.event_type_string(),
                    entry.summary()
                );
            }
        }
        AuditCommands::Violations { hours } => {
            println!("🚨 Security Violations (last {} hours)", hours);
            println!("==============================");
            
            let start_time = Utc::now() - chrono::Duration::hours(hours as i64);
            
            let filters = AuditQueryFilters {
                start_time: Some(start_time),
                end_time: None,
                event_types: Some(vec![AuditEventType::SecurityViolation]),
                user_id: None,
                tool_name: None,
                outcome: Some(AuditOutcome::Blocked),
                limit: Some(100),
                offset: None,
            };
            
            let entries = audit_service.query(&filters).await
                .map_err(|e| anyhow::anyhow!("Failed to query security violations: {}", e))?;
            
            println!("Found {} violations", entries.len());
            
            for (i, entry) in entries.iter().enumerate() {
                println!("{}. {} - {}", 
                    i + 1, 
                    entry.timestamp.format("%Y-%m-%d %H:%M:%S"),
                    entry.summary()
                );
                if let Some(error) = &entry.error {
                    println!("   🚫 {}", error.message);
                }
                if let Some(user) = &entry.user {
                    println!("   👤 User: {:?} (Roles: {:?})", user.id, user.roles);
                }
                println!();
            }
        }
    }
    
    Ok(())
}

async fn init_security_config(output: &PathBuf, level: &str) -> Result<()> {
    println!("🔧 Initializing Security Configuration");
    println!("======================================");
    
    let security_config = match level {
        "basic" => SecurityConfig {
            enabled: true,
            allowlist: Some(AllowlistConfig {
                enabled: true,
                default_action: magictunnel::security::AllowlistAction::Allow,
                ..Default::default()
            }),
            sanitization: Some(SanitizationConfig::default()),
            rbac: Some(RbacConfig {
                enabled: true,
                ..Default::default()
            }),
            audit: Some(AuditConfig {
                enabled: true,
                ..Default::default()
            }),
            emergency_lockdown: Some(magictunnel::security::emergency::EmergencyLockdownConfig::default()),
        },
        "strict" => SecurityConfig::secure_defaults(),
        _ => SecurityConfig {
            enabled: true,
            allowlist: Some(AllowlistConfig {
                enabled: true,
                default_action: magictunnel::security::AllowlistAction::Deny,
                ..Default::default()
            }),
            sanitization: Some(SanitizationConfig::default()),
            rbac: Some(RbacConfig {
                enabled: true,
                ..Default::default()
            }),
            audit: Some(AuditConfig {
                enabled: true,
                ..Default::default()
            }),
            emergency_lockdown: Some(magictunnel::security::emergency::EmergencyLockdownConfig::default()),
        },
    };
    
    let yaml = serde_yaml::to_string(&security_config)?;
    std::fs::write(output, format!("# MagicTunnel Security Configuration ({})\n# Generated by magictunnel-security init\n\n{}", level, yaml))?;
    
    println!("✅ Security configuration written to: {}", output.display());
    println!("📝 Review and customize the configuration, then add to your main config.yaml:");
    println!("   security:");
    println!("     # Copy the generated configuration here");
    
    Ok(())
}