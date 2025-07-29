//! Parameter substitution system for routing configurations

use crate::error::{ProxyError, Result};
use serde_json::Value;
use std::collections::HashMap;
use tracing::debug;

/// Substitute parameters in a vector of strings
pub fn substitute_parameters(args: &[String], parameters: &Value) -> Result<Vec<String>> {
    args.iter()
        .map(|arg| substitute_parameter_string(arg, parameters))
        .collect()
}

/// Substitute parameters in a single string
pub fn substitute_parameter_string(template: &str, parameters: &Value) -> Result<String> {
    let mut result = template.to_string();

    // Handle both {{parameter}} and {parameter} syntax for flexibility
    if let Some(obj) = parameters.as_object() {
        for (key, value) in obj {
            // Double brace syntax: {{parameter}}
            let double_placeholder = format!("{{{{{}}}}}", key);
            // Single brace syntax: {parameter}
            let single_placeholder = format!("{{{}}}", key);
            
            let replacement = value_to_string(value)?;
            
            result = result.replace(&double_placeholder, &replacement);
            result = result.replace(&single_placeholder, &replacement);
            
            // Handle array indexing: {parameter[0]}, {parameter[1]}, etc.
            if let Some(array) = value.as_array() {
                for (index, item) in array.iter().enumerate() {
                    let array_double_placeholder = format!("{{{{{}[{}]}}}}", key, index);
                    let array_single_placeholder = format!("{{{}[{}]}}", key, index);
                    
                    let array_replacement = value_to_string(item)?;
                    
                    result = result.replace(&array_double_placeholder, &array_replacement);
                    result = result.replace(&array_single_placeholder, &array_replacement);
                }
            }
        }
    }

    debug!("Parameter substitution: '{}' -> '{}'", template, result);
    Ok(result)
}

/// Substitute parameters in a URL with query parameters
pub fn substitute_url_parameters(url: &str, parameters: &Value) -> Result<String> {
    substitute_parameter_string(url, parameters)
}

/// Substitute parameters in a JSON value recursively
pub fn substitute_json_value(value: &Value, parameters: &Value) -> Result<Value> {
    match value {
        Value::String(s) => {
            // Check if the string is a pure placeholder (e.g., "{{tags}}")
            if let Some(pure_value) = extract_pure_placeholder(s, parameters)? {
                Ok(pure_value)
            } else {
                let substituted = substitute_parameter_string(s, parameters)?;
                Ok(Value::String(substituted))
            }
        }
        Value::Object(obj) => {
            let mut new_obj = serde_json::Map::new();
            for (key, val) in obj {
                let substituted_key = substitute_parameter_string(key, parameters)?;
                let substituted_val = substitute_json_value(val, parameters)?;
                new_obj.insert(substituted_key, substituted_val);
            }
            Ok(Value::Object(new_obj))
        }
        Value::Array(arr) => {
            let mut new_arr = Vec::new();
            for item in arr {
                let substituted_item = substitute_json_value(item, parameters)?;
                new_arr.push(substituted_item);
            }
            Ok(Value::Array(new_arr))
        }
        // For other types (Number, Bool, Null), return as-is
        _ => Ok(value.clone()),
    }
}

/// Substitute parameters in HTTP headers
pub fn substitute_headers(
    headers: &Option<HashMap<String, String>>,
    parameters: &Value
) -> Result<Option<HashMap<String, String>>> {
    match headers {
        Some(header_map) => {
            let mut substituted_headers = HashMap::new();
            
            for (key, value) in header_map {
                let substituted_key = substitute_parameter_string(key, parameters)?;
                let substituted_value = substitute_parameter_string(value, parameters)?;
                substituted_headers.insert(substituted_key, substituted_value);
            }
            
            Ok(Some(substituted_headers))
        }
        None => Ok(None),
    }
}

/// Substitute parameters in environment variables
pub fn substitute_env_vars(
    env: &Option<HashMap<String, String>>,
    parameters: &Value
) -> Result<Option<HashMap<String, String>>> {
    match env {
        Some(env_map) => {
            let mut substituted_env = HashMap::new();
            
            for (key, value) in env_map {
                let substituted_key = substitute_parameter_string(key, parameters)?;
                let substituted_value = substitute_parameter_string(value, parameters)?;
                substituted_env.insert(substituted_key, substituted_value);
            }
            
            Ok(Some(substituted_env))
        }
        None => Ok(None),
    }
}

/// Extract a pure placeholder value if the string contains only one placeholder
fn extract_pure_placeholder(template: &str, parameters: &Value) -> Result<Option<Value>> {
    // Check if the template is exactly "{{key}}" or "{key}"
    if let Some(obj) = parameters.as_object() {
        for (key, value) in obj {
            let double_placeholder = format!("{{{{{}}}}}", key);
            let single_placeholder = format!("{{{}}}", key);

            if template == double_placeholder || template == single_placeholder {
                return Ok(Some(value.clone()));
            }
        }
    }
    Ok(None)
}

/// Convert a JSON value to a string for parameter substitution
fn value_to_string(value: &Value) -> Result<String> {
    match value {
        Value::String(s) => Ok(s.clone()),
        Value::Number(n) => Ok(n.to_string()),
        Value::Bool(b) => Ok(b.to_string()),
        Value::Null => Ok("null".to_string()),
        Value::Array(_) | Value::Object(_) => {
            // For complex types, serialize to JSON string
            serde_json::to_string(value)
                .map_err(|e| ProxyError::validation(format!("Failed to serialize parameter value: {}", e)))
        }
    }
}

/// Advanced parameter substitution with type conversion and validation
pub struct ParameterSubstitution {
    /// Whether to allow missing parameters (default: false)
    pub allow_missing: bool,
    /// Default values for parameters
    pub defaults: HashMap<String, String>,
}

impl ParameterSubstitution {
    /// Create a new parameter substitution instance
    pub fn new() -> Self {
        Self {
            allow_missing: false,
            defaults: HashMap::new(),
        }
    }

    /// Create with default values
    pub fn with_defaults(defaults: HashMap<String, String>) -> Self {
        Self {
            allow_missing: true,
            defaults,
        }
    }

    /// Substitute parameters with advanced options
    pub fn substitute_advanced(&self, template: &str, parameters: &Value) -> Result<String> {
        let mut result = template.to_string();
        let mut missing_params = Vec::new();

        // Find all parameter placeholders
        let placeholders = self.find_placeholders(template);
        
        for placeholder in placeholders {
            let param_name = &placeholder.name;
            let full_placeholder = &placeholder.full_match;
            
            // Try to get value from parameters
            let replacement = if let Some(obj) = parameters.as_object() {
                if let Some(value) = obj.get(param_name) {
                    value_to_string(value)?
                } else if let Some(default) = self.defaults.get(param_name) {
                    default.clone()
                } else if self.allow_missing {
                    continue; // Leave placeholder as-is
                } else {
                    missing_params.push(param_name.clone());
                    continue;
                }
            } else if let Some(default) = self.defaults.get(param_name) {
                default.clone()
            } else if self.allow_missing {
                continue;
            } else {
                missing_params.push(param_name.clone());
                continue;
            };

            result = result.replace(full_placeholder, &replacement);
        }

        if !missing_params.is_empty() {
            return Err(ProxyError::validation(format!(
                "Missing required parameters: {}",
                missing_params.join(", ")
            )));
        }

        Ok(result)
    }

    /// Find all parameter placeholders in a template
    fn find_placeholders(&self, template: &str) -> Vec<Placeholder> {
        let mut placeholders = Vec::new();

        // Find {{param}} patterns
        if let Ok(double_brace_regex) = regex::Regex::new(r"\{\{([^}]+)\}\}") {
            for cap in double_brace_regex.captures_iter(template) {
                placeholders.push(Placeholder {
                    name: cap[1].to_string(),
                    full_match: cap[0].to_string(),
                });
            }
        }

        // Find {param} patterns (but not {{param}} which we already found)
        if let Ok(single_brace_regex) = regex::Regex::new(r"(?<!\{)\{([^{}]+)\}(?!\})") {
            for cap in single_brace_regex.captures_iter(template) {
                placeholders.push(Placeholder {
                    name: cap[1].to_string(),
                    full_match: cap[0].to_string(),
                });
            }
        }

        placeholders
    }
}

impl Default for ParameterSubstitution {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents a parameter placeholder found in a template
#[derive(Debug, Clone)]
struct Placeholder {
    /// The parameter name (without braces)
    name: String,
    /// The full placeholder text (with braces)
    full_match: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_basic_parameter_substitution() {
        let template = "echo {message}";
        let params = json!({"message": "hello world"});
        
        let result = substitute_parameter_string(template, &params).unwrap();
        assert_eq!(result, "echo hello world");
    }

    #[test]
    fn test_double_brace_substitution() {
        let template = "echo {{message}}";
        let params = json!({"message": "hello world"});
        
        let result = substitute_parameter_string(template, &params).unwrap();
        assert_eq!(result, "echo hello world");
    }

    #[test]
    fn test_multiple_parameters() {
        let template = "curl -X {method} {url} -H 'Content-Type: {content_type}'";
        let params = json!({
            "method": "POST",
            "url": "https://api.example.com",
            "content_type": "application/json"
        });
        
        let result = substitute_parameter_string(template, &params).unwrap();
        assert_eq!(result, "curl -X POST https://api.example.com -H 'Content-Type: application/json'");
    }

    #[test]
    fn test_parameter_vector_substitution() {
        let args = vec!["-c".to_string(), "{command}".to_string()];
        let params = json!({"command": "ls -la"});
        
        let result = substitute_parameters(&args, &params).unwrap();
        assert_eq!(result, vec!["-c", "ls -la"]);
    }

    #[test]
    fn test_number_parameter_substitution() {
        let template = "timeout {seconds}s";
        let params = json!({"seconds": 30});
        
        let result = substitute_parameter_string(template, &params).unwrap();
        assert_eq!(result, "timeout 30s");
    }

    #[test]
    fn test_boolean_parameter_substitution() {
        let template = "verbose={verbose}";
        let params = json!({"verbose": true});
        
        let result = substitute_parameter_string(template, &params).unwrap();
        assert_eq!(result, "verbose=true");
    }
}
