//! JWT Authentication Example
//! 
//! This example demonstrates how to use JWT authentication with the MCP Proxy.
//! It shows how to generate JWT tokens and use them to authenticate requests.

use magictunnel::auth::{JwtValidator, JwtUserInfo};
use magictunnel::config::JwtConfig;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔐 JWT Authentication Example");
    println!("=============================\n");

    // 1. Create JWT configuration
    let jwt_config = JwtConfig {
        secret: "my_super_secret_jwt_key_that_is_at_least_32_characters_long".to_string(),
        algorithm: "HS256".to_string(),
        expiration: 3600, // 1 hour
        issuer: Some("magictunnel-example".to_string()),
        audience: Some("mcp-clients".to_string()),
    };

    println!("📋 JWT Configuration:");
    println!("  Algorithm: {}", jwt_config.algorithm);
    println!("  Expiration: {} seconds", jwt_config.expiration);
    println!("  Issuer: {:?}", jwt_config.issuer);
    println!("  Audience: {:?}\n", jwt_config.audience);

    // 2. Create JWT validator
    let validator = JwtValidator::new(Some(jwt_config.clone()))?;
    println!("✅ JWT Validator created successfully\n");

    // 3. Create user information
    let user_info = JwtUserInfo {
        id: "user_123".to_string(),
        email: Some("john.doe@example.com".to_string()),
        name: Some("John Doe".to_string()),
        roles: Some(vec!["user".to_string(), "admin".to_string()]),
    };

    let permissions = vec![
        "read".to_string(),
        "write".to_string(),
        "admin".to_string(),
    ];

    println!("👤 User Information:");
    println!("  ID: {}", user_info.id);
    println!("  Email: {:?}", user_info.email);
    println!("  Name: {:?}", user_info.name);
    println!("  Roles: {:?}", user_info.roles);
    println!("  Permissions: {:?}\n", permissions);

    // 4. Generate JWT token
    let token = validator.generate_token(
        &user_info.id,
        permissions.clone(),
        Some(user_info.clone()),
    )?;

    println!("🎫 Generated JWT Token:");
    println!("  {}\n", token);

    // 5. Validate the token
    let validation_result = validator.validate_token(&token, &jwt_config)?
        .expect("Token validation should succeed");

    println!("✅ Token Validation Result:");
    println!("  User ID: {}", validation_result.user_info.id);
    println!("  User Email: {:?}", validation_result.user_info.email);
    println!("  User Name: {:?}", validation_result.user_info.name);
    println!("  User Roles: {:?}", validation_result.user_info.roles);
    println!("  Permissions: {:?}\n", validation_result.permissions);

    // 6. Check specific permissions
    println!("🔍 Permission Checks:");
    for permission in &["read", "write", "admin", "delete"] {
        let has_permission = validator.check_permission(&validation_result, permission);
        let status = if has_permission { "✅" } else { "❌" };
        println!("  {} {}: {}", status, permission, has_permission);
    }
    println!();

    // 7. Show how to use the token in HTTP requests
    println!("🌐 HTTP Usage Examples:");
    println!("  Authorization Header:");
    println!("    curl -H \"Authorization: Bearer {}\" \\", token);
    println!("         http://localhost:8080/mcp/tools\n");
    
    println!("  Query Parameter:");
    println!("    curl \"http://localhost:8080/mcp/tools?token={}\"", urlencoding::encode(&token));
    println!();

    // 8. Demonstrate token expiration (for educational purposes)
    println!("⏰ Token Claims Information:");
    let claims = &validation_result.claims;
    println!("  Subject (sub): {}", claims.sub);
    println!("  Issued At (iat): {}", claims.iat);
    println!("  Expires At (exp): {}", claims.exp);
    println!("  Issuer (iss): {:?}", claims.iss);
    println!("  Audience (aud): {:?}", claims.aud);
    
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    let time_until_expiry = claims.exp.saturating_sub(current_time);
    println!("  Time until expiry: {} seconds\n", time_until_expiry);

    // 9. Show configuration for MCP Proxy
    println!("⚙️  MCP Proxy Configuration (config.yaml):");
    println!("auth:");
    println!("  enabled: true");
    println!("  type: \"jwt\"");
    println!("  jwt:");
    println!("    secret: \"{}\"", jwt_config.secret);
    println!("    algorithm: \"{}\"", jwt_config.algorithm);
    println!("    expiration: {}", jwt_config.expiration);
    if let Some(issuer) = &jwt_config.issuer {
        println!("    issuer: \"{}\"", issuer);
    }
    if let Some(audience) = &jwt_config.audience {
        println!("    audience: \"{}\"", audience);
    }
    println!();

    println!("🎉 JWT Authentication Example completed successfully!");
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_jwt_example() {
        // This test ensures the example code works correctly
        let result = main().await;
        assert!(result.is_ok(), "JWT example should run without errors");
    }

    #[test]
    fn test_jwt_config_creation() {
        let jwt_config = JwtConfig {
            secret: "test_secret_key_that_is_at_least_32_characters_long".to_string(),
            algorithm: "HS256".to_string(),
            expiration: 3600,
            issuer: Some("test-issuer".to_string()),
            audience: Some("test-audience".to_string()),
        };

        assert_eq!(jwt_config.algorithm, "HS256");
        assert_eq!(jwt_config.expiration, 3600);
        assert_eq!(jwt_config.issuer, Some("test-issuer".to_string()));
        assert_eq!(jwt_config.audience, Some("test-audience".to_string()));
    }

    #[test]
    fn test_user_info_creation() {
        let user_info = JwtUserInfo {
            id: "test_user".to_string(),
            email: Some("test@example.com".to_string()),
            name: Some("Test User".to_string()),
            roles: Some(vec!["user".to_string()]),
        };

        assert_eq!(user_info.id, "test_user");
        assert_eq!(user_info.email, Some("test@example.com".to_string()));
        assert_eq!(user_info.name, Some("Test User".to_string()));
        assert_eq!(user_info.roles, Some(vec!["user".to_string()]));
    }
}
