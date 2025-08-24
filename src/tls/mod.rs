// TLS and reverse proxy handling module

pub mod trusted_proxy;
pub mod headers;
pub mod validation;
pub mod auto_detection;
pub mod security_headers;
pub mod security_audit;
pub mod cert_monitoring;



pub use trusted_proxy::TrustedProxyValidator;
pub use headers::{ProxyHeaders, ForwardedHeaders};
pub use validation::{ProxyValidator, ProxyRequestInfo, ProxyValidationUtils};
pub use auto_detection::{TlsAutoDetector, AutoDetectionConfig, DetectionStats, AutoDetectionUtils};
pub use security_headers::{SecurityHeadersMiddleware, SecurityHeadersConfig, SecurityHeadersUtils};
pub use security_audit::{SecurityAuditLogger, SecurityAuditConfig, SecurityEvent, SecurityEventType, SecuritySeverity};
pub use cert_monitoring::{CertificateMonitor, CertMonitoringConfig, CertificateInfo, CertificateStatus};
