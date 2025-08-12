//! Client Identity Extraction System for MCP Protocol Extension
//!
//! This module implements the MCP protocol extension for exchanging client identity
//! information during connection establishment. It addresses the critical security
//! vulnerability where multiple users connecting to remote MagicTunnel instances
//! are indistinguishable, leading to session collisions and token conflicts.

use crate::{auth::{ClientIdentity, ClientProcessInfo, ForwardedInfo}};
use crate::error::ProxyError;
use actix_web::HttpRequest;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, net::IpAddr};
use tracing::{debug, trace, warn};
use crate::error::Result;

/// Extended MCP initialization request with client identity information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtendedMcpInitRequest {
    /// Standard MCP initialization fields
    #[serde(flatten)]
    pub standard_init: HashMap<String, Value>,
    
    /// Client identity extension
    #[serde(rename = "clientIdentity")]
    pub client_identity: Option<McpClientIdentity>,
    
    /// Security context for session validation
    #[serde(rename = "securityContext")]
    pub security_context: Option<McpSecurityContext>,
}

/// Client identity information for MCP protocol extension
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpClientIdentity {
    /// Client machine hostname
    pub hostname: Option<String>,
    
    /// Client username
    pub username: Option<String>,
    
    /// Client process information
    #[serde(rename = "processInfo")]
    pub process_info: Option<McpProcessInfo>,
    
    /// Client platform details
    pub platform: Option<McpPlatformInfo>,
    
    /// Client network information (optional, for validation)
    #[serde(rename = "networkInfo")]
    pub network_info: Option<McpNetworkInfo>,
    
    /// Client-provided session token for continuity
    #[serde(rename = "sessionToken")]
    pub session_token: Option<String>,
}

/// MCP process information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpProcessInfo {
    /// Application name (e.g., "Claude Desktop", "VSCode")
    pub name: String,
    
    /// Application version
    pub version: String,
    
    /// Process ID
    pub pid: Option<u32>,
    
    /// Working directory
    #[serde(rename = "workingDirectory")]
    pub working_directory: Option<String>,
    
    /// Command line arguments (sanitized)
    pub args: Option<Vec<String>>,
}

/// MCP platform information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPlatformInfo {
    /// Operating system (e.g., "macOS", "Windows", "Linux")
    pub os: String,
    
    /// OS version
    #[serde(rename = "osVersion")]
    pub os_version: Option<String>,
    
    /// CPU architecture
    pub arch: Option<String>,
    
    /// Timezone
    pub timezone: Option<String>,
    
    /// Locale
    pub locale: Option<String>,
}

/// MCP network information for validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpNetworkInfo {
    /// Client's perceived IP address (for NAT detection)
    #[serde(rename = "clientIp")]
    pub client_ip: Option<String>,
    
    /// Network interface name
    pub interface: Option<String>,
    
    /// Network domain
    pub domain: Option<String>,
}

/// Security context for enhanced validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpSecurityContext {
    /// Authentication method preference
    #[serde(rename = "authMethods")]
    pub auth_methods: Option<Vec<String>>,
    
    /// Security level requirement
    #[serde(rename = "securityLevel")]
    pub security_level: Option<String>,
    
    /// Client certificate fingerprint
    #[serde(rename = "certFingerprint")]
    pub cert_fingerprint: Option<String>,
    
    /// Trusted proxy chain information
    #[serde(rename = "proxyChain")]
    pub proxy_chain: Option<Vec<String>>,
}

/// Client identity extraction results
#[derive(Debug, Clone)]
pub struct ClientIdentityExtractionResult {
    /// Extracted client identity
    pub client_identity: ClientIdentity,
    
    /// MCP-provided identity (if available)
    pub mcp_identity: Option<McpClientIdentity>,
    
    /// Validation score for identity confidence
    pub confidence_score: f64,
    
    /// Extraction warnings
    pub warnings: Vec<String>,
    
    /// Security recommendations
    pub recommendations: Vec<String>,
}

/// Client identity extractor service
pub struct ClientIdentityExtractor {
    /// Require MCP client identity information
    require_mcp_identity: bool,
    
    /// Validate client IP consistency
    validate_ip_consistency: bool,
    
    /// Minimum confidence score required
    min_confidence_score: f64,
    
    /// Trusted proxy networks
    trusted_proxies: Vec<IpAddr>,
}

impl ClientIdentityExtractor {
    /// Create a new client identity extractor
    pub fn new() -> Self {
        Self {
            require_mcp_identity: false,
            validate_ip_consistency: true,
            min_confidence_score: 0.5,
            trusted_proxies: vec![
                "127.0.0.1".parse().unwrap(),
                "::1".parse().unwrap(),
            ],
        }
    }
    
    /// Configure extractor settings
    pub fn configure(
        &mut self,
        require_mcp_identity: bool,
        validate_ip_consistency: bool,
        min_confidence_score: f64,
        trusted_proxies: Vec<IpAddr>,
    ) {
        self.require_mcp_identity = require_mcp_identity;
        self.validate_ip_consistency = validate_ip_consistency;
        self.min_confidence_score = min_confidence_score;
        self.trusted_proxies = trusted_proxies;
    }

    /// Extract comprehensive client identity from HTTP request and MCP data
    pub fn extract_client_identity(
        &self,
        req: &HttpRequest,
        mcp_init: Option<&ExtendedMcpInitRequest>,
    ) -> Result<ClientIdentityExtractionResult> {
        debug!("Extracting client identity from request and MCP data");
        
        let mut warnings = Vec::new();
        let mut recommendations = Vec::new();
        let mut confidence_score = 1.0;

        // Extract basic client identity from HTTP request
        let mut client_identity = self.extract_http_client_identity(req, &mut warnings, &mut confidence_score)?;

        // Enhance with MCP-provided identity if available
        let mcp_identity = if let Some(init) = mcp_init {
            let mcp_client_identity = init.client_identity.clone();
            if let Some(ref mcp_id) = mcp_client_identity {
                self.enhance_with_mcp_identity(&mut client_identity, mcp_id, &mut warnings, &mut confidence_score)?;
            }
            mcp_client_identity
        } else {
            if self.require_mcp_identity {
                warnings.push("MCP client identity required but not provided".to_string());
                confidence_score -= 0.3;
                recommendations.push("Include clientIdentity in MCP initialization".to_string());
            }
            None
        };

        // Validate IP consistency if required
        if self.validate_ip_consistency {
            self.validate_ip_consistency_check(&client_identity, mcp_init, &mut warnings, &mut confidence_score);
        }

        // Check if confidence score meets requirements
        if confidence_score < self.min_confidence_score {
            warnings.push(format!("Confidence score {} below minimum {}", confidence_score, self.min_confidence_score));
            recommendations.push("Provide more client identity information".to_string());
        }

        Ok(ClientIdentityExtractionResult {
            client_identity,
            mcp_identity,
            confidence_score,
            warnings,
            recommendations,
        })
    }

    /// Extract client identity from HTTP request headers and connection info
    fn extract_http_client_identity(
        &self,
        req: &HttpRequest,
        warnings: &mut Vec<String>,
        confidence_score: &mut f64,
    ) -> Result<ClientIdentity> {
        // Get client IP and port
        let connection_info = req.connection_info();
        let client_addr = connection_info
            .realip_remote_addr()
            .unwrap_or_else(|| connection_info.peer_addr().unwrap_or("127.0.0.1:0"));
            
        let socket_addr: std::net::SocketAddr = client_addr
            .parse()
            .map_err(|e| ProxyError::auth(format!("Invalid client address {}: {}", client_addr, e)))?;
            
        let client_ip = socket_addr.ip();
        let client_port = if socket_addr.port() == 0 { None } else { Some(socket_addr.port()) };

        // Extract standard identity headers
        let client_headers = self.extract_identity_headers(req);
        let client_hostname = client_headers.get("x-client-hostname").cloned();
        let client_username = client_headers.get("x-client-username").cloned();

        // Extract user agent
        let user_agent = req
            .headers()
            .get("user-agent")
            .and_then(|h| h.to_str().ok())
            .map(String::from);

        // Extract forwarded information for proxy chain detection
        let forwarded_info = self.extract_forwarded_info(req, warnings, confidence_score);

        // Check for client identity indicators in headers
        if client_hostname.is_none() {
            warnings.push("Missing client hostname in headers".to_string());
            *confidence_score -= 0.1;
        }

        if client_username.is_none() {
            warnings.push("Missing client username in headers".to_string());
            *confidence_score -= 0.1;
        }

        if user_agent.is_none() {
            warnings.push("Missing User-Agent header".to_string());
            *confidence_score -= 0.05;
        }

        Ok(ClientIdentity {
            client_ip,
            client_port,
            client_hostname,
            client_username,
            client_process_info: None, // Will be populated from MCP data
            client_headers,
            user_agent,
            forwarded_info,
            capability_fingerprint: None, // Will be populated from MCP data
        })
    }

    /// Extract identity-related headers from HTTP request
    fn extract_identity_headers(&self, req: &HttpRequest) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        
        // Standard client identity headers
        const IDENTITY_HEADERS: &[&str] = &[
            "x-client-id",
            "x-client-hostname",
            "x-client-username", 
            "x-client-app",
            "x-client-version",
            "x-client-platform",
            "x-client-pid",
            "x-client-session-token",
            "x-forwarded-client-cert",
            "authorization",
        ];
        
        for header_name in IDENTITY_HEADERS {
            if let Some(header_value) = req.headers().get(*header_name) {
                if let Ok(value_str) = header_value.to_str() {
                    headers.insert(header_name.to_string(), value_str.to_string());
                }
            }
        }
        
        trace!("Extracted {} identity headers", headers.len());
        headers
    }

    /// Extract and validate forwarded information
    fn extract_forwarded_info(
        &self,
        req: &HttpRequest,
        warnings: &mut Vec<String>,
        confidence_score: &mut f64,
    ) -> Option<ForwardedInfo> {
        let mut original_ip = None;
        let mut proxy_chain = Vec::new();
        let mut original_headers = HashMap::new();

        // Parse X-Forwarded-For header
        if let Some(forwarded_for) = req.headers().get("x-forwarded-for") {
            if let Ok(forwarded_str) = forwarded_for.to_str() {
                let ips: Vec<&str> = forwarded_str
                    .split(',')
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .collect();
                    
                if let Some(first_ip) = ips.first() {
                    match first_ip.parse::<IpAddr>() {
                        Ok(ip) => {
                            original_ip = Some(ip);
                            // Validate if original IP is from trusted proxy
                            if !self.trusted_proxies.contains(&ip) && !self.is_private_ip(&ip) {
                                // This could indicate a proxy chain or direct connection
                                trace!("Original IP {} not in trusted proxies", ip);
                            }
                        }
                        Err(_) => {
                            warnings.push(format!("Invalid IP in X-Forwarded-For: {}", first_ip));
                            *confidence_score -= 0.05;
                        }
                    }
                }
                
                proxy_chain.extend(ips.into_iter().map(String::from));
            }
        }

        // Extract other forwarded headers
        const FORWARDED_HEADERS: &[&str] = &[
            "x-forwarded-proto",
            "x-forwarded-host",
            "x-forwarded-port",
            "x-real-ip",
            "x-original-forwarded-for",
            "forwarded",
        ];

        for header_name in FORWARDED_HEADERS {
            if let Some(header_value) = req.headers().get(*header_name) {
                if let Ok(value_str) = header_value.to_str() {
                    original_headers.insert(header_name.to_string(), value_str.to_string());
                }
            }
        }

        if original_ip.is_some() || !proxy_chain.is_empty() || !original_headers.is_empty() {
            Some(ForwardedInfo {
                original_ip,
                proxy_chain,
                original_headers,
            })
        } else {
            None
        }
    }

    /// Check if IP address is private/local
    fn is_private_ip(&self, ip: &IpAddr) -> bool {
        match ip {
            IpAddr::V4(ipv4) => {
                ipv4.is_private() || ipv4.is_loopback() || ipv4.is_link_local()
            }
            IpAddr::V6(ipv6) => {
                ipv6.is_loopback() || 
                ipv6.segments()[0] & 0xffc0 == 0xfe80 || // Link-local
                ipv6.segments()[0] & 0xfe00 == 0xfc00    // Unique local
            }
        }
    }

    /// Enhance client identity with MCP-provided information
    fn enhance_with_mcp_identity(
        &self,
        client_identity: &mut ClientIdentity,
        mcp_identity: &McpClientIdentity,
        warnings: &mut Vec<String>,
        confidence_score: &mut f64,
    ) -> Result<()> {
        debug!("Enhancing client identity with MCP-provided information");

        // Update hostname if provided and not already set
        if let Some(ref hostname) = mcp_identity.hostname {
            if client_identity.client_hostname.is_none() {
                client_identity.client_hostname = Some(hostname.clone());
                *confidence_score += 0.1;
            } else if client_identity.client_hostname.as_ref() != Some(hostname) {
                warnings.push("Hostname mismatch between HTTP headers and MCP data".to_string());
                *confidence_score -= 0.1;
            }
        }

        // Update username if provided and not already set
        if let Some(ref username) = mcp_identity.username {
            if client_identity.client_username.is_none() {
                client_identity.client_username = Some(username.clone());
                *confidence_score += 0.1;
            } else if client_identity.client_username.as_ref() != Some(username) {
                warnings.push("Username mismatch between HTTP headers and MCP data".to_string());
                *confidence_score -= 0.1;
            }
        }

        // Convert MCP process info to internal format
        if let Some(ref mcp_process) = mcp_identity.process_info {
            let process_info = ClientProcessInfo {
                app_name: mcp_process.name.clone(),
                app_version: mcp_process.version.clone(),
                platform: mcp_identity.platform.as_ref().map(|p| p.os.clone()),
                pid: mcp_process.pid,
                working_dir: mcp_process.working_directory.clone(),
            };
            client_identity.client_process_info = Some(process_info);
            *confidence_score += 0.15;
        }

        // Generate capability fingerprint from MCP data
        if let Some(fingerprint) = self.generate_mcp_capability_fingerprint(mcp_identity) {
            client_identity.capability_fingerprint = Some(fingerprint);
            *confidence_score += 0.1;
        }

        // Update session token if provided
        if let Some(ref session_token) = mcp_identity.session_token {
            client_identity.client_headers.insert(
                "x-mcp-session-token".to_string(),
                session_token.clone(),
            );
            *confidence_score += 0.05;
        }

        Ok(())
    }

    /// Generate capability fingerprint from MCP identity
    fn generate_mcp_capability_fingerprint(&self, mcp_identity: &McpClientIdentity) -> Option<String> {
        use sha2::{Sha256, Digest};
        
        let mut hasher = Sha256::new();
        
        if let Some(ref process) = mcp_identity.process_info {
            hasher.update(process.name.as_bytes());
            hasher.update(process.version.as_bytes());
        }
        
        if let Some(ref platform) = mcp_identity.platform {
            hasher.update(platform.os.as_bytes());
            if let Some(ref version) = platform.os_version {
                hasher.update(version.as_bytes());
            }
        }
        
        let fingerprint = format!("{:x}", hasher.finalize());
        Some(fingerprint[..16].to_string())
    }

    /// Validate IP address consistency between HTTP and MCP data
    fn validate_ip_consistency_check(
        &self,
        client_identity: &ClientIdentity,
        mcp_init: Option<&ExtendedMcpInitRequest>,
        warnings: &mut Vec<String>,
        confidence_score: &mut f64,
    ) {
        if let Some(init) = mcp_init {
            if let Some(ref mcp_identity) = init.client_identity {
                if let Some(ref network_info) = mcp_identity.network_info {
                    if let Some(ref client_ip_str) = network_info.client_ip {
                        match client_ip_str.parse::<IpAddr>() {
                            Ok(mcp_ip) => {
                                // Check for NAT or proxy scenarios
                                if mcp_ip != client_identity.client_ip {
                                    if self.is_private_ip(&mcp_ip) && !self.is_private_ip(&client_identity.client_ip) {
                                        // Client behind NAT - this is normal
                                        trace!("Client behind NAT: reported {} vs actual {}", mcp_ip, client_identity.client_ip);
                                    } else if client_identity.forwarded_info.is_some() {
                                        // Client behind proxy - validate against forwarded info
                                        trace!("Client behind proxy: validating forwarded info");
                                    } else {
                                        warnings.push("IP address mismatch without proxy indication".to_string());
                                        *confidence_score -= 0.15;
                                    }
                                }
                            }
                            Err(_) => {
                                warnings.push("Invalid client IP in MCP network info".to_string());
                                *confidence_score -= 0.05;
                            }
                        }
                    }
                }
            }
        }
    }

    /// Create extended MCP initialization request from standard init data
    pub fn parse_extended_mcp_init(init_data: &HashMap<String, Value>) -> Option<ExtendedMcpInitRequest> {
        // Try to deserialize as extended MCP init request
        match serde_json::to_value(init_data) {
            Ok(value) => {
                match serde_json::from_value::<ExtendedMcpInitRequest>(value) {
                    Ok(extended_init) => Some(extended_init),
                    Err(e) => {
                        trace!("Failed to parse as extended MCP init: {}", e);
                        // Create basic extended init with just standard fields
                        Some(ExtendedMcpInitRequest {
                            standard_init: init_data.clone(),
                            client_identity: None,
                            security_context: None,
                        })
                    }
                }
            }
            Err(e) => {
                warn!("Failed to convert init data to value: {}", e);
                None
            }
        }
    }

    /// Validate extracted client identity result
    pub fn validate_extraction_result(&self, result: &ClientIdentityExtractionResult) -> Result<()> {
        if result.confidence_score < self.min_confidence_score {
            return Err(ProxyError::auth(format!(
                "Client identity confidence score {} below minimum {}",
                result.confidence_score,
                self.min_confidence_score
            )));
        }

        if self.require_mcp_identity && result.mcp_identity.is_none() {
            return Err(ProxyError::auth("MCP client identity required but not provided".to_string()));
        }

        // Check for critical warnings
        for warning in &result.warnings {
            if warning.contains("mismatch") {
                warn!("Critical identity warning: {}", warning);
            }
        }

        Ok(())
    }
}

impl Default for ClientIdentityExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test::TestRequest;
    use serde_json::json;

    #[test]
    fn test_basic_client_identity_extraction() {
        let extractor = ClientIdentityExtractor::new();
        let req = TestRequest::default()
            .insert_header(("user-agent", "Claude Desktop/1.0"))
            .insert_header(("x-client-hostname", "user-macbook"))
            .insert_header(("x-client-username", "john"))
            .peer_addr("192.168.1.100:12345".parse().unwrap())
            .to_http_request();

        let result = extractor.extract_client_identity(&req, None);
        assert!(result.is_ok());

        let result = result.unwrap();
        assert_eq!(result.client_identity.client_ip.to_string(), "192.168.1.100");
        assert_eq!(result.client_identity.client_hostname, Some("user-macbook".to_string()));
        assert_eq!(result.client_identity.client_username, Some("john".to_string()));
        assert!(result.confidence_score > 0.5);
    }

    #[test]
    fn test_mcp_identity_enhancement() {
        let extractor = ClientIdentityExtractor::new();
        let req = TestRequest::default()
            .peer_addr("192.168.1.100:12345".parse().unwrap())
            .to_http_request();

        let mut init_data = HashMap::new();
        init_data.insert("method".to_string(), json!("initialize"));
        
        let mcp_init = ExtendedMcpInitRequest {
            standard_init: init_data,
            client_identity: Some(McpClientIdentity {
                hostname: Some("johns-macbook".to_string()),
                username: Some("john".to_string()),
                process_info: Some(McpProcessInfo {
                    name: "Claude Desktop".to_string(),
                    version: "1.2.3".to_string(),
                    pid: Some(12345),
                    working_directory: Some("/Users/john".to_string()),
                    args: None,
                }),
                platform: Some(McpPlatformInfo {
                    os: "macOS".to_string(),
                    os_version: Some("14.0".to_string()),
                    arch: Some("arm64".to_string()),
                    timezone: Some("America/New_York".to_string()),
                    locale: Some("en_US".to_string()),
                }),
                network_info: Some(McpNetworkInfo {
                    client_ip: Some("192.168.1.100".to_string()),
                    interface: Some("en0".to_string()),
                    domain: Some("example.com".to_string()),
                }),
                session_token: Some("session_123".to_string()),
            }),
            security_context: None,
        };

        let result = extractor.extract_client_identity(&req, Some(&mcp_init));
        assert!(result.is_ok());

        let result = result.unwrap();
        assert_eq!(result.client_identity.client_hostname, Some("johns-macbook".to_string()));
        assert_eq!(result.client_identity.client_username, Some("john".to_string()));
        assert!(result.client_identity.client_process_info.is_some());
        assert!(result.client_identity.capability_fingerprint.is_some());
        assert!(result.confidence_score > 0.8);
    }

    #[test]
    fn test_forwarded_header_extraction() {
        let extractor = ClientIdentityExtractor::new();
        let req = TestRequest::default()
            .insert_header(("x-forwarded-for", "203.0.113.1, 198.51.100.1"))
            .insert_header(("x-forwarded-proto", "https"))
            .peer_addr("10.0.0.1:8080".parse().unwrap())
            .to_http_request();

        let result = extractor.extract_client_identity(&req, None);
        assert!(result.is_ok());

        let result = result.unwrap();
        assert!(result.client_identity.forwarded_info.is_some());
        
        let forwarded = result.client_identity.forwarded_info.unwrap();
        assert_eq!(forwarded.original_ip, Some("203.0.113.1".parse().unwrap()));
        assert_eq!(forwarded.proxy_chain.len(), 2);
        assert!(forwarded.original_headers.contains_key("x-forwarded-proto"));
    }

    #[test]
    fn test_ip_consistency_validation() {
        let mut extractor = ClientIdentityExtractor::new();
        extractor.validate_ip_consistency = true;
        
        let req = TestRequest::default()
            .peer_addr("198.51.100.1:8080".parse().unwrap())
            .to_http_request();

        let mut init_data = HashMap::new();
        init_data.insert("method".to_string(), json!("initialize"));
        
        let mcp_init = ExtendedMcpInitRequest {
            standard_init: init_data,
            client_identity: Some(McpClientIdentity {
                hostname: None,
                username: None,
                process_info: None,
                platform: None,
                network_info: Some(McpNetworkInfo {
                    client_ip: Some("192.168.1.100".to_string()), // Different from actual
                    interface: None,
                    domain: None,
                }),
                session_token: None,
            }),
            security_context: None,
        };

        let result = extractor.extract_client_identity(&req, Some(&mcp_init));
        assert!(result.is_ok());

        let result = result.unwrap();
        // Should have warning about IP mismatch
        assert!(!result.warnings.is_empty());
        assert!(result.warnings.iter().any(|w| w.contains("mismatch")));
    }

    #[test]
    fn test_confidence_score_calculation() {
        let extractor = ClientIdentityExtractor::new();
        
        // Minimal request - should have lower confidence
        let req1 = TestRequest::default()
            .peer_addr("127.0.0.1:12345".parse().unwrap())
            .to_http_request();
        let result1 = extractor.extract_client_identity(&req1, None).unwrap();
        
        // Rich request with headers - should have higher confidence
        let req2 = TestRequest::default()
            .insert_header(("user-agent", "Claude Desktop/1.0"))
            .insert_header(("x-client-hostname", "test-machine"))
            .insert_header(("x-client-username", "testuser"))
            .peer_addr("192.168.1.100:12345".parse().unwrap())
            .to_http_request();
        let result2 = extractor.extract_client_identity(&req2, None).unwrap();
        
        assert!(result2.confidence_score > result1.confidence_score);
    }
}