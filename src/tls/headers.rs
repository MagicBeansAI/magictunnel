use actix_web::HttpRequest;
use std::net::IpAddr;
use std::str::FromStr;
use tracing::debug;

/// Extracted proxy headers from HTTP request
#[derive(Debug, Clone)]
pub struct ProxyHeaders {
    /// Original client IP address
    pub client_ip: Option<IpAddr>,
    /// Original protocol (http/https)
    pub original_proto: Option<String>,
    /// Original host header
    pub original_host: Option<String>,
    /// Original port
    pub original_port: Option<u16>,
    /// Forwarded for chain
    pub forwarded_for: Vec<IpAddr>,
    /// Whether request appears to be from a proxy
    pub is_proxied: bool,
}

/// Standard forwarded headers used by reverse proxies
#[derive(Debug, Clone)]
pub struct ForwardedHeaders {
    /// X-Forwarded-For header value
    pub x_forwarded_for: Option<String>,
    /// X-Forwarded-Proto header value
    pub x_forwarded_proto: Option<String>,
    /// X-Forwarded-Host header value
    pub x_forwarded_host: Option<String>,
    /// X-Forwarded-Port header value
    pub x_forwarded_port: Option<String>,
    /// X-Real-IP header value
    pub x_real_ip: Option<String>,
}

impl ProxyHeaders {
    /// Extract proxy headers from HTTP request
    pub fn from_request(req: &HttpRequest) -> Self {
        let forwarded = ForwardedHeaders::from_request(req);
        
        // Determine if request is proxied
        let is_proxied = forwarded.x_forwarded_proto.is_some()
            || forwarded.x_forwarded_for.is_some()
            || forwarded.x_real_ip.is_some();
        
        // Extract client IP (prefer X-Real-IP, then first IP in X-Forwarded-For)
        let client_ip = forwarded.x_real_ip
            .as_ref()
            .and_then(|ip| IpAddr::from_str(ip).ok())
            .or_else(|| {
                forwarded.x_forwarded_for
                    .as_ref()
                    .and_then(|xff| Self::parse_first_ip(xff))
            });
        
        // Extract forwarded for chain
        let forwarded_for = forwarded.x_forwarded_for
            .as_ref()
            .map(|xff| Self::parse_forwarded_for_chain(xff))
            .unwrap_or_default();
        
        // Extract original protocol
        let original_proto = forwarded.x_forwarded_proto.clone();
        
        // Extract original host
        let original_host = forwarded.x_forwarded_host.clone();
        
        // Extract original port
        let original_port = forwarded.x_forwarded_port
            .as_ref()
            .and_then(|port| port.parse().ok());
        
        debug!(
            "Extracted proxy headers: client_ip={:?}, proto={:?}, host={:?}, port={:?}, is_proxied={}",
            client_ip, original_proto, original_host, original_port, is_proxied
        );
        
        Self {
            client_ip,
            original_proto,
            original_host,
            original_port,
            forwarded_for,
            is_proxied,
        }
    }
    
    /// Get the effective client IP (real client, not proxy)
    pub fn get_client_ip(&self, fallback_ip: Option<IpAddr>) -> Option<IpAddr> {
        self.client_ip.or(fallback_ip)
    }
    
    /// Check if the original request was HTTPS
    pub fn is_original_https(&self) -> bool {
        self.original_proto
            .as_ref()
            .map(|proto| proto.to_lowercase() == "https")
            .unwrap_or(false)
    }
    
    /// Get the original scheme (http/https)
    pub fn get_original_scheme(&self) -> &str {
        if self.is_original_https() {
            "https"
        } else {
            "http"
        }
    }
    
    /// Parse the first IP address from X-Forwarded-For header
    fn parse_first_ip(xff: &str) -> Option<IpAddr> {
        xff.split(',')
            .next()
            .and_then(|ip| IpAddr::from_str(ip.trim()).ok())
    }
    
    /// Parse the complete forwarded-for chain
    fn parse_forwarded_for_chain(xff: &str) -> Vec<IpAddr> {
        xff.split(',')
            .filter_map(|ip| IpAddr::from_str(ip.trim()).ok())
            .collect()
    }
}

impl ForwardedHeaders {
    /// Extract forwarded headers from HTTP request
    pub fn from_request(req: &HttpRequest) -> Self {
        let headers = req.headers();
        
        Self {
            x_forwarded_for: headers
                .get("x-forwarded-for")
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string()),
            x_forwarded_proto: headers
                .get("x-forwarded-proto")
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string()),
            x_forwarded_host: headers
                .get("x-forwarded-host")
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string()),
            x_forwarded_port: headers
                .get("x-forwarded-port")
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string()),
            x_real_ip: headers
                .get("x-real-ip")
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string()),
        }
    }
    
    /// Check if any forwarded headers are present
    pub fn has_forwarded_headers(&self) -> bool {
        self.x_forwarded_for.is_some()
            || self.x_forwarded_proto.is_some()
            || self.x_forwarded_host.is_some()
            || self.x_forwarded_port.is_some()
            || self.x_real_ip.is_some()
    }
    
    /// Validate that required headers are present for behind-proxy mode
    pub fn validate_required_headers(&self, require_proto: bool, require_for: bool) -> Result<(), String> {
        if require_proto && self.x_forwarded_proto.is_none() {
            return Err("X-Forwarded-Proto header is required but missing".to_string());
        }
        
        if require_for && self.x_forwarded_for.is_none() && self.x_real_ip.is_none() {
            return Err("X-Forwarded-For or X-Real-IP header is required but missing".to_string());
        }
        
        Ok(())
    }
}

/// Utility functions for working with proxy headers
pub struct ProxyHeaderUtils;

impl ProxyHeaderUtils {
    /// Extract connection info from request, considering proxy headers
    pub fn get_connection_info(req: &HttpRequest) -> ConnectionInfo {
        let proxy_headers = ProxyHeaders::from_request(req);
        let connection_info = req.connection_info();
        
        // Use proxy headers if available, otherwise fall back to connection info
        let scheme = if proxy_headers.is_proxied {
            proxy_headers.get_original_scheme().to_string()
        } else {
            connection_info.scheme().to_string()
        };
        
        let host = proxy_headers.original_host
            .unwrap_or_else(|| connection_info.host().to_string());
        
        let remote_addr = proxy_headers.client_ip
            .or_else(|| connection_info.peer_addr().and_then(|addr| addr.parse().ok()));
        
        ConnectionInfo {
            scheme,
            host,
            remote_addr,
            is_proxied: proxy_headers.is_proxied,
        }
    }
    
    /// Check if request should be treated as HTTPS based on headers
    pub fn is_secure_request(req: &HttpRequest) -> bool {
        let proxy_headers = ProxyHeaders::from_request(req);
        
        if proxy_headers.is_proxied {
            proxy_headers.is_original_https()
        } else {
            req.connection_info().scheme() == "https"
        }
    }
    
    /// Get the real client IP address, considering proxy headers
    pub fn get_real_client_ip(req: &HttpRequest) -> Option<IpAddr> {
        let proxy_headers = ProxyHeaders::from_request(req);
        let fallback_ip = req.connection_info().peer_addr()
            .and_then(|addr| IpAddr::from_str(addr).ok());
        
        proxy_headers.get_client_ip(fallback_ip)
    }
}

/// Connection information extracted from request
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    /// Request scheme (http/https)
    pub scheme: String,
    /// Host header value
    pub host: String,
    /// Remote client IP address
    pub remote_addr: Option<IpAddr>,
    /// Whether request came through a proxy
    pub is_proxied: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test::TestRequest;
    
    #[test]
    fn test_proxy_headers_extraction() {
        let req = TestRequest::default()
            .insert_header(("x-forwarded-for", "203.0.113.1, 198.51.100.1"))
            .insert_header(("x-forwarded-proto", "https"))
            .insert_header(("x-forwarded-host", "example.com"))
            .insert_header(("x-real-ip", "203.0.113.1"))
            .to_http_request();
        
        let headers = ProxyHeaders::from_request(&req);
        
        assert!(headers.is_proxied);
        assert_eq!(headers.client_ip, Some(IpAddr::from_str("203.0.113.1").unwrap()));
        assert_eq!(headers.original_proto, Some("https".to_string()));
        assert_eq!(headers.original_host, Some("example.com".to_string()));
        assert!(headers.is_original_https());
        assert_eq!(headers.forwarded_for.len(), 2);
    }
    
    #[test]
    fn test_forwarded_for_parsing() {
        let xff = "203.0.113.1, 198.51.100.1, 192.168.1.1";
        let chain = ProxyHeaders::parse_forwarded_for_chain(xff);
        
        assert_eq!(chain.len(), 3);
        assert_eq!(chain[0], IpAddr::from_str("203.0.113.1").unwrap());
        assert_eq!(chain[1], IpAddr::from_str("198.51.100.1").unwrap());
        assert_eq!(chain[2], IpAddr::from_str("192.168.1.1").unwrap());
    }
    
    #[test]
    fn test_no_proxy_headers() {
        let req = TestRequest::default().to_http_request();
        let headers = ProxyHeaders::from_request(&req);
        
        assert!(!headers.is_proxied);
        assert!(headers.client_ip.is_none());
        assert!(headers.original_proto.is_none());
        assert!(!headers.is_original_https());
    }
}
