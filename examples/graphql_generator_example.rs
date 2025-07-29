use magictunnel::registry::graphql_generator::{GraphQLCapabilityGenerator, AuthConfig, AuthType};
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Example 1: Basic GraphQL schema capability generation
    println!("=== Example 1: Basic GraphQL Schema ===");

    let mut generator = GraphQLCapabilityGenerator::new("https://api.example.com/graphql".to_string());
    
    let simple_schema = r#"
        type Query {
            ping: String
            getUser(id: ID!): User
            listUsers(limit: Int): [User!]
        }
        
        type Mutation {
            createUser(name: String!, email: String!): User
            updateUser(id: ID!, name: String): User
        }
        
        type User {
            id: ID!
            name: String!
            email: String!
        }
    "#;
    
    let capability_file = generator.generate_from_sdl(simple_schema)?;
    
    println!("Generated {} tools:", capability_file.tools.len());
    for tool in &capability_file.tools {
        println!("  - {}: {}", tool.name, tool.description);
    }
    
    // Example 2: GraphQL schema with authentication and prefix
    println!("\n=== Example 2: With Authentication and Prefix ===");
    
    let mut auth_headers = HashMap::new();
    auth_headers.insert("X-API-Version".to_string(), "v1".to_string());
    
    let auth_config = AuthConfig {
        auth_type: AuthType::Bearer { token: "your-api-token".to_string() },
        headers: auth_headers,
    };
    
    let mut generator_with_auth = GraphQLCapabilityGenerator::new("https://api.github.com/graphql".to_string())
        .with_auth(auth_config)
        .with_prefix("github".to_string());
    
    let github_schema = r#"
        type Query {
            viewer: User
            repository(owner: String!, name: String!): Repository
        }
        
        type Mutation {
            createIssue(input: CreateIssueInput!): CreateIssuePayload
        }
        
        type User {
            login: String!
            name: String
        }
        
        type Repository {
            name: String!
            description: String
        }
        
        input CreateIssueInput {
            repositoryId: ID!
            title: String!
            body: String
        }
        
        type CreateIssuePayload {
            issue: Issue
        }
        
        type Issue {
            id: ID!
            title: String!
            body: String
        }
    "#;
    
    let github_capability_file = generator_with_auth.generate_from_sdl(github_schema)?;
    
    println!("Generated {} GitHub tools:", github_capability_file.tools.len());
    for tool in &github_capability_file.tools {
        println!("  - {}: {}", tool.name, tool.description);
        
        // Show routing configuration for the first tool
        if tool.name == "github_viewer" {
            let url = tool.routing.config.as_object()
                .and_then(|obj| obj.get("url"))
                .unwrap_or(&serde_json::Value::Null);
            println!("    Routing: {} to {}",
                tool.routing.routing_type(),
                url
            );
        }
    }
    
    // Example 3: Using the real GraphQL schema file
    println!("\n=== Example 3: Real GraphQL Schema File ===");
    
    if let Ok(schema_content) = std::fs::read_to_string("data/GraphQL Schema.graphql") {
        let mut real_generator = GraphQLCapabilityGenerator::new("https://api.real-app.com/graphql".to_string())
            .with_prefix("app".to_string());
        
        let real_capability_file = real_generator.generate_from_sdl(&schema_content)?;
        
        println!("Generated {} tools from real schema", real_capability_file.tools.len());
        println!("First 5 tools:");
        for tool in real_capability_file.tools.iter().take(5) {
            println!("  - {}: {}", tool.name, tool.description);
        }
        
        // Show metadata
        if let Some(metadata) = &real_capability_file.metadata {
            println!("Metadata:");
            if let Some(name) = &metadata.name {
                println!("  Name: {}", name);
            }
            if let Some(description) = &metadata.description {
                println!("  Description: {}", description);
            }
            if let Some(version) = &metadata.version {
                println!("  Version: {}", version);
            }
        }
    } else {
        println!("Real GraphQL schema file not found at 'data/GraphQL Schema.graphql'");
    }
    
    // Example 4: Generating YAML output
    println!("\n=== Example 4: YAML Output ===");
    
    let mut yaml_generator = GraphQLCapabilityGenerator::new("https://api.example.com/graphql".to_string())
        .with_prefix("api".to_string());
    
    let simple_capability = yaml_generator.generate_from_sdl(simple_schema)?;
    
    // Convert to YAML (would require serde_yaml dependency)
    println!("Capability file structure:");
    println!("  Tools: {}", simple_capability.tools.len());
    println!("  Metadata: {:?}", simple_capability.metadata.is_some());
    
    // Show a sample tool's JSON schema
    if let Some(first_tool) = simple_capability.tools.first() {
        println!("  Sample tool '{}' input schema:", first_tool.name);
        println!("    {}", serde_json::to_string_pretty(&first_tool.input_schema)?);
    }
    
    Ok(())
}
