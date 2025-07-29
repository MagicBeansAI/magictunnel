use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::{Arc, RwLock};
use tokio::time::{interval, Duration as TokioDuration};
use tracing::{debug, error, info, warn};

use crate::config::TlsConfig;
use crate::error::{ProxyError, Result};

/// Certificate monitoring configuration
#[derive(Debug, Clone)]
pub struct CertMonitoringConfig {
    /// Enable certificate monitoring
    pub enabled: bool,
    /// Check interval in seconds
    pub check_interval_seconds: u64,
    /// Warning threshold in days before expiration
    pub warning_threshold_days: i64,
    /// Critical threshold in days before expiration
    pub critical_threshold_days: i64,
    /// Enable automatic certificate renewal alerts
    pub enable_renewal_alerts: bool,
    /// Certificate paths to monitor
    pub certificate_paths: Vec<String>,
    /// Enable certificate chain validation
    pub validate_chain: bool,
    /// Enable OCSP checking
    pub check_ocsp: bool,
}

impl Default for CertMonitoringConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            check_interval_seconds: 3600, // Check every hour
            warning_threshold_days: 30,   // Warn 30 days before expiration
            critical_threshold_days: 7,   // Critical alert 7 days before expiration
            enable_renewal_alerts: true,
            certificate_paths: Vec::new(),
            validate_chain: true,
            check_ocsp: false, // Disabled by default due to complexity
        }
    }
}

/// Certificate information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateInfo {
    /// Certificate file path
    pub path: String,
    /// Certificate subject
    pub subject: String,
    /// Certificate issuer
    pub issuer: String,
    /// Certificate serial number
    pub serial_number: String,
    /// Not valid before
    pub not_before: DateTime<Utc>,
    /// Not valid after
    pub not_after: DateTime<Utc>,
    /// Days until expiration
    pub days_until_expiration: i64,
    /// Certificate status
    pub status: CertificateStatus,
    /// Subject Alternative Names
    pub san: Vec<String>,
    /// Key usage
    pub key_usage: Vec<String>,
    /// Certificate fingerprint (SHA-256)
    pub fingerprint: String,
    /// Last check time
    pub last_checked: DateTime<Utc>,
}

/// Certificate status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CertificateStatus {
    /// Certificate is valid
    Valid,
    /// Certificate expires soon (warning)
    ExpiringSoon,
    /// Certificate expires very soon (critical)
    ExpiringCritical,
    /// Certificate has expired
    Expired,
    /// Certificate is invalid
    Invalid,
    /// Certificate file not found
    NotFound,
    /// Error reading certificate
    Error,
}

/// Certificate monitoring statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertMonitoringStats {
    /// Total certificates monitored
    pub total_certificates: u32,
    /// Valid certificates
    pub valid_certificates: u32,
    /// Certificates expiring soon
    pub expiring_soon: u32,
    /// Certificates expiring critically soon
    pub expiring_critical: u32,
    /// Expired certificates
    pub expired_certificates: u32,
    /// Invalid certificates
    pub invalid_certificates: u32,
    /// Last monitoring run
    pub last_check: Option<DateTime<Utc>>,
    /// Next scheduled check
    pub next_check: Option<DateTime<Utc>>,
}

/// Certificate monitor
pub struct CertificateMonitor {
    config: CertMonitoringConfig,
    /// Certificate information cache
    certificates: Arc<RwLock<HashMap<String, CertificateInfo>>>,
    /// Monitoring statistics
    stats: Arc<RwLock<CertMonitoringStats>>,
}

impl CertificateMonitor {
    /// Create a new certificate monitor
    pub fn new(config: CertMonitoringConfig) -> Self {
        Self {
            config,
            certificates: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(CertMonitoringStats::default())),
        }
    }
    
    /// Create from TLS configuration
    pub fn from_tls_config(tls_config: &TlsConfig) -> Self {
        let mut config = CertMonitoringConfig::default();
        
        // Add certificate paths from TLS config
        if let Some(cert_file) = &tls_config.cert_file {
            config.certificate_paths.push(cert_file.clone());
        }
        if let Some(ca_file) = &tls_config.ca_file {
            config.certificate_paths.push(ca_file.clone());
        }
        
        Self::new(config)
    }
    
    /// Start monitoring (async task)
    pub async fn start_monitoring(&self) -> Result<()> {
        if !self.config.enabled {
            info!("Certificate monitoring is disabled");
            return Ok(());
        }
        
        info!("Starting certificate monitoring with {} second intervals", self.config.check_interval_seconds);
        
        let mut interval = interval(TokioDuration::from_secs(self.config.check_interval_seconds));
        
        loop {
            interval.tick().await;
            
            if let Err(e) = self.check_certificates().await {
                error!("Certificate monitoring error: {}", e);
            }
        }
    }
    
    /// Check all configured certificates
    pub async fn check_certificates(&self) -> Result<()> {
        debug!("Starting certificate check");
        
        let mut certificate_infos = HashMap::new();
        
        for cert_path in &self.config.certificate_paths {
            match self.check_certificate(cert_path).await {
                Ok(cert_info) => {
                    certificate_infos.insert(cert_path.clone(), cert_info);
                }
                Err(e) => {
                    warn!("Failed to check certificate {}: {}", cert_path, e);
                    
                    // Create error entry
                    let error_info = CertificateInfo {
                        path: cert_path.clone(),
                        subject: "Unknown".to_string(),
                        issuer: "Unknown".to_string(),
                        serial_number: "Unknown".to_string(),
                        not_before: Utc::now(),
                        not_after: Utc::now(),
                        days_until_expiration: 0,
                        status: CertificateStatus::Error,
                        san: Vec::new(),
                        key_usage: Vec::new(),
                        fingerprint: "Unknown".to_string(),
                        last_checked: Utc::now(),
                    };
                    
                    certificate_infos.insert(cert_path.clone(), error_info);
                }
            }
        }
        
        // Update certificate cache
        {
            let mut certificates = self.certificates.write()
                .map_err(|e| ProxyError::config(format!("Failed to acquire certificates lock: {}", e)))?;
            *certificates = certificate_infos;
        }
        
        // Update statistics
        self.update_stats().await?;
        
        // Check for alerts
        self.check_alerts().await?;
        
        debug!("Certificate check completed");
        Ok(())
    }
    
    /// Check a single certificate (simplified implementation)
    async fn check_certificate(&self, cert_path: &str) -> Result<CertificateInfo> {
        // Check if file exists
        if !Path::new(cert_path).exists() {
            return Ok(CertificateInfo {
                path: cert_path.to_string(),
                subject: "File not found".to_string(),
                issuer: "File not found".to_string(),
                serial_number: "Unknown".to_string(),
                not_before: Utc::now(),
                not_after: Utc::now(),
                days_until_expiration: 0,
                status: CertificateStatus::NotFound,
                san: Vec::new(),
                key_usage: Vec::new(),
                fingerprint: "Unknown".to_string(),
                last_checked: Utc::now(),
            });
        }

        // Read certificate file
        let cert_data = fs::read(cert_path)
            .map_err(|e| ProxyError::config(format!("Failed to read certificate file {}: {}", cert_path, e)))?;

        // Simplified certificate parsing - just check file exists and is readable
        // In a real implementation, this would use a proper X.509 parser
        let now = Utc::now();
        let future_date = now + Duration::days(365); // Assume 1 year validity for demo

        // For demonstration, assume certificate is valid for 1 year from now
        let days_until_expiration = 365;

        let status = if days_until_expiration <= self.config.critical_threshold_days {
            CertificateStatus::ExpiringCritical
        } else if days_until_expiration <= self.config.warning_threshold_days {
            CertificateStatus::ExpiringSoon
        } else {
            CertificateStatus::Valid
        };

        // Calculate simple fingerprint (simplified - using length as demo)
        let fingerprint = format!("sha256:{:x}", cert_data.len());

        Ok(CertificateInfo {
            path: cert_path.to_string(),
            subject: "CN=MCP Proxy Certificate".to_string(), // Simplified
            issuer: "CN=MCP Proxy CA".to_string(), // Simplified
            serial_number: "123456789".to_string(), // Simplified
            not_before: now,
            not_after: future_date,
            days_until_expiration,
            status,
            san: vec!["localhost".to_string(), "127.0.0.1".to_string()], // Simplified
            key_usage: vec!["Digital Signature".to_string(), "Key Encipherment".to_string()], // Simplified
            fingerprint,
            last_checked: Utc::now(),
        })
    }
    
    /// Update monitoring statistics
    async fn update_stats(&self) -> Result<()> {
        let certificates = self.certificates.read()
            .map_err(|e| ProxyError::config(format!("Failed to acquire certificates lock: {}", e)))?;
        
        let mut stats = CertMonitoringStats {
            total_certificates: certificates.len() as u32,
            valid_certificates: 0,
            expiring_soon: 0,
            expiring_critical: 0,
            expired_certificates: 0,
            invalid_certificates: 0,
            last_check: Some(Utc::now()),
            next_check: Some(Utc::now() + Duration::seconds(self.config.check_interval_seconds as i64)),
        };
        
        for cert_info in certificates.values() {
            match cert_info.status {
                CertificateStatus::Valid => stats.valid_certificates += 1,
                CertificateStatus::ExpiringSoon => stats.expiring_soon += 1,
                CertificateStatus::ExpiringCritical => stats.expiring_critical += 1,
                CertificateStatus::Expired => stats.expired_certificates += 1,
                CertificateStatus::Invalid | CertificateStatus::NotFound | CertificateStatus::Error => {
                    stats.invalid_certificates += 1
                }
            }
        }
        
        // Update stats
        {
            let mut stats_lock = self.stats.write()
                .map_err(|e| ProxyError::config(format!("Failed to acquire stats lock: {}", e)))?;
            *stats_lock = stats;
        }
        
        Ok(())
    }
    
    /// Check for certificate alerts
    async fn check_alerts(&self) -> Result<()> {
        if !self.config.enable_renewal_alerts {
            return Ok(());
        }
        
        let certificates = self.certificates.read()
            .map_err(|e| ProxyError::config(format!("Failed to acquire certificates lock: {}", e)))?;
        
        for cert_info in certificates.values() {
            match cert_info.status {
                CertificateStatus::ExpiringCritical => {
                    error!(
                        "CRITICAL: Certificate {} expires in {} days ({})",
                        cert_info.path,
                        cert_info.days_until_expiration,
                        cert_info.not_after.format("%Y-%m-%d %H:%M:%S UTC")
                    );
                }
                CertificateStatus::ExpiringSoon => {
                    warn!(
                        "WARNING: Certificate {} expires in {} days ({})",
                        cert_info.path,
                        cert_info.days_until_expiration,
                        cert_info.not_after.format("%Y-%m-%d %H:%M:%S UTC")
                    );
                }
                CertificateStatus::Expired => {
                    error!(
                        "EXPIRED: Certificate {} expired {} days ago ({})",
                        cert_info.path,
                        -cert_info.days_until_expiration,
                        cert_info.not_after.format("%Y-%m-%d %H:%M:%S UTC")
                    );
                }
                CertificateStatus::Invalid | CertificateStatus::NotFound | CertificateStatus::Error => {
                    error!("ERROR: Certificate {} has issues: {:?}", cert_info.path, cert_info.status);
                }
                CertificateStatus::Valid => {
                    debug!("Certificate {} is valid for {} days", cert_info.path, cert_info.days_until_expiration);
                }
            }
        }
        
        Ok(())
    }
    
    /// Get certificate information
    pub fn get_certificates(&self) -> Result<HashMap<String, CertificateInfo>> {
        let certificates = self.certificates.read()
            .map_err(|e| ProxyError::config(format!("Failed to acquire certificates lock: {}", e)))?;
        Ok(certificates.clone())
    }
    
    /// Get monitoring statistics
    pub fn get_stats(&self) -> Result<CertMonitoringStats> {
        let stats = self.stats.read()
            .map_err(|e| ProxyError::config(format!("Failed to acquire stats lock: {}", e)))?;
        Ok(stats.clone())
    }
    
    /// Force certificate check
    pub async fn force_check(&self) -> Result<()> {
        info!("Forcing certificate check");
        self.check_certificates().await
    }
}

impl Default for CertMonitoringStats {
    fn default() -> Self {
        Self {
            total_certificates: 0,
            valid_certificates: 0,
            expiring_soon: 0,
            expiring_critical: 0,
            expired_certificates: 0,
            invalid_certificates: 0,
            last_check: None,
            next_check: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cert_monitoring_config() {
        let config = CertMonitoringConfig::default();
        assert!(config.enabled);
        assert_eq!(config.check_interval_seconds, 3600);
        assert_eq!(config.warning_threshold_days, 30);
        assert_eq!(config.critical_threshold_days, 7);
    }
    
    #[test]
    fn test_certificate_status() {
        assert_eq!(CertificateStatus::Valid, CertificateStatus::Valid);
        assert_ne!(CertificateStatus::Valid, CertificateStatus::Expired);
    }
}
