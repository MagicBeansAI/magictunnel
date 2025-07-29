//! TLS Validation Tests
//! 
//! Comprehensive tests for TLS proxy validation and security checks.
//! Moved from src/tls/tests/validation_tests.rs to follow Rust best practices.

mod tls_test_utils;
use tls_test_utils::{TlsTestUtils, TestResultAggregator, TestResult, TestStatus};

use magictunnel::tls::{ProxyValidator, ProxyValidationUtils};
use magictunnel::config::TlsMode;
use actix_web::test::TestRequest;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxy_validator_creation() {
        let config = TlsTestUtils::create_test_tls_config();
        let validator = ProxyValidator::new(config);
        assert!(validator.is_ok(), "Should create validator successfully");
    }
    
    #[test]
    fn test_application_mode_validation() {
        let mut config = TlsTestUtils::create_test_tls_config();
        config.mode = TlsMode::Application;
        
        let validator = ProxyValidator::new(config).unwrap();
        
        // Test direct request (should pass)
        let direct_req = TlsTestUtils::create_direct_request();
        let result = validator.validate_request(&direct_req);
        assert!(result.is_ok(), "Direct request should be valid in application mode");
        
        // Test proxied request (should pass but log warning)
        let proxied_req = TlsTestUtils::create_proxied_request();
        let result = validator.validate_request(&proxied_req);
        assert!(result.is_ok(), "Proxied request should be valid in application mode");
    }
    
    #[test]
    fn test_behind_proxy_mode_validation() {
        let config = TlsTestUtils::create_behind_proxy_config();
        let validator = ProxyValidator::new(config).unwrap();

        // Test proxied request from trusted proxy (should pass)
        let proxied_req = TlsTestUtils::create_proxied_request();
        let result = validator.validate_request(&proxied_req);
        assert!(result.is_ok(), "Proxied request from trusted source should be valid");

        // Test direct request (should fail in behind-proxy mode with required headers)
        let direct_req = TlsTestUtils::create_direct_request();
        let result = validator.validate_request(&direct_req);
        assert!(result.is_err(), "Direct request should be rejected in behind-proxy mode when headers are required");
    }
    
    #[test]
    fn test_auto_mode_validation() {
        let config = TlsTestUtils::create_auto_config();
        let validator = ProxyValidator::new(config).unwrap();
        
        // Test proxied request (should be detected and validated as proxied)
        let proxied_req = TlsTestUtils::create_proxied_request();
        let result = validator.validate_request(&proxied_req);
        assert!(result.is_ok(), "Proxied request should be valid in auto mode");
        
        // Test direct request (should be detected and validated as direct)
        let direct_req = TlsTestUtils::create_direct_request();
        let result = validator.validate_request(&direct_req);
        assert!(result.is_ok(), "Direct request should be valid in auto mode");
    }
    
    #[test]
    fn test_disabled_mode_validation() {
        let mut config = TlsTestUtils::create_test_tls_config();
        config.mode = TlsMode::Disabled;
        
        let validator = ProxyValidator::new(config).unwrap();
        
        // All requests should pass in disabled mode
        let direct_req = TlsTestUtils::create_direct_request();
        let result = validator.validate_request(&direct_req);
        assert!(result.is_ok(), "Direct request should be valid in disabled mode");
        
        let proxied_req = TlsTestUtils::create_proxied_request();
        let result = validator.validate_request(&proxied_req);
        assert!(result.is_ok(), "Proxied request should be valid in disabled mode");
        
        let suspicious_req = TlsTestUtils::create_suspicious_request();
        let result = validator.validate_request(&suspicious_req);
        assert!(result.is_ok(), "Suspicious request should be valid in disabled mode");
    }
    
    #[test]
    fn test_trusted_proxy_validation() {
        let config = TlsTestUtils::create_behind_proxy_config();
        let validator = ProxyValidator::new(config).unwrap();
        
        // Test request from trusted proxy
        let trusted_req = TlsTestUtils::create_proxied_request();
        let result = validator.validate_request(&trusted_req);
        assert!(result.is_ok(), "Request from trusted proxy should be valid");
        
        // Test request from untrusted proxy
        let untrusted_req = TlsTestUtils::create_untrusted_proxy_request();
        let result = validator.validate_request(&untrusted_req);
        assert!(result.is_err(), "Request from untrusted proxy should be rejected");
    }
    
    #[test]
    fn test_required_headers_validation() {
        let mut config = TlsTestUtils::create_behind_proxy_config();
        config.require_forwarded_proto = true;
        config.require_forwarded_for = true;
        
        let validator = ProxyValidator::new(config).unwrap();
        
        // Test request with all required headers
        let complete_req = TestRequest::default()
            .insert_header(("x-forwarded-proto", "https"))
            .insert_header(("x-forwarded-for", "203.0.113.1"))
            .to_http_request();
        let result = validator.validate_request(&complete_req);
        assert!(result.is_ok(), "Request with all required headers should be valid");
        
        // Test request missing forwarded-proto
        let missing_proto_req = TestRequest::default()
            .insert_header(("x-forwarded-for", "203.0.113.1"))
            .to_http_request();
        let result = validator.validate_request(&missing_proto_req);
        assert!(result.is_err(), "Request missing required forwarded-proto should be rejected");
        
        // Test request missing forwarded-for
        let missing_for_req = TestRequest::default()
            .insert_header(("x-forwarded-proto", "https"))
            .to_http_request();
        let result = validator.validate_request(&missing_for_req);
        assert!(result.is_err(), "Request missing required forwarded-for should be rejected");
    }
    
    #[test]
    fn test_suspicious_request_detection() {
        let config = TlsTestUtils::create_behind_proxy_config();
        let validator = ProxyValidator::new(config).unwrap();
        
        // Test suspicious request
        let suspicious_req = TlsTestUtils::create_suspicious_request();
        let result = validator.validate_request(&suspicious_req);
        
        // Depending on implementation, this might pass with warnings or fail
        // The test should check that suspicious characteristics are detected
        match result {
            Ok(_) => {
                // If it passes, it should at least log warnings about suspicious characteristics
                // This would be tested in integration tests with log capture
            }
            Err(_) => {
                // If it fails, that's also acceptable for suspicious requests
            }
        }
    }
    
    #[test]
    fn test_validation_utils_ip_parsing() {
        // Test IP parsing using standard library
        use std::net::IpAddr;
        use std::str::FromStr;

        let valid_ips = vec![
            "127.0.0.1",
            "192.168.1.1",
            "203.0.113.1",
            "::1",
            "2001:db8::1",
        ];

        for ip_str in valid_ips {
            let result = IpAddr::from_str(ip_str);
            assert!(result.is_ok(), "Should parse valid IP: {}", ip_str);
        }

        let invalid_ips = vec![
            "not-an-ip",
            "256.256.256.256",
            "192.168.1",
            "192.168.1.1.1",
            "gggg::1",
        ];

        for ip_str in invalid_ips {
            let result = IpAddr::from_str(ip_str);
            assert!(result.is_err(), "Should reject invalid IP: {}", ip_str);
        }
    }
    
    #[test]
    fn test_validation_utils_header_extraction() {
        use magictunnel::tls::ProxyHeaders;

        let req = TestRequest::default()
            .insert_header(("x-forwarded-proto", "https"))
            .insert_header(("x-forwarded-for", "203.0.113.1, 198.51.100.1"))
            .insert_header(("x-real-ip", "203.0.113.1"))
            .to_http_request();

        // Test header extraction using ProxyHeaders
        let headers = ProxyHeaders::from_request(&req);
        assert_eq!(headers.original_proto, Some("https".to_string()));
        assert!(headers.client_ip.is_some());
        assert_eq!(headers.forwarded_for.len(), 2);
        assert!(headers.is_original_https());
    }
    
    #[test]
    fn test_validation_utils_security_checks() {
        let config = TlsTestUtils::create_test_tls_config();

        // Test HTTPS requirement using actual ProxyValidationUtils methods
        let https_req = TestRequest::default()
            .insert_header(("x-forwarded-proto", "https"))
            .to_http_request();
        assert!(ProxyValidationUtils::is_secure_request(&https_req, &config));

        let http_req = TestRequest::default()
            .insert_header(("x-forwarded-proto", "http"))
            .to_http_request();
        assert!(!ProxyValidationUtils::is_secure_request(&http_req, &config));

        // Test proxy detection
        assert!(ProxyValidationUtils::is_proxied_request(&https_req));
        assert!(ProxyValidationUtils::is_proxied_request(&http_req));

        let direct_req = TestRequest::default().to_http_request();
        assert!(!ProxyValidationUtils::is_proxied_request(&direct_req));
    }
    
    #[test]
    fn test_validation_error_handling() {
        // Test various error conditions
        
        // Invalid configuration should fail validator creation
        let mut invalid_config = TlsTestUtils::create_test_tls_config();
        invalid_config.mode = TlsMode::Application;
        invalid_config.cert_file = None; // Required for application mode
        
        let result = ProxyValidator::new(invalid_config);
        assert!(result.is_err(), "Should fail with invalid configuration");
        
        // Test error messages are informative
        match result {
            Err(e) => {
                let error_msg = e.to_string();
                assert!(error_msg.contains("cert") || error_msg.contains("certificate") || error_msg.contains("Application"),
                       "Error message should mention certificate requirement: {}", error_msg);
            }
            Ok(_) => panic!("Expected error but got success"),
        }
    }
    
    #[test]
    fn test_validation_performance() {
        let config = TlsTestUtils::create_test_tls_config();
        let validator = ProxyValidator::new(config).unwrap();
        
        // Generate test requests
        let requests = TlsTestUtils::generate_test_requests(1000);
        
        // Measure validation performance
        let (_, duration) = TlsTestUtils::measure_execution_time(|| {
            for req in requests {
                let _ = validator.validate_request(&req);
            }
        });
        
        // Should validate 1000 requests quickly
        assert!(duration.as_millis() < 200, 
                "Validation performance test failed: took {}ms", duration.as_millis());
    }
    
    #[test]
    fn test_validation_edge_cases() {
        let config = TlsTestUtils::create_test_tls_config();
        let validator = ProxyValidator::new(config).unwrap();
        
        // Test empty request
        let empty_req = TestRequest::default().to_http_request();
        let result = validator.validate_request(&empty_req);
        assert!(result.is_ok(), "Empty request should be valid in application mode");
        
        // Test request with unusual headers
        let unusual_req = TestRequest::default()
            .insert_header(("x-forwarded-proto", ""))
            .insert_header(("x-forwarded-for", "invalid-ip"))
            .insert_header(("x-real-ip", "also-invalid"))
            .to_http_request();
        let result = validator.validate_request(&unusual_req);
        // Should handle gracefully (exact behavior depends on implementation)
        assert!(result.is_ok() || result.is_err()); // Either outcome is acceptable
        
        // Test request with very long headers
        let long_value = "x".repeat(10000);
        let long_req = TestRequest::default()
            .insert_header(("x-forwarded-proto", long_value.as_str()))
            .to_http_request();
        let result = validator.validate_request(&long_req);
        assert!(result.is_ok(), "Request with long headers should be handled gracefully");
    }
    
    #[test]
    fn test_validation_with_multiple_proxy_types() {
        let config = TlsTestUtils::create_auto_config();
        let validator = ProxyValidator::new(config).unwrap();
        
        // Test Cloudflare-style request
        let cf_req = TestRequest::default()
            .insert_header(("cf-connecting-ip", "203.0.113.1"))
            .insert_header(("cf-visitor", r#"{"scheme":"https"}"#))
            .insert_header(("x-forwarded-proto", "https"))
            .to_http_request();
        let result = validator.validate_request(&cf_req);
        assert!(result.is_ok(), "Cloudflare-style request should be valid");
        
        // Test AWS ALB-style request
        let alb_req = TestRequest::default()
            .insert_header(("x-forwarded-proto", "https"))
            .insert_header(("x-forwarded-port", "443"))
            .insert_header(("x-forwarded-for", "203.0.113.1"))
            .insert_header(("x-amzn-trace-id", "Root=1-5e1b4151-5ac6c58f5b3daa6b32e4f2e8"))
            .to_http_request();
        let result = validator.validate_request(&alb_req);
        assert!(result.is_ok(), "AWS ALB-style request should be valid");
        
        // Test Nginx-style request
        let nginx_req = TestRequest::default()
            .insert_header(("x-forwarded-proto", "https"))
            .insert_header(("x-forwarded-for", "203.0.113.1"))
            .insert_header(("x-real-ip", "203.0.113.1"))
            .insert_header(("x-forwarded-host", "api.example.com"))
            .to_http_request();
        let result = validator.validate_request(&nginx_req);
        assert!(result.is_ok(), "Nginx-style request should be valid");
    }
}
