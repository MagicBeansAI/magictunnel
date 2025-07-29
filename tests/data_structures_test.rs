//! Tests for core data structures and validation

use magictunnel::mcp::types::*;
use magictunnel::registry::types::*;
use serde_json::json;

#[cfg(test)]
mod tool_tests {
    use super::*;

    #[test]
    fn test_tool_creation_and_validation() {
        // Valid tool creation
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            },
            "required": ["name"]
        });
        
        let tool = Tool::new(
            "test_tool".to_string(),
            "A test tool".to_string(),
            schema
        );
        
        assert!(tool.is_ok());
        let tool = tool.unwrap();
        assert_eq!(tool.name, "test_tool");
        assert_eq!(tool.description, Some("A test tool".to_string()));
        assert!(tool.is_mcp_compliant());
    }

    #[test]
    fn test_tool_validation_failures() {
        let schema = json!({"type": "object"});
        
        // Empty name should fail
        let result = Tool::new("".to_string(), "Description".to_string(), schema.clone());
        assert!(result.is_err());
        
        // Empty description should fail
        let result = Tool::new("name".to_string(), "".to_string(), schema);
        assert!(result.is_err());
    }

    #[test]
    fn test_tool_argument_validation() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "number"}
            },
            "required": ["name"]
        });
        
        let tool = Tool::new(
            "test_tool".to_string(),
            "A test tool".to_string(),
            schema
        ).unwrap();
        
        // Valid arguments
        let valid_args = json!({"name": "John", "age": 30});
        assert!(tool.validate_arguments(&valid_args).is_ok());
        
        // Missing required field
        let invalid_args = json!({"age": 30});
        assert!(tool.validate_arguments(&invalid_args).is_err());
        
        // Wrong type
        let invalid_args = json!({"name": 123});
        assert!(tool.validate_arguments(&invalid_args).is_err());
    }

    #[test]
    fn test_tool_with_annotations() {
        let schema = json!({"type": "object"});
        let annotations = ToolAnnotations::new()
            .read_only(true)
            .idempotent(true);
        
        let tool = Tool::with_annotations(
            "safe_tool".to_string(),
            "A safe tool".to_string(),
            schema,
            annotations
        );
        
        assert!(tool.is_ok());
        let tool = tool.unwrap();
        assert!(tool.annotations.is_some());
        assert!(tool.annotations.as_ref().unwrap().is_safe());
    }
}

#[cfg(test)]
mod annotations_tests {
    use super::*;

    #[test]
    fn test_annotations_creation() {
        let annotations = ToolAnnotations::with_title("Test Tool".to_string())
            .read_only(true)
            .destructive(false)
            .idempotent(true);
        
        assert_eq!(annotations.title, Some("Test Tool".to_string()));
        assert_eq!(annotations.read_only_hint, Some(true));
        assert_eq!(annotations.destructive_hint, Some(false));
        assert_eq!(annotations.idempotent_hint, Some(true));
        assert!(annotations.is_safe());
    }

    #[test]
    fn test_annotations_validation() {
        // Conflicting hints should fail validation
        let annotations = ToolAnnotations::new()
            .read_only(true)
            .destructive(true);
        
        assert!(annotations.validate().is_err());
        
        // Non-conflicting hints should pass
        let annotations = ToolAnnotations::new()
            .read_only(true)
            .destructive(false);
        
        assert!(annotations.validate().is_ok());
    }

    #[test]
    fn test_annotations_safety() {
        // Read-only tool is safe
        let annotations = ToolAnnotations::new().read_only(true);
        assert!(annotations.is_safe());
        
        // Non-destructive tool is safe
        let annotations = ToolAnnotations::new().destructive(false);
        assert!(annotations.is_safe());
        
        // Destructive tool is not safe
        let annotations = ToolAnnotations::new().destructive(true);
        assert!(!annotations.is_safe());
    }
}

#[cfg(test)]
mod tool_call_tests {
    use super::*;

    #[test]
    fn test_tool_call_creation() {
        let args = json!({"param": "value"});
        let call = ToolCall::new("test_tool".to_string(), args.clone());
        
        assert_eq!(call.name, "test_tool");
        assert_eq!(call.arguments, args);
        assert!(call.validate().is_ok());
    }

    #[test]
    fn test_tool_call_validation() {
        let args = json!({});
        
        // Empty name should fail
        let call = ToolCall::new("".to_string(), args);
        assert!(call.validate().is_err());
    }

    #[test]
    fn test_tool_call_against_tool() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            },
            "required": ["name"]
        });
        
        let tool = Tool::new(
            "test_tool".to_string(),
            "A test tool".to_string(),
            schema
        ).unwrap();
        
        // Valid call
        let call = ToolCall::new("test_tool".to_string(), json!({"name": "test"}));
        assert!(call.validate_against_tool(&tool).is_ok());
        
        // Wrong tool name
        let call = ToolCall::new("wrong_tool".to_string(), json!({"name": "test"}));
        assert!(call.validate_against_tool(&tool).is_err());
        
        // Invalid arguments
        let call = ToolCall::new("test_tool".to_string(), json!({}));
        assert!(call.validate_against_tool(&tool).is_err());
    }
}

#[cfg(test)]
mod tool_result_tests {
    use super::*;

    #[test]
    fn test_successful_result() {
        let data = json!({"result": "success"});
        let result = ToolResult::success(data.clone());
        
        assert!(result.success);
        assert_eq!(result.data, Some(data));
        assert!(result.error.is_none());
        assert!(result.validate().is_ok());
    }

    #[test]
    fn test_error_result() {
        let result = ToolResult::error("Something went wrong".to_string());
        
        assert!(!result.success);
        assert!(result.data.is_none());
        assert_eq!(result.error, Some("Something went wrong".to_string()));
        assert!(result.validate().is_ok());
    }

    #[test]
    fn test_result_with_metadata() {
        let data = json!({"result": "success"});
        let metadata = json!({"execution_time": 100});
        let result = ToolResult::success_with_metadata(data.clone(), metadata.clone());

        assert!(result.success);
        assert_eq!(result.data, Some(data));
        assert_eq!(result.metadata, Some(metadata));
        assert!(result.validate().is_ok());
    }
}

#[cfg(test)]
mod routing_config_tests {
    use super::*;

    #[test]
    fn test_routing_config_creation() {
        let config = json!({
            "command": "echo",
            "args": ["hello"]
        });

        let routing = RoutingConfig::new("subprocess".to_string(), config);
        assert_eq!(routing.routing_type(), "subprocess");
        assert!(routing.is_supported_type());
        assert!(routing.validate().is_ok());
    }

    #[test]
    fn test_subprocess_routing_validation() {
        // Valid subprocess config
        let config = json!({"command": "echo", "args": ["hello"]});
        let routing = RoutingConfig::new("subprocess".to_string(), config);
        assert!(routing.validate().is_ok());

        // Missing command should fail
        let config = json!({"args": ["hello"]});
        let routing = RoutingConfig::new("subprocess".to_string(), config);
        assert!(routing.validate().is_err());
    }

    #[test]
    fn test_http_routing_validation() {
        // Valid HTTP config
        let config = json!({
            "method": "POST",
            "url": "https://api.example.com/tool"
        });
        let routing = RoutingConfig::new("http".to_string(), config);
        assert!(routing.validate().is_ok());

        // Missing URL should fail
        let config = json!({"method": "POST"});
        let routing = RoutingConfig::new("http".to_string(), config);
        assert!(routing.validate().is_err());

        // Missing method should fail
        let config = json!({"url": "https://api.example.com/tool"});
        let routing = RoutingConfig::new("http".to_string(), config);
        assert!(routing.validate().is_err());
    }

    #[test]
    fn test_llm_routing_validation() {
        // Valid LLM config
        let config = json!({
            "provider": "openai",
            "model": "gpt-4"
        });
        let routing = RoutingConfig::new("llm".to_string(), config);
        assert!(routing.validate().is_ok());

        // Missing provider should fail
        let config = json!({"model": "gpt-4"});
        let routing = RoutingConfig::new("llm".to_string(), config);
        assert!(routing.validate().is_err());
    }

    #[test]
    fn test_unknown_routing_type() {
        let config = json!({"some": "config"});
        let routing = RoutingConfig::new("unknown_type".to_string(), config);

        assert!(!routing.is_supported_type());
        // Should still validate (with warning)
        assert!(routing.validate().is_ok());
    }
}

#[cfg(test)]
mod tool_definition_tests {
    use super::*;

    #[test]
    fn test_tool_definition_creation() {
        let schema = json!({"type": "object"});
        let tool = Tool::new("test_tool".to_string(), "Test tool".to_string(), schema).unwrap();
        let routing = RoutingConfig::new("subprocess".to_string(), json!({"command": "echo"}));

        let tool_def = ToolDefinition::new(tool, routing);
        assert!(tool_def.is_ok());

        let tool_def = tool_def.unwrap();
        assert_eq!(tool_def.name(), "test_tool");
        assert_eq!(tool_def.description(), "Test tool");
        assert_eq!(tool_def.routing_type(), "subprocess");
        assert!(tool_def.is_safe());
    }

    #[test]
    fn test_tool_definition_with_unsafe_tool() {
        let schema = json!({"type": "object"});
        let annotations = ToolAnnotations::new().destructive(true);
        let tool = Tool::with_annotations(
            "dangerous_tool".to_string(),
            "A dangerous tool".to_string(),
            schema,
            annotations
        ).unwrap();
        let routing = RoutingConfig::new("subprocess".to_string(), json!({"command": "rm"}));

        let tool_def = ToolDefinition::new(tool, routing).unwrap();
        assert!(!tool_def.is_safe());
    }
}

#[cfg(test)]
mod file_metadata_tests {
    use super::*;

    #[test]
    fn test_metadata_creation() {
        let metadata = FileMetadata::with_name("test_file".to_string())
            .description("A test capability file".to_string())
            .version("1.0.0".to_string())
            .author("Test Team".to_string())
            .tags(vec!["test".to_string(), "example".to_string()]);

        assert_eq!(metadata.name, Some("test_file".to_string()));
        assert_eq!(metadata.description, Some("A test capability file".to_string()));
        assert_eq!(metadata.version, Some("1.0.0".to_string()));
        assert!(metadata.is_complete());
        assert!(metadata.validate().is_ok());
    }

    #[test]
    fn test_metadata_validation() {
        // Empty version should fail
        let metadata = FileMetadata::new().version("".to_string());
        assert!(metadata.validate().is_err());

        // Empty tag should fail
        let metadata = FileMetadata::new().tags(vec!["valid".to_string(), "".to_string()]);
        assert!(metadata.validate().is_err());

        // Valid metadata should pass
        let metadata = FileMetadata::new().version("1.0.0".to_string());
        assert!(metadata.validate().is_ok());
    }
}

#[cfg(test)]
mod capability_file_tests {
    use super::*;

    fn create_test_tool(name: &str) -> ToolDefinition {
        let schema = json!({"type": "object"});
        let tool = Tool::new(name.to_string(), format!("{} description", name), schema).unwrap();
        let routing = RoutingConfig::new("subprocess".to_string(), json!({"command": "echo"}));
        ToolDefinition::new(tool, routing).unwrap()
    }

    #[test]
    fn test_capability_file_creation() {
        let tools = vec![
            create_test_tool("tool1"),
            create_test_tool("tool2"),
        ];

        let file = CapabilityFile::new(tools);
        assert!(file.is_ok());

        let file = file.unwrap();
        assert_eq!(file.tool_count(), 2);
        assert!(file.get_tool("tool1").is_some());
        assert!(file.get_tool("tool2").is_some());
        assert!(file.get_tool("nonexistent").is_none());
    }

    #[test]
    fn test_capability_file_with_metadata() {
        let metadata = FileMetadata::with_name("test_file".to_string())
            .description("Test capability file".to_string());
        let tools = vec![create_test_tool("tool1")];

        let file = CapabilityFile::with_metadata(metadata, tools);
        assert!(file.is_ok());

        let file = file.unwrap();
        assert!(file.metadata.is_some());
        assert_eq!(file.metadata.as_ref().unwrap().name, Some("test_file".to_string()));
    }

    #[test]
    fn test_capability_file_duplicate_tools() {
        let tools = vec![
            create_test_tool("duplicate"),
            create_test_tool("duplicate"), // Same name
        ];

        let file = CapabilityFile::new(tools);
        assert!(file.is_err()); // Should fail due to duplicate names
    }

    #[test]
    fn test_capability_file_filtering() {
        let mut tools = vec![
            create_test_tool("tool1"),
            create_test_tool("tool2"),
        ];

        // Make one tool unsafe
        let schema = json!({"type": "object"});
        let annotations = ToolAnnotations::new().destructive(true);
        let unsafe_tool = Tool::with_annotations(
            "unsafe_tool".to_string(),
            "Unsafe tool".to_string(),
            schema,
            annotations
        ).unwrap();
        let routing = RoutingConfig::new("subprocess".to_string(), json!({"command": "rm"}));
        tools.push(ToolDefinition::new(unsafe_tool, routing).unwrap());

        let file = CapabilityFile::new(tools).unwrap();

        // Test filtering
        assert_eq!(file.tool_count(), 3);
        assert_eq!(file.safe_tools().len(), 2); // Only 2 safe tools
        assert_eq!(file.tools_by_routing_type("subprocess").len(), 3);

        let tool_names = file.tool_names();
        assert!(tool_names.contains(&"tool1"));
        assert!(tool_names.contains(&"tool2"));
        assert!(tool_names.contains(&"unsafe_tool"));
    }
}
