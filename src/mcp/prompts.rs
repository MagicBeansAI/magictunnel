//! MCP Prompt Template Management
//! 
//! This module provides prompt template management functionality for the MCP proxy,
//! including template storage, argument substitution, and MCP protocol compliance.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde_json::Value;
use tracing::{debug, info, warn};

use crate::error::{ProxyError, Result};
use crate::mcp::types::{PromptTemplate, PromptMessage, PromptGetResponse};

/// Trait for prompt template providers
#[async_trait::async_trait]
pub trait PromptProvider: Send + Sync {
    /// List available prompt templates
    async fn list_templates(&self, cursor: Option<&str>) -> Result<(Vec<PromptTemplate>, Option<String>)>;
    
    /// Get template content by name
    async fn get_template_content(&self, name: &str) -> Result<String>;
    
    /// Check if provider supports the given template name
    fn supports_template(&self, name: &str) -> bool;
    
    /// Get provider name for debugging
    fn name(&self) -> &str;
}

/// In-memory prompt template provider
pub struct InMemoryPromptProvider {
    name: String,
    templates: HashMap<String, PromptTemplate>,
    template_content: HashMap<String, String>,
}

impl InMemoryPromptProvider {
    /// Create a new in-memory prompt provider
    pub fn new(name: String) -> Self {
        Self {
            name,
            templates: HashMap::new(),
            template_content: HashMap::new(),
        }
    }

    /// Add a template to the provider
    pub fn add_template(&mut self, template: PromptTemplate, content: String) -> Result<()> {
        template.validate()?;
        
        let name = template.name.clone();
        self.templates.insert(name.clone(), template);
        self.template_content.insert(name.clone(), content);

        debug!("Added template '{}' to provider '{}'", name, self.name);
        Ok(())
    }

    /// Remove a template from the provider
    pub fn remove_template(&mut self, name: &str) -> bool {
        let removed = self.templates.remove(name).is_some();
        self.template_content.remove(name);
        
        if removed {
            debug!("Removed template '{}' from provider '{}'", name, self.name);
        }
        
        removed
    }

    /// Get template count
    pub fn template_count(&self) -> usize {
        self.templates.len()
    }
}

#[async_trait::async_trait]
impl PromptProvider for InMemoryPromptProvider {
    async fn list_templates(&self, _cursor: Option<&str>) -> Result<(Vec<PromptTemplate>, Option<String>)> {
        let templates: Vec<PromptTemplate> = self.templates.values().cloned().collect();
        debug!("Listed {} templates from provider '{}'", templates.len(), self.name);
        Ok((templates, None)) // No pagination for in-memory provider
    }
    
    async fn get_template_content(&self, name: &str) -> Result<String> {
        self.template_content.get(name)
            .cloned()
            .ok_or_else(|| ProxyError::mcp(format!("Template not found: {}", name)))
    }
    
    fn supports_template(&self, name: &str) -> bool {
        self.templates.contains_key(name)
    }
    
    fn name(&self) -> &str {
        &self.name
    }
}

/// Prompt template manager
pub struct PromptManager {
    providers: Arc<RwLock<Vec<Arc<dyn PromptProvider>>>>,
}

impl PromptManager {
    /// Create a new prompt manager
    pub fn new() -> Self {
        Self {
            providers: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Add a prompt provider
    pub async fn add_provider(&self, provider: Arc<dyn PromptProvider>) {
        let mut providers = self.providers.write().await;
        providers.push(provider);
        info!("Added prompt provider, total providers: {}", providers.len());
    }

    /// List all available prompt templates
    pub async fn list_templates(&self, cursor: Option<&str>) -> Result<(Vec<PromptTemplate>, Option<String>)> {
        let providers = self.providers.read().await;
        let mut all_templates = Vec::new();
        
        for provider in providers.iter() {
            match provider.list_templates(cursor).await {
                Ok((templates, _)) => {
                    all_templates.extend(templates);
                }
                Err(e) => {
                    warn!("Failed to list templates from provider '{}': {}", provider.name(), e);
                }
            }
        }
        
        debug!("Listed {} total templates from {} providers", all_templates.len(), providers.len());
        Ok((all_templates, None)) // No pagination across providers for now
    }

    /// Get a specific template with argument substitution
    pub async fn get_template(&self, name: &str, arguments: Option<&Value>) -> Result<PromptGetResponse> {
        let providers = self.providers.read().await;
        
        // Find provider that supports this template
        let provider = providers.iter()
            .find(|p| p.supports_template(name))
            .ok_or_else(|| ProxyError::mcp(format!("Template not found: {}", name)))?;
        
        // Get template metadata and content
        let (templates, _) = provider.list_templates(None).await?;
        let template = templates.iter()
            .find(|t| t.name == name)
            .ok_or_else(|| ProxyError::mcp(format!("Template metadata not found: {}", name)))?;
        
        let content = provider.get_template_content(name).await?;
        
        // Validate arguments
        self.validate_arguments(template, arguments)?;
        
        // Substitute arguments in template
        let rendered_content = self.substitute_arguments(&content, template, arguments)?;
        
        // Create response with rendered message
        let messages = vec![PromptMessage::user(rendered_content)];
        
        Ok(PromptGetResponse {
            messages,
            description: template.description.clone(),
        })
    }

    /// Validate template arguments
    fn validate_arguments(&self, template: &PromptTemplate, arguments: Option<&Value>) -> Result<()> {
        let args = arguments.unwrap_or(&Value::Null);
        
        // Check required arguments
        for required_arg in template.required_arguments() {
            if !args.is_object() || !args.as_object().unwrap().contains_key(&required_arg.name) {
                return Err(ProxyError::validation(
                    format!("Missing required argument: {}", required_arg.name)
                ));
            }
        }
        
        Ok(())
    }

    /// Substitute arguments in template content
    fn substitute_arguments(&self, content: &str, template: &PromptTemplate, arguments: Option<&Value>) -> Result<String> {
        let mut result = content.to_string();

        // First, substitute provided arguments
        if let Some(args) = arguments {
            if let Some(args_obj) = args.as_object() {
                for (key, value) in args_obj {
                    let placeholder = format!("{{{{{}}}}}", key);
                    let replacement = match value {
                        Value::String(s) => s.clone(),
                        _ => value.to_string(),
                    };
                    result = result.replace(&placeholder, &replacement);
                }
            }
        }

        // Then, substitute optional arguments that weren't provided with empty strings
        for optional_arg in template.optional_arguments() {
            let placeholder = format!("{{{{{}}}}}", optional_arg.name);
            if result.contains(&placeholder) {
                result = result.replace(&placeholder, "");
            }
        }

        // Check for unsubstituted required arguments
        for required_arg in template.required_arguments() {
            let placeholder = format!("{{{{{}}}}}", required_arg.name);
            if result.contains(&placeholder) {
                return Err(ProxyError::validation(
                    format!("Required argument '{}' was not substituted", required_arg.name)
                ));
            }
        }

        debug!("Substituted arguments in template '{}', result length: {}", template.name, result.len());
        Ok(result)
    }

    /// Get provider count
    pub async fn provider_count(&self) -> usize {
        self.providers.read().await.len()
    }
}

impl Default for PromptManager {
    fn default() -> Self {
        Self::new()
    }
}
