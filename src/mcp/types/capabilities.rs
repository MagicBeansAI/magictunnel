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
}