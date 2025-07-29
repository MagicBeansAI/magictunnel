use crate::error::{ProxyError, Result};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::str::FromStr;
use tracing::{debug, warn};

/// Validator for trusted proxy IP addresses and CIDR ranges
#[derive(Debug, Clone)]
pub struct TrustedProxyValidator {
    /// List of trusted proxy CIDR ranges
    trusted_ranges: Vec<CidrRange>,
}

/// Represents a CIDR range for IP validation
#[derive(Debug, Clone)]
struct CidrRange {
    network: IpAddr,
    prefix_len: u8,
}

impl TrustedProxyValidator {
    /// Create a new trusted proxy validator
    pub fn new(trusted_proxies: &[String]) -> Result<Self> {
        let mut trusted_ranges = Vec::new();
        
        for proxy_str in trusted_proxies {
            let range = CidrRange::from_str(proxy_str)
                .map_err(|e| ProxyError::config(format!("Invalid trusted proxy CIDR: {}: {}", proxy_str, e)))?;
            trusted_ranges.push(range);
        }
        
        debug!("Initialized trusted proxy validator with {} ranges", trusted_ranges.len());
        Ok(Self { trusted_ranges })
    }
    
    /// Check if an IP address is from a trusted proxy
    pub fn is_trusted_proxy(&self, ip: &IpAddr) -> bool {
        for range in &self.trusted_ranges {
            if range.contains(ip) {
                debug!("IP {} matches trusted proxy range", ip);
                return true;
            }
        }
        
        debug!("IP {} is not from a trusted proxy", ip);
        false
    }
    
    /// Validate that a request comes from a trusted proxy
    pub fn validate_proxy_request(&self, client_ip: &IpAddr) -> Result<()> {
        if self.is_trusted_proxy(client_ip) {
            Ok(())
        } else {
            warn!("Request from untrusted proxy: {}", client_ip);
            Err(ProxyError::config(format!(
                "Request from untrusted proxy: {}. Add this IP to trusted_proxies configuration.",
                client_ip
            )))
        }
    }
    
    /// Get the list of trusted proxy ranges as strings
    pub fn get_trusted_ranges(&self) -> Vec<String> {
        self.trusted_ranges.iter().map(|r| r.to_string()).collect()
    }
}

impl CidrRange {
    /// Parse a CIDR range from string
    fn from_str(cidr: &str) -> Result<Self> {
        if let Some((ip_str, prefix_str)) = cidr.split_once('/') {
            let network = IpAddr::from_str(ip_str)
                .map_err(|e| ProxyError::config(format!("Invalid IP address in CIDR: {}", e)))?;
            let prefix_len = prefix_str.parse::<u8>()
                .map_err(|e| ProxyError::config(format!("Invalid prefix length in CIDR: {}", e)))?;
            
            // Validate prefix length based on IP version
            let max_prefix = match network {
                IpAddr::V4(_) => 32,
                IpAddr::V6(_) => 128,
            };
            
            if prefix_len > max_prefix {
                return Err(ProxyError::config(format!(
                    "Invalid prefix length {} for {:?} address (max: {})",
                    prefix_len,
                    if network.is_ipv4() { "IPv4" } else { "IPv6" },
                    max_prefix
                )));
            }
            
            Ok(Self { network, prefix_len })
        } else {
            // Single IP address - treat as /32 for IPv4 or /128 for IPv6
            let network = IpAddr::from_str(cidr)
                .map_err(|e| ProxyError::config(format!("Invalid IP address: {}", e)))?;
            let prefix_len = match network {
                IpAddr::V4(_) => 32,
                IpAddr::V6(_) => 128,
            };
            
            Ok(Self { network, prefix_len })
        }
    }
    
    /// Check if an IP address is within this CIDR range
    fn contains(&self, ip: &IpAddr) -> bool {
        match (self.network, ip) {
            (IpAddr::V4(net), IpAddr::V4(addr)) => {
                self.ipv4_contains(&net, addr)
            }
            (IpAddr::V6(net), IpAddr::V6(addr)) => {
                self.ipv6_contains(&net, addr)
            }
            _ => false, // Different IP versions don't match
        }
    }
    
    /// Check if IPv4 address is in range
    fn ipv4_contains(&self, network: &Ipv4Addr, addr: &Ipv4Addr) -> bool {
        if self.prefix_len == 0 {
            return true; // 0.0.0.0/0 matches everything
        }
        
        let network_bits = u32::from(*network);
        let addr_bits = u32::from(*addr);
        let mask = !((1u32 << (32 - self.prefix_len)) - 1);
        
        (network_bits & mask) == (addr_bits & mask)
    }
    
    /// Check if IPv6 address is in range
    fn ipv6_contains(&self, network: &Ipv6Addr, addr: &Ipv6Addr) -> bool {
        if self.prefix_len == 0 {
            return true; // ::/0 matches everything
        }
        
        let network_bits = u128::from(*network);
        let addr_bits = u128::from(*addr);
        let mask = !((1u128 << (128 - self.prefix_len)) - 1);
        
        (network_bits & mask) == (addr_bits & mask)
    }
}

impl std::fmt::Display for CidrRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.network, self.prefix_len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cidr_parsing() {
        // Valid CIDR ranges
        assert!(CidrRange::from_str("192.168.1.0/24").is_ok());
        assert!(CidrRange::from_str("10.0.0.0/8").is_ok());
        assert!(CidrRange::from_str("127.0.0.1/32").is_ok());
        assert!(CidrRange::from_str("127.0.0.1").is_ok()); // Single IP
        
        // IPv6
        assert!(CidrRange::from_str("::1/128").is_ok());
        assert!(CidrRange::from_str("2001:db8::/32").is_ok());
        
        // Invalid ranges
        assert!(CidrRange::from_str("invalid").is_err());
        assert!(CidrRange::from_str("192.168.1.0/33").is_err()); // Invalid prefix for IPv4
        assert!(CidrRange::from_str("::1/129").is_err()); // Invalid prefix for IPv6
    }
    
    #[test]
    fn test_ipv4_contains() {
        let range = CidrRange::from_str("192.168.1.0/24").unwrap();
        
        assert!(range.contains(&IpAddr::from_str("192.168.1.1").unwrap()));
        assert!(range.contains(&IpAddr::from_str("192.168.1.254").unwrap()));
        assert!(!range.contains(&IpAddr::from_str("192.168.2.1").unwrap()));
        assert!(!range.contains(&IpAddr::from_str("10.0.0.1").unwrap()));
    }
    
    #[test]
    fn test_trusted_proxy_validator() {
        let trusted_proxies = vec![
            "127.0.0.1/32".to_string(),
            "10.0.0.0/8".to_string(),
            "192.168.0.0/16".to_string(),
        ];
        
        let validator = TrustedProxyValidator::new(&trusted_proxies).unwrap();
        
        // Trusted IPs
        assert!(validator.is_trusted_proxy(&IpAddr::from_str("127.0.0.1").unwrap()));
        assert!(validator.is_trusted_proxy(&IpAddr::from_str("10.1.2.3").unwrap()));
        assert!(validator.is_trusted_proxy(&IpAddr::from_str("192.168.100.50").unwrap()));
        
        // Untrusted IPs
        assert!(!validator.is_trusted_proxy(&IpAddr::from_str("8.8.8.8").unwrap()));
        assert!(!validator.is_trusted_proxy(&IpAddr::from_str("172.16.0.1").unwrap()));
    }
}
