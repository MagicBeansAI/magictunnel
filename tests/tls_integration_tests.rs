//! TLS Integration Tests
//! 
//! Comprehensive integration tests for the entire TLS and security system.
//! Moved from src/tls/tests/integration_tests.rs to follow Rust best practices.

mod tls_test_utils;
use tls_test_utils::{TlsTestUtils, TestResultAggregator, TestResult, TestStatus};

use magictunnel::tls::*;
use magictunnel::config::{TlsMode, TlsConfig};
use magictunnel::error::Result;
use actix_web::test::TestRequest;
use std::time::Duration;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_end_to_end_tls_flow() {
        // Test complete TLS configuration and validation flow
        let config = TlsTestUtils::create_test_tls_config();
        
        // Validate configuration
        assert!(TlsTestUtils::validate_tls_config(&config).is_ok());
        
        // Test request processing
        let req = TlsTestUtils::create_proxied_request();
        let headers = ProxyHeaders::from_request(&req);
        
        assert!(headers.is_proxied);
        assert!(headers.is_original_https());
        assert!(headers.client_ip.is_some());
    }
    
    #[test]
    fn test_security_features_integration() {
        let config = TlsTestUtils::create_test_tls_config();
        
        // Test HSTS configuration
        assert!(config.hsts_enabled);
        assert_eq!(config.hsts_max_age, 31536000);
        assert!(config.hsts_include_subdomains);
        assert!(config.hsts_preload);
        
        // Test TLS version requirements
        assert_eq!(config.min_tls_version, "1.2");
        
        // Test cipher suite configuration
        assert!(config.cipher_suites.is_some());
        let cipher_suites = config.cipher_suites.unwrap();
        assert!(cipher_suites.contains(&"TLS_AES_256_GCM_SHA384".to_string()));
    }
    
    #[test]
    fn test_auto_detection_scenarios() {
        let config = TlsTestUtils::create_auto_config();
        assert_eq!(config.mode, TlsMode::Auto);
        assert_eq!(config.fallback_mode, TlsMode::Application);
        
        // Test auto-detection headers
        assert!(!config.auto_detect_headers.is_empty());
        assert!(config.auto_detect_headers.contains(&"X-Forwarded-Proto".to_string()));
        assert!(config.auto_detect_headers.contains(&"X-Forwarded-For".to_string()));
        assert!(config.auto_detect_headers.contains(&"X-Real-IP".to_string()));
    }
    
    #[test]
    fn test_behind_proxy_configuration() {
        let config = TlsTestUtils::create_behind_proxy_config();
        
        assert_eq!(config.mode, TlsMode::BehindProxy);
        assert!(config.behind_proxy);
        assert!(config.require_forwarded_proto);
        assert!(config.require_forwarded_for);
        
        // Should not require cert/key files when behind proxy
        assert!(config.cert_file.is_none());
        assert!(config.key_file.is_none());
        
        // Should have trusted proxies configured
        assert!(!config.trusted_proxies.is_empty());
    }
    
    #[test]
    fn test_trusted_proxy_validation() {
        let config = TlsTestUtils::create_behind_proxy_config();
        
        // Test that trusted proxy ranges are properly configured
        let trusted_proxies = &config.trusted_proxies;
        assert!(trusted_proxies.contains(&"127.0.0.1/32".to_string()));
        assert!(trusted_proxies.contains(&"10.0.0.0/8".to_string()));
        assert!(trusted_proxies.contains(&"192.168.0.0/16".to_string()));
        assert!(trusted_proxies.contains(&"203.0.113.0/24".to_string()));
    }
    
    #[test]
    fn test_request_processing_pipeline() {
        // Test the complete request processing pipeline
        let proxied_req = TlsTestUtils::create_proxied_request();
        let direct_req = TlsTestUtils::create_direct_request();
        let suspicious_req = TlsTestUtils::create_suspicious_request();
        
        // Process proxied request
        let proxied_headers = ProxyHeaders::from_request(&proxied_req);
        assert!(proxied_headers.is_proxied);
        assert!(proxied_headers.is_original_https());
        
        // Process direct request
        let direct_headers = ProxyHeaders::from_request(&direct_req);
        assert!(!direct_headers.is_proxied);
        
        // Process suspicious request
        let suspicious_headers = ProxyHeaders::from_request(&suspicious_req);
        assert!(suspicious_headers.is_proxied);
        assert!(!suspicious_headers.is_original_https()); // HTTP, not HTTPS
    }
    
    #[test]
    fn test_configuration_validation_edge_cases() {
        // Test various configuration edge cases
        
        // Application mode without certificates should fail
        let mut invalid_config = TlsTestUtils::create_test_tls_config();
        invalid_config.cert_file = None;
        invalid_config.key_file = None;
        assert!(TlsTestUtils::validate_tls_config(&invalid_config).is_err());
        
        // Behind proxy mode without trusted proxies should fail
        let mut invalid_proxy_config = TlsTestUtils::create_behind_proxy_config();
        invalid_proxy_config.trusted_proxies.clear();
        assert!(TlsTestUtils::validate_tls_config(&invalid_proxy_config).is_err());
        
        // Auto mode without detection headers should fail
        let mut invalid_auto_config = TlsTestUtils::create_auto_config();
        invalid_auto_config.auto_detect_headers.clear();
        assert!(TlsTestUtils::validate_tls_config(&invalid_auto_config).is_err());
        
        // Disabled mode should always be valid
        let mut disabled_config = TlsTestUtils::create_test_tls_config();
        disabled_config.mode = TlsMode::Disabled;
        disabled_config.cert_file = None;
        disabled_config.key_file = None;
        disabled_config.trusted_proxies.clear();
        disabled_config.auto_detect_headers.clear();
        assert!(TlsTestUtils::validate_tls_config(&disabled_config).is_ok());
    }
    
    #[test]
    fn test_performance_characteristics() {
        // Test performance characteristics of header parsing
        let requests = TlsTestUtils::generate_test_requests(1000);
        
        let (_, duration) = TlsTestUtils::measure_execution_time(|| {
            for req in requests {
                let _headers = ProxyHeaders::from_request(&req);
            }
        });
        
        // Should process 1000 requests in under 100ms
        assert!(duration.as_millis() < 100, 
                "Performance test failed: took {}ms", duration.as_millis());
    }
    
    #[test]
    fn test_header_parsing_robustness() {
        // Test robustness of header parsing with various edge cases
        
        // Empty headers
        let empty_req = TestRequest::default().to_http_request();
        let empty_headers = ProxyHeaders::from_request(&empty_req);
        assert!(!empty_headers.is_proxied);
        
        // Headers with unusual values
        let unusual_req = TestRequest::default()
            .insert_header(("x-forwarded-proto", "unknown-protocol"))
            .insert_header(("x-forwarded-for", "not-an-ip"))
            .to_http_request();
        let unusual_headers = ProxyHeaders::from_request(&unusual_req);
        assert!(unusual_headers.is_proxied); // Header present, so considered proxied
        assert_eq!(unusual_headers.original_proto, Some("unknown-protocol".to_string()));
        assert!(unusual_headers.client_ip.is_none()); // Invalid IP should be None
        
        // Very long header values
        let long_value = "x".repeat(10000);
        let long_req = TestRequest::default()
            .insert_header(("x-forwarded-proto", long_value.as_str()))
            .to_http_request();
        let long_headers = ProxyHeaders::from_request(&long_req);
        assert!(long_headers.is_proxied);
        assert_eq!(long_headers.original_proto, Some(long_value));
    }
    
    #[test]
    fn test_configuration_equivalence() {
        // Test that configuration comparison works correctly
        let config1 = TlsTestUtils::create_test_tls_config();
        let config2 = TlsTestUtils::create_test_tls_config();
        
        // Should be equivalent
        TlsTestUtils::assert_tls_config_eq(&config1, &config2);
        
        // Modify one and test inequality
        let mut config3 = TlsTestUtils::create_test_tls_config();
        config3.hsts_max_age = 86400; // Different value
        
        // This should panic if configs are not equal (which is expected)
        let result = std::panic::catch_unwind(|| {
            TlsTestUtils::assert_tls_config_eq(&config1, &config3);
        });
        assert!(result.is_err()); // Should panic due to inequality
    }
    
    #[test]
    fn test_environment_variable_integration() {
        // Test environment variable handling
        let env_vars = TlsTestUtils::create_test_env_vars();
        
        // Verify expected environment variables are present
        assert_eq!(env_vars.get("TLS_MODE"), Some(&"application".to_string()));
        assert_eq!(env_vars.get("TLS_CERT_FILE"), Some(&"/etc/ssl/certs/server.crt".to_string()));
        assert_eq!(env_vars.get("TLS_KEY_FILE"), Some(&"/etc/ssl/private/server.key".to_string()));
        assert_eq!(env_vars.get("TLS_MIN_VERSION"), Some(&"1.2".to_string()));
        assert_eq!(env_vars.get("HSTS_ENABLED"), Some(&"true".to_string()));
        assert_eq!(env_vars.get("HSTS_MAX_AGE"), Some(&"31536000".to_string()));
        assert_eq!(env_vars.get("TRUSTED_PROXIES"), Some(&"127.0.0.1/32,10.0.0.0/8".to_string()));
    }
    
    #[test]
    fn test_certificate_data_handling() {
        // Test certificate data handling
        let cert_data = TlsTestUtils::create_test_certificate_data();
        
        // Should be valid certificate-like data
        assert!(!cert_data.is_empty());
        assert!(String::from_utf8_lossy(&cert_data).contains("BEGIN CERTIFICATE"));
        assert!(String::from_utf8_lossy(&cert_data).contains("END CERTIFICATE"));
    }
    
    #[test]
    fn test_multiple_proxy_scenarios() {
        // Test various proxy scenarios
        
        // Cloudflare-style proxy
        let cf_req = TestRequest::default()
            .insert_header(("cf-connecting-ip", "203.0.113.1"))
            .insert_header(("cf-visitor", r#"{"scheme":"https"}"#))
            .insert_header(("x-forwarded-proto", "https"))
            .to_http_request();
        let cf_headers = ProxyHeaders::from_request(&cf_req);
        assert!(cf_headers.is_proxied);
        assert!(cf_headers.is_original_https());
        
        // AWS ALB-style proxy
        let alb_req = TestRequest::default()
            .insert_header(("x-forwarded-proto", "https"))
            .insert_header(("x-forwarded-port", "443"))
            .insert_header(("x-forwarded-for", "203.0.113.1"))
            .insert_header(("x-amzn-trace-id", "Root=1-5e1b4151-5ac6c58f5b3daa6b32e4f2e8"))
            .to_http_request();
        let alb_headers = ProxyHeaders::from_request(&alb_req);
        assert!(alb_headers.is_proxied);
        assert!(alb_headers.is_original_https());
        
        // Nginx-style proxy
        let nginx_req = TestRequest::default()
            .insert_header(("x-forwarded-proto", "https"))
            .insert_header(("x-forwarded-for", "203.0.113.1"))
            .insert_header(("x-real-ip", "203.0.113.1"))
            .insert_header(("x-forwarded-host", "api.example.com"))
            .to_http_request();
        let nginx_headers = ProxyHeaders::from_request(&nginx_req);
        assert!(nginx_headers.is_proxied);
        assert!(nginx_headers.is_original_https());
        assert_eq!(nginx_headers.original_host, Some("api.example.com".to_string()));
    }
}
