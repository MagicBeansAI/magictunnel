#[cfg(test)]
mod tests {
    use magictunnel::registry::openapi_generator::{OpenAPICapabilityGenerator, NamingConvention};

    #[tokio::test]
    async fn test_enhanced_yaml_generation() {
        // Test with the OpenAPI example petstore
        let spec_path = "data/petstore_openapi3.json";
        let base_url = "https://petstore.swagger.io/v2";
        
        let generator = OpenAPICapabilityGenerator::new_enhanced(
            base_url.to_string(),
            NamingConvention::SnakeCase,
            None, // auth_config
            None, // tool_prefix  
            None, // operation_filter
        );
        
        let result = generator.generate_from_file(spec_path).await;
        match result {
            Ok(capability_file) => {
                println\!("Generated enhanced capability file successfully\!");
                
                // Print the first tool to verify structure
                if let Some(tool) = capability_file.enhanced_tools.first() {
                    println\!("First enhanced tool: {}", tool.name);
                    println\!("Core description: {}", tool.core.description);
                    println\!("Execution routing type: {}", tool.execution.routing.r#type);
                }
                
                // Verify it has enhanced metadata
                println\!("Enhanced metadata:");
                println\!("  Name: {}", capability_file.enhanced_metadata.name);
                println\!("  Version: {}", capability_file.enhanced_metadata.version);
                
                if let Some(classification) = &capability_file.enhanced_metadata.classification {
                    println\!("  Security level: {}", classification.security_level);
                    println\!("  Domain: {}", classification.domain);
                }
            }
            Err(e) => {
                println\!("Error generating capability file: {}", e);
            }
        }
    }
}
