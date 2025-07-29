# OpenAPI Capability Generator

The OpenAPI Capability Generator is a powerful tool that automatically generates MCP (Model Context Protocol) capability files from OpenAPI 3.0 and Swagger 2.0 specifications. It provides comprehensive schema parsing, reference resolution, and inheritance pattern support for creating production-ready MCP tools.

## Features

### Core Capabilities

- **Multi-Format Support**: Supports both OpenAPI 3.0 and Swagger 2.0 specifications
- **Format Auto-Detection**: Automatically detects JSON and YAML formats
- **Complete HTTP Method Support**: All standard HTTP methods (GET, POST, PUT, PATCH, DELETE, HEAD, OPTIONS, TRACE)
- **Advanced Schema Processing**: Complex nested objects, inheritance patterns, and validation
- **Component & Reference Resolution**: Full $ref resolution for OpenAPI components and definitions
- **Comprehensive Authentication**: API Key, Bearer token, Basic auth, and OAuth 2.0 support

### Advanced Schema Features

- **Complex Type Support**: Objects, arrays, primitives with full type validation
- **Inheritance Patterns**: AllOf, OneOf, AnyOf schema composition support
- **Reference Resolution**: Complete $ref resolution for components, definitions, and external references
- **Schema Validation**: Automatic schema validation and enhancement with metadata
- **Nested Object Support**: Recursive resolution of complex nested structures
- **Format Constraints**: Support for format specifications, min/max values, and enumerations

### Configuration Options

- **Flexible Filtering**: Filter by HTTP methods, operation IDs, paths, or tags
- **Naming Conventions**: Multiple naming strategies (operation-id, method-path, custom)
- **Tool Prefixes**: Configurable prefixes for tool organization
- **Authentication Integration**: Seamless integration with various auth schemes
- **Deprecation Handling**: Optional inclusion/exclusion of deprecated operations

## Usage

### Command Line Interface

```bash
# Basic usage
mcp-generator openapi --spec openapi.json --base-url https://api.example.com --output capabilities.yaml

# With authentication
mcp-generator openapi \
  --spec api.yaml \
  --base-url https://api.example.com \
  --output api-tools.yaml \
  --auth-type bearer \
  --auth-token "your-token-here" \
  --prefix api

# With filtering and custom naming (using Swagger 2.0 example)
mcp-generator openapi \
  --spec examples/swagger2_petstore.json \
  --base-url https://api.example.com/v1 \
  --output petstore.yaml \
  --methods GET,POST,PUT,DELETE \
  --naming operation-id \
  --include-deprecated false
```

### Configuration File

```toml
[openapi]
base_url = "https://api.example.com"
tool_prefix = "api"
naming_convention = "operation-id"
methods = ["GET", "POST", "PUT", "DELETE"]
include_deprecated = false

[openapi.auth]
auth_type = "bearer"
token = "your-token-here"
headers = { "X-API-Version" = "v1" }

[openapi.filtering]
operation_filter = ["getUserById", "createUser"]
path_filter = ["/users/*", "/posts/*"]
tag_filter = ["users", "posts"]
```

## Schema Processing

### Complex Object Handling

The generator handles complex nested objects with recursive schema resolution:

```yaml
# Generated from complex OpenAPI schema
tools:
  - name: createUser
    description: Create a new user
    input_schema:
      type: object
      properties:
        user:
          type: object
          properties:
            profile:
              type: object
              properties:
                address:
                  type: object
                  properties:
                    street: { type: string }
                    city: { type: string }
                    country: { type: string }
              required: [address]
          required: [profile]
      required: [user]
```

### Reference Resolution

Supports all OpenAPI reference types:

- **Component References**: `#/components/schemas/User`
- **Definition References**: `#/definitions/User` (Swagger 2.0)
- **Parameter References**: `#/components/parameters/UserId`
- **Response References**: `#/components/responses/UserResponse`
- **External References**: `https://api.example.com/schemas/user.json`

### Inheritance Patterns

Handles complex inheritance using AllOf schemas:

```json
{
  "allOf": [
    { "$ref": "#/components/schemas/BaseUser" },
    {
      "type": "object",
      "properties": {
        "adminLevel": { "type": "integer" }
      }
    }
  ]
}
```

Generates merged schemas with all inherited properties and constraints.

## Authentication Integration

### Supported Authentication Types

1. **API Key Authentication**
   ```bash
   --auth-type apikey --auth-token "your-api-key" --auth-header "X-API-Key"
   ```

2. **Bearer Token Authentication**
   ```bash
   --auth-type bearer --auth-token "your-bearer-token"
   ```

3. **Basic Authentication**
   ```bash
   --auth-type basic --auth-username "user" --auth-password "pass"
   ```

4. **OAuth 2.0**
   ```bash
   --auth-type oauth --auth-token "oauth-token"
   ```

### Custom Headers

Add custom headers for API requirements:

```toml
[openapi.auth]
auth_type = "bearer"
token = "your-token"
headers = {
  "X-API-Version" = "v2",
  "X-Client-ID" = "magictunnel"
}
```

## Advanced Features

### Schema Validation and Enhancement

The generator automatically:
- Validates schema structure and fixes common issues
- Adds MCP-specific metadata for better tool integration
- Ensures required fields have proper defaults
- Enhances schemas with validation constraints

### Error Handling

Robust error handling for:
- Invalid OpenAPI specifications
- Missing or broken references
- Unsupported schema features
- Network errors for external references

### Performance Optimization

- Efficient recursive schema resolution
- Caching of resolved references
- Optimized for large specifications
- Memory-efficient processing

## Examples

### Example Files

The project includes example specification files for testing and demonstration:

- **`examples/swagger2_petstore.json`** - Swagger 2.0 example with pets API
- **`data/petstore_openapi3.json`** - OpenAPI 3.0 example for comparison

```bash
# Using Swagger 2.0 example
mcp-generator openapi --spec examples/swagger2_petstore.json --base-url https://api.example.com/v1 --output swagger2_tools.yaml

# Using OpenAPI 3.0 example
mcp-generator openapi --spec data/petstore_openapi3.json --base-url https://petstore.swagger.io/v2 --output openapi3_tools.yaml
```

### Real-World API Integration

```bash
# Stripe API
mcp-generator openapi \
  --spec https://raw.githubusercontent.com/stripe/openapi/master/openapi/spec3.json \
  --base-url https://api.stripe.com \
  --output stripe-capabilities.yaml \
  --auth-type bearer \
  --prefix stripe

# GitHub API
mcp-generator openapi \
  --spec github-api.yaml \
  --base-url https://api.github.com \
  --output github-capabilities.yaml \
  --auth-type bearer \
  --methods GET,POST,PATCH,DELETE \
  --naming operation-id
```

### Complex Schema Example

For an OpenAPI specification with complex nested schemas, inheritance, and references, the generator produces comprehensive MCP tools with full type validation and proper parameter mapping.

## Testing and Validation

The OpenAPI generator includes comprehensive test coverage:

- **13 test cases** covering all major features
- **Real-world validation** with Petstore and complex APIs
- **Swagger 2.0 compatibility** testing
- **Reference resolution** validation
- **Schema inheritance** testing
- **Authentication integration** testing

## Troubleshooting

### Common Issues

1. **Invalid Specification**: Ensure your OpenAPI/Swagger specification is valid
2. **Reference Resolution Errors**: Check that all $ref paths are correct
3. **Authentication Failures**: Verify tokens and credentials
4. **Schema Parsing Issues**: Validate complex schemas for correctness

### Debug Mode

Enable verbose logging for troubleshooting:

```bash
mcp-generator openapi --spec api.yaml --base-url https://api.example.com --output tools.yaml --verbose
```

## Integration with MCP Proxy

Generated capability files integrate seamlessly with the MCP Proxy:

1. Generate capabilities from your OpenAPI specification
2. Place the generated YAML file in your capabilities directory
3. Configure the MCP Proxy to load the capabilities
4. Access your API through MCP-compatible clients

The generator ensures full MCP protocol compliance and optimal tool organization for efficient API access.
