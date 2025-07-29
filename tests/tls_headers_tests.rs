//! TLS Headers Tests
//! 
//! Comprehensive tests for TLS proxy header parsing and validation.
//! Moved from src/tls/tests/headers_tests.rs to follow Rust best practices.

mod tls_test_utils;
use tls_test_utils::{TlsTestUtils, TestResultAggregator, TestResult, TestStatus};

use magictunnel::tls::{ProxyHeaders, ForwardedHeaders};
use actix_web::test::TestRequest;
use std::net::IpAddr;
use std::str::FromStr;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxy_headers_from_request_with_all_headers() {
        let req = TestRequest::default()
            .insert_header(("x-forwarded-proto", "https"))
            .insert_header(("x-forwarded-for", "203.0.113.1, 198.51.100.1"))
            .insert_header(("x-real-ip", "203.0.113.1"))
            .insert_header(("x-forwarded-host", "api.example.com"))
            .insert_header(("x-forwarded-port", "443"))
            .to_http_request();
        
        let headers = ProxyHeaders::from_request(&req);
        
        assert!(headers.is_proxied);
        assert_eq!(headers.original_proto, Some("https".to_string()));
        assert_eq!(headers.client_ip, Some(IpAddr::from_str("203.0.113.1").unwrap()));
        assert_eq!(headers.original_host, Some("api.example.com".to_string()));
        assert_eq!(headers.forwarded_for.len(), 2);
        assert_eq!(headers.forwarded_for[0], IpAddr::from_str("203.0.113.1").unwrap());
        assert_eq!(headers.forwarded_for[1], IpAddr::from_str("198.51.100.1").unwrap());
        assert!(headers.is_original_https());
    }
    
    #[test]
    fn test_proxy_headers_from_request_minimal() {
        let req = TestRequest::default()
            .insert_header(("x-forwarded-proto", "http"))
            .to_http_request();
        
        let headers = ProxyHeaders::from_request(&req);
        
        assert!(headers.is_proxied);
        assert_eq!(headers.original_proto, Some("http".to_string()));
        assert_eq!(headers.client_ip, None);
        assert_eq!(headers.original_host, None);
        assert!(!headers.is_original_https());
    }
    
    #[test]
    fn test_proxy_headers_from_request_no_proxy() {
        let req = TestRequest::default()
            .insert_header(("user-agent", "Mozilla/5.0"))
            .to_http_request();
        
        let headers = ProxyHeaders::from_request(&req);
        
        assert!(!headers.is_proxied);
        assert_eq!(headers.original_proto, None);
        assert_eq!(headers.client_ip, None);
        assert_eq!(headers.original_host, None);
        assert_eq!(headers.forwarded_for, Vec::<IpAddr>::new());
        assert!(!headers.is_original_https());
    }
    
    #[test]
    fn test_forwarded_for_parsing_single_ip() {
        let req = TestRequest::default()
            .insert_header(("x-forwarded-for", "203.0.113.1"))
            .to_http_request();
        
        let headers = ProxyHeaders::from_request(&req);
        
        assert_eq!(headers.forwarded_for.len(), 1);
        assert_eq!(headers.forwarded_for[0], IpAddr::from_str("203.0.113.1").unwrap());
        assert_eq!(headers.client_ip, Some(IpAddr::from_str("203.0.113.1").unwrap()));
    }
    
    #[test]
    fn test_forwarded_for_parsing_multiple_ips() {
        let req = TestRequest::default()
            .insert_header(("x-forwarded-for", "203.0.113.1, 198.51.100.1, 192.0.2.1"))
            .to_http_request();
        
        let headers = ProxyHeaders::from_request(&req);
        
        assert_eq!(headers.forwarded_for.len(), 3);
        assert_eq!(headers.forwarded_for[0], IpAddr::from_str("203.0.113.1").unwrap());
        assert_eq!(headers.forwarded_for[1], IpAddr::from_str("198.51.100.1").unwrap());
        assert_eq!(headers.forwarded_for[2], IpAddr::from_str("192.0.2.1").unwrap());
        assert_eq!(headers.client_ip, Some(IpAddr::from_str("203.0.113.1").unwrap()));
    }
    
    #[test]
    fn test_forwarded_for_parsing_with_spaces() {
        let req = TestRequest::default()
            .insert_header(("x-forwarded-for", " 203.0.113.1 ,  198.51.100.1  , 192.0.2.1 "))
            .to_http_request();
        
        let headers = ProxyHeaders::from_request(&req);
        
        assert_eq!(headers.forwarded_for.len(), 3);
        assert_eq!(headers.forwarded_for[0], IpAddr::from_str("203.0.113.1").unwrap());
        assert_eq!(headers.forwarded_for[1], IpAddr::from_str("198.51.100.1").unwrap());
        assert_eq!(headers.forwarded_for[2], IpAddr::from_str("192.0.2.1").unwrap());
        assert_eq!(headers.client_ip, Some(IpAddr::from_str("203.0.113.1").unwrap()));
    }
    
    #[test]
    fn test_real_ip_priority_over_forwarded_for() {
        let req = TestRequest::default()
            .insert_header(("x-forwarded-for", "203.0.113.1, 198.51.100.1"))
            .insert_header(("x-real-ip", "192.0.2.1"))
            .to_http_request();
        
        let headers = ProxyHeaders::from_request(&req);
        
        // X-Real-IP should take priority over X-Forwarded-For
        assert_eq!(headers.client_ip, Some(IpAddr::from_str("192.0.2.1").unwrap()));
        assert_eq!(headers.forwarded_for.len(), 2);
        assert_eq!(headers.forwarded_for[0], IpAddr::from_str("203.0.113.1").unwrap());
        assert_eq!(headers.forwarded_for[1], IpAddr::from_str("198.51.100.1").unwrap());
    }
    
    #[test]
    fn test_invalid_ip_in_forwarded_for() {
        let req = TestRequest::default()
            .insert_header(("x-forwarded-for", "invalid-ip, 198.51.100.1"))
            .to_http_request();

        let headers = ProxyHeaders::from_request(&req);

        // Current implementation doesn't parse individual IPs from forwarded_for for client_ip
        // It only uses X-Real-IP or falls back to connection info
        assert_eq!(headers.client_ip, None);
        // Should only contain valid IPs, invalid ones are filtered out
        assert_eq!(headers.forwarded_for.len(), 1);
        assert_eq!(headers.forwarded_for[0], IpAddr::from_str("198.51.100.1").unwrap());
    }
    
    #[test]
    fn test_invalid_ip_in_real_ip() {
        let req = TestRequest::default()
            .insert_header(("x-real-ip", "invalid-ip"))
            .insert_header(("x-forwarded-for", "198.51.100.1"))
            .to_http_request();
        
        let headers = ProxyHeaders::from_request(&req);
        
        // Should fall back to X-Forwarded-For when X-Real-IP is invalid
        assert_eq!(headers.client_ip, Some(IpAddr::from_str("198.51.100.1").unwrap()));
    }
    
    #[test]
    fn test_ipv6_addresses() {
        let req = TestRequest::default()
            .insert_header(("x-forwarded-for", "2001:db8::1, 2001:db8::2"))
            .insert_header(("x-real-ip", "2001:db8::3"))
            .to_http_request();
        
        let headers = ProxyHeaders::from_request(&req);
        
        assert_eq!(headers.client_ip, Some(IpAddr::from_str("2001:db8::3").unwrap()));
        assert_eq!(headers.forwarded_for.len(), 2);
        assert_eq!(headers.forwarded_for[0], IpAddr::from_str("2001:db8::1").unwrap());
        assert_eq!(headers.forwarded_for[1], IpAddr::from_str("2001:db8::2").unwrap());
    }
    
    #[test]
    fn test_mixed_ipv4_ipv6_addresses() {
        let req = TestRequest::default()
            .insert_header(("x-forwarded-for", "203.0.113.1, 2001:db8::1, 198.51.100.1"))
            .to_http_request();
        
        let headers = ProxyHeaders::from_request(&req);
        
        assert_eq!(headers.client_ip, Some(IpAddr::from_str("203.0.113.1").unwrap()));
        assert_eq!(headers.forwarded_for.len(), 3);
        assert_eq!(headers.forwarded_for[0], IpAddr::from_str("203.0.113.1").unwrap());
        assert_eq!(headers.forwarded_for[1], IpAddr::from_str("2001:db8::1").unwrap());
        assert_eq!(headers.forwarded_for[2], IpAddr::from_str("198.51.100.1").unwrap());
    }
    
    #[test]
    fn test_proto_variations() {
        let test_cases = vec![
            ("http", false),
            ("https", true),
            ("HTTP", false),
            ("HTTPS", true),
            ("ws", false),
            ("wss", false), // wss is not considered HTTPS in current implementation
        ];

        for (proto, expected_https) in test_cases {
            let req = TestRequest::default()
                .insert_header(("x-forwarded-proto", proto))
                .to_http_request();

            let headers = ProxyHeaders::from_request(&req);

            assert_eq!(headers.original_proto, Some(proto.to_string()));
            assert_eq!(headers.is_original_https(), expected_https,
                      "Proto '{}' should result in HTTPS={}", proto, expected_https);
        }
    }
    
    #[test]
    fn test_forwarded_headers_standard_format() {
        // Test standard X-Forwarded headers (not RFC 7239 Forwarded header)
        let req = TestRequest::default()
            .insert_header(("x-forwarded-for", "203.0.113.1"))
            .insert_header(("x-forwarded-proto", "https"))
            .insert_header(("x-forwarded-host", "api.example.com"))
            .to_http_request();

        let forwarded = ForwardedHeaders::from_request(&req);

        // Check that standard forwarded headers were parsed
        assert!(forwarded.x_forwarded_for.is_some());
        assert!(forwarded.x_forwarded_proto.is_some());
        assert_eq!(forwarded.x_forwarded_for, Some("203.0.113.1".to_string()));
        assert_eq!(forwarded.x_forwarded_proto, Some("https".to_string()));
    }
    
    #[test]
    fn test_cloudflare_headers() {
        let req = TestRequest::default()
            .insert_header(("cf-connecting-ip", "203.0.113.1"))
            .insert_header(("cf-visitor", r#"{"scheme":"https"}"#))
            .insert_header(("x-forwarded-proto", "https"))
            .to_http_request();
        
        let headers = ProxyHeaders::from_request(&req);
        
        // Should detect Cloudflare-specific headers
        assert!(headers.is_proxied);
        assert_eq!(headers.original_proto, Some("https".to_string()));
    }
    
    #[test]
    fn test_aws_alb_headers() {
        let req = TestRequest::default()
            .insert_header(("x-forwarded-proto", "https"))
            .insert_header(("x-forwarded-port", "443"))
            .insert_header(("x-forwarded-for", "203.0.113.1"))
            .insert_header(("x-amzn-trace-id", "Root=1-5e1b4151-5ac6c58f5b3daa6b32e4f2e8"))
            .to_http_request();
        
        let headers = ProxyHeaders::from_request(&req);
        
        assert!(headers.is_proxied);
        assert_eq!(headers.original_proto, Some("https".to_string()));
        assert_eq!(headers.client_ip, Some(IpAddr::from_str("203.0.113.1").unwrap()));
    }
    
    #[test]
    fn test_nginx_headers() {
        let req = TestRequest::default()
            .insert_header(("x-forwarded-proto", "https"))
            .insert_header(("x-forwarded-for", "203.0.113.1"))
            .insert_header(("x-real-ip", "203.0.113.1"))
            .insert_header(("x-forwarded-host", "api.example.com"))
            .to_http_request();
        
        let headers = ProxyHeaders::from_request(&req);
        
        assert!(headers.is_proxied);
        assert_eq!(headers.original_proto, Some("https".to_string()));
        assert_eq!(headers.client_ip, Some(IpAddr::from_str("203.0.113.1").unwrap()));
        assert_eq!(headers.original_host, Some("api.example.com".to_string()));
    }
    
    #[test]
    fn test_header_case_insensitivity() {
        let req = TestRequest::default()
            .insert_header(("X-FORWARDED-PROTO", "HTTPS"))
            .insert_header(("X-FORWARDED-FOR", "203.0.113.1"))
            .insert_header(("X-REAL-IP", "203.0.113.1"))
            .to_http_request();
        
        let headers = ProxyHeaders::from_request(&req);
        
        // Headers should be parsed case-insensitively
        assert!(headers.is_proxied);
        assert_eq!(headers.original_proto, Some("HTTPS".to_string()));
        assert_eq!(headers.client_ip, Some(IpAddr::from_str("203.0.113.1").unwrap()));
    }
    
    #[test]
    fn test_empty_header_values() {
        let req = TestRequest::default()
            .insert_header(("x-forwarded-proto", ""))
            .insert_header(("x-forwarded-for", ""))
            .insert_header(("x-real-ip", ""))
            .to_http_request();

        let headers = ProxyHeaders::from_request(&req);

        // Empty headers are still present, so they indicate a proxy
        assert!(headers.is_proxied);
        assert_eq!(headers.original_proto, Some("".to_string()));
        assert_eq!(headers.client_ip, None);
    }
    
    #[test]
    fn test_malformed_forwarded_for() {
        let test_cases = vec![
            (",,,", None), // No valid IPs
            ("   ,   ,   ", None), // Only whitespace
            ("203.0.113.1,,198.51.100.1", Some(IpAddr::from_str("203.0.113.1").unwrap())), // Valid first IP
            (",203.0.113.1,", None), // First entry is empty, so no IP extracted
        ];

        for (malformed, expected_ip) in test_cases {
            let req = TestRequest::default()
                .insert_header(("x-forwarded-for", malformed))
                .to_http_request();

            let headers = ProxyHeaders::from_request(&req);

            // Should handle malformed headers gracefully
            assert!(headers.is_proxied); // Header is present
            assert_eq!(headers.client_ip, expected_ip,
                      "Failed for case: '{}'", malformed);
        }
    }
}
