//! Tests for Swagger 2.0 support in OpenAPI Capability Generator

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::registry::types::RoutingConfig;
    use serde_json::json;

    #[test]
    fn test_swagger2_format_detection() {
        let mut generator = OpenAPICapabilityGenerator::new("https://api.example.com".to_string());
        
        let swagger2_spec = r#"
        {
            "swagger": "2.0",
            "info": {
                "title": "Test API",
                "version": "1.0.0"
            },
            "paths": {
                "/test": {
                    "get": {
                        "operationId": "testGet",
                        "responses": {
                            "200": {
                                "description": "Success"
                            }
                        }
                    }
                }
            }
        }
        "#;
        
        // Should not error when detecting Swagger 2.0 format
        let result = generator.generate_from_spec(swagger2_spec);
        if let Err(ref e) = result {
            println!("Format detection error: {:?}", e);
        }
        assert!(result.is_ok());
    }

    #[test]
    fn test_swagger2_basic_parsing() {
        let mut generator = OpenAPICapabilityGenerator::new("https://api.example.com".to_string());
        
        let swagger2_spec = r#"
        {
            "swagger": "2.0",
            "info": {
                "title": "Test API",
                "description": "A test API",
                "version": "1.0.0"
            },
            "host": "api.example.com",
            "basePath": "/v1",
            "schemes": ["https"],
            "paths": {
                "/pets": {
                    "get": {
                        "summary": "List pets",
                        "operationId": "listPets",
                        "responses": {
                            "200": {
                                "description": "A list of pets"
                            }
                        }
                    }
                }
            }
        }
        "#;
        
        let result = generator.generate_from_swagger2(swagger2_spec);
        assert!(result.is_ok());
        
        let capability_file = result.unwrap();
        let empty_tools = vec![];
        let tools = capability_file.get_enhanced_tools().unwrap_or(&empty_tools);
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "enhanced_listpets");
        assert!(tools[0].core.description.contains("List pets"));
    }

    #[test]
    fn test_swagger2_parameter_conversion() {
        let mut generator = OpenAPICapabilityGenerator::new("https://api.example.com".to_string());
        
        let swagger2_spec = r#"
        {
            "swagger": "2.0",
            "info": {
                "title": "Test API",
                "version": "1.0.0"
            },
            "paths": {
                "/pets/{petId}": {
                    "get": {
                        "summary": "Get pet by ID",
                        "operationId": "getPetById",
                        "parameters": [
                            {
                                "name": "petId",
                                "in": "path",
                                "description": "Pet ID",
                                "required": true,
                                "type": "integer",
                                "format": "int64"
                            },
                            {
                                "name": "limit",
                                "in": "query",
                                "description": "Limit results",
                                "required": false,
                                "type": "integer",
                                "default": 10
                            }
                        ],
                        "responses": {
                            "200": {
                                "description": "Pet details"
                            }
                        }
                    }
                }
            }
        }
        "#;
        
        let result = generator.generate_from_swagger2(swagger2_spec);
        assert!(result.is_ok());
        
        let capability_file = result.unwrap();
        let empty_tools = vec![];
        let tools = capability_file.get_enhanced_tools().unwrap_or(&empty_tools);
        assert_eq!(tools.len(), 1);
        
        let tool = &tools[0];
        assert_eq!(tool.name, "enhanced_getpetbyid");
        
        // Check that parameters are properly converted
        let input_schema = &tool.core.input_schema;
        let properties = input_schema.get("properties").unwrap().as_object().unwrap();
        
        // Should have petId parameter
        assert!(properties.contains_key("petId"));
        let pet_id_schema = properties.get("petId").unwrap();
        assert_eq!(pet_id_schema.get("type").unwrap(), "integer");
        assert_eq!(pet_id_schema.get("format").unwrap(), "int64");
        
        // Should have limit parameter
        assert!(properties.contains_key("limit"));
        let limit_schema = properties.get("limit").unwrap();
        assert_eq!(limit_schema.get("type").unwrap(), "integer");
        assert_eq!(limit_schema.get("default").unwrap(), 10);
        
        // Check required fields
        let required = input_schema.get("required").unwrap().as_array().unwrap();
        assert!(required.contains(&json!("petId")));
        assert!(!required.contains(&json!("limit")));
    }

    #[test]
    fn test_swagger2_request_body_conversion() {
        let mut generator = OpenAPICapabilityGenerator::new("https://api.example.com".to_string());
        
        let swagger2_spec = r#"
        {
            "swagger": "2.0",
            "info": {
                "title": "Test API",
                "version": "1.0.0"
            },
            "paths": {
                "/pets": {
                    "post": {
                        "summary": "Create pet",
                        "operationId": "createPet",
                        "parameters": [
                            {
                                "name": "pet",
                                "in": "body",
                                "description": "Pet to create",
                                "required": true,
                                "schema": {
                                    "type": "object",
                                    "properties": {
                                        "name": {
                                            "type": "string"
                                        },
                                        "age": {
                                            "type": "integer"
                                        }
                                    }
                                }
                            }
                        ],
                        "responses": {
                            "201": {
                                "description": "Pet created"
                            }
                        }
                    }
                }
            }
        }
        "#;
        
        let result = generator.generate_from_swagger2(swagger2_spec);
        assert!(result.is_ok());
        
        let capability_file = result.unwrap();
        let empty_tools = vec![];
        let tools = capability_file.get_enhanced_tools().unwrap_or(&empty_tools);
        assert_eq!(tools.len(), 1);
        
        let tool = &tools[0];
        assert_eq!(tool.name, "enhanced_createpet");
        
        // Check routing configuration for body parameter
        assert!(tool.execution.routing.r#type == "http" || tool.execution.routing.r#type == "enhanced_http");
        // Enhanced routing stores actual routing config in the 'primary' field
        let empty_config = serde_json::Value::Object(serde_json::Map::new());
        let config = tool.execution.routing.primary.as_ref().unwrap_or(&empty_config).as_object().unwrap();
        assert_eq!(config.get("method").unwrap().as_str().unwrap(), "POST");
        assert!(config.get("body_param").is_some());
        assert_eq!(config.get("body_param").unwrap().as_str().unwrap(), "body");
    }

    #[test]
    fn test_swagger2_response_conversion() {
        let mut generator = OpenAPICapabilityGenerator::new("https://api.example.com".to_string());
        
        let swagger2_spec = r#"
        {
            "swagger": "2.0",
            "info": {
                "title": "Test API",
                "version": "1.0.0"
            },
            "paths": {
                "/pets": {
                    "get": {
                        "summary": "List pets",
                        "operationId": "listPets",
                        "responses": {
                            "200": {
                                "description": "A list of pets",
                                "schema": {
                                    "type": "array",
                                    "items": {
                                        "type": "object",
                                        "properties": {
                                            "id": {
                                                "type": "integer"
                                            },
                                            "name": {
                                                "type": "string"
                                            }
                                        }
                                    }
                                }
                            },
                            "400": {
                                "description": "Bad request"
                            }
                        }
                    }
                }
            }
        }
        "#;
        
        let result = generator.generate_from_swagger2(swagger2_spec);
        assert!(result.is_ok());
        
        let capability_file = result.unwrap();
        let empty_tools = vec![];
        let tools = capability_file.get_enhanced_tools().unwrap_or(&empty_tools);
        assert_eq!(tools.len(), 1);
        
        let tool = &tools[0];
        assert_eq!(tool.name, "enhanced_listpets");
        
        // The response conversion is internal to the generator
        // We mainly test that it doesn't error and produces a valid tool
        assert!(!tool.core.description.is_empty());
    }

    #[test]
    fn test_swagger2_extensions_support() {
        let mut generator = OpenAPICapabilityGenerator::new("https://api.example.com".to_string());
        
        let swagger2_spec = r#"
        {
            "swagger": "2.0",
            "info": {
                "title": "Test API",
                "version": "1.0.0"
            },
            "paths": {
                "/pets": {
                    "get": {
                        "summary": "List pets",
                        "operationId": "listPets",
                        "responses": {
                            "200": {
                                "description": "A list of pets",
                                "schema": {
                                    "type": "object",
                                    "x-custom-extension": "custom-value",
                                    "x-another-extension": {
                                        "nested": "value"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        "#;
        
        let result = generator.generate_from_swagger2(swagger2_spec);
        assert!(result.is_ok());
        
        let capability_file = result.unwrap();
        let empty_tools = vec![];
        let tools = capability_file.get_enhanced_tools().unwrap_or(&empty_tools);
        assert_eq!(tools.len(), 1);
        
        // Extensions should be handled without errors
        let tool = &tools[0];
        assert_eq!(tool.name, "enhanced_listpets");
    }

    #[test]
    fn test_swagger2_deprecated_operations() {
        let mut generator = OpenAPICapabilityGenerator::new("https://api.example.com".to_string());
        
        let swagger2_spec = r#"
        {
            "swagger": "2.0",
            "info": {
                "title": "Test API",
                "version": "1.0.0"
            },
            "paths": {
                "/pets": {
                    "get": {
                        "summary": "List pets",
                        "operationId": "listPets",
                        "deprecated": true,
                        "responses": {
                            "200": {
                                "description": "A list of pets"
                            }
                        }
                    },
                    "post": {
                        "summary": "Create pet",
                        "operationId": "createPet",
                        "responses": {
                            "201": {
                                "description": "Pet created"
                            }
                        }
                    }
                }
            }
        }
        "#;
        
        // Test with deprecated operations excluded (default)
        let result = generator.generate_from_swagger2(swagger2_spec);
        assert!(result.is_ok());
        
        let capability_file = result.unwrap();
        let empty_tools = vec![];
        let tools = capability_file.get_enhanced_tools().unwrap_or(&empty_tools);
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "enhanced_createpet");
        
        // Test with deprecated operations included
        let mut generator_with_deprecated = OpenAPICapabilityGenerator::new("https://api.example.com".to_string())
            .include_deprecated();
        
        let result_with_deprecated = generator_with_deprecated.generate_from_swagger2(swagger2_spec);
        assert!(result_with_deprecated.is_ok());
        
        let capability_file_with_deprecated = result_with_deprecated.unwrap();
        let empty_tools_deprecated = vec![];
        let tools_with_deprecated = capability_file_with_deprecated.get_enhanced_tools().unwrap_or(&empty_tools_deprecated);
        assert_eq!(tools_with_deprecated.len(), 2);
        
        let tool_names: Vec<&str> = tools_with_deprecated.iter()
            .map(|t| t.name.as_str())
            .collect();
        assert!(tool_names.contains(&"enhanced_listpets"));
        assert!(tool_names.contains(&"enhanced_createpet"));
    }
}
