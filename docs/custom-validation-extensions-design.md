# Custom Validation Extensions Design

## Overview

MagicTunnel capability YAML files use custom validation extensions that extend standard JSON Schema with domain-specific validation rules. This document outlines approaches to handle these extensions in API converters.

## Current Validation Extensions

### 1. Range Validation
```yaml
confidence_threshold:
  type: number
  validation:
    optimal_range: [0.5, 0.9]
```

### 2. Security Validation  
```yaml
context:
  type: string
  validation:
    privacy_scan: true
    content_filter: true
    injection_protection: true
```

### 3. Tool Validation
```yaml
preferred_tools:
  items:
    validation:
      tool_accessible: true
      tool_exists: true
```

### 4. Rule-Based Validation
```yaml
url:
  validation:
    - rule: required_validation
      message: "url must be provided and valid"
```

## Implementation Approaches

### Option 1: JSON Schema Extensions (Recommended)

**Approach**: Use JSON Schema's extension mechanism with custom keywords

**Implementation**:
```rust
// In registry/types.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationExtensions {
    // Range validation
    pub optimal_range: Option<[f64; 2]>,
    
    // Security validation
    pub privacy_scan: Option<bool>,
    pub content_filter: Option<bool>,  
    pub injection_protection: Option<bool>,
    pub semantic_analysis: Option<bool>,
    pub path_traversal_protection: Option<bool>,
    
    // Tool validation
    pub tool_accessible: Option<bool>,
    pub tool_exists: Option<bool>,
    
    // Size validation  
    pub max_size_mb: Option<u64>,
    
    // Rule-based validation
    pub rules: Option<Vec<ValidationRule>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRule {
    pub rule: String,
    pub message: String,
}
```

**Benefits**:
- ✅ Preserves all validation information
- ✅ Standard JSON Schema compatibility  
- ✅ Type-safe validation extensions
- ✅ Easy to extend with new validation types

### Option 2: JSON Schema Custom Properties

**Approach**: Use JSON Schema's `x-*` custom properties convention

**Implementation**:
```json
{
  "type": "number",
  "minimum": 0.0,
  "maximum": 1.0,
  "x-validation": {
    "optimal_range": [0.5, 0.9]
  }
}
```

**Benefits**:
- ✅ Standard JSON Schema practice
- ✅ Tool compatibility
- ✅ Clear separation of standard vs custom validation

### Option 3: Annotation-Based Extensions

**Approach**: Use JSON Schema annotations to store validation metadata

**Implementation**:
```json
{
  "type": "string",
  "maxLength": 1000,
  "$comment": "validation:privacy_scan=true,relevance_check=true"
}
```

**Benefits**:
- ✅ JSON Schema compliant
- ❌ Less structured, harder to parse

### Option 4: Separate Validation Schema

**Approach**: Keep JSON Schema pure, store validation extensions separately

**Implementation**:
```rust
pub struct EnhancedInputSchema {
    pub json_schema: Value,
    pub validation_extensions: Option<HashMap<String, ValidationExtensions>>,
}
```

**Benefits**:
- ✅ Clean separation
- ✅ JSON Schema purity
- ❌ More complex data structure

## Recommended Implementation: Option 1 + Option 2 Hybrid

### Phase 1: Type-Safe Extensions
1. **Add ValidationExtensions struct** to registry types
2. **Modify input schema processing** to recognize validation fields
3. **Update API converters** to generate validation extensions

### Phase 2: JSON Schema Integration  
1. **Add x-validation support** in generated schemas
2. **Implement validation runtime** that processes extensions
3. **Add validation middleware** for tool execution

### Phase 3: Advanced Features
1. **LLM-assisted validation** using validation metadata
2. **Dynamic validation rules** based on context
3. **Validation reporting** and error handling

## Implementation Plan

### 1. Registry Types Enhancement

```rust
// src/registry/types.rs additions

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationExtensions {
    // Range validation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optimal_range: Option<[f64; 2]>,
    
    // Security validation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub privacy_scan: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_filter: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub injection_protection: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_analysis: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path_traversal_protection: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub security_scan: Option<bool>,
    
    // Tool validation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_accessible: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_exists: Option<bool>,
    
    // Size validation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_size_mb: Option<u64>,
    
    // Rule-based validation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rules: Option<Vec<ValidationRule>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRule {
    pub rule: String,
    pub message: String,
}

// Helper function to extract validation from JSON Schema
pub fn extract_validation_extensions(schema: &Value) -> Option<ValidationExtensions> {
    if let Some(validation_obj) = schema.get("validation") {
        serde_json::from_value(validation_obj.clone()).ok()
    } else {
        None
    }
}

// Helper function to inject validation into JSON Schema  
pub fn inject_validation_extensions(schema: &mut Value, validation: &ValidationExtensions) {
    if let Some(obj) = schema.as_object_mut() {
        obj.insert("x-validation".to_string(), serde_json::to_value(validation).unwrap());
    }
}
```

### 2. API Converter Integration

```rust
// src/registry/openapi_generator.rs modifications

impl OpenAPICapabilityGenerator {
    fn generate_input_schema_with_validation(&self, operation: &OpenAPIOperation) -> Result<Value> {
        let mut schema = self.generate_input_schema(operation)?;
        
        // Add common validation extensions for API operations
        if let Some(obj) = schema.as_object_mut() {
            if let Some(properties) = obj.get_mut("properties").and_then(|p| p.as_object_mut()) {
                for (param_name, param_schema) in properties {
                    self.add_parameter_validation(param_name, param_schema);
                }
            }
        }
        
        Ok(schema)
    }
    
    fn add_parameter_validation(&self, param_name: &str, schema: &mut Value) {
        let validation_extensions = match param_name {
            "url" => ValidationExtensions {
                injection_protection: Some(true),
                content_filter: Some(true),
                ..Default::default()
            },
            "api_key" | "token" => ValidationExtensions {
                privacy_scan: Some(true),
                security_scan: Some(true),
                ..Default::default()
            },
            "file_path" | "path" => ValidationExtensions {
                path_traversal_protection: Some(true),
                security_scan: Some(true),
                ..Default::default()
            },
            _ => return, // No validation extensions for other parameters
        };
        
        inject_validation_extensions(schema, &validation_extensions);
    }
}
```

### 3. Validation Processing

```rust
// src/mcp/validation_processor.rs (new file)

pub struct ValidationProcessor {
    config: ValidationConfig,
}

impl ValidationProcessor {
    pub async fn validate_with_extensions(
        &self,
        input: &Value,
        schema: &Value,
    ) -> Result<ValidationResult> {
        // Standard JSON Schema validation first
        let mut result = self.validate_json_schema(input, schema)?;
        
        // Process custom validation extensions
        if let Some(validation) = extract_validation_extensions(schema) {
            result.extend(self.process_validation_extensions(input, &validation).await?);
        }
        
        Ok(result)
    }
    
    async fn process_validation_extensions(
        &self,
        input: &Value,
        validation: &ValidationExtensions,
    ) -> Result<Vec<ValidationError>> {
        let mut errors = Vec::new();
        
        // Range validation
        if let Some(range) = validation.optimal_range {
            if let Some(num) = input.as_f64() {
                if num < range[0] || num > range[1] {
                    errors.push(ValidationError::new(
                        "optimal_range", 
                        format!("Value {} is outside optimal range [{}, {}]", num, range[0], range[1])
                    ));
                }
            }
        }
        
        // Security validation
        if validation.privacy_scan == Some(true) {
            errors.extend(self.scan_privacy(input).await?);
        }
        
        if validation.content_filter == Some(true) {
            errors.extend(self.filter_content(input).await?);
        }
        
        // Tool validation
        if validation.tool_accessible == Some(true) {
            errors.extend(self.check_tool_accessibility(input).await?);
        }
        
        Ok(errors)
    }
}
```

## Migration Strategy

### Phase 1: Backwards Compatibility (Week 1)
1. Add ValidationExtensions types without breaking existing code
2. Update YAML loading to recognize and preserve validation fields
3. Ensure existing tools work unchanged

### Phase 2: API Converter Enhancement (Week 2)
1. Update OpenAPI, GraphQL, and gRPC generators to add validation extensions
2. Add parameter-specific validation based on API semantics
3. Test with generated capability files

### Phase 3: Runtime Integration (Week 3)
1. Implement ValidationProcessor for runtime validation
2. Integrate with tool execution pipeline
3. Add validation error reporting

### Phase 4: Advanced Features (Week 4)
1. LLM-assisted validation for semantic checks
2. Context-aware validation rules
3. Validation performance optimization

## Benefits

1. **Preserves Existing Functionality**: All current YAML files continue to work
2. **Enhances API Converters**: Generated tools get appropriate validation
3. **Type Safety**: Validation extensions are strongly typed
4. **Standards Compliant**: Uses JSON Schema extension mechanisms
5. **Extensible**: Easy to add new validation types
6. **Runtime Validation**: Actual validation enforcement during execution

## Considerations

1. **Performance**: Validation processing adds runtime overhead
2. **Complexity**: More complex data structures and processing logic  
3. **Maintenance**: Need to keep validation types in sync with requirements
4. **Documentation**: Need comprehensive documentation for validation extensions

## Conclusion

The hybrid approach combining typed ValidationExtensions with JSON Schema x-validation properties provides the best balance of functionality, compatibility, and maintainability. This allows MagicTunnel to preserve its advanced validation capabilities while ensuring API converters generate semantically appropriate validation rules.