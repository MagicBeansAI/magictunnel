//! Capability Validator
//!
//! This module provides functionality to validate capability files against
//! a set of rules or schemas.
//!
//! # Overview
//!
//! The capability validator ensures that capability files conform to the MCP specification
//! and follow best practices. It applies a series of validation rules to check for issues
//! such as duplicate tool names, empty descriptions, and invalid routing configurations.
//!
//! # Validation Rules
//!
//! The validator includes several built-in validation rules:
//!
//! - **Duplicate Tool Names**: Ensures all tool names are unique within a capability file
//! - **Empty Tool Names**: Checks that all tools have non-empty names
//! - **Empty Descriptions**: Verifies that all tools have meaningful descriptions
//! - **Invalid Routing**: Validates that routing configurations are correct and supported
//!
//! # Usage
//!
//! The validator can be used in two main ways:
//!
//! 1. **Strict Validation**: Fails on the first validation error (`validate` method)
//! 2. **Issue Collection**: Collects all validation issues without failing (`get_validation_issues` method)
//!
//! This allows for both strict validation during CI/CD pipelines and more informative
//! validation during development.

use crate::error::{ProxyError, Result};
use crate::registry::types::{CapabilityFile, ToolDefinition};
use std::collections::HashSet;

/// Capability Validator
///
/// Provides functionality to validate capability files against a set of rules.
/// It ensures that capability files conform to the MCP specification and follow best practices.
///
/// # Example
///
/// ```
/// use magictunnel::registry::commands::CapabilityValidator;
/// use magictunnel::registry::types::CapabilityFile;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let validator = CapabilityValidator::new();
/// let capability_file = CapabilityFile::new(vec![])?;
///
/// // Strict validation (fails on first error)
/// match validator.validate(&capability_file) {
///     Ok(_) => println!("Validation passed!"),
///     Err(e) => println!("Validation failed: {}", e),
/// }
///
/// // Collect all issues
/// let issues = validator.get_validation_issues(&capability_file);
/// for issue in issues {
///     println!("Issue: {}", issue);
/// }
/// # Ok(())
/// # }
/// ```
pub struct CapabilityValidator {
    /// Validation rules to apply
    rules: Vec<Box<dyn ValidationRule>>,
}

impl Clone for CapabilityValidator {
    fn clone(&self) -> Self {
        // Create a new validator with default rules
        // This is a simplification since we can't clone the trait objects directly
        Self::new()
    }
}

impl std::fmt::Debug for CapabilityValidator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CapabilityValidator")
            .field("rules_count", &self.rules.len())
            .finish()
    }
}

impl CapabilityValidator {
    /// Create a new capability validator with default rules
    pub fn new() -> Self {
        let mut validator = Self {
            rules: Vec::new(),
        };
        
        // Add default validation rules
        validator.add_rule(Box::new(DuplicateToolNameRule));
        validator.add_rule(Box::new(EmptyToolNameRule));
        validator.add_rule(Box::new(EmptyDescriptionRule));
        validator.add_rule(Box::new(InvalidRoutingRule));
        
        validator
    }

    /// Add a validation rule
    pub fn add_rule(&mut self, rule: Box<dyn ValidationRule>) -> &mut Self {
        self.rules.push(rule);
        self
    }

    /// Validate a capability file
    /// 
    /// This method validates a capability file against all registered rules.
    /// 
    /// # Arguments
    /// 
    /// * `file` - The capability file to validate
    /// 
    /// # Returns
    /// 
    /// A Result containing Ok(()) if validation passes, or an error with details
    pub fn validate(&self, file: &CapabilityFile) -> Result<()> {
        let mut validation_errors = Vec::new();

        // Apply each validation rule
        for rule in &self.rules {
            if let Err(err) = rule.validate(file) {
                validation_errors.push(format!("{}: {}", rule.name(), err));
            }
        }

        // If there are any validation errors, return them
        if !validation_errors.is_empty() {
            return Err(ProxyError::validation(format!(
                "Capability file validation failed:\n- {}", 
                validation_errors.join("\n- ")
            )));
        }

        // Run the built-in validation as well
        file.validate()?;

        Ok(())
    }

    /// Validate multiple capability files
    /// 
    /// This method validates multiple capability files against all registered rules.
    /// 
    /// # Arguments
    /// 
    /// * `files` - A vector of capability files to validate
    /// 
    /// # Returns
    /// 
    /// A Result containing Ok(()) if validation passes for all files, or an error with details
    pub fn validate_multiple(&self, files: &[CapabilityFile]) -> Result<()> {
        for (i, file) in files.iter().enumerate() {
            if let Err(err) = self.validate(file) {
                return Err(ProxyError::validation(format!(
                    "Validation failed for file #{}: {}", i + 1, err
                )));
            }
        }
        Ok(())
    }

    /// Get a summary of validation issues without failing
    /// 
    /// This method validates a capability file and returns a list of validation issues
    /// without failing on the first error.
    /// 
    /// # Arguments
    /// 
    /// * `file` - The capability file to validate
    /// 
    /// # Returns
    /// 
    /// A vector of validation issue strings
    pub fn get_validation_issues(&self, file: &CapabilityFile) -> Vec<String> {
        let mut issues = Vec::new();

        // Apply each validation rule
        for rule in &self.rules {
            if let Err(err) = rule.validate(file) {
                issues.push(format!("{}: {}", rule.name(), err));
            }
        }

        // Check built-in validation
        if let Err(err) = file.validate() {
            issues.push(format!("Built-in validation: {}", err));
        }

        issues
    }
}

impl Default for CapabilityValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Validation rule trait
///
/// This trait defines the interface for all validation rules.
/// Custom validation rules can be implemented by implementing this trait.
///
/// # Example
///
/// ```
/// use magictunnel::registry::commands::validate::{ValidationRule, CapabilityValidator};
/// use magictunnel::registry::types::CapabilityFile;
/// use magictunnel::error::Result;
///
/// struct MyCustomRule;
///
/// impl ValidationRule for MyCustomRule {
///     fn name(&self) -> &str {
///         "My Custom Rule"
///     }
///
///     fn validate(&self, file: &CapabilityFile) -> Result<()> {
///         // Custom validation logic
///         Ok(())
///     }
/// }
///
/// let mut validator = CapabilityValidator::new();
/// validator.add_rule(Box::new(MyCustomRule));
/// ```
pub trait ValidationRule: Send + Sync {
    /// Get the rule name
    fn name(&self) -> &str;
    
    /// Validate a capability file against this rule
    fn validate(&self, file: &CapabilityFile) -> Result<()>;
}

/// Rule to check for duplicate tool names
///
/// This rule ensures that all tool names within a capability file are unique.
/// Duplicate tool names can cause confusion and conflicts when the capability file is used.
#[derive(Debug)]
pub struct DuplicateToolNameRule;

impl ValidationRule for DuplicateToolNameRule {
    fn name(&self) -> &str {
        "Duplicate Tool Names"
    }
    
    fn validate(&self, file: &CapabilityFile) -> Result<()> {
        let mut tool_names = HashSet::new();
        
        for tool in &file.tools {
            if !tool_names.insert(&tool.name) {
                return Err(ProxyError::validation(
                    format!("Duplicate tool name found: {}", tool.name)
                ));
            }
        }
        
        Ok(())
    }
}

/// Rule to check for empty tool names
///
/// This rule ensures that all tools have non-empty names.
/// Tool names are required for proper identification and usage.
#[derive(Debug)]
pub struct EmptyToolNameRule;

impl ValidationRule for EmptyToolNameRule {
    fn name(&self) -> &str {
        "Empty Tool Names"
    }
    
    fn validate(&self, file: &CapabilityFile) -> Result<()> {
        for tool in &file.tools {
            if tool.name.trim().is_empty() {
                return Err(ProxyError::validation("Tool name cannot be empty"));
            }
        }
        
        Ok(())
    }
}

/// Rule to check for empty descriptions
///
/// This rule ensures that all tools have meaningful descriptions.
/// Descriptions are essential for users to understand what each tool does.
#[derive(Debug)]
pub struct EmptyDescriptionRule;

impl ValidationRule for EmptyDescriptionRule {
    fn name(&self) -> &str {
        "Empty Descriptions"
    }
    
    fn validate(&self, file: &CapabilityFile) -> Result<()> {
        for tool in &file.tools {
            if tool.description.trim().is_empty() {
                return Err(ProxyError::validation(
                    format!("Tool '{}' has an empty description", tool.name)
                ));
            }
        }
        
        Ok(())
    }
}

/// Rule to check for invalid routing configurations
///
/// This rule validates that routing configurations are correct and supported.
/// It checks both the routing type and the specific configuration for that type.
#[derive(Debug)]
pub struct InvalidRoutingRule;

impl ValidationRule for InvalidRoutingRule {
    fn name(&self) -> &str {
        "Invalid Routing"
    }
    
    fn validate(&self, file: &CapabilityFile) -> Result<()> {
        for tool in &file.tools {
            // Validate routing configuration
            if let Err(err) = tool.routing.validate() {
                return Err(ProxyError::validation(
                    format!("Tool '{}' has invalid routing configuration: {}",
                            tool.name, err)
                ));
            }
        }

        Ok(())
    }
}