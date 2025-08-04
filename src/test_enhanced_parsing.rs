//! Test enhanced format parsing
use crate::registry::types::EnhancedCapabilityFileRaw;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_enhanced_format_deserialization() {
        let yaml_content = r#"
metadata:
  name: Test Enhanced
  description: Simple test enhanced file
  version: 1.0.0
  author: Test Author
  classification:
    security_level: safe
    complexity_level: simple
    domain: test
    use_cases:
    - test
  mcp_capabilities:
    version: '2025-06-18'
    supports_cancellation: true
    supports_progress: true
    supports_sampling: false
    supports_validation: true
    supports_elicitation: false

tools:
- name: test_tool
  core:
    description: Test tool description
    input_schema:
      type: object
      properties:
        test_param:
          type: string
  execution:
    routing:
      type: subprocess
      primary:
        command: echo
        args: ["test"]
"#;

        // Test deserialization
        let result: Result<EnhancedCapabilityFileRaw, _> = serde_yaml::from_str(yaml_content);
        
        match result {
            Ok(parsed) => {
                println!("✅ Enhanced format parsed successfully!");
                println!("Metadata: {:?}", parsed.metadata);
                println!("Tools count: {}", parsed.tools.len());
            }
            Err(e) => {
                println!("❌ Enhanced format parsing failed: {}", e);
                // Let's see exactly what field is missing
                panic!("Enhanced format deserialization failed: {}", e);
            }
        }
    }
}