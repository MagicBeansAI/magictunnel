//! Comprehensive tests for MCP prompt template management
//!
//! This test suite validates the prompt template functionality including:
//! - Template creation and validation
//! - Argument substitution
//! - Provider management
//! - MCP protocol compliance
//! - Error handling

use std::sync::Arc;
use serde_json::json;
use magictunnel::mcp::prompts::{PromptManager, InMemoryPromptProvider, PromptProvider};
use magictunnel::mcp::types::{PromptTemplate, PromptArgument, PromptMessage};
use magictunnel::error::Result;

/// Test basic prompt template creation and validation
#[tokio::test]
async fn test_prompt_template_creation() -> Result<()> {
    // Test basic template creation
    let template = PromptTemplate::new("test_template".to_string());
    assert_eq!(template.name, "test_template");
    assert!(template.description.is_none());
    assert!(template.arguments.is_empty());

    // Test template with description
    let template = PromptTemplate::with_description(
        "test_template".to_string(),
        "A test template".to_string()
    );
    assert_eq!(template.description, Some("A test template".to_string()));

    // Test template with arguments
    let arg1 = PromptArgument::new("name".to_string());
    let arg2 = PromptArgument::optional("age".to_string());
    
    let template = PromptTemplate::new("test_template".to_string())
        .with_argument(arg1)
        .with_argument(arg2);
    
    assert_eq!(template.arguments.len(), 2);
    assert_eq!(template.required_arguments().len(), 1);
    assert_eq!(template.optional_arguments().len(), 1);

    Ok(())
}

/// Test prompt argument creation
#[tokio::test]
async fn test_prompt_arguments() -> Result<()> {
    // Test required argument
    let arg = PromptArgument::new("name".to_string());
    assert_eq!(arg.name, "name");
    assert!(arg.required);
    assert!(arg.description.is_none());

    // Test required argument with description
    let arg = PromptArgument::with_description(
        "name".to_string(),
        "User's name".to_string()
    );
    assert_eq!(arg.description, Some("User's name".to_string()));
    assert!(arg.required);

    // Test optional argument
    let arg = PromptArgument::optional("age".to_string());
    assert!(!arg.required);

    // Test optional argument with description
    let arg = PromptArgument::optional_with_description(
        "age".to_string(),
        "User's age".to_string()
    );
    assert!(!arg.required);
    assert_eq!(arg.description, Some("User's age".to_string()));

    Ok(())
}

/// Test prompt template validation
#[tokio::test]
async fn test_prompt_template_validation() -> Result<()> {
    // Test valid template
    let template = PromptTemplate::new("valid_template".to_string())
        .with_argument(PromptArgument::new("arg1".to_string()))
        .with_argument(PromptArgument::optional("arg2".to_string()));
    
    assert!(template.validate().is_ok());

    // Test empty name
    let template = PromptTemplate::new("".to_string());
    assert!(template.validate().is_err());

    // Test duplicate argument names
    let template = PromptTemplate::new("test_template".to_string())
        .with_argument(PromptArgument::new("duplicate".to_string()))
        .with_argument(PromptArgument::optional("duplicate".to_string()));
    
    assert!(template.validate().is_err());

    Ok(())
}

/// Test in-memory prompt provider
#[tokio::test]
async fn test_in_memory_prompt_provider() -> Result<()> {
    let mut provider = InMemoryPromptProvider::new("test_provider".to_string());
    
    // Test empty provider
    let (templates, cursor) = provider.list_templates(None).await?;
    assert!(templates.is_empty());
    assert!(cursor.is_none());
    assert_eq!(provider.template_count(), 0);

    // Add a template
    let template = PromptTemplate::new("greeting".to_string())
        .with_argument(PromptArgument::new("name".to_string()));
    let content = "Hello, {{name}}! How are you today?".to_string();
    
    provider.add_template(template.clone(), content.clone())?;
    assert_eq!(provider.template_count(), 1);

    // Test listing templates
    let (templates, _) = provider.list_templates(None).await?;
    assert_eq!(templates.len(), 1);
    assert_eq!(templates[0].name, "greeting");

    // Test getting template content
    let retrieved_content = provider.get_template_content("greeting").await?;
    assert_eq!(retrieved_content, content);

    // Test template support
    assert!(provider.supports_template("greeting"));
    assert!(!provider.supports_template("nonexistent"));

    // Test removing template
    assert!(provider.remove_template("greeting"));
    assert!(!provider.remove_template("nonexistent"));
    assert_eq!(provider.template_count(), 0);

    Ok(())
}

/// Test prompt manager basic functionality
#[tokio::test]
async fn test_prompt_manager_basic() -> Result<()> {
    let manager = PromptManager::new();
    
    // Test empty manager
    let (templates, cursor) = manager.list_templates(None).await?;
    assert!(templates.is_empty());
    assert!(cursor.is_none());
    assert_eq!(manager.provider_count().await, 0);

    // Add a provider with templates
    let mut provider = InMemoryPromptProvider::new("test_provider".to_string());
    
    let template1 = PromptTemplate::new("greeting".to_string())
        .with_argument(PromptArgument::new("name".to_string()));
    let content1 = "Hello, {{name}}!".to_string();
    
    let template2 = PromptTemplate::new("farewell".to_string())
        .with_argument(PromptArgument::new("name".to_string()));
    let content2 = "Goodbye, {{name}}!".to_string();
    
    provider.add_template(template1, content1)?;
    provider.add_template(template2, content2)?;
    
    manager.add_provider(Arc::new(provider)).await;
    assert_eq!(manager.provider_count().await, 1);

    // Test listing templates
    let (templates, _) = manager.list_templates(None).await?;
    assert_eq!(templates.len(), 2);
    
    let template_names: Vec<&str> = templates.iter().map(|t| t.name.as_str()).collect();
    assert!(template_names.contains(&"greeting"));
    assert!(template_names.contains(&"farewell"));

    Ok(())
}

/// Test prompt template argument substitution
#[tokio::test]
async fn test_prompt_argument_substitution() -> Result<()> {
    let manager = PromptManager::new();
    let mut provider = InMemoryPromptProvider::new("test_provider".to_string());
    
    // Create template with required and optional arguments
    let template = PromptTemplate::new("personalized_greeting".to_string())
        .with_argument(PromptArgument::new("name".to_string()))
        .with_argument(PromptArgument::optional("title".to_string()));
    
    let content = "Hello, {{title}} {{name}}! Welcome to our service.".to_string();
    provider.add_template(template, content)?;
    
    manager.add_provider(Arc::new(provider)).await;

    // Test with all arguments provided
    let args = json!({
        "name": "Alice",
        "title": "Dr."
    });
    
    let response = manager.get_template("personalized_greeting", Some(&args)).await?;
    assert_eq!(response.messages.len(), 1);
    assert_eq!(response.messages[0].role, "user");
    assert_eq!(response.messages[0].content, "Hello, Dr. Alice! Welcome to our service.");

    // Test with only required arguments
    let args = json!({
        "name": "Bob"
    });
    
    let response = manager.get_template("personalized_greeting", Some(&args)).await?;
    assert_eq!(response.messages[0].content, "Hello,  Bob! Welcome to our service.");

    Ok(())
}

/// Test prompt template error handling
#[tokio::test]
async fn test_prompt_error_handling() -> Result<()> {
    let manager = PromptManager::new();
    let mut provider = InMemoryPromptProvider::new("test_provider".to_string());
    
    // Create template with required argument
    let template = PromptTemplate::new("greeting".to_string())
        .with_argument(PromptArgument::new("name".to_string()));
    let content = "Hello, {{name}}!".to_string();
    
    provider.add_template(template, content)?;
    manager.add_provider(Arc::new(provider)).await;

    // Test missing template
    let result = manager.get_template("nonexistent", None).await;
    assert!(result.is_err());

    // Test missing required argument
    let result = manager.get_template("greeting", None).await;
    assert!(result.is_err());

    // Test missing required argument with empty object
    let args = json!({});
    let result = manager.get_template("greeting", Some(&args)).await;
    assert!(result.is_err());

    Ok(())
}

/// Test prompt message creation
#[tokio::test]
async fn test_prompt_messages() -> Result<()> {
    // Test basic message creation
    let msg = PromptMessage::new("user".to_string(), "Hello!".to_string());
    assert_eq!(msg.role, "user");
    assert_eq!(msg.content, "Hello!");

    // Test convenience methods
    let user_msg = PromptMessage::user("User message".to_string());
    assert_eq!(user_msg.role, "user");

    let assistant_msg = PromptMessage::assistant("Assistant message".to_string());
    assert_eq!(assistant_msg.role, "assistant");

    let system_msg = PromptMessage::system("System message".to_string());
    assert_eq!(system_msg.role, "system");

    Ok(())
}

/// Test multiple providers in prompt manager
#[tokio::test]
async fn test_multiple_prompt_providers() -> Result<()> {
    let manager = PromptManager::new();
    
    // Create first provider
    let mut provider1 = InMemoryPromptProvider::new("provider1".to_string());
    let template1 = PromptTemplate::new("template1".to_string());
    provider1.add_template(template1, "Content 1".to_string())?;
    
    // Create second provider
    let mut provider2 = InMemoryPromptProvider::new("provider2".to_string());
    let template2 = PromptTemplate::new("template2".to_string());
    provider2.add_template(template2, "Content 2".to_string())?;
    
    // Add both providers
    manager.add_provider(Arc::new(provider1)).await;
    manager.add_provider(Arc::new(provider2)).await;
    
    assert_eq!(manager.provider_count().await, 2);

    // Test listing templates from both providers
    let (templates, _) = manager.list_templates(None).await?;
    assert_eq!(templates.len(), 2);
    
    let template_names: Vec<&str> = templates.iter().map(|t| t.name.as_str()).collect();
    assert!(template_names.contains(&"template1"));
    assert!(template_names.contains(&"template2"));

    // Test getting templates from different providers
    let args = json!({});
    let response1 = manager.get_template("template1", Some(&args)).await?;
    assert_eq!(response1.messages[0].content, "Content 1");
    
    let response2 = manager.get_template("template2", Some(&args)).await?;
    assert_eq!(response2.messages[0].content, "Content 2");

    Ok(())
}
