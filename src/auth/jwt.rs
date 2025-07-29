//! JWT authentication for MCP Proxy

use crate::config::JwtConfig;
use crate::error::{ProxyError, Result};
use actix_web::HttpRequest;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, error, warn};
use urlencoding;

/// JWT claims structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    /// Subject (user ID)
    pub sub: String,
    /// Issued at timestamp
    pub iat: u64,
    /// Expiration timestamp
    pub exp: u64,
    /// Issuer
    pub iss: Option<String>,
    /// Audience
    pub aud: Option<String>,
    /// User permissions
    pub permissions: Option<Vec<String>>,
    /// Additional user information
    pub user_info: Option<JwtUserInfo>,
}

/// User information embedded in JWT
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtUserInfo {
    /// User ID
    pub id: String,
    /// User email
    pub email: Option<String>,
    /// User name
    pub name: Option<String>,
    /// User roles
    pub roles: Option<Vec<String>>,
}

/// JWT validation result
#[derive(Debug, Clone)]
pub struct JwtValidationResult {
    /// JWT claims
    pub claims: JwtClaims,
    /// User information
    pub user_info: JwtUserInfo,
    /// User permissions
    pub permissions: Vec<String>,
}

impl JwtValidationResult {
    /// Get user ID
    pub fn get_user_id(&self) -> &str {
        &self.user_info.id
    }

    /// Get user permissions
    pub fn get_permissions(&self) -> &[String] {
        &self.permissions
    }
}

/// JWT validator for handling JWT token authentication
pub struct JwtValidator {
    /// JWT configuration
    config: Option<JwtConfig>,
    /// Decoding key for token validation
    decoding_key: Option<DecodingKey>,
    /// Encoding key for token generation
    encoding_key: Option<EncodingKey>,
    /// JWT algorithm
    algorithm: Option<Algorithm>,
    /// Token validation rules
    validation: Option<Validation>,
}

impl JwtValidator {
    /// Create a new JWT validator
    pub fn new(config: Option<JwtConfig>) -> Result<Self> {
        let (decoding_key, encoding_key, algorithm, validation) = if let Some(ref jwt_config) = config {
            // Parse algorithm
            let algorithm = Self::parse_algorithm(&jwt_config.algorithm)?;
            
            // Create keys based on algorithm
            let (decoding_key, encoding_key) = match algorithm {
                Algorithm::HS256 | Algorithm::HS384 | Algorithm::HS512 => {
                    // HMAC algorithms use the same key for encoding and decoding
                    let key = jwt_config.secret.as_bytes();
                    (
                        DecodingKey::from_secret(key),
                        EncodingKey::from_secret(key),
                    )
                }
                Algorithm::RS256 | Algorithm::RS384 | Algorithm::RS512 => {
                    // RSA algorithms require separate public/private keys
                    // For now, we'll treat the secret as a PEM-encoded private key
                    let private_key = EncodingKey::from_rsa_pem(jwt_config.secret.as_bytes())
                        .map_err(|e| ProxyError::config(format!("Invalid RSA private key: {}", e)))?;
                    
                    // For RSA, we need the public key for decoding
                    // In a real implementation, you'd typically have separate public/private key files
                    // For simplicity, we'll extract the public key from the private key
                    let decoding_key = DecodingKey::from_rsa_pem(jwt_config.secret.as_bytes())
                        .map_err(|e| ProxyError::config(format!("Invalid RSA key for decoding: {}", e)))?;
                    
                    (decoding_key, private_key)
                }
                Algorithm::ES256 | Algorithm::ES384 => {
                    // ECDSA algorithms
                    let private_key = EncodingKey::from_ec_pem(jwt_config.secret.as_bytes())
                        .map_err(|e| ProxyError::config(format!("Invalid ECDSA private key: {}", e)))?;
                    
                    let decoding_key = DecodingKey::from_ec_pem(jwt_config.secret.as_bytes())
                        .map_err(|e| ProxyError::config(format!("Invalid ECDSA key for decoding: {}", e)))?;
                    
                    (decoding_key, private_key)
                }
                _ => {
                    return Err(ProxyError::config(format!("Unsupported JWT algorithm: {:?}", algorithm)));
                }
            };

            // Set up validation rules
            let mut validation = Validation::new(algorithm);
            if let Some(ref issuer) = jwt_config.issuer {
                validation.set_issuer(&[issuer]);
            }
            if let Some(ref audience) = jwt_config.audience {
                validation.set_audience(&[audience]);
            }

            (Some(decoding_key), Some(encoding_key), Some(algorithm), Some(validation))
        } else {
            (None, None, None, None)
        };

        Ok(Self {
            config,
            decoding_key,
            encoding_key,
            algorithm,
            validation,
        })
    }

    /// Parse JWT algorithm from string
    fn parse_algorithm(algorithm: &str) -> Result<Algorithm> {
        match algorithm {
            "HS256" => Ok(Algorithm::HS256),
            "HS384" => Ok(Algorithm::HS384),
            "HS512" => Ok(Algorithm::HS512),
            "RS256" => Ok(Algorithm::RS256),
            "RS384" => Ok(Algorithm::RS384),
            "RS512" => Ok(Algorithm::RS512),
            "ES256" => Ok(Algorithm::ES256),
            "ES384" => Ok(Algorithm::ES384),

            _ => Err(ProxyError::config(format!(
                "Unsupported JWT algorithm: '{}'. Supported: HS256, HS384, HS512, RS256, RS384, RS512, ES256, ES384",
                algorithm
            ))),
        }
    }

    /// Check if JWT authentication is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.is_some()
    }

    /// Validate a JWT token from an HTTP request
    pub fn validate_request(&self, req: &HttpRequest) -> Result<Option<JwtValidationResult>> {
        // If JWT is not configured, return None
        let jwt_config = match &self.config {
            Some(config) => config,
            None => {
                debug!("JWT authentication not configured");
                return Ok(None);
            }
        };

        // Extract token from Authorization header
        let token = self.extract_token_from_request(req)?;

        // Validate the token
        self.validate_token(&token, jwt_config)
    }

    /// Extract JWT token from HTTP request
    fn extract_token_from_request(&self, req: &HttpRequest) -> Result<String> {
        // Check Authorization header
        if let Some(auth_header) = req.headers().get("Authorization") {
            let auth_str = auth_header.to_str().map_err(|_| {
                ProxyError::auth("Invalid Authorization header format")
            })?;

            // Support both "Bearer <token>" and just "<token>" formats
            if auth_str.starts_with("Bearer ") {
                return Ok(auth_str[7..].to_string());
            } else {
                return Ok(auth_str.to_string());
            }
        }

        // Check for token in query parameters
        if let Some(query_string) = req.query_string().split('&').find(|param| param.starts_with("token=")) {
            if let Some(token) = query_string.strip_prefix("token=") {
                return Ok(urlencoding::decode(token)
                    .map_err(|_| ProxyError::auth("Invalid token in query parameter"))?
                    .into_owned());
            }
        }

        Err(ProxyError::auth("No JWT token found in request"))
    }

    /// Validate a JWT token
    pub fn validate_token(&self, token: &str, _jwt_config: &JwtConfig) -> Result<Option<JwtValidationResult>> {
        let decoding_key = self.decoding_key.as_ref()
            .ok_or_else(|| ProxyError::auth("JWT decoding key not configured"))?;
        
        let validation = self.validation.as_ref()
            .ok_or_else(|| ProxyError::auth("JWT validation not configured"))?;

        // Decode and validate the token
        let token_data = decode::<JwtClaims>(token, decoding_key, validation)
            .map_err(|e| {
                warn!("JWT validation failed: {}", e);
                ProxyError::auth("Invalid JWT token")
            })?;

        let claims = token_data.claims;

        // Extract user information
        let user_info = claims.user_info.clone().unwrap_or_else(|| JwtUserInfo {
            id: claims.sub.clone(),
            email: None,
            name: None,
            roles: None,
        });

        // Extract permissions
        let permissions = claims.permissions.clone().unwrap_or_default();

        debug!("JWT validation successful for user: {}", user_info.id);

        Ok(Some(JwtValidationResult {
            claims,
            user_info,
            permissions,
        }))
    }

    /// Generate a JWT token
    pub fn generate_token(&self, user_id: &str, permissions: Vec<String>, user_info: Option<JwtUserInfo>) -> Result<String> {
        let jwt_config = self.config.as_ref()
            .ok_or_else(|| ProxyError::auth("JWT not configured"))?;
        
        let encoding_key = self.encoding_key.as_ref()
            .ok_or_else(|| ProxyError::auth("JWT encoding key not configured"))?;
        
        let algorithm = self.algorithm
            .ok_or_else(|| ProxyError::auth("JWT algorithm not configured"))?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| ProxyError::auth("Failed to get current time"))?
            .as_secs();

        let claims = JwtClaims {
            sub: user_id.to_string(),
            iat: now,
            exp: now + jwt_config.expiration,
            iss: jwt_config.issuer.clone(),
            aud: jwt_config.audience.clone(),
            permissions: Some(permissions),
            user_info,
        };

        let header = Header::new(algorithm);
        
        encode(&header, &claims, encoding_key)
            .map_err(|e| {
                error!("Failed to generate JWT token: {}", e);
                ProxyError::auth("Failed to generate JWT token")
            })
    }

    /// Check if a user has a specific permission
    pub fn check_permission(&self, validation_result: &JwtValidationResult, permission: &str) -> bool {
        validation_result.permissions.contains(&permission.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::JwtConfig;

    fn create_test_jwt_config() -> JwtConfig {
        JwtConfig {
            secret: "test_secret_key_that_is_at_least_32_characters_long".to_string(),
            algorithm: "HS256".to_string(),
            expiration: 3600,
            issuer: Some("test-issuer".to_string()),
            audience: Some("test-audience".to_string()),
        }
    }

    #[test]
    fn test_jwt_validator_creation() {
        let config = create_test_jwt_config();
        let validator = JwtValidator::new(Some(config)).unwrap();
        assert!(validator.is_enabled());
    }

    #[test]
    fn test_jwt_validator_disabled() {
        let validator = JwtValidator::new(None).unwrap();
        assert!(!validator.is_enabled());
    }

    #[test]
    fn test_token_generation_and_validation() {
        let config = create_test_jwt_config();
        let validator = JwtValidator::new(Some(config.clone())).unwrap();

        let user_info = JwtUserInfo {
            id: "test_user".to_string(),
            email: Some("test@example.com".to_string()),
            name: Some("Test User".to_string()),
            roles: Some(vec!["user".to_string()]),
        };

        let permissions = vec!["read".to_string(), "write".to_string()];
        let token = validator.generate_token("test_user", permissions.clone(), Some(user_info.clone())).unwrap();

        let validation_result = validator.validate_token(&token, &config).unwrap().unwrap();
        assert_eq!(validation_result.user_info.id, "test_user");
        assert_eq!(validation_result.permissions, permissions);
    }
}
