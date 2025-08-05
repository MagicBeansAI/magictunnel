//! MCP Client Capability Types
//!
//! Defines structures for tracking client capabilities according to MCP 2025-06-18 specification

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Client capabilities as reported in MCP initialize request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientCapabilities {
    /// Tools capability - client can execute tools
    #[serde(default)]
    pub tools: Option<ToolsCapability>,
    
    /// Resources capability - client can access resources
    #[serde(default)]
    pub resources: Option<ResourcesCapability>,
    
    /// Prompts capability - client can use prompts
    #[serde(default)]
    pub prompts: Option<PromptsCapability>,
    
    /// Sampling capability - client can handle sampling requests (MCP 2025-06-18)
    #[serde(default)]
    pub sampling: Option<SamplingCapability>,
    
    /// Elicitation capability - client can handle elicitation requests (MCP 2025-06-18)
    #[serde(default)]
    pub elicitation: Option<ElicitationCapability>,
}

/// Tools capability details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsCapability {
    /// Client supports tool execution
    #[serde(default)]
    pub enabled: bool,
    
    /// Additional tool-related capabilities
    #[serde(flatten)]
    pub additional: Value,
}

/// Resources capability details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcesCapability {
    /// Client supports resource access
    #[serde(default)]
    pub enabled: bool,
    
    /// Client supports resource subscriptions
    #[serde(default)]
    pub subscribe: bool,
    
    /// Additional resource-related capabilities
    #[serde(flatten)]
    pub additional: Value,
}

/// Prompts capability details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptsCapability {
    /// Client supports prompt usage
    #[serde(default)]
    pub enabled: bool,
    
    /// Additional prompt-related capabilities
    #[serde(flatten)]
    pub additional: Value,
}

/// Sampling capability details (MCP 2025-06-18)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamplingCapability {
    /// Client can create sampling requests
    #[serde(default)]
    pub create: bool,
    
    /// Client can handle sampling responses
    #[serde(default)]
    pub handle: bool,
    
    /// Additional sampling-related capabilities
    #[serde(flatten)]
    pub additional: Value,
}

/// Elicitation capability details (MCP 2025-06-18)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElicitationCapability {
    /// Client can create elicitation requests
    #[serde(default)]
    pub create: bool,
    
    /// Client can accept elicitation requests
    #[serde(default)]
    pub accept: bool,
    
    /// Client can reject elicitation requests
    #[serde(default)]
    pub reject: bool,
    
    /// Client can cancel elicitation requests
    #[serde(default)]
    pub cancel: bool,
    
    /// Additional elicitation-related capabilities
    #[serde(flatten)]
    pub additional: Value,
}

impl Default for ClientCapabilities {
    fn default() -> Self {
        Self {
            tools: None,
            resources: None,
            prompts: None,
            sampling: None,
            elicitation: None,
        }
    }
}

impl Default for ToolsCapability {
    fn default() -> Self {
        Self {
            enabled: false,
            additional: Value::Object(serde_json::Map::new()),
        }
    }
}

impl Default for ResourcesCapability {
    fn default() -> Self {
        Self {
            enabled: false,
            subscribe: false,
            additional: Value::Object(serde_json::Map::new()),
        }
    }
}

impl Default for PromptsCapability {
    fn default() -> Self {
        Self {
            enabled: false,
            additional: Value::Object(serde_json::Map::new()),
        }
    }
}

impl Default for SamplingCapability {
    fn default() -> Self {
        Self {
            create: false,
            handle: false,
            additional: Value::Object(serde_json::Map::new()),
        }
    }
}

impl Default for ElicitationCapability {
    fn default() -> Self {
        Self {
            create: false,
            accept: false,
            reject: false,
            cancel: false,
            additional: Value::Object(serde_json::Map::new()),
        }
    }
}

impl ClientCapabilities {
    /// Check if client supports elicitation functionality
    pub fn supports_elicitation(&self) -> bool {
        self.elicitation
            .as_ref()
            .map(|e| e.create && e.accept)
            .unwrap_or(false)
    }
    
    /// Check if client supports sampling functionality
    pub fn supports_sampling(&self) -> bool {
        self.sampling
            .as_ref()
            .map(|s| s.create)
            .unwrap_or(false)
    }
    
    /// Check if client supports tools functionality
    pub fn supports_tools(&self) -> bool {
        self.tools
            .as_ref()
            .map(|t| t.enabled)
            .unwrap_or(false)
    }
    
    /// Check if client supports resources functionality
    pub fn supports_resources(&self) -> bool {
        self.resources
            .as_ref()
            .map(|r| r.enabled)
            .unwrap_or(false)
    }
    
    /// Check if client supports prompts functionality
    pub fn supports_prompts(&self) -> bool {
        self.prompts
            .as_ref()
            .map(|p| p.enabled)
            .unwrap_or(false)
    }
    
    /// Get safe capability advertisement for external servers using minimum intersection
    /// Only advertises capabilities that both MagicTunnel and the client support
    pub fn get_safe_external_advertisement(&self) -> serde_json::Value {
        use serde_json::json;
        
        // MagicTunnel's native capabilities
        let magictunnel_capabilities = json!({
            "logging": {},
            "completion": {},
            "tools": { "listChanged": true },
            "resources": { "subscribe": true, "listChanged": true },
            "prompts": { "listChanged": true },
            "roots": {},
            "sampling": {},
            "elicitation": {
                "create": true,
                "accept": true,
                "reject": true,
                "cancel": true
            }
        });
        
        // Build intersection: only include capabilities both MagicTunnel AND client support
        let mut safe_capabilities = json!({});
        let capabilities_obj = safe_capabilities.as_object_mut().unwrap();
        
        // Always include basic capabilities (no client interaction needed)
        capabilities_obj.insert("logging".to_string(), json!({}));
        capabilities_obj.insert("completion".to_string(), json!({}));
        capabilities_obj.insert("roots".to_string(), json!({}));
        
        // Tools - include if client supports
        if self.supports_tools() {
            capabilities_obj.insert("tools".to_string(), json!({"listChanged": true}));
        }
        
        // Resources - include if client supports
        if self.supports_resources() {
            let mut resource_caps = json!({});
            if let Some(resources) = &self.resources {
                if resources.subscribe {
                    resource_caps["subscribe"] = json!(true);
                }
            }
            resource_caps["listChanged"] = json!(true);
            capabilities_obj.insert("resources".to_string(), resource_caps);
        }
        
        // Prompts - include if client supports
        if self.supports_prompts() {
            capabilities_obj.insert("prompts".to_string(), json!({"listChanged": true}));
        }
        
        // Sampling - only if client supports
        if self.supports_sampling() {
            capabilities_obj.insert("sampling".to_string(), json!({}));
        }
        
        // Elicitation - only if client supports BOTH create AND accept
        if self.supports_elicitation() {
            capabilities_obj.insert("elicitation".to_string(), json!({
                "create": true,
                "accept": true,
                "reject": true,
                "cancel": true
            }));
        }
        
        safe_capabilities
    }
    
    /// Log capability advertisement decisions
    pub fn log_capability_advertisement(&self, context: &str, advertised_capabilities: &serde_json::Value) {
        use tracing::{info, debug};
        
        info!("🔧 Capability Advertisement for {}", context);
        
        debug!("📋 Client Capabilities Summary:");
        debug!("   • Tools: {}", self.supports_tools());
        debug!("   • Resources: {} (subscribe: {})", 
               self.supports_resources(),
               self.resources.as_ref().map(|r| r.subscribe).unwrap_or(false));
        debug!("   • Prompts: {}", self.supports_prompts());
        debug!("   • Sampling: {}", self.supports_sampling());
        debug!("   • Elicitation: {}", self.supports_elicitation());
        
        debug!("📢 Advertised Capabilities to {}:", context);
        if let Some(caps) = advertised_capabilities.as_object() {
            for (key, value) in caps {
                debug!("   • {}: {}", key, value);
            }
        }
        
        // Log any capabilities NOT advertised due to client limitations
        let missing_capabilities = self.get_missing_capabilities_reasons();
        if !missing_capabilities.is_empty() {
            debug!("⚠️  Capabilities NOT advertised due to client limitations:");
            for reason in missing_capabilities {
                debug!("   • {}", reason);
            }
        }
    }
    
    /// Get reasons why certain capabilities are not advertised
    fn get_missing_capabilities_reasons(&self) -> Vec<String> {
        let mut reasons = Vec::new();
        
        if !self.supports_tools() {
            reasons.push("Tools: Client doesn't support tools".to_string());
        }
        
        if !self.supports_resources() {
            reasons.push("Resources: Client doesn't support resources".to_string());
        }
        
        if !self.supports_prompts() {
            reasons.push("Prompts: Client doesn't support prompts".to_string());
        }
        
        if !self.supports_sampling() {
            reasons.push("Sampling: Client doesn't support sampling creation".to_string());
        }
        
        if !self.supports_elicitation() {
            if let Some(elicit) = &self.elicitation {
                if !elicit.create {
                    reasons.push("Elicitation: Client doesn't support elicitation creation".to_string());
                } else if !elicit.accept {
                    reasons.push("Elicitation: Client doesn't support elicitation acceptance".to_string());
                }
            } else {
                reasons.push("Elicitation: Client has no elicitation capability".to_string());
            }
        }
        
        reasons
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_client_capabilities_serialization() {
        let capabilities = ClientCapabilities {
            elicitation: Some(ElicitationCapability {
                create: true,
                accept: true,
                reject: true,
                cancel: true,
                additional: Value::Object(serde_json::Map::new()),
            }),
            sampling: Some(SamplingCapability {
                create: true,
                handle: false,
                additional: Value::Object(serde_json::Map::new()),
            }),
            ..Default::default()
        };

        let json = serde_json::to_value(&capabilities).unwrap();
        let deserialized: ClientCapabilities = serde_json::from_value(json).unwrap();
        
        assert!(deserialized.supports_elicitation());
        assert!(deserialized.supports_sampling());
    }

    #[test]
    fn test_elicitation_support_detection() {
        let capabilities = ClientCapabilities {
            elicitation: Some(ElicitationCapability {
                create: true,
                accept: true,
                reject: false,
                cancel: false,
                additional: Value::Object(serde_json::Map::new()),
            }),
            ..Default::default()
        };

        assert!(capabilities.supports_elicitation());

        let incomplete_capabilities = ClientCapabilities {
            elicitation: Some(ElicitationCapability {
                create: true,
                accept: false, // Missing accept capability
                reject: false,
                cancel: false,
                additional: Value::Object(serde_json::Map::new()),
            }),
            ..Default::default()
        };

        assert!(!incomplete_capabilities.supports_elicitation());
    }

    #[test]
    fn test_parsing_mcp_initialize_capabilities() {
        let mcp_capabilities = json!({
            "tools": {},
            "resources": {
                "subscribe": true
            },
            "prompts": {},
            "sampling": {
                "create": true
            },
            "elicitation": {
                "create": true,
                "accept": true,
                "reject": true,
                "cancel": true
            }
        });

        let capabilities: ClientCapabilities = serde_json::from_value(mcp_capabilities).unwrap();
        
        assert!(capabilities.supports_elicitation());
        assert!(capabilities.supports_sampling());
        assert!(capabilities.resources.as_ref().unwrap().subscribe);
    }

    #[test]
    fn test_safe_external_advertisement_full_client() {
        let capabilities = ClientCapabilities {
            tools: Some(ToolsCapability { enabled: true, ..Default::default() }),
            resources: Some(ResourcesCapability { enabled: true, subscribe: true, ..Default::default() }),
            prompts: Some(PromptsCapability { enabled: true, ..Default::default() }),
            sampling: Some(SamplingCapability { create: true, handle: true, ..Default::default() }),
            elicitation: Some(ElicitationCapability { 
                create: true, accept: true, reject: true, cancel: true, 
                ..Default::default() 
            }),
        };

        let advertisement = capabilities.get_safe_external_advertisement();
        
        // Should include all capabilities since client supports everything
        assert!(advertisement.get("tools").is_some());
        assert!(advertisement.get("resources").is_some());
        assert!(advertisement.get("prompts").is_some());
        assert!(advertisement.get("sampling").is_some());
        assert!(advertisement.get("elicitation").is_some());
        
        // Always include basic capabilities
        assert!(advertisement.get("logging").is_some());
        assert!(advertisement.get("completion").is_some());
        assert!(advertisement.get("roots").is_some());
    }

    #[test]
    fn test_safe_external_advertisement_limited_client() {
        let capabilities = ClientCapabilities {
            tools: Some(ToolsCapability { enabled: true, ..Default::default() }),
            sampling: Some(SamplingCapability { create: true, handle: false, ..Default::default() }),
            // No resources, prompts, or elicitation support
            ..Default::default()
        };

        let advertisement = capabilities.get_safe_external_advertisement();
        
        // Should only include supported capabilities
        assert!(advertisement.get("tools").is_some());
        assert!(advertisement.get("sampling").is_some());
        
        // Should NOT include unsupported capabilities
        assert!(advertisement.get("resources").is_none());
        assert!(advertisement.get("prompts").is_none());
        assert!(advertisement.get("elicitation").is_none());
        
        // Always include basic capabilities
        assert!(advertisement.get("logging").is_some());
        assert!(advertisement.get("completion").is_some());
        assert!(advertisement.get("roots").is_some());
    }

    #[test]
    fn test_safe_external_advertisement_partial_elicitation() {
        let capabilities = ClientCapabilities {
            // Client can create elicitation requests but can't accept them
            elicitation: Some(ElicitationCapability { 
                create: true, 
                accept: false,  // Missing accept capability
                reject: true, 
                cancel: true, 
                ..Default::default() 
            }),
            ..Default::default()
        };

        let advertisement = capabilities.get_safe_external_advertisement();
        
        // Should NOT advertise elicitation since client can't accept
        assert!(advertisement.get("elicitation").is_none());
        assert!(!capabilities.supports_elicitation());
    }

    #[test]
    fn test_capability_missing_reasons() {
        let capabilities = ClientCapabilities {
            sampling: Some(SamplingCapability { create: false, ..Default::default() }),
            elicitation: Some(ElicitationCapability { 
                create: true, accept: false, ..Default::default() 
            }),
            ..Default::default()
        };

        let reasons = capabilities.get_missing_capabilities_reasons();
        
        assert!(reasons.iter().any(|r| r.contains("Tools: Client doesn't support tools")));
        assert!(reasons.iter().any(|r| r.contains("Resources: Client doesn't support resources")));
        assert!(reasons.iter().any(|r| r.contains("Prompts: Client doesn't support prompts")));
        assert!(reasons.iter().any(|r| r.contains("Sampling: Client doesn't support sampling creation")));
        assert!(reasons.iter().any(|r| r.contains("Elicitation: Client doesn't support elicitation acceptance")));
    }
}