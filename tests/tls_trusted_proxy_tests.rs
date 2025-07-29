//! TLS Trusted Proxy Tests
//! 
//! Comprehensive tests for trusted proxy validation and IP range checking.
//! Moved from src/tls/tests/trusted_proxy_tests.rs to follow Rust best practices.

mod tls_test_utils;
use tls_test_utils::{TlsTestUtils, TestResultAggregator, TestResult, TestStatus};

use magictunnel::tls::TrustedProxyValidator;
use std::net::IpAddr;
use std::str::FromStr;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trusted_proxy_validator_creation() {
        let trusted_proxies = vec![
            "127.0.0.1/32".to_string(),
            "10.0.0.0/8".to_string(),
            "192.168.0.0/16".to_string(),
        ];
        
        let validator = TrustedProxyValidator::new(&trusted_proxies);
        assert!(validator.is_ok(), "Should create validator successfully");
    }
    
    #[test]
    fn test_trusted_proxy_validator_invalid_cidr() {
        let trusted_proxies = vec![
            "invalid-cidr".to_string(),
            "256.256.256.256/32".to_string(),
        ];
        
        let validator = TrustedProxyValidator::new(&trusted_proxies);
        assert!(validator.is_err(), "Should fail with invalid CIDR");
    }
    
    #[test]
    fn test_ipv4_trusted_proxy_validation() {
        let trusted_proxies = vec![
            "127.0.0.1/32".to_string(),
            "10.0.0.0/8".to_string(),
            "192.168.0.0/16".to_string(),
            "172.16.0.0/12".to_string(),
        ];
        
        let validator = TrustedProxyValidator::new(&trusted_proxies).unwrap();
        
        // Test trusted IPs
        let trusted_ips = vec![
            "127.0.0.1",
            "10.0.0.1",
            "10.255.255.255",
            "192.168.1.1",
            "192.168.255.255",
            "172.16.0.1",
            "172.31.255.255",
        ];
        
        for ip_str in trusted_ips {
            let ip = IpAddr::from_str(ip_str).unwrap();
            assert!(validator.is_trusted_proxy(&ip), "IP {} should be trusted", ip_str);
        }

        // Test untrusted IPs
        let untrusted_ips = vec![
            "8.8.8.8",
            "1.1.1.1",
            "203.0.113.1",
            "198.51.100.1",
            "192.0.2.1",
        ];

        for ip_str in untrusted_ips {
            let ip = IpAddr::from_str(ip_str).unwrap();
            assert!(!validator.is_trusted_proxy(&ip), "IP {} should not be trusted", ip_str);
        }
    }
    
    #[test]
    fn test_ipv6_trusted_proxy_validation() {
        let trusted_proxies = vec![
            "::1/128".to_string(),
            "fc00::/7".to_string(),
            "fe80::/10".to_string(),
            "2001:db8::/32".to_string(),
        ];
        
        let validator = TrustedProxyValidator::new(&trusted_proxies).unwrap();
        
        // Test trusted IPv6 addresses
        let trusted_ips = vec![
            "::1",
            "fc00::1",
            "fdff:ffff:ffff:ffff:ffff:ffff:ffff:ffff",
            "fe80::1",
            "febf:ffff:ffff:ffff:ffff:ffff:ffff:ffff",
            "2001:db8::1",
            "2001:db8:ffff:ffff:ffff:ffff:ffff:ffff",
        ];
        
        for ip_str in trusted_ips {
            let ip = IpAddr::from_str(ip_str).unwrap();
            assert!(validator.is_trusted_proxy(&ip), "IPv6 {} should be trusted", ip_str);
        }

        // Test untrusted IPv6 addresses
        let untrusted_ips = vec![
            "2001:4860:4860::8888", // Google DNS
            "2606:4700:4700::1111", // Cloudflare DNS
            "2001:db9::1",          // Outside 2001:db8::/32
        ];

        for ip_str in untrusted_ips {
            let ip = IpAddr::from_str(ip_str).unwrap();
            assert!(!validator.is_trusted_proxy(&ip), "IPv6 {} should not be trusted", ip_str);
        }
    }
    
    #[test]
    fn test_mixed_ipv4_ipv6_validation() {
        let trusted_proxies = vec![
            "127.0.0.1/32".to_string(),
            "10.0.0.0/8".to_string(),
            "::1/128".to_string(),
            "2001:db8::/32".to_string(),
        ];
        
        let validator = TrustedProxyValidator::new(&trusted_proxies).unwrap();
        
        // Test mixed trusted IPs
        assert!(validator.is_trusted_proxy(&IpAddr::from_str("127.0.0.1").unwrap()));
        assert!(validator.is_trusted_proxy(&IpAddr::from_str("10.0.0.1").unwrap()));
        assert!(validator.is_trusted_proxy(&IpAddr::from_str("::1").unwrap()));
        assert!(validator.is_trusted_proxy(&IpAddr::from_str("2001:db8::1").unwrap()));

        // Test mixed untrusted IPs
        assert!(!validator.is_trusted_proxy(&IpAddr::from_str("8.8.8.8").unwrap()));
        assert!(!validator.is_trusted_proxy(&IpAddr::from_str("2001:4860:4860::8888").unwrap()));
    }
    
    #[test]
    fn test_edge_case_cidr_ranges() {
        // Test edge cases for CIDR ranges
        let trusted_proxies = vec![
            "0.0.0.0/0".to_string(),    // All IPv4
            "::/0".to_string(),         // All IPv6
        ];
        
        let validator = TrustedProxyValidator::new(&trusted_proxies).unwrap();
        
        // All IPs should be trusted
        assert!(validator.is_trusted_proxy(&IpAddr::from_str("8.8.8.8").unwrap()));
        assert!(validator.is_trusted_proxy(&IpAddr::from_str("192.168.1.1").unwrap()));
        assert!(validator.is_trusted_proxy(&IpAddr::from_str("2001:4860:4860::8888").unwrap()));
        assert!(validator.is_trusted_proxy(&IpAddr::from_str("::1").unwrap()));
    }
    
    #[test]
    fn test_single_ip_cidr() {
        // Test /32 and /128 CIDR ranges (single IPs)
        let trusted_proxies = vec![
            "203.0.113.1/32".to_string(),
            "2001:db8::1/128".to_string(),
        ];
        
        let validator = TrustedProxyValidator::new(&trusted_proxies).unwrap();
        
        // Exact IPs should be trusted
        assert!(validator.is_trusted_proxy(&IpAddr::from_str("203.0.113.1").unwrap()));
        assert!(validator.is_trusted_proxy(&IpAddr::from_str("2001:db8::1").unwrap()));

        // Similar but different IPs should not be trusted
        assert!(!validator.is_trusted_proxy(&IpAddr::from_str("203.0.113.2").unwrap()));
        assert!(!validator.is_trusted_proxy(&IpAddr::from_str("2001:db8::2").unwrap()));
    }
    
    #[test]
    fn test_common_proxy_ranges() {
        // Test common proxy and load balancer IP ranges
        let trusted_proxies = vec![
            // AWS ALB ranges (example)
            "10.0.0.0/8".to_string(),
            "172.16.0.0/12".to_string(),
            "192.168.0.0/16".to_string(),
            // Cloudflare ranges (example subset)
            "103.21.244.0/22".to_string(),
            "103.22.200.0/22".to_string(),
            // Local ranges
            "127.0.0.0/8".to_string(),
        ];
        
        let validator = TrustedProxyValidator::new(&trusted_proxies).unwrap();
        
        // Test AWS private ranges
        assert!(validator.is_trusted_proxy(&IpAddr::from_str("10.0.0.1").unwrap()));
        assert!(validator.is_trusted_proxy(&IpAddr::from_str("172.16.0.1").unwrap()));
        assert!(validator.is_trusted_proxy(&IpAddr::from_str("192.168.1.1").unwrap()));

        // Test Cloudflare ranges
        assert!(validator.is_trusted_proxy(&IpAddr::from_str("103.21.244.1").unwrap()));
        assert!(validator.is_trusted_proxy(&IpAddr::from_str("103.22.200.1").unwrap()));

        // Test localhost ranges
        assert!(validator.is_trusted_proxy(&IpAddr::from_str("127.0.0.1").unwrap()));
        assert!(validator.is_trusted_proxy(&IpAddr::from_str("127.255.255.255").unwrap()));

        // Test public IPs that should not be trusted
        assert!(!validator.is_trusted_proxy(&IpAddr::from_str("8.8.8.8").unwrap()));
        assert!(!validator.is_trusted_proxy(&IpAddr::from_str("1.1.1.1").unwrap()));
    }
    
    #[test]
    fn test_empty_trusted_proxies() {
        let trusted_proxies: Vec<String> = vec![];
        let validator = TrustedProxyValidator::new(&trusted_proxies).unwrap();
        
        // No IPs should be trusted
        assert!(!validator.is_trusted_proxy(&IpAddr::from_str("127.0.0.1").unwrap()));
        assert!(!validator.is_trusted_proxy(&IpAddr::from_str("10.0.0.1").unwrap()));
        assert!(!validator.is_trusted_proxy(&IpAddr::from_str("::1").unwrap()));
    }
    
    #[test]
    fn test_malformed_cidr_handling() {
        let malformed_cidrs = vec![
            "not-an-ip".to_string(),
            "127.0.0.1/33".to_string(),  // Invalid prefix length
            "127.0.0.1/-1".to_string(),  // Negative prefix
            "127.0.0.1/".to_string(),    // Missing prefix
            "/24".to_string(),           // Missing IP
            "127.0.0.1/24/extra".to_string(), // Extra parts
        ];
        
        for malformed in malformed_cidrs {
            let result = TrustedProxyValidator::new(&vec![malformed.clone()]);
            assert!(result.is_err(), "Should fail for malformed CIDR: {}", malformed);
        }
    }
    
    #[test]
    fn test_performance_with_many_ranges() {
        // Test performance with many CIDR ranges
        let mut trusted_proxies = Vec::new();
        
        // Add many /24 ranges
        for i in 0..100 {
            trusted_proxies.push(format!("10.{}.0.0/24", i));
        }
        
        let validator = TrustedProxyValidator::new(&trusted_proxies).unwrap();
        
        // Test performance
        let (_, duration) = TlsTestUtils::measure_execution_time(|| {
            for i in 0..1000 {
                let ip = IpAddr::from_str(&format!("10.{}.0.1", i % 100)).unwrap();
                let _ = validator.is_trusted_proxy(&ip);
            }
        });
        
        // Should complete quickly even with many ranges
        assert!(duration.as_millis() < 100, 
                "Performance test failed: took {}ms", duration.as_millis());
    }
    
    #[test]
    fn test_boundary_conditions() {
        let trusted_proxies = vec![
            "192.168.1.0/24".to_string(),
        ];
        
        let validator = TrustedProxyValidator::new(&trusted_proxies).unwrap();
        
        // Test boundary IPs
        assert!(validator.is_trusted_proxy(&IpAddr::from_str("192.168.1.0").unwrap()));   // Network address
        assert!(validator.is_trusted_proxy(&IpAddr::from_str("192.168.1.255").unwrap())); // Broadcast address
        assert!(validator.is_trusted_proxy(&IpAddr::from_str("192.168.1.1").unwrap()));   // First host
        assert!(validator.is_trusted_proxy(&IpAddr::from_str("192.168.1.254").unwrap())); // Last host

        // Test outside boundaries
        assert!(!validator.is_trusted_proxy(&IpAddr::from_str("192.168.0.255").unwrap())); // One below
        assert!(!validator.is_trusted_proxy(&IpAddr::from_str("192.168.2.0").unwrap()));   // One above
    }
    
    #[test]
    fn test_validator_with_test_utils() {
        // Test integration with test utilities
        let config = TlsTestUtils::create_behind_proxy_config();
        let validator = TrustedProxyValidator::new(&config.trusted_proxies).unwrap();
        
        // Test with test utility generated requests
        let proxied_req = TlsTestUtils::create_proxied_request();
        let untrusted_req = TlsTestUtils::create_untrusted_proxy_request();
        
        // The test utilities should create requests that work with the validator
        // Note: This is more of an integration test to ensure test utilities are consistent
        assert!(!config.trusted_proxies.is_empty());
        
        // Test that localhost is trusted (should be in default config)
        assert!(validator.is_trusted_proxy(&IpAddr::from_str("127.0.0.1").unwrap()));
    }
}
