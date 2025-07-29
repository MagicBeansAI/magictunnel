//! TLS Test Utilities
//! 
//! Comprehensive test utilities for TLS and security testing.
//! Moved from src/tls/tests/mod.rs to follow Rust best practices.

use magictunnel::config::{TlsConfig, TlsMode};
use magictunnel::error::Result;
use actix_web::test::TestRequest;
use std::collections::HashMap;

/// Test utilities for TLS and security testing
pub struct TlsTestUtils;

impl TlsTestUtils {
    /// Create a basic TLS configuration for testing
    pub fn create_test_tls_config() -> TlsConfig {
        TlsConfig {
            mode: TlsMode::Application,
            cert_file: Some("test-cert.pem".to_string()),
            key_file: Some("test-key.pem".to_string()),
            ca_file: None,
            behind_proxy: false,
            trusted_proxies: vec![
                "127.0.0.1/32".to_string(),
                "10.0.0.0/8".to_string(),
                "192.168.0.0/16".to_string(),
                "203.0.113.0/24".to_string(), // TEST-NET-3 range for testing
            ],
            min_tls_version: "1.2".to_string(),
            cipher_suites: Some(vec![
                "TLS_AES_256_GCM_SHA384".to_string(),
                "TLS_CHACHA20_POLY1305_SHA256".to_string(),
            ]),
            hsts_enabled: true,
            hsts_max_age: 31536000,
            hsts_include_subdomains: true,
            hsts_preload: true,
            require_forwarded_proto: false,
            require_forwarded_for: false,
            auto_detect_headers: vec![
                "X-Forwarded-Proto".to_string(),
                "X-Forwarded-For".to_string(),
                "X-Real-IP".to_string(),
            ],
            fallback_mode: TlsMode::Application,
        }
    }
    
    /// Create a behind-proxy TLS configuration for testing
    pub fn create_behind_proxy_config() -> TlsConfig {
        let mut config = Self::create_test_tls_config();
        config.mode = TlsMode::BehindProxy;
        config.behind_proxy = true;
        config.require_forwarded_proto = true;
        config.require_forwarded_for = true;
        config.cert_file = None;
        config.key_file = None;
        config
    }
    
    /// Create an auto-detection TLS configuration for testing
    pub fn create_auto_config() -> TlsConfig {
        let mut config = Self::create_test_tls_config();
        config.mode = TlsMode::Auto;
        config.fallback_mode = TlsMode::Application;
        config
    }
    
    /// Create a test HTTP request with proxy headers
    pub fn create_proxied_request() -> actix_web::HttpRequest {
        TestRequest::default()
            .insert_header(("x-forwarded-proto", "https"))
            .insert_header(("x-forwarded-for", "203.0.113.1, 198.51.100.1"))
            .insert_header(("x-real-ip", "203.0.113.1"))
            .insert_header(("x-forwarded-host", "api.example.com"))
            .insert_header(("user-agent", "Mozilla/5.0 Test Agent"))
            .to_http_request()
    }
    
    /// Create a test HTTP request without proxy headers (direct)
    pub fn create_direct_request() -> actix_web::HttpRequest {
        TestRequest::default()
            .insert_header(("user-agent", "Mozilla/5.0 Test Agent"))
            .to_http_request()
    }
    
    /// Create a test HTTP request with suspicious characteristics
    pub fn create_suspicious_request() -> actix_web::HttpRequest {
        TestRequest::default()
            .insert_header(("x-forwarded-for", "192.168.1.1"))  // Private IP in X-Forwarded-For
            .insert_header(("x-forwarded-proto", "http"))       // HTTP instead of HTTPS
            .insert_header(("user-agent", "curl/7.68.0"))       // Automated tool
            .to_http_request()
    }
    
    /// Create a test HTTP request from an untrusted proxy
    pub fn create_untrusted_proxy_request() -> actix_web::HttpRequest {
        TestRequest::default()
            .insert_header(("x-forwarded-proto", "https"))
            .insert_header(("x-forwarded-for", "8.8.8.8"))      // Public IP (untrusted)
            .insert_header(("x-real-ip", "8.8.8.8"))
            .to_http_request()
    }
    
    /// Validate that a TLS configuration is valid
    pub fn validate_tls_config(config: &TlsConfig) -> Result<()> {
        // Basic validation logic
        match config.mode {
            TlsMode::Application => {
                if config.cert_file.is_none() || config.key_file.is_none() {
                    return Err(magictunnel::error::ProxyError::config(
                        "Application mode requires cert_file and key_file".to_string()
                    ));
                }
            }
            TlsMode::BehindProxy => {
                if config.trusted_proxies.is_empty() {
                    return Err(magictunnel::error::ProxyError::config(
                        "Behind proxy mode should have trusted_proxies configured".to_string()
                    ));
                }
            }
            TlsMode::Auto => {
                if config.auto_detect_headers.is_empty() {
                    return Err(magictunnel::error::ProxyError::config(
                        "Auto mode should have auto_detect_headers configured".to_string()
                    ));
                }
            }
            TlsMode::Disabled => {
                // No specific requirements for disabled mode
            }
        }
        
        Ok(())
    }
    
    /// Create test certificate data for certificate monitoring tests
    pub fn create_test_certificate_data() -> Vec<u8> {
        // Simplified test certificate data (not a real certificate)
        b"-----BEGIN CERTIFICATE-----
MIIDXTCCAkWgAwIBAgIJAKoK/heBjcOuMA0GCSqGSIb3DQEBBQUAMEUxCzAJBgNV
BAYTAkFVMRMwEQYDVQQIDApTb21lLVN0YXRlMSEwHwYDVQQKDBhJbnRlcm5ldCBX
aWRnaXRzIFB0eSBMdGQwHhcNMTMwOTEyMjE1MjAyWhcNMTQwOTEyMjE1MjAyWjBF
MQswCQYDVQQGEwJBVTETMBEGA1UECAwKU29tZS1TdGF0ZTEhMB8GA1UECgwYSW50
ZXJuZXQgV2lkZ2l0cyBQdHkgTHRkMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIB
CgKCAQEAwUdHPiQnlWXRJEuJzHrGciQXqcVKAA+IuVGZq4hI+5ppvfgI3YozhTb3
+cfMpgGFCvUpHQmpsaGlBwPj/sLABwjzYSgGqgBjxWWXdCu0A0nt9rPiZs6+4F5H
hFlyDqNiLE1jXmxzMh5DjkxK67A+5Ay6zAo4LSbcyDlBMP1dfvuK4HqBcItSD0I2
EB1M7gStWuGIF4N4MlM3g5+2jJdnkjqGVlD3aM8=
-----END CERTIFICATE-----".to_vec()
    }
    
    /// Create a map of test environment variables
    pub fn create_test_env_vars() -> HashMap<String, String> {
        let mut env_vars = HashMap::new();
        env_vars.insert("TLS_MODE".to_string(), "application".to_string());
        env_vars.insert("TLS_CERT_FILE".to_string(), "/etc/ssl/certs/server.crt".to_string());
        env_vars.insert("TLS_KEY_FILE".to_string(), "/etc/ssl/private/server.key".to_string());
        env_vars.insert("TLS_MIN_VERSION".to_string(), "1.2".to_string());
        env_vars.insert("HSTS_ENABLED".to_string(), "true".to_string());
        env_vars.insert("HSTS_MAX_AGE".to_string(), "31536000".to_string());
        env_vars.insert("TRUSTED_PROXIES".to_string(), "127.0.0.1/32,10.0.0.0/8".to_string());
        env_vars
    }
    
    /// Assert that two TLS configurations are equivalent
    pub fn assert_tls_config_eq(config1: &TlsConfig, config2: &TlsConfig) {
        assert_eq!(config1.mode, config2.mode);
        assert_eq!(config1.cert_file, config2.cert_file);
        assert_eq!(config1.key_file, config2.key_file);
        assert_eq!(config1.ca_file, config2.ca_file);
        assert_eq!(config1.behind_proxy, config2.behind_proxy);
        assert_eq!(config1.trusted_proxies, config2.trusted_proxies);
        assert_eq!(config1.min_tls_version, config2.min_tls_version);
        assert_eq!(config1.hsts_enabled, config2.hsts_enabled);
        assert_eq!(config1.hsts_max_age, config2.hsts_max_age);
        assert_eq!(config1.require_forwarded_proto, config2.require_forwarded_proto);
        assert_eq!(config1.require_forwarded_for, config2.require_forwarded_for);
        assert_eq!(config1.auto_detect_headers, config2.auto_detect_headers);
        assert_eq!(config1.fallback_mode, config2.fallback_mode);
    }
    
    /// Generate test data for performance testing
    pub fn generate_test_requests(count: usize) -> Vec<actix_web::HttpRequest> {
        let mut requests = Vec::new();
        
        for i in 0..count {
            let req = if i % 3 == 0 {
                Self::create_proxied_request()
            } else if i % 3 == 1 {
                Self::create_direct_request()
            } else {
                Self::create_suspicious_request()
            };
            requests.push(req);
        }
        
        requests
    }
    
    /// Measure execution time of a function
    pub fn measure_execution_time<F, R>(f: F) -> (R, std::time::Duration)
    where
        F: FnOnce() -> R,
    {
        let start = std::time::Instant::now();
        let result = f();
        let duration = start.elapsed();
        (result, duration)
    }
}

/// Test result aggregator for collecting test statistics
pub struct TestResultAggregator {
    pub total_tests: u32,
    pub passed_tests: u32,
    pub failed_tests: u32,
    pub skipped_tests: u32,
    pub test_results: Vec<TestResult>,
}

/// Individual test result
#[derive(Debug, Clone)]
pub struct TestResult {
    pub test_name: String,
    pub status: TestStatus,
    pub duration: std::time::Duration,
    pub error_message: Option<String>,
}

/// Test status enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum TestStatus {
    Passed,
    Failed,
    Skipped,
}

impl TestResultAggregator {
    pub fn new() -> Self {
        Self {
            total_tests: 0,
            passed_tests: 0,
            failed_tests: 0,
            skipped_tests: 0,
            test_results: Vec::new(),
        }
    }

    pub fn add_result(&mut self, result: TestResult) {
        self.total_tests += 1;
        match result.status {
            TestStatus::Passed => self.passed_tests += 1,
            TestStatus::Failed => self.failed_tests += 1,
            TestStatus::Skipped => self.skipped_tests += 1,
        }
        self.test_results.push(result);
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_tests == 0 {
            0.0
        } else {
            (self.passed_tests as f64) / (self.total_tests as f64) * 100.0
        }
    }

    pub fn print_summary(&self) {
        println!("📊 Test Summary:");
        println!("  Total Tests: {}", self.total_tests);
        println!("  ✅ Passed: {}", self.passed_tests);
        println!("  ❌ Failed: {}", self.failed_tests);
        println!("  ⏭️  Skipped: {}", self.skipped_tests);
        println!("  📈 Success Rate: {:.1}%", self.success_rate());

        if self.failed_tests > 0 {
            println!("\n❌ Failed Tests:");
            for result in &self.test_results {
                if result.status == TestStatus::Failed {
                    println!("  - {}: {}", result.test_name,
                             result.error_message.as_deref().unwrap_or("Unknown error"));
                }
            }
        }
    }
}

impl Default for TestResultAggregator {
    fn default() -> Self {
        Self::new()
    }
}
