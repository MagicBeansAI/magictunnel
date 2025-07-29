use actix_web::HttpRequest;
use std::net::IpAddr;
use tracing::{debug, warn};

use crate::config::{TlsConfig, TlsMode};
use crate::error::{ProxyError, Result};
use super::{ProxyHeaders, TrustedProxyValidator};
use super::headers::ProxyHeaderUtils;

/// Proxy request validator for different TLS modes
pub struct ProxyValidator {
    tls_config: TlsConfig,
    trusted_proxy_validator: Option<TrustedProxyValidator>,
}

impl ProxyValidator {
    /// Create a new proxy validator
    pub fn new(tls_config: TlsConfig) -> Result<Self> {
        // Validate TLS configuration based on mode
        match tls_config.mode {
            TlsMode::Application => {
                // Application mode requires certificate files
                if tls_config.cert_file.is_none() {
                    return Err(ProxyError::config(
                        "TLS certificate file (cert_file) is required for application mode"
                    ));
                }
                if tls_config.key_file.is_none() {
                    return Err(ProxyError::config(
                        "TLS private key file (key_file) is required for application mode"
                    ));
                }
            }
            _ => {
                // Other modes don't require certificate validation here
            }
        }

        let trusted_proxy_validator = if !tls_config.trusted_proxies.is_empty() {
            Some(TrustedProxyValidator::new(&tls_config.trusted_proxies)?)
        } else {
            None
        };

        Ok(Self {
            tls_config,
            trusted_proxy_validator,
        })
    }
    
    /// Validate a request based on TLS configuration
    pub fn validate_request(&self, req: &HttpRequest) -> Result<ProxyRequestInfo> {
        let proxy_headers = ProxyHeaders::from_request(req);
        let connection_info = ProxyHeaderUtils::get_connection_info(req);
        
        debug!(
            "Validating request: path={}, is_proxied={}, scheme={}, client_ip={:?}",
            req.path(),
            proxy_headers.is_proxied,
            connection_info.scheme,
            connection_info.remote_addr
        );
        
        // Validate based on TLS mode
        match self.tls_config.mode {
            TlsMode::Disabled => {
                // No validation needed for disabled mode
                Ok(())
            }
            TlsMode::Application => {
                // In application mode, we shouldn't receive proxy headers
                if proxy_headers.is_proxied {
                    warn!(
                        "Received proxy headers in application TLS mode. This might indicate misconfiguration."
                    );
                }
                Ok(())
            }
            TlsMode::BehindProxy => {
                self.validate_behind_proxy_request(&proxy_headers, &connection_info)
            }
            TlsMode::Auto => {
                self.validate_auto_mode_request(&proxy_headers, &connection_info)
            }
        }?;
        
        // Determine if HTTPS before moving values
        let is_https = connection_info.scheme == "https" || proxy_headers.is_original_https();

        // Return proxy request information
        Ok(ProxyRequestInfo {
            is_proxied: proxy_headers.is_proxied,
            client_ip: connection_info.remote_addr,
            original_scheme: connection_info.scheme,
            original_host: connection_info.host,
            forwarded_for: proxy_headers.forwarded_for,
            is_https,
        })
    }
    
    /// Validate request in behind-proxy mode
    fn validate_behind_proxy_request(
        &self,
        proxy_headers: &ProxyHeaders,
        connection_info: &super::headers::ConnectionInfo,
    ) -> Result<()> {
        // Validate that request comes from trusted proxy
        if let Some(validator) = &self.trusted_proxy_validator {
            if let Some(client_ip) = connection_info.remote_addr {
                validator.validate_proxy_request(&client_ip)?;
            }
        }
        
        // Validate required headers are present
        if self.tls_config.require_forwarded_proto && proxy_headers.original_proto.is_none() {
            return Err(ProxyError::config(
                "X-Forwarded-Proto header is required in behind-proxy mode but was not found"
            ));
        }
        
        if self.tls_config.require_forwarded_for && proxy_headers.client_ip.is_none() {
            return Err(ProxyError::config(
                "X-Forwarded-For or X-Real-IP header is required in behind-proxy mode but was not found"
            ));
        }
        
        debug!(
            "Behind-proxy request validated: original_proto={:?}, client_ip={:?}",
            proxy_headers.original_proto,
            proxy_headers.client_ip
        );
        
        Ok(())
    }
    
    /// Validate request in auto mode
    fn validate_auto_mode_request(
        &self,
        proxy_headers: &ProxyHeaders,
        connection_info: &super::headers::ConnectionInfo,
    ) -> Result<()> {
        // Auto-detection logic: determine mode based on request characteristics
        let detected_mode = self.auto_detect_mode(proxy_headers, connection_info)?;

        match detected_mode {
            TlsMode::BehindProxy => {
                debug!("Auto mode: detected behind-proxy, validating as proxy request");
                self.validate_behind_proxy_request(proxy_headers, connection_info)
            }
            TlsMode::Application => {
                debug!("Auto mode: detected direct connection, treating as application request");
                // In application mode, we shouldn't receive proxy headers
                if proxy_headers.is_proxied {
                    warn!(
                        "Received proxy headers in auto-detected application mode. This might indicate misconfiguration."
                    );
                }
                Ok(())
            }
            _ => {
                // Fallback to configured fallback mode
                debug!("Auto mode: using fallback mode {:?}", self.tls_config.fallback_mode);
                match self.tls_config.fallback_mode {
                    TlsMode::BehindProxy => self.validate_behind_proxy_request(proxy_headers, connection_info),
                    _ => Ok(()),
                }
            }
        }
    }

    /// Auto-detect TLS mode based on request characteristics
    fn auto_detect_mode(
        &self,
        proxy_headers: &ProxyHeaders,
        connection_info: &super::headers::ConnectionInfo,
    ) -> Result<TlsMode> {
        // Check if request has proxy headers
        if proxy_headers.is_proxied {
            // Validate that the proxy is trusted if we have a validator
            if let Some(validator) = &self.trusted_proxy_validator {
                if let Some(client_ip) = connection_info.remote_addr {
                    if validator.is_trusted_proxy(&client_ip) {
                        debug!("Auto-detection: trusted proxy detected, using BehindProxy mode");
                        return Ok(TlsMode::BehindProxy);
                    } else {
                        warn!("Auto-detection: untrusted proxy detected, using fallback mode");
                        return Ok(self.tls_config.fallback_mode.clone());
                    }
                }
            }

            // Check if required headers are present for behind-proxy mode
            let has_required_headers = self.check_required_proxy_headers(proxy_headers);
            if has_required_headers {
                debug!("Auto-detection: proxy headers present and valid, using BehindProxy mode");
                return Ok(TlsMode::BehindProxy);
            } else {
                debug!("Auto-detection: proxy headers incomplete, using fallback mode");
                return Ok(self.tls_config.fallback_mode.clone());
            }
        } else {
            // No proxy headers - likely direct connection
            debug!("Auto-detection: no proxy headers, using Application mode");
            return Ok(TlsMode::Application);
        }
    }

    /// Check if required proxy headers are present
    fn check_required_proxy_headers(&self, proxy_headers: &ProxyHeaders) -> bool {
        // Check for presence of key proxy headers
        let has_proto = proxy_headers.original_proto.is_some();
        let has_client_ip = proxy_headers.client_ip.is_some();

        // Basic validation - at least one key header should be present
        has_proto || has_client_ip
    }
}

/// Information about a validated proxy request
#[derive(Debug, Clone)]
pub struct ProxyRequestInfo {
    /// Whether the request came through a proxy
    pub is_proxied: bool,
    /// Real client IP address
    pub client_ip: Option<IpAddr>,
    /// Original request scheme (http/https)
    pub original_scheme: String,
    /// Original host header
    pub original_host: String,
    /// Forwarded-for chain
    pub forwarded_for: Vec<IpAddr>,
    /// Whether the original request was HTTPS
    pub is_https: bool,
}

impl ProxyRequestInfo {
    /// Get the effective client IP
    pub fn get_client_ip(&self) -> Option<IpAddr> {
        self.client_ip
    }
    
    /// Check if the original request was HTTPS
    pub fn is_original_https(&self) -> bool {
        self.is_https
    }
    
    /// Get the original scheme
    pub fn get_original_scheme(&self) -> &str {
        &self.original_scheme
    }
    
    /// Get the original host
    pub fn get_original_host(&self) -> &str {
        &self.original_host
    }
}

/// Utility functions for proxy validation in handlers
pub struct ProxyValidationUtils;

impl ProxyValidationUtils {
    /// Validate request and return proxy information
    pub fn validate_and_extract(req: &HttpRequest, tls_config: &TlsConfig) -> Result<ProxyRequestInfo> {
        let validator = ProxyValidator::new(tls_config.clone())?;
        validator.validate_request(req)
    }
    
    /// Check if request should be treated as HTTPS
    pub fn is_secure_request(req: &HttpRequest, tls_config: &TlsConfig) -> bool {
        match Self::validate_and_extract(req, tls_config) {
            Ok(info) => info.is_original_https(),
            Err(_) => req.connection_info().scheme() == "https",
        }
    }
    
    /// Get the real client IP address
    pub fn get_real_client_ip(req: &HttpRequest, tls_config: &TlsConfig) -> Option<IpAddr> {
        match Self::validate_and_extract(req, tls_config) {
            Ok(info) => info.get_client_ip(),
            Err(_) => {
                // Fallback to connection info
                req.connection_info().peer_addr()
                    .and_then(|addr| addr.parse().ok())
            }
        }
    }
    
    /// Check if request came through a proxy
    pub fn is_proxied_request(req: &HttpRequest) -> bool {
        let proxy_headers = ProxyHeaders::from_request(req);
        proxy_headers.is_proxied
    }

    /// Auto-detect TLS mode for a request
    pub fn auto_detect_tls_mode(req: &HttpRequest, tls_config: &TlsConfig) -> Result<TlsMode> {
        let validator = ProxyValidator::new(tls_config.clone())?;
        let proxy_headers = ProxyHeaders::from_request(req);
        let connection_info = ProxyHeaderUtils::get_connection_info(req);

        validator.auto_detect_mode(&proxy_headers, &connection_info)
    }

    /// Check if request appears to be from a trusted proxy
    pub fn is_from_trusted_proxy(req: &HttpRequest, tls_config: &TlsConfig) -> bool {
        if tls_config.trusted_proxies.is_empty() {
            return false;
        }

        if let Ok(validator) = TrustedProxyValidator::new(&tls_config.trusted_proxies) {
            if let Some(client_ip) = req.connection_info().peer_addr()
                .and_then(|addr| addr.parse().ok()) {
                return validator.is_trusted_proxy(&client_ip);
            }
        }

        false
    }

    /// Get auto-detection confidence score (0.0 to 1.0)
    pub fn get_detection_confidence(req: &HttpRequest, tls_config: &TlsConfig) -> f64 {
        let proxy_headers = ProxyHeaders::from_request(req);

        if !proxy_headers.is_proxied {
            // No proxy headers - high confidence for direct connection
            return 0.9;
        }

        let mut confidence: f64 = 0.5; // Base confidence for proxy detection

        // Increase confidence based on header completeness
        if proxy_headers.original_proto.is_some() {
            confidence += 0.2;
        }
        if proxy_headers.client_ip.is_some() {
            confidence += 0.2;
        }
        if proxy_headers.original_host.is_some() {
            confidence += 0.1;
        }

        // Increase confidence if from trusted proxy
        if Self::is_from_trusted_proxy(req, tls_config) {
            confidence += 0.2;
        }

        confidence.min(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test::TestRequest;
    use crate::config::TlsMode;
    
    fn create_test_tls_config(mode: TlsMode) -> TlsConfig {
        let behind_proxy = mode == TlsMode::BehindProxy;
        TlsConfig {
            mode,
            cert_file: None,
            key_file: None,
            ca_file: None,
            behind_proxy,
            trusted_proxies: vec!["127.0.0.1/32".to_string(), "10.0.0.0/8".to_string()],
            min_tls_version: "1.2".to_string(),
            cipher_suites: None,
            hsts_enabled: false,
            hsts_max_age: 0,
            hsts_include_subdomains: false,
            hsts_preload: false,
            require_forwarded_proto: false,
            require_forwarded_for: false,
            auto_detect_headers: vec!["X-Forwarded-Proto".to_string()],
            fallback_mode: TlsMode::Application,
        }
    }
    
    #[test]
    fn test_proxy_validation_disabled_mode() {
        let tls_config = create_test_tls_config(TlsMode::Disabled);
        let validator = ProxyValidator::new(tls_config).unwrap();
        
        let req = TestRequest::default().to_http_request();
        let result = validator.validate_request(&req);
        
        assert!(result.is_ok());
        let info = result.unwrap();
        assert!(!info.is_proxied);
    }
    
    #[test]
    fn test_proxy_validation_behind_proxy_mode() {
        let mut tls_config = create_test_tls_config(TlsMode::BehindProxy);
        tls_config.require_forwarded_proto = true;
        // Add the test IP to trusted proxies
        tls_config.trusted_proxies.push("203.0.113.1/32".to_string());
        
        let validator = ProxyValidator::new(tls_config).unwrap();
        
        // Request with proxy headers should pass
        let req = TestRequest::default()
            .insert_header(("x-forwarded-proto", "https"))
            .insert_header(("x-forwarded-for", "203.0.113.1"))
            .to_http_request();
        
        let result = validator.validate_request(&req);
        assert!(result.is_ok());
        
        let info = result.unwrap();
        assert!(info.is_proxied);
        assert!(info.is_original_https());
    }
    
    #[test]
    fn test_proxy_validation_utils() {
        let tls_config = create_test_tls_config(TlsMode::Auto);
        
        let req = TestRequest::default()
            .insert_header(("x-forwarded-proto", "https"))
            .to_http_request();
        
        assert!(ProxyValidationUtils::is_secure_request(&req, &tls_config));
        assert!(ProxyValidationUtils::is_proxied_request(&req));
    }
}
