use actix_web::{HttpRequest, HttpResponse};
use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::Error as ActixError;
use futures_util::future::{ok, Ready};
use std::collections::HashMap;
use std::task::{Context, Poll};
use tracing::{debug, warn};

use crate::config::TlsConfig;
use crate::error::Result;

/// Security headers middleware configuration
#[derive(Debug, Clone)]
pub struct SecurityHeadersConfig {
    /// Content Security Policy
    pub csp: Option<String>,
    /// X-Frame-Options header
    pub x_frame_options: Option<String>,
    /// X-Content-Type-Options header
    pub x_content_type_options: bool,
    /// X-XSS-Protection header
    pub x_xss_protection: Option<String>,
    /// Referrer-Policy header
    pub referrer_policy: Option<String>,
    /// Permissions-Policy header
    pub permissions_policy: Option<String>,
    /// Custom security headers
    pub custom_headers: HashMap<String, String>,
    /// Enable HSTS header
    pub hsts_enabled: bool,
    /// HSTS configuration
    pub hsts_config: HstsConfig,
}

/// HSTS (HTTP Strict Transport Security) configuration
#[derive(Debug, Clone)]
pub struct HstsConfig {
    /// Max age in seconds
    pub max_age: u64,
    /// Include subdomains
    pub include_subdomains: bool,
    /// Enable preload
    pub preload: bool,
}

impl Default for SecurityHeadersConfig {
    fn default() -> Self {
        Self {
            csp: Some("default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self'; font-src 'self'; object-src 'none'; media-src 'self'; frame-src 'none';".to_string()),
            x_frame_options: Some("DENY".to_string()),
            x_content_type_options: true,
            x_xss_protection: Some("1; mode=block".to_string()),
            referrer_policy: Some("strict-origin-when-cross-origin".to_string()),
            permissions_policy: Some("geolocation=(), microphone=(), camera=()".to_string()),
            custom_headers: HashMap::new(),
            hsts_enabled: true,
            hsts_config: HstsConfig {
                max_age: 31536000, // 1 year
                include_subdomains: false,
                preload: false,
            },
        }
    }
}

impl From<&TlsConfig> for SecurityHeadersConfig {
    fn from(tls_config: &TlsConfig) -> Self {
        let mut config = SecurityHeadersConfig::default();
        
        // Configure HSTS from TLS config
        config.hsts_enabled = tls_config.hsts_enabled;
        config.hsts_config = HstsConfig {
            max_age: tls_config.hsts_max_age,
            include_subdomains: tls_config.hsts_include_subdomains,
            preload: tls_config.hsts_preload,
        };
        
        config
    }
}

/// Security headers middleware
pub struct SecurityHeadersMiddleware {
    config: SecurityHeadersConfig,
}

impl SecurityHeadersMiddleware {
    /// Create new security headers middleware
    pub fn new(config: SecurityHeadersConfig) -> Self {
        Self { config }
    }
    
    /// Create from TLS configuration
    pub fn from_tls_config(tls_config: &TlsConfig) -> Self {
        Self::new(SecurityHeadersConfig::from(tls_config))
    }
    
    /// Apply security headers to response
    pub fn apply_headers(&self, req: &HttpRequest, mut response: HttpResponse) -> HttpResponse {
        let headers = response.headers_mut();
        
        // Content Security Policy
        if let Some(csp) = &self.config.csp {
            headers.insert(
                actix_web::http::header::HeaderName::from_static("content-security-policy"),
                actix_web::http::header::HeaderValue::from_str(csp).unwrap_or_else(|_| actix_web::http::header::HeaderValue::from_static("")),
            );
        }
        
        // X-Frame-Options
        if let Some(x_frame_options) = &self.config.x_frame_options {
            headers.insert(
                actix_web::http::header::HeaderName::from_static("x-frame-options"),
                actix_web::http::header::HeaderValue::from_str(x_frame_options).unwrap_or_else(|_| actix_web::http::header::HeaderValue::from_static("")),
            );
        }
        
        // X-Content-Type-Options
        if self.config.x_content_type_options {
            headers.insert(
                actix_web::http::header::HeaderName::from_static("x-content-type-options"),
                actix_web::http::header::HeaderValue::from_static("nosniff"),
            );
        }
        
        // X-XSS-Protection
        if let Some(x_xss_protection) = &self.config.x_xss_protection {
            headers.insert(
                actix_web::http::header::HeaderName::from_static("x-xss-protection"),
                actix_web::http::header::HeaderValue::from_str(x_xss_protection).unwrap_or_else(|_| actix_web::http::header::HeaderValue::from_static("")),
            );
        }
        
        // Referrer-Policy
        if let Some(referrer_policy) = &self.config.referrer_policy {
            headers.insert(
                actix_web::http::header::HeaderName::from_static("referrer-policy"),
                actix_web::http::header::HeaderValue::from_str(referrer_policy).unwrap_or_else(|_| actix_web::http::header::HeaderValue::from_static("")),
            );
        }
        
        // Permissions-Policy
        if let Some(permissions_policy) = &self.config.permissions_policy {
            headers.insert(
                actix_web::http::header::HeaderName::from_static("permissions-policy"),
                actix_web::http::header::HeaderValue::from_str(permissions_policy).unwrap_or_else(|_| actix_web::http::header::HeaderValue::from_static("")),
            );
        }
        
        // HSTS (only for HTTPS requests)
        if self.config.hsts_enabled && self.is_secure_request(req) {
            let hsts_value = self.build_hsts_header();
            headers.insert(
                actix_web::http::header::HeaderName::from_static("strict-transport-security"),
                actix_web::http::header::HeaderValue::from_str(&hsts_value).unwrap_or_else(|_| actix_web::http::header::HeaderValue::from_static("")),
            );
        }
        
        // Custom headers
        for (name, value) in &self.config.custom_headers {
            if let Ok(header_name) = actix_web::http::header::HeaderName::from_bytes(name.as_bytes()) {
                if let Ok(header_value) = actix_web::http::header::HeaderValue::from_str(value) {
                    headers.insert(header_name, header_value);
                } else {
                    warn!("Invalid header value for {}: {}", name, value);
                }
            } else {
                warn!("Invalid header name: {}", name);
            }
        }
        
        debug!("Applied security headers to response");
        response
    }
    
    /// Check if request is secure (HTTPS)
    fn is_secure_request(&self, req: &HttpRequest) -> bool {
        // Check if request is HTTPS or has X-Forwarded-Proto: https
        req.connection_info().scheme() == "https" ||
        req.headers()
            .get("x-forwarded-proto")
            .and_then(|h| h.to_str().ok())
            .map(|proto| proto.to_lowercase() == "https")
            .unwrap_or(false)
    }
    
    /// Build HSTS header value
    fn build_hsts_header(&self) -> String {
        let mut hsts = format!("max-age={}", self.config.hsts_config.max_age);
        
        if self.config.hsts_config.include_subdomains {
            hsts.push_str("; includeSubDomains");
        }
        
        if self.config.hsts_config.preload {
            hsts.push_str("; preload");
        }
        
        hsts
    }
}

/// Transform implementation for Actix Web middleware
impl<S, B> Transform<S, ServiceRequest> for SecurityHeadersMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = ActixError>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = ActixError;
    type InitError = ();
    type Transform = SecurityHeadersService<S>;
    type Future = Ready<std::result::Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(SecurityHeadersService {
            service,
            config: self.config.clone(),
        })
    }
}

/// Security headers service
pub struct SecurityHeadersService<S> {
    service: S,
    config: SecurityHeadersConfig,
}

impl<S, B> Service<ServiceRequest> for SecurityHeadersService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = ActixError>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = ActixError;
    type Future = futures_util::future::LocalBoxFuture<'static, std::result::Result<Self::Response, Self::Error>>;

    fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<std::result::Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let _config = self.config.clone();
        let fut = self.service.call(req);
        
        Box::pin(async move {
            let res = fut.await?;

            // For now, just return the response as-is
            // TODO: Implement proper security headers middleware
            Ok(res)
        })
    }
}

/// Security headers utilities
pub struct SecurityHeadersUtils;

impl SecurityHeadersUtils {
    /// Create a strict CSP policy for API endpoints
    pub fn strict_api_csp() -> String {
        "default-src 'none'; connect-src 'self'; frame-ancestors 'none';".to_string()
    }
    
    /// Create a relaxed CSP policy for web interfaces
    pub fn relaxed_web_csp() -> String {
        "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self'; font-src 'self'; object-src 'none'; media-src 'self'; frame-src 'none';".to_string()
    }
    
    /// Validate CSP policy syntax (basic validation)
    pub fn validate_csp_policy(policy: &str) -> Result<()> {
        // Basic validation - check for common CSP directives
        let valid_directives = [
            "default-src", "script-src", "style-src", "img-src", "connect-src",
            "font-src", "object-src", "media-src", "frame-src", "frame-ancestors",
            "base-uri", "form-action", "upgrade-insecure-requests"
        ];
        
        for directive in policy.split(';') {
            let directive = directive.trim();
            if directive.is_empty() {
                continue;
            }
            
            let directive_name = directive.split_whitespace().next().unwrap_or("");
            if !valid_directives.contains(&directive_name) && !directive_name.is_empty() {
                warn!("Unknown CSP directive: {}", directive_name);
            }
        }
        
        Ok(())
    }
    
    /// Get recommended security headers for different environments
    pub fn get_recommended_config(environment: &str) -> SecurityHeadersConfig {
        match environment {
            "production" => SecurityHeadersConfig {
                csp: Some(Self::strict_api_csp()),
                x_frame_options: Some("DENY".to_string()),
                x_content_type_options: true,
                x_xss_protection: Some("1; mode=block".to_string()),
                referrer_policy: Some("strict-origin".to_string()),
                permissions_policy: Some("geolocation=(), microphone=(), camera=(), payment=(), usb=()".to_string()),
                custom_headers: HashMap::new(),
                hsts_enabled: true,
                hsts_config: HstsConfig {
                    max_age: 63072000, // 2 years
                    include_subdomains: true,
                    preload: true,
                },
            },
            "development" => SecurityHeadersConfig {
                csp: Some(Self::relaxed_web_csp()),
                x_frame_options: Some("SAMEORIGIN".to_string()),
                x_content_type_options: true,
                x_xss_protection: Some("1; mode=block".to_string()),
                referrer_policy: Some("strict-origin-when-cross-origin".to_string()),
                permissions_policy: Some("geolocation=(), microphone=(), camera=()".to_string()),
                custom_headers: HashMap::new(),
                hsts_enabled: false, // Disabled for development
                hsts_config: HstsConfig::default(),
            },
            _ => SecurityHeadersConfig::default(),
        }
    }
}

impl Default for HstsConfig {
    fn default() -> Self {
        Self {
            max_age: 31536000, // 1 year
            include_subdomains: false,
            preload: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::config::{TlsConfig, TlsMode};
    
    fn create_test_tls_config() -> TlsConfig {
        TlsConfig {
            mode: TlsMode::Application,
            cert_file: None,
            key_file: None,
            ca_file: None,
            behind_proxy: false,
            trusted_proxies: vec![],
            min_tls_version: "1.2".to_string(),
            cipher_suites: None,
            hsts_enabled: true,
            hsts_max_age: 31536000,
            hsts_include_subdomains: true,
            hsts_preload: true,
            require_forwarded_proto: false,
            require_forwarded_for: false,
            auto_detect_headers: vec![],
            fallback_mode: TlsMode::Application,
        }
    }
    
    #[test]
    fn test_security_headers_config_from_tls() {
        let tls_config = create_test_tls_config();
        let security_config = SecurityHeadersConfig::from(&tls_config);
        
        assert!(security_config.hsts_enabled);
        assert_eq!(security_config.hsts_config.max_age, 31536000);
        assert!(security_config.hsts_config.include_subdomains);
        assert!(security_config.hsts_config.preload);
    }
    
    #[test]
    fn test_hsts_header_building() {
        let config = SecurityHeadersConfig {
            hsts_enabled: true,
            hsts_config: HstsConfig {
                max_age: 31536000,
                include_subdomains: true,
                preload: true,
            },
            ..Default::default()
        };
        
        let middleware = SecurityHeadersMiddleware::new(config);
        let hsts_header = middleware.build_hsts_header();
        
        assert_eq!(hsts_header, "max-age=31536000; includeSubDomains; preload");
    }
    
    #[test]
    fn test_csp_validation() {
        let valid_csp = "default-src 'self'; script-src 'self' 'unsafe-inline';";
        assert!(SecurityHeadersUtils::validate_csp_policy(valid_csp).is_ok());
        
        let invalid_csp = "invalid-directive 'self';";
        // Should still pass but log warning
        assert!(SecurityHeadersUtils::validate_csp_policy(invalid_csp).is_ok());
    }
}
