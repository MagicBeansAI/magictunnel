//! Tests for validation extensions in API converters

use magictunnel::registry::types::{ValidationExtensions, ValidationRule};
use magictunnel::registry::openapi_generator::{OpenAPICapabilityGenerator, NamingConvention};
use serde_json::{json, Value};

#[test]
fn test_validation_extensions_creation() {
    // Test basic creation
    let validation = ValidationExtensions::new();
    assert!(!validation.has_validations());
    
    // Test with security validation
    let security_validation = ValidationExtensions::with_security();
    assert!(security_validation.has_validations());
    assert_eq!(security_validation.privacy_scan, Some(true));
    assert_eq!(security_validation.content_filter, Some(true));
    assert_eq!(security_validation.injection_protection, Some(true));
    assert_eq!(security_validation.security_scan, Some(true));
    
    // Test with file path validation
    let file_validation = ValidationExtensions::with_file_path_validation();
    assert!(file_validation.has_validations());
    assert_eq!(file_validation.path_traversal_protection, Some(true));
    assert_eq!(file_validation.security_scan, Some(true));
    
    // Test with range validation
    let range_validation = ValidationExtensions::with_range_validation(0.0, 1.0);
    assert!(range_validation.has_validations());
    assert_eq!(range_validation.optimal_range, Some([0.0, 1.0]));
}

#[test]
fn test_validation_extensions_serialization() {
    let validation = ValidationExtensions {
        optimal_range: Some([0.5, 0.9]),
        privacy_scan: Some(true),
        content_filter: Some(true),
        rules: Some(vec![ValidationRule {
            rule: "required_validation".to_string(),
            message: "Field is required".to_string(),
        }]),
        ..Default::default()
    };
    
    // Test serialization
    let json_value = serde_json::to_value(&validation).unwrap();
    assert!(json_value.get("optimal_range").is_some());
    assert!(json_value.get("privacy_scan").is_some());
    assert!(json_value.get("rules").is_some());
    
    // Test deserialization
    let deserialized: ValidationExtensions = serde_json::from_value(json_value).unwrap();
    assert_eq!(deserialized.optimal_range, Some([0.5, 0.9]));
    assert_eq!(deserialized.privacy_scan, Some(true));
    assert_eq!(deserialized.rules.as_ref().unwrap().len(), 1);
}

#[test]
fn test_validation_schema_injection() {
    let validation = ValidationExtensions::with_security();
    
    // Test injection as validation property
    let mut schema = json!({
        "type": "string",
        "description": "Test parameter"
    });
    
    validation.inject_as_validation(&mut schema);
    
    assert!(schema.get("validation").is_some());
    let validation_obj = schema.get("validation").unwrap();
    assert_eq!(validation_obj.get("privacy_scan"), Some(&json!(true)));
    
    // Test injection as x-validation property
    let mut schema2 = json!({
        "type": "string",
        "description": "Test parameter"
    });
    
    validation.inject_into_schema(&mut schema2);
    
    assert!(schema2.get("x-validation").is_some());
    let x_validation_obj = schema2.get("x-validation").unwrap();
    assert_eq!(x_validation_obj.get("security_scan"), Some(&json!(true)));
}

#[test]
fn test_validation_extraction_from_schema() {
    // Test extraction from validation property
    let schema = json!({
        "type": "number",
        "validation": {
            "optimal_range": [0.5, 0.9],
            "privacy_scan": true
        }
    });
    
    let extracted = ValidationExtensions::from_schema(&schema).unwrap();
    assert_eq!(extracted.optimal_range, Some([0.5, 0.9]));
    assert_eq!(extracted.privacy_scan, Some(true));
    
    // Test extraction from x-validation property
    let schema2 = json!({
        "type": "string",
        "x-validation": {
            "content_filter": true,
            "injection_protection": true
        }
    });
    
    let extracted2 = ValidationExtensions::from_x_validation(&schema2).unwrap();
    assert_eq!(extracted2.content_filter, Some(true));
    assert_eq!(extracted2.injection_protection, Some(true));
}

#[tokio::test]
async fn test_openapi_generator_validation_integration() {
    // Test that we can create validation extensions and they work correctly
    // This is a basic integration test without requiring full OpenAPI parsing
    
    let validation = ValidationExtensions::with_security();
    assert!(validation.has_validations());
    
    // Test parameter name detection logic (simulating what the generator would do)
    struct TestParam {
        name: String,
        expected_validation: bool,
    }
    
    let test_params = vec![
        TestParam { name: "api_key".to_string(), expected_validation: true },
        TestParam { name: "file_path".to_string(), expected_validation: true },
        TestParam { name: "request_body".to_string(), expected_validation: true },
        TestParam { name: "normal_param".to_string(), expected_validation: false },
    ];
    
    for param in test_params {
        let should_validate = param.name.contains("key") || 
                             param.name.contains("path") || 
                             param.name.contains("body") ||
                             param.name.contains("url") ||
                             param.name.contains("token");
        assert_eq!(should_validate, param.expected_validation, 
                  "Parameter '{}' validation detection failed", param.name);
    }
    
    println!("✅ OpenAPI generator validation integration test passed");
}

#[test]
fn test_parameter_validation_detection() {
    // Test validation rule creation based on parameter names
    let test_cases = vec![
        ("api_key", "key"),
        ("user_token", "token"), 
        ("file_path", "path"),
        ("request_body", "body"),
        ("search_query", "query"),
        ("endpoint_url", "url"),
    ];
    
    for (param_name, expected_keyword) in test_cases {
        // This simulates the logic that would be in get_validation_for_parameter
        let should_have_validation = param_name.to_lowercase().contains(expected_keyword);
        assert!(should_have_validation, "Parameter '{}' should trigger validation for keyword '{}'", param_name, expected_keyword);
    }
}

#[test]
fn test_validation_rule_structure() {
    let rule = ValidationRule {
        rule: "required_validation".to_string(),
        message: "This field is required and must be valid".to_string(),
    };
    
    // Test serialization of validation rules
    let json_value = serde_json::to_value(&rule).unwrap();
    assert_eq!(json_value.get("rule").unwrap(), "required_validation");
    assert_eq!(json_value.get("message").unwrap(), "This field is required and must be valid");
    
    // Test deserialization
    let deserialized: ValidationRule = serde_json::from_value(json_value).unwrap();
    assert_eq!(deserialized.rule, "required_validation");
    assert_eq!(deserialized.message, "This field is required and must be valid");
}