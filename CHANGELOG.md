# Changelog

All notable changes to the MagicTunnel project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.2] - 2025-08-03

### Added
- **🎨 LLM Backend Management APIs Complete**: Comprehensive REST API implementation for all LLM services
  - **Resource Management APIs**: 7 complete endpoints for resource browsing, reading, validation, and statistics
  - **Enhancement Pipeline APIs**: 9 complete endpoints for tool enhancement management, job tracking, and cache control
  - **Prompt Management APIs**: Complete backend implementation (previously completed)
  - **Sampling Service APIs**: Full management interface for AI-powered tool enhancement
  - **Elicitation Service APIs**: Complete metadata extraction and validation management
  - **Provider Management APIs**: LLM provider configuration and health monitoring
- **📊 Statistics and Analytics**: Comprehensive analytics for resource types, provider health, and enhancement metrics
- **🔄 Batch Processing Support**: Enhanced batch operations for tool enhancement and resource management
- **🛡️ Enhanced Error Handling**: Robust error handling patterns across all LLM management APIs

### Changed
- **📈 API Architecture**: All LLM management functionality now available through REST APIs for UI integration
- **🎯 Service Integration**: Enhanced integration between enhancement pipeline, resource management, and tool discovery

### Fixed
- **⚙️ Compilation Issues**: Resolved all compilation errors in Enhancement Pipeline APIs implementation
- **📝 Type Definitions**: Added missing struct definitions and fixed field access patterns

## [0.3.1] - 2025-08-03

### Added
- **🛠️ Development Tools Organization**: Comprehensive reorganization of development and operational tools
  - **tools/validation/**: YAML validation and MCP compliance tools with automated testing integration
  - **tools/migration/**: Format migration utilities for MCP 2025-06-18 compatibility
  - **tools/integrations/**: External service integration tools (Google Sheets, OAuth2)
  - **tools/testing/**: Semantic search and API testing utilities for development workflows
  - **tools/release/**: Version management and release automation tools
- **📚 Comprehensive Tool Documentation**: Complete README files for each tool category with usage examples, integration patterns, and troubleshooting guides
- **🔄 Development Workflow Integration Strategy**: Detailed plan for integrating tools into existing workflows (cargo test, CI/CD, git hooks, make targets)

### Changed
- **📋 TODO Documentation Consolidation**: 
  - Split massive TODO.md (3,381 lines) into manageable TODO.md (233 lines) and TODO_DONE.md (386 lines)
  - TODO.md now focuses on current and future tasks with clear priorities
  - TODO_DONE.md preserves complete implementation history and achievements
- **📖 Enhanced Project Documentation**: Added development tools references to main README.md and docs/README.md for better discoverability

### Improved
- **🎯 Script Discoverability**: All 12 Python and bash scripts now properly organized with clear categorization and usage documentation
- **🚀 Development Experience**: Clear path forward for transforming standalone scripts into integrated development workflow tools
- **📚 Documentation Structure**: Better organization of documentation with cross-references and navigation between current tasks and completed work

## [0.3.0]

### Added

#### **🚀 MCP 2025-06-18 Specification Compliance Complete**
- **Full MCP 2025-06-18 Implementation**: Complete implementation of latest MCP spec with sampling and elicitation
- **OAuth 2.1 Framework**: Upgraded authentication with PKCE and Resource Indicators (RFC 8707)
- **Dual Transport Support**: HTTP+SSE (deprecated) and Streamable HTTP (preferred) with graceful migration
- **Enhanced Security Model**: MCP-specific consent flows, capability permissions, and tool approval workflows
- **Streamable HTTP Transport**: NDJSON streaming, enhanced batching, and session management
- **Backward Compatibility**: Maintained HTTP+SSE support at `/mcp/stream` with deprecation guidance

#### **🤖 Automatic LLM Generation Workflow Complete**
- **Sampling Service**: AI-powered tool description enhancement with OpenAI/Anthropic/Ollama support
- **Elicitation Service**: Automatic metadata extraction and parameter validation using structured LLM analysis
- **Enhancement Pipeline**: Coordinated sampling + elicitation with parallel processing and error handling
- **LLM Management CLI**: Unified `magictunnel-llm` tool for all LLM service management with external MCP protection
- **External MCP Protection**: Automatic detection and protection of external MCP server content with warnings
- **Performance Optimization**: Multi-level caching, rate limiting, and asynchronous processing for enterprise scale

#### **🔒 Enterprise Security Features Complete**
- **Allowlisting System**: Tool and domain allowlisting with granular permissions and policy enforcement
- **Security Policy Engine**: Visual policy builder with condition-based rules and enterprise validation
- **MCP Security Compliance**: Enhanced consent flows and capability validation per MCP 2025-06-18
- **Enterprise Authentication**: OAuth 2.1 with PKCE and Resource Indicators for secure enterprise deployment
- **Audit Logging**: Comprehensive security event tracking and monitoring with OpenTelemetry integration

#### **🎨 Modern UI and Frontend Complete**
- **LLM Services Dashboard**: Complete frontend for sampling, elicitation, and enhancement management
- **Accessibility Improvements**: WCAG 2.1 AA compliance across all components with comprehensive accessibility fixes
- **Responsive Design**: Mobile-first approach with enhanced user experience and real-time updates
- **Real-time Monitoring**: Live service status and health indicators with automatic refresh
- **Interactive Tool Management**: Dynamic forms and real-time enhancement execution with progress tracking

#### **⚙️ Enhanced Configuration System**
- **YAML Format Evolution**: Enhanced capability file format with metadata support and versioning
- **Service Configuration**: Comprehensive LLM provider and enhancement settings with validation
- **Security Configuration**: Granular security policy and allowlisting configuration management
- **Performance Tuning**: Caching, batching, and optimization settings for enterprise deployments
- **Environment Management**: Advanced environment variable and deployment configuration support

#### **🛠️ Developer and Operations Tools**
- **Advanced CLI Tools**: Complete suite including `magictunnel-llm` and `magictunnel-security` for enterprise management
- **OpenAPI 3.1 Integration**: Complete Custom GPT support and API generation for seamless integrations
- **Enhanced Documentation**: Comprehensive documentation including automatic LLM generation workflow guide
- **Monitoring Integration**: OpenTelemetry, Prometheus metrics, and health checks for production monitoring

### Enhanced - Architecture & Platform Improvements
- **Enterprise-Grade Architecture**: Scalable platform supporting complex enterprise deployments
- **Performance Optimization**: Sub-second response times with multi-level caching and distributed support
- **Migration Support**: Backward-compatible upgrades with comprehensive migration documentation
- **Production Ready**: Complete enterprise platform ready for production deployment

### Documentation - Comprehensive Platform Documentation
- **Automatic LLM Generation Workflow**: Complete documentation with architecture diagrams and implementation details
- **Enterprise Security Guide**: Comprehensive security documentation with policy management and compliance
- **Accessibility Implementation**: Detailed accessibility fixes and WCAG 2.1 AA compliance documentation
- **Platform Migration Guide**: Step-by-step migration guide for existing installations

### Technical - Major Version Breaking Changes
- **Version Bump**: Major version change from 0.2.x to 0.3.0 reflecting comprehensive platform maturity
- **Enterprise Focus**: Shift from development platform to enterprise-ready solution
- **Complete Platform**: Full-featured enterprise platform with security, LLM enhancement, and modern UI

## [0.2.50] - DEVELOPMENT BRANCH (ARCHIVED)
### Added - 🎉 MAJOR: Enterprise-Grade Smart Discovery System
- **🧠 Complete MCP 2025-06-18 Smart Discovery Integration**
  - **Server-side LLM Request Generation**: Full RequestGeneratorService with OpenAI, Anthropic, and Ollama integration
  - **Sampling Request Generation**: Automatic enhanced tool description generation for improved discoverability
  - **Elicitation Request Generation**: Dynamic parameter collection with LLM-powered template generation
  - **Tool Enhancement Pipeline**: Complete pipeline: base → sampling → elicitation → ranking with pre-generation
  - **Event-driven Enhancement**: Real-time tool enhancement on registry changes with startup pre-generation
  - **Performance Optimization**: Pre-generated enhanced descriptions maintain sub-second response times

- **🛡️ External MCP Protection & Integration**
  - **External MCP Detection**: Automatic detection of external/remote MCP tools using routing type analysis
  - **Capability Inheritance**: Query external MCP servers for their own sampling/elicitation capabilities first
  - **Protection System**: Prevent automatic LLM generation for external MCP tools to respect original capabilities
  - **MCP Version Detection**: Automatic MCP protocol version detection and metadata tracking for external servers
  - **Capability File Versioning**: Automatic saving of older versions when capability files change with rollback support

- **🔧 CLI Management & Monitoring**
  - **Enhanced Visibility CLI**: Added `show-mcp-warnings` command to `magictunnel-visibility` tool
  - **MCP Capability Override Warnings**: Detect and warn when local enhancement would override original MCP capabilities
  - **Detailed External MCP Analysis**: Complete analysis of external MCP tools with routing type and capability information
  - **Configuration Integration**: Mirror enhancement service configuration logic for accurate warning detection
  - **Actionable Recommendations**: Specific suggestions for resolving capability conflicts with config examples

### Enhanced - Architecture & Reliability
- **⚠️ Graceful Degradation**: Complete fallback system ensuring 100% reliability when enhancement fails
- **🔄 Batch Processing**: Efficient batch enhancement processing for multiple tool additions with concurrent execution
- **📊 Comprehensive Logging**: Enhanced logging with startup enhancement statistics and warning display
- **🎯 Error Handling**: Robust error handling with retry logic and timeout-based fallback to base descriptions
- **⚡ Caching System**: Smart caching of enhanced descriptions with failure cache to avoid repeated failures

### Technical - MCP 2025-06-18 Compliance
- **Extended MCP Types**: Enhanced ToolAnnotations with SamplingCapability and ElicitationCapability fields
- **Protocol Integration**: Complete integration with existing MCP server infrastructure
- **Configuration Schema**: Enhanced configuration with sampling/elicitation sections and smart discovery flags
- **External MCP Fetch Logic**: Updated to handle 2025-06-18 sampling/elicitation spec with metadata preservation
- **Lifetime Management**: Fixed compilation issues with proper lifetime handling in external manager

## [0.2.49]
### Added - Major Platform Enhancements
- **🌐 Network MCP Services & Protocol Gateway**
  - **HTTP MCP Client**: HTTP-over-MCP protocol support with connection pooling, authentication (Bearer, API Key, Basic Auth), and comprehensive error handling
  - **SSE MCP Client**: Server-Sent Events streaming protocol with connection lifecycle management, single-session request queuing, and automatic reconnection
  - **NetworkMcpServiceManager**: Unified service management for all network-based MCP services with health monitoring and capability discovery
  - **Protocol Translation**: Universal MCP protocol gateway supporting HTTP, SSE, and stdio protocols with seamless translation
  - **Configuration Integration**: Enhanced configuration schema supporting network services alongside process-based MCP servers

- **📊 Enhanced Monitoring & Observability**
  - **Real-time Web Dashboard**: Complete web-based management interface with live system monitoring, tool management, and configuration editing
  - **Live Log Viewer**: Professional real-time log streaming with filtering, search, pagination, and CSV export capabilities
  - **Performance Metrics**: System health monitoring with uptime tracking, performance metrics, and service status indicators
  - **MCP Testing Interface**: Interactive JSON-RPC 2.0 command testing with parameter validation and response analysis
  - **Service Management**: External MCP service monitoring with health checks, process control, and tool integration

### Enhanced - Documentation & Developer Experience
- **📋 Modular Documentation**: Restructuring with dedicated guides for Web Dashboard, Supervisor, and Frontend development
- **🌐 Protocol Compatibility Guide**: Comprehensive documentation for network MCP protocol translation with examples and troubleshooting
- **⚡ Quick Start Optimization**: Featured the recommended developer workflow: `make build-release-semantic && make pregenerate-embeddings-ollama MAGICTUNNEL_ENV=development`
- **🖥️ Web Dashboard Guide**: Complete web interface documentation with architecture diagrams, API references, and troubleshooting
- **🔧 Supervisor Guide**: Detailed process management documentation with configuration examples and production deployment guidance

### Fixed - User Experience & Architecture
- **🌐 Correct Port Configuration**: Fixed documentation to show correct dashboard URL (`http://localhost:5173/dashboard`) instead of incorrect port references
- **📁 Documentation Organization**: Moved `frontend_todo.md` to `docs/` folder and created proper documentation hierarchy

### Technical - Infrastructure Improvements
- **Architecture Clarity**: Updated all documentation to reflect the three-tier architecture: Frontend (5173) → MagicTunnel API (3001) → Supervisor TCP (8081)
- **Protocol Gateway**: Implementation of universal MCP protocol translation supporting multiple transport protocols
- **Service Integration**: Unified service management for both process-based and network-based MCP services
- **Development Workflow**: Optimized developer experience with recommended quick-start commands and proper environment setup

## [0.2.48]
### Added
- OpenAPI3.1 spec for making MCP tools available for OpenAI custom GPT
- Version Manager

## [0.2.47]
### Fix
- Claude not working
- Sequential mode not working

## [0.2.46]
### Added
- UI management for Prompts and Resources
- CLI management for Prompts and resources

## [0.2.45]
### Added
- Added UI management for Prompts and Resources
- Added CLI management for Prompts and Resources

## [0.2.44]
### Fixed
- Hot reloading of capabilities

## [0.2.43]
### Added
- **Hidden Tool Support**: Enhanced tool visibility management for cleaner user interfaces
  - All Google Sheets tools now marked as `hidden: true` to reduce tool list clutter
  - Hidden tools remain fully functional and discoverable through smart discovery
  - Specific sheet tools (e.g., `so_today_visit_performance`) hidden but accessible via natural language queries

### Changed
- **Tool Organization**: Improved tool categorization and visibility management
  - General Google Sheets tools (`google_sheets_read`, `google_sheets_write`, etc.) hidden from standard listings
  - Sheet-specific tools for sales performance data hidden but semantically discoverable
  - Clean separation between user-facing tools and background integration tools

### Enhanced
- **Smart Discovery Experience**: Users can access all Google Sheets functionality through natural language
  - Query "show me today's sales performance" → routes to hidden `so_today_visit_performance` tool
  - Maintains full functionality while presenting cleaner tool interface
  - Background tools available when needed but don't overwhelm general tool listings

## [0.2.42]
### Added
- **Google Sheets Integration**: Custom subprocess-based Google Sheets integration with service account authentication
  - Created custom Python-based Google Sheets tools (`google_sheets_tools.py`) supporting read, write, create, list operations
  - Added comprehensive tool definitions (`capabilities/google/google_sheets.yaml`) with proper schema validation
  - Service account authentication using `GOOGLE_APPLICATION_CREDENTIALS` environment variable
  - Range normalization for malformed sheet ranges (e.g., `Sheet1!A1:Z` → `Sheet1!A:Z`)
  - Support for special characters in sheet names (apostrophes, spaces)
  - Smart discovery integration for natural language Google Sheets requests

### Changed
- **Alternative MCP Integration**: Used subprocess routing instead of broken third-party MCP packages
  - Avoided issues with `xing5/mcp-google-sheets` (coroutine errors) and `mcp-gdrive` (import failures)
  - Implemented reliable subprocess-based approach for better stability
  - Enhanced parameter substitution for Google API integration

### Fixed
- **YAML Schema Validation**: Fixed `input:` vs `inputSchema:` field requirements in capability definitions
- **Embedding Generation**: Ensured proper tool registration and smart discovery filtering
- **Authentication Compatibility**: Service account support instead of OAuth-only approaches

### Documentation
- Added comprehensive Google Sheets integration guide (`GOOGLE_SHEETS_INTEGRATION.md`)
- Documented authentication setup, usage examples, and troubleshooting steps
- Included security best practices for service account credential management

## [0.2.41]
### Added
- Work towards Environment variables

## [0.2.40]

### Added
- Results UI for all tools

### Changed
- Updated Docs

## [0.2.39]

### Fixed
- Debug details 

## [0.2.38]

### Fixed 
- Proper details in Smart Discovery UI and backend info
- Segreagation of MCP result and API result.
- Enrichment of metadata

## [0.2.37]

### Added 
- MCP Services visibility, Tools visiblity, Service management
- New sidecar magictunnel-supervisor to allow for start/stop of magictunnel.
- Support for sending MCP queries in stdio mode

## [0.2.36]

### Added
- External MCP services view
- Proper filterig of Tools.

## [0.2.35]

### Added
- Added Configurations and templates and environment variables viewing

## [0.2.34]

### Added
- Added Frontend Dashboard

## [0.2.33]

### Added
- Sequential mode for multi-step workflows
- Fixed failing tests
 The tests were failing because the smart discovery integration test configurations were missing several required fields that were added to the configuration schema:

  1. max_high_quality_matches and high_quality_threshold
  2. llm_tool_selection section with all its required subfields
  3. fallback section with all its configuration options
  4. semantic_search section with storage, model, and performance subsections
  5. type field in the auth section

## [0.2.32]

## Fixed
- Better prompt for smart_tool_discovery

## [0.2.31]

### Added
- Implemented Semantic search, Hybrid search
- 3.8.1, 3.8.2, 3.8.3

## [0.2.30]

### Added
- Tests and Documentation for Smart router

## [0.2.29]

### Added
- User experience features

## [0.2.28]

### Added
- Option to early bail out of tool selection if we have enough high-quality matches
- Added configuration for the same

## [0.2.27]

### Fixed
- Cache isoldation of Rule Based vs LLM based
- Key extraction from config

## [0.2.26]

### Fixed
- Smart discovery
- LLM selection
- Better parameter extraction
- fixing discovery of external-mcp-servers.yaml when running from Claude 

## [0.2.24]

### Added
- Support for env files
- OpenAPI

## [0.2.23]

### Added
- Hidden and Enabled options for tools. Quick cli tool to manage these 
- Smart Discovery via one exposed too

## [0.2.22]

### Fixed
- Slowness of notifications/initialized call

## [0.2.21] 

### Added
- Websocket MCP integration

## [0.2.20] - Better External MCP naming

### Changed
- External MCP tools now have a better naming convention: `tool_name_server_name` instead of `server_name_tool_name`

## [0.2.19] - Fix Http tool execution

### Fixed
- Server was recreating a new instance and thus losing the router to handle external_mcp

## [0.2.18] - External MCP proxy fixes

### Fixed
- Making Description optional for Tool struct (despite being required by MCP protocol. GlobalPing misses description)
- Code change to support
- Missed passing params: {} earlier in tools list

## [0.2.17] - TLS Test Suite Fixes & Enhancements

### Fixed
- **TLS Test Suite Compilation**: Fixed compilation errors in TLS validation tests
  - Fixed method name mismatches: Changed `is_trusted()` calls to `is_trusted_proxy()` in trusted proxy tests
  - Updated test utilities to use actual available methods from TLS validation module
  - Replaced non-existent utility methods with working implementations using `ProxyHeaders::from_request()` and `ProxyValidationUtils::is_secure_request()`
  - All TLS tests now compile successfully and pass validation

## [0.2.16] - Test Suite Organization & Documentation

### Added
- Comprehensive test organization improvements
- Better test file naming conventions for clarity

### Changed
- **BREAKING**: Removed legacy remote MCP and local MCP configuration support
- **BREAKING**: Migrated from hybrid routing system to unified external MCP system
- Moved all MCP test files from `src/mcp/` to `tests/` directory following Rust best practices
- Renamed `mcp_phase3_tests.rs` to `mcp_session_and_validation_tests.rs` for descriptive clarity
- Updated test file imports to use proper crate-level imports (`magictunnel::`)
- Cleaned up deprecated configuration fields and structs

### Removed
- **BREAKING**: `remote_mcp` configuration field and `RemoteMcpDiscoveryConfig` struct
- **BREAKING**: `local_mcp` configuration field and `LocalMcpConfig` struct
- **BREAKING**: `RemoteMcpConflictResolution` struct and related conflict resolution types
- **BREAKING**: Environment variables `REMOTE_MCP_ENABLED` and `REMOTE_MCP_CONFLICT_RESOLUTION`
- Legacy hybrid routing support in favor of external MCP system
- Deprecated module exports and unused configuration types
- Test module declarations from `src/mcp/mod.rs` after moving tests

### Fixed
- Test compilation issues after moving test files to proper location
- Import path resolution for moved test files using `magictunnel::` crate imports
- Security validation test that was failing due to deprecated field usage
- All 300+ tests now passing with 100% success rate

### Technical Debt Reduction
- Removed over 1000 lines of legacy code and configuration
- Simplified configuration structure by removing deprecated fields
- Consolidated MCP functionality under single external MCP system
- Improved code maintainability by removing unused imports and dead code paths
- Enhanced test organization following Rust community best practices

### Test Organization Improvements
- **Test File Migration**: Moved 5 MCP test files from `src/mcp/` to `tests/` directory:
  - `external_tests.rs` → `mcp_external_tests.rs`
  - `integration_tests.rs` → `mcp_integration_tests.rs`
  - `logging_tests.rs` → `mcp_logging_tests.rs`
  - `notifications_tests.rs` → `mcp_notifications_tests.rs`
  - `phase3_tests.rs` → `mcp_session_and_validation_tests.rs`
- **Descriptive Naming**: Replaced ambiguous test file names with clear, functional descriptions
- **Import Path Updates**: Fixed all import paths to use proper crate-level imports
- **Test Documentation**: Updated test file headers with clear descriptions of test coverage

## [0.2.15] - External MCP Migration & Hybrid Routing Removal

### Added
- **External MCP System**: Complete unified External MCP system for all external MCP server integration
- **Claude Desktop Compatibility**: Exact same configuration format as Claude Desktop for familiar setup
- **Process Management**: Automatic spawning and lifecycle management of MCP servers
- **Container Support**: Built-in Docker/Podman integration for containerized MCP servers
- **Automatic Discovery**: Tools and capabilities discovered automatically from spawned processes

### Changed
- **BREAKING**: Completely removed hybrid routing system in favor of unified External MCP
- **Architecture Simplification**: Single integration point instead of multiple routing mechanisms
- **Test Suite Migration**: Updated all 457 tests to reflect new architecture with 100% pass rate
- **Configuration Migration**: All configurations now use External MCP format

### Removed
- **Hybrid Routing Code**: Completely removed all hybrid routing code and dependencies
- **Legacy MCP Systems**: Removed remote_mcp and local_mcp implementations
- **Complex Routing Logic**: Eliminated complex conflict resolution and tool aggregation systems
- **Unused Dependencies**: Removed WebSocket/SSE client libraries and authentication configs

### Fixed
- **Clean Build**: Zero compilation errors with no legacy code remnants
- **Test Stability**: All 457 tests passing with 100% success rate
- **Configuration Consistency**: Unified configuration format throughout project

### Enhanced
- **Modern Architecture**: Future-proof, extensible architecture for additional MCP server types
- **Better Performance**: Optimized for modern MCP server patterns
- **Enhanced Reliability**: Robust process management and error handling
- **Simplified Maintenance**: Single codebase for all external MCP integration

## [0.2.14]

### Added
- **Remote MCP Configuration Enhancement**: Enhanced remote MCP server configuration with proper conflict resolution
- **Configuration Template Documentation**: Comprehensive template files with accurate field structures and examples
- **Conflict Resolution Strategies**: Added local_first, remote_first, and prefix strategies for remote MCP integration
- **Authentication Format Improvements**: Simplified authentication configuration format (Bearer vs !Bearer)

### Changed
- **BREAKING**: Capability files now only support YAML format (removed JSON support)
  - All capability file output is now in YAML format only
  - Configuration system updated to reflect YAML-only support
  - Updated documentation and examples to use YAML format
- **Configuration File Consistency**: Fixed major inconsistencies between config.yaml and config.yaml.template
- **Project Rename**: Completed rename from mcp-proxy to MagicTunnel throughout codebase

### Fixed
- **Configuration Schema Alignment**: Fixed config.yaml to match actual Rust code implementation
- **Invalid Field Removal**: Removed invalid grpc_port field (auto-calculated as port + 1000)
- **Registry Path Correction**: Removed remote capability paths from registry (handled by remote_mcp)
- **Template Accuracy**: Updated config.yaml.template to reflect actual code structure
- **Build System**: Verified compilation and functionality after configuration fixes

### Enhanced
- **Documentation Updates**: Updated all documentation files to reflect current project state
- **Template Files**: Enhanced template files with comprehensive examples and documentation
- **Configuration Validation**: Improved validation and error reporting for configuration files

## [0.2.12]

### Changed
- Renamed to MagicTunnel

## [0.2.11]

### Added
- Config files and configurations for Remote MCP proxy

## [0.2.10] - YAML Standardization & Configuration Cleanup

### Added
- **YAML Tag Support for AuthType** - Implemented proper YAML tag format (`!Bearer`, `!ApiKey`) for authentication configuration
- **Example File Organization** - Moved `test_swagger2.json` to `examples/swagger2_petstore.json` with proper documentation

### Changed
- **Configuration Format** - AuthType now supports both YAML tag format and nested format for backward compatibility
- **Documentation Updates** - Updated OpenAPI and unified CLI documentation to reference new example file locations
- **Example Config Generation** - CLI now generates user-friendly example configs with commented auth sections

### Removed
- **Unnecessary Test Files** - Removed `test-yaml-only.yaml` and `config/registry_test.yaml` (unused test files)
- **Temporary Files** - Cleaned up project root from temporary test configuration files

### Fixed
- **YAML Parsing** - Improved YAML configuration parsing with proper serde annotations
- **Example Config Validation** - All example configurations now parse correctly without errors

### Enhanced - OpenAPI Capability Generator Advanced Features ✅ **MAJOR ENHANCEMENT**

#### Advanced Schema Processing ✅ **FULLY IMPLEMENTED**
- **Complex nested object support** ✅ **FULLY IMPLEMENTED** - Recursive resolution of deeply nested object structures
- **Inheritance pattern support** ✅ **FULLY IMPLEMENTED** - Complete AllOf, OneOf, AnyOf schema composition handling
- **Schema validation and enhancement** ✅ **FULLY IMPLEMENTED** - Automatic validation with metadata enrichment
- **Type conversion improvements** ✅ **FULLY IMPLEMENTED** - Enhanced type mapping with format constraints

#### Component & Reference Resolution ✅ **FULLY IMPLEMENTED**
- **Comprehensive $ref resolution** ✅ **FULLY IMPLEMENTED** - Support for all OpenAPI reference types
- **Component reference support** ✅ **FULLY IMPLEMENTED** - Full components/schemas, parameters, responses resolution
- **External reference handling** ✅ **FULLY IMPLEMENTED** - HTTP/HTTPS external reference support
- **Swagger 2.0 compatibility** ✅ **FULLY IMPLEMENTED** - Complete definitions/ reference support

#### Enhanced Schema Features ✅ **FULLY IMPLEMENTED**
- **Recursive property resolution** ✅ **FULLY IMPLEMENTED** - Deep object property schema resolution
- **Array item schema resolution** ✅ **FULLY IMPLEMENTED** - Complex array item type handling
- **Format and constraint support** ✅ **FULLY IMPLEMENTED** - Min/max values, enums, format specifications
- **Schema metadata enrichment** ✅ **FULLY IMPLEMENTED** - MCP-specific metadata for better integration

## [0.2.9] - Test Suite Stabilization & Code Quality Improvements

### Fixed - Critical Test Infrastructure Issues ✅ **MAJOR STABILITY IMPROVEMENT**

#### Complete Test Suite Restoration ✅ **FULLY RESOLVED**
- **All 265 library tests now passing** ✅ **CRITICAL FIX** - Achieved 100% test success rate across entire codebase
- **Fixed import and type issues** ✅ **FULLY IMPLEMENTED** - Resolved missing imports for `CapabilityFile`, `ToolDefinition`, `FileMetadata`
- **Updated configuration structures** ✅ **FULLY IMPLEMENTED** - Fixed `GeneratorConfigFile` usage and field mappings
- **Resolved test file path issues** ✅ **FULLY IMPLEMENTED** - Fixed GraphQL schema file path references and data dependencies
- **Enhanced struct and enum compatibility** ✅ **FULLY IMPLEMENTED** - Updated test code to match current codebase structure

#### Test Infrastructure Improvements ✅ **FULLY IMPLEMENTED**
- **Fixed configuration structure usage** ✅ **CRITICAL FIX** - Updated `AuthConfig` with missing `headers` field in TOML examples
- **Resolved namespace conflicts** ✅ **FULLY IMPLEMENTED** - Fixed module path issues and type references
- **Enhanced test reliability** ✅ **FULLY IMPLEMENTED** - Improved test data management and file organization
- **Updated field access patterns** ✅ **FULLY IMPLEMENTED** - Fixed struct initialization and field access across tests

#### Code Quality Enhancements ✅ **FULLY IMPLEMENTED**
- **Comprehensive codebase validation** ✅ **FULLY IMPLEMENTED** - All core functionality tests operational
- **Enhanced maintainability** ✅ **FULLY IMPLEMENTED** - Improved code organization and test structure
- **Future-proof test infrastructure** ✅ **FULLY IMPLEMENTED** - Robust foundation for continued development
- **Documentation consistency** ✅ **FULLY IMPLEMENTED** - Updated test documentation and examples

### Enhanced - Testing & Quality Assurance

#### Test Suite Statistics ✅ **COMPREHENSIVE COVERAGE**
- **Total Library Tests**: 265 ✅ (0 ❌) - 100% success rate
- **Integration Tests**: 18 ✅ (0 ❌) - All critical functionality validated
- **Test Categories**: All major components covered (GraphQL, gRPC, OpenAPI, MCP protocol, authentication, routing)
- **Test Infrastructure**: Robust and maintainable test framework established

#### Quality Improvements
- **Zero failing tests** ✅ **ACHIEVED** - Complete test suite operational
- **Enhanced error handling** ✅ **IMPLEMENTED** - Better test failure diagnostics and debugging
- **Improved test organization** ✅ **IMPLEMENTED** - Cleaner test structure and data management
- **Comprehensive validation** ✅ **IMPLEMENTED** - All core features thoroughly tested

### Technical - Infrastructure Improvements

#### Codebase Stability
- **Type system consistency** ✅ **FULLY IMPLEMENTED** - All type references and imports properly aligned
- **Configuration system reliability** ✅ **FULLY IMPLEMENTED** - Robust configuration parsing and validation
- **Module organization** ✅ **FULLY IMPLEMENTED** - Clean module structure with proper exports
- **Dependency management** ✅ **FULLY IMPLEMENTED** - All dependencies properly configured and tested

#### Development Experience
- **Reliable test execution** ✅ **FULLY IMPLEMENTED** - Consistent test results across environments
- **Clear error messages** ✅ **FULLY IMPLEMENTED** - Improved debugging and troubleshooting
- **Maintainable codebase** ✅ **FULLY IMPLEMENTED** - Clean code structure for future development
- **Comprehensive validation** ✅ **FULLY IMPLEMENTED** - All changes thoroughly tested and validated

### Added - Unified Capability Generation CLI ✅ **MAJOR FEATURE**

#### Unified Command-Line Interface
- **Single CLI Tool** ✅ **FULLY IMPLEMENTED** - Unified `mcp-generator` CLI for all generator types
- **Common Command Structure** ✅ **FULLY IMPLEMENTED** - Consistent command-line arguments across generators
- **Generator Adapters** ✅ **FULLY IMPLEMENTED** - Adapter classes for GraphQL, gRPC, and OpenAPI generators
- **Extensible Architecture** ✅ **FULLY IMPLEMENTED** - Easily add new generator types with minimal code

#### Configuration File Support
- **TOML Configuration** ✅ **FULLY IMPLEMENTED** - Rich configuration file support for complex setups
- **Global Settings** ✅ **FULLY IMPLEMENTED** - Common settings applied to all generators
- **Generator-Specific Settings** ✅ **FULLY IMPLEMENTED** - Override global settings for specific generators
- **Environment Variable Support** ✅ **FULLY IMPLEMENTED** - Configuration via environment variables
- **Validation System** ✅ **FULLY IMPLEMENTED** - Comprehensive configuration validation

#### Advanced Features
- **Merging Functionality** ✅ **FULLY IMPLEMENTED** - Combine multiple capability files with conflict resolution
- **Validation Command** ✅ **FULLY IMPLEMENTED** - Validate capability files against schema and best practices
- **Authentication Support** ✅ **FULLY IMPLEMENTED** - Bearer, API Key, Basic, and OAuth authentication
- **Output Formatting** ✅ **FULLY IMPLEMENTED** - JSON and YAML output with pretty-printing options

### Enhanced - Testing & Quality Assurance

#### Unified CLI Test Suite ✅ **NEW TESTS**

**Configuration Tests** ✅ **FULLY IMPLEMENTED**
- **Configuration Parsing Tests** - TOML configuration file parsing and validation
- **Environment Variable Tests** - Environment variable override testing
- **Validation Tests** - Configuration validation with error handling

**Generator Adapter Tests** ✅ **FULLY IMPLEMENTED**
- **GraphQL Adapter Tests** - GraphQL generator adapter functionality
- **gRPC Adapter Tests** - gRPC generator adapter functionality
- **OpenAPI Adapter Tests** - OpenAPI generator adapter functionality
- **Common Interface Tests** - CapabilityGeneratorBase trait implementation

**Capability Merging Tests** ✅ **FULLY IMPLEMENTED**
- **Basic Merging Tests** - Combining multiple capability files
- **Conflict Resolution Tests** - Different strategies for handling duplicates
- **Edge Case Tests** - Empty files, invalid files, schema version mismatches

**Capability Validation Tests** ✅ **FULLY IMPLEMENTED**
- **Schema Validation Tests** - JSON Schema validation for capability files
- **Best Practice Tests** - Validation against best practices
- **Validation Level Tests** - Strict, normal, and relaxed validation levels

**Integration Tests** ✅ **FULLY IMPLEMENTED**
- **End-to-End Tests** - Complete CLI workflow testing
- **Command-Line Argument Tests** - Argument parsing and validation
- **Real-World Scenario Tests** - Testing with complex schemas and specifications

#### Test Infrastructure Improvements
- **Test Count** ✅ **INCREASED** - Added comprehensive test suite for unified CLI
- **Test Coverage** ✅ **COMPREHENSIVE** - Complete coverage of unified CLI functionality
- **Test Organization** ✅ **IMPROVED** - Structured test files for better maintainability

### Documentation & Examples

#### CLI Usage Examples
```bash
# Generate from GraphQL schema
mcp-generator graphql \
  --endpoint "https://api.example.com/graphql" \
  --schema "schema.graphql" \
  --output "capabilities/graphql_tools.json" \
  --prefix "graphql"

# Generate from gRPC proto file
mcp-generator grpc \
  --endpoint "grpc.example.com:50051" \
  --proto "service.proto" \
  --output "capabilities/grpc_tools.json" \
  --prefix "grpc"

# Generate from OpenAPI specification
mcp-generator openapi \
  --base-url "https://api.example.com" \
  --spec "openapi.yaml" \
  --output "capabilities/openapi_tools.json" \
  --prefix "api"

# Using a configuration file
mcp-generator --config "generator_config.toml" graphql

# Merge multiple capability files
mcp-generator merge \
  --input "capabilities/graphql_tools.json" \
  --input "capabilities/grpc_tools.json" \
  --output "capabilities/merged_tools.json" \
  --strategy "rename"

# Validate a capability file
mcp-generator validate \
  --input "capabilities/tools.json" \
  --level "strict"
```

#### Configuration File Example
```yaml
# Global settings for all generators
global:
  tool_prefix: "mcp"
  output_dir: "./capabilities"

# GraphQL generator settings
graphql:
  endpoint: "https://api.example.com/graphql"
  schema: "schema.graphql"
  tool_prefix: "graphql"  # Overrides global prefix

# gRPC generator settings
grpc:
  endpoint: "grpc.example.com:50051"
  proto: "service.proto"
  tool_prefix: "grpc"  # Overrides global prefix

# OpenAPI generator settings
openapi:
  base_url: "https://api.example.com"
  spec: "openapi.yaml"
  tool_prefix: "api"  # Overrides global prefix

# Output settings
output:
  format: "json"  # or "yaml"
  pretty: true
```

## [0.2.8] - GraphQL & OpenAPI Capability Generators Complete

### Added - Capability Generation System ✅ **MAJOR FEATURE**

#### Complete GraphQL Specification Compliance (100%)
- **GraphQL SDL Parser** ✅ **FULLY IMPLEMENTED** - Complete Schema Definition Language parsing with multi-line support
- **Introspection JSON Parser** ✅ **FULLY IMPLEMENTED** - Support for GraphQL introspection query results in multiple formats
- **Operation Extraction** ✅ **FULLY IMPLEMENTED** - Automatic MCP tool generation from queries, mutations, and subscriptions
- **Type System Support** ✅ **FULLY IMPLEMENTED** - All GraphQL types (scalars, objects, enums, interfaces, unions, input objects)
- **Schema Extensions** ✅ **FULLY IMPLEMENTED** - Support for `extend` keyword with intelligent type merging
- **Directive Processing** ✅ **FULLY IMPLEMENTED** - Parse and process GraphQL directives for operation customization

#### Advanced Schema Processing
- **Schema Validation** ✅ **FULLY IMPLEMENTED** - Comprehensive validation with type checking and safety analysis
- **Circular Reference Handling** ✅ **FULLY IMPLEMENTED** - Intelligent handling of circular type dependencies
- **Default Value Support** ✅ **FULLY IMPLEMENTED** - Argument default values with type validation
- **Custom Scalar Recognition** ✅ **FULLY IMPLEMENTED** - Support for custom scalar types
- **Multi-line Argument Parsing** ✅ **FULLY IMPLEMENTED** - Handle complex real-world schemas with multi-line definitions

#### Production Features
- **Authentication Integration** ✅ **FULLY IMPLEMENTED** - Support for Bearer tokens, API keys, and custom headers
- **Real-World Schema Support** ✅ **FULLY IMPLEMENTED** - Tested with complex schemas (9,951 lines, 484 operations)
- **Performance Optimization** ✅ **FULLY IMPLEMENTED** - Optimized for large schemas and high-throughput scenarios
- **Comprehensive Error Handling** ✅ **FULLY IMPLEMENTED** - Detailed validation messages and robust error recovery

#### OpenAPI Capability Generator ✅ **MAJOR FEATURE**

#### Complete OpenAPI 3.0 Specification Support
- **OpenAPI 3.0 Parser** ✅ **FULLY IMPLEMENTED** - Complete JSON/YAML specification parsing with auto-detection
- **HTTP Method Support** ✅ **FULLY IMPLEMENTED** - All standard HTTP methods (GET, POST, PUT, PATCH, DELETE, HEAD, OPTIONS, TRACE)
- **Parameter Mapping** ✅ **FULLY IMPLEMENTED** - Path, query, header, and cookie parameters with JSON Schema conversion
- **Request/Response Processing** ✅ **FULLY IMPLEMENTED** - Basic schema conversion for request bodies and responses
- **Operation Filtering** ✅ **FULLY IMPLEMENTED** - Filter by HTTP methods, operation IDs, tags, or path patterns

#### Comprehensive Authentication Support
- **API Key Authentication** ✅ **FULLY IMPLEMENTED** - Support for API key in headers, query parameters, or cookies
- **Bearer Token Authentication** ✅ **FULLY IMPLEMENTED** - JWT and OAuth bearer token authentication
- **Basic Authentication** ✅ **FULLY IMPLEMENTED** - Username/password basic authentication with base64 encoding
- **OAuth 2.0 Authentication** ✅ **FULLY IMPLEMENTED** - OAuth token authentication with configurable token types
- **Custom Headers** ✅ **FULLY IMPLEMENTED** - Support for additional authentication headers

#### Advanced Configuration Features
- **Flexible Naming Conventions** ✅ **FULLY IMPLEMENTED** - Multiple naming strategies (operation-id, method-path, custom templates)
- **Tool Prefixes** ✅ **FULLY IMPLEMENTED** - Namespace tools with configurable prefixes for organization
- **Deprecation Handling** ✅ **FULLY IMPLEMENTED** - Option to include or exclude deprecated operations
- **CLI Tool** ✅ **FULLY IMPLEMENTED** - Production-ready command-line interface for capability generation

#### Production Features
- **Real-World Validation** ✅ **FULLY IMPLEMENTED** - Tested with Petstore OpenAPI specification (6 endpoints)
- **Error Handling** ✅ **FULLY IMPLEMENTED** - Robust error handling with detailed validation messages
- **Performance Optimization** ✅ **FULLY IMPLEMENTED** - Optimized for large API specifications and high-throughput scenarios
- **Auto-Detection** ✅ **FULLY IMPLEMENTED** - Automatic format detection (JSON/YAML) and version validation

### Enhanced - Testing & Quality Assurance

#### Capability Generator Test Suite ✅ **51 NEW TESTS**

**GraphQL Generator Tests** ✅ **45 TESTS**
- **SDL Schema Parsing Tests** ✅ **FULLY IMPLEMENTED** - Complete SDL syntax validation
- **Introspection JSON Tests** ✅ **FULLY IMPLEMENTED** - Multiple introspection format support
- **Type System Tests** ✅ **FULLY IMPLEMENTED** - All GraphQL type validation
- **Schema Extension Tests** ✅ **FULLY IMPLEMENTED** - Extension merging and validation
- **Directive Processing Tests** ✅ **FULLY IMPLEMENTED** - Directive parsing and usage logic
- **Schema Validation Tests** ✅ **FULLY IMPLEMENTED** - Comprehensive schema safety analysis
- **Real-World Integration Tests** ✅ **FULLY IMPLEMENTED** - Production GraphQL API testing
- **Authentication Tests** ✅ **FULLY IMPLEMENTED** - Auth header and token validation

**OpenAPI Generator Tests** ✅ **6 TESTS**
- **OpenAPI 3.0 Parsing Tests** ✅ **FULLY IMPLEMENTED** - JSON/YAML specification parsing with auto-detection
- **HTTP Method Tests** ✅ **FULLY IMPLEMENTED** - All standard HTTP methods validation
- **Authentication Tests** ✅ **FULLY IMPLEMENTED** - API Key, Bearer, Basic, OAuth authentication schemes
- **Parameter Mapping Tests** ✅ **FULLY IMPLEMENTED** - Path, query, header, cookie parameter handling
- **Configuration Tests** ✅ **FULLY IMPLEMENTED** - Naming conventions, filtering, and prefix options
- **Real-World Integration Tests** ✅ **FULLY IMPLEMENTED** - Petstore OpenAPI specification validation

#### Test Infrastructure Improvements
- **Total Test Count** ✅ **168 TESTS** - Increased from 117 to 168 tests (+45 GraphQL, +6 OpenAPI tests)
- **Test Coverage** ✅ **COMPREHENSIVE** - Complete GraphQL and OpenAPI specification coverage
- **Performance Testing** ✅ **FULLY IMPLEMENTED** - Large schema performance validation
- **Error Scenario Testing** ✅ **FULLY IMPLEMENTED** - Edge case and error handling validation

#### CLI Usage Examples

**GraphQL Capability Generation:**
```bash
# Generate MCP tools from GraphQL schema
cargo run --bin graphql_generator -- \
  --endpoint "https://api.github.com/graphql" \
  --auth-header "Authorization: Bearer YOUR_TOKEN" \
  --prefix "github" \
  --output "capabilities/github_tools.yaml"

# Generate from local SDL file
cargo run --bin graphql_generator -- \
  --schema-file "schema.graphql" \
  --endpoint "https://api.example.com/graphql" \
  --prefix "api" \
  --output "capabilities/api_tools.yaml"
```

**OpenAPI Capability Generation:**
```bash
# Generate MCP tools from OpenAPI specification
cargo run --bin openapi-generator -- \
  --spec "petstore.json" \
  --base-url "https://petstore.swagger.io/v2" \
  --prefix "petstore" \
  --auth-type "bearer" \
  --auth-token "YOUR_API_TOKEN" \
  --output "capabilities/petstore_tools.yaml"

# Generate with filtering and custom naming
cargo run --bin openapi-generator -- \
  --spec "https://api.example.com/openapi.yaml" \
  --base-url "https://api.example.com" \
  --prefix "api" \
  --filter-methods "GET,POST,PUT,DELETE" \
  --naming-convention "operation-id" \
  --auth-type "api-key" \
  --auth-key "YOUR_API_KEY" \
  --output "capabilities/api_tools.yaml"
```

#### Generated Tool Examples

**OpenAPI Generated Tool:**
```yaml
# Generated from OpenAPI specification
tools:
  - name: "petstore_getPetById"
    description: "Find pet by ID"
    inputSchema:
      type: "object"
      properties:
        petId:
          type: "string"
          description: "ID of pet to return"
      required: ["petId"]
    routing:
      type: "http"
      config:
        method: "GET"
        url: "https://petstore.swagger.io/v2/pet/{petId}"
        headers:
          Authorization: "Bearer {{auth_token}}"
          Content-Type: "application/json"
        path_params: ["petId"]
        timeout: 30
```

**GraphQL Generated Tool:**
```yaml
# Generated from GraphQL schema
tools:
  - name: "github_getRepository"
    description: "Lookup a given repository by the owner and repository name"
    inputSchema:
      type: "object"
      properties:
        owner:
          type: "string"
          description: "The login field of a user or organization"
        name:
          type: "string"
          description: "The name of the repository"
      required: ["owner", "name"]
    routing:
      type: "graphql"
      config:
        endpoint: "https://api.github.com/graphql"
        query: "query($owner: String!, $name: String!) { repository(owner: $owner, name: $name) { ... } }"
        headers:
          Authorization: "Bearer {{auth_token}}"
```

### Updated - Documentation & Configuration

#### Documentation Updates
- **README.md** ✅ **UPDATED** - Added comprehensive GraphQL and OpenAPI capability generator sections
- **TODO.md** ✅ **UPDATED** - Marked Phase 3.5 and 3.6 complete, updated OpenAPI implementation status
- **DEVELOPMENT.md** ✅ **UPDATED** - Added GraphQL and OpenAPI generator development guides with architecture
- **Test Documentation** ✅ **UPDATED** - Updated test counts and coverage information for both generators

#### Version Updates
- **Project Version** ✅ **UPDATED** - Bumped to 0.2.8 reflecting GraphQL and OpenAPI generator completion
- **Feature Status** ✅ **UPDATED** - Phase 3.5 and 3.6 marked complete in all documentation
- **Next Phase Planning** ✅ **UPDATED** - Advanced OpenAPI features and gRPC capability generation outlined

## [0.2.7] - Complete Test Suite & JSON-RPC Compliance

### Fixed - Critical Bug Fixes

#### JSON-RPC 2.0 Compliance ✅ **FULLY IMPLEMENTED**
- **Added missing `jsonrpc` field** ✅ **CRITICAL FIX** - All MCP responses now include required `"jsonrpc": "2.0"` field
- **Updated McpResponse struct** ✅ **FULLY IMPLEMENTED** - Added jsonrpc field to response structure
- **Fixed response creation functions** ✅ **FULLY IMPLEMENTED** - Updated all response builders to include jsonrpc field
- **Fixed hardcoded error responses** ✅ **FULLY IMPLEMENTED** - Updated fallback error strings with jsonrpc field
- **WebSocket protocol compliance** ✅ **FULLY IMPLEMENTED** - WebSocket MCP responses now fully JSON-RPC 2.0 compliant

#### Test Infrastructure Fixes ✅ **FULLY IMPLEMENTED**
- **Fixed test server configuration** ✅ **CRITICAL FIX** - Properly configured test servers with required dependencies
- **Updated async test functions** ✅ **FULLY IMPLEMENTED** - Made create_test_server() async with proper initialization
- **Fixed dependency injection** ✅ **FULLY IMPLEMENTED** - Added RegistryService and McpServer to test infrastructure
- **Fixed TLS validation test** ✅ **FULLY IMPLEMENTED** - Added test IP to trusted proxies configuration
- **Updated test expectations** ✅ **FULLY IMPLEMENTED** - Fixed assertions to match actual response formats

### Enhanced - Test Suite Improvements

#### Complete Test Coverage ✅ **400+ TESTS PASSING**
- **Performance tests** ✅ **8 tests** - Response times, memory usage, concurrency validation
- **Streaming protocol tests** ✅ **8 tests** - WebSocket, SSE, HTTP streaming with JSON-RPC compliance
- **Security validation tests** ✅ **9 tests** - Input sanitization, injection prevention
- **MCP protocol tests** ✅ **65+ tests** - Protocol compliance, error handling, message formats
- **Agent router tests** ✅ **14 tests** - Routing configuration, parameter substitution
- **Configuration tests** ✅ **7 tests** - Validation, environment overrides
- **Integration tests** ✅ **300+ tests** - End-to-end functionality validation

#### Test Quality Improvements
- **100% pass rate** ✅ **FULLY ACHIEVED** - All 400+ tests now passing successfully
- **JSON-RPC compliance testing** ✅ **FULLY IMPLEMENTED** - Validates proper protocol adherence
- **Error response validation** ✅ **FULLY IMPLEMENTED** - Tests expect proper 4xx/5xx status codes
- **WebSocket message format testing** ✅ **FULLY IMPLEMENTED** - Validates proper JSON-RPC message structure
- **Concurrent execution testing** ✅ **FULLY IMPLEMENTED** - Multi-threaded test stability

### Technical - Infrastructure Improvements

#### Protocol Compliance
- **JSON-RPC 2.0 specification adherence** ✅ **FULLY IMPLEMENTED** - All responses include required fields
- **WebSocket message format compliance** ✅ **FULLY IMPLEMENTED** - Proper JSON-RPC over WebSocket
- **Error code standardization** ✅ **FULLY IMPLEMENTED** - Consistent error response formats
- **Message ID handling** ✅ **FULLY IMPLEMENTED** - Proper request/response correlation

#### Test Infrastructure Robustness
- **Async test server creation** ✅ **FULLY IMPLEMENTED** - Proper async/await patterns in tests
- **Dependency injection in tests** ✅ **FULLY IMPLEMENTED** - Clean test setup with required services
- **TLS configuration testing** ✅ **FULLY IMPLEMENTED** - Proper proxy validation with trusted IPs
- **Mock service integration** ✅ **FULLY IMPLEMENTED** - Reliable test doubles for external dependencies

### Testing - Quality Assurance Achievements

#### Comprehensive Test Statistics
- **Total tests**: 400+ tests across all modules and integration scenarios
- **Unit tests**: 92 tests for core library functionality
- **Integration tests**: 300+ tests across 25 comprehensive test files
- **Success rate**: 100% pass rate across all test suites
- **Coverage areas**: 10 major functional areas fully tested

#### Test Categories Validated
- **MCP Protocol Compliance**: JSON-RPC 2.0, message formats, error codes
- **Streaming Protocols**: WebSocket, SSE, HTTP streaming with proper formatting
- **Performance**: Response times, memory usage, concurrent request handling
- **Security**: Input validation, injection prevention, authentication
- **Agent Routing**: HTTP, gRPC, SSE, subprocess routing with parameter substitution
- **Registry Service**: Tool discovery, capability loading, hot-reload functionality
- **Configuration Management**: Validation, environment overrides, security settings
- **Error Handling**: Proper error responses, edge case handling
- **Authentication**: API keys, JWT, OAuth with comprehensive permission testing
- **Data Structures**: Tool definitions, routing configs, validation schemas

## [0.2.6] - Complete JWT Authentication System

### Added - New Features

#### Complete JWT Authentication Implementation ✅ **FULLY IMPLEMENTED**
- **JWT token validation** ✅ **FULLY IMPLEMENTED** - Complete JWT parsing, validation, and generation
- **Multi-algorithm support** ✅ **FULLY IMPLEMENTED** - HMAC (HS256/384/512), RSA (RS256/384/512), ECDSA (ES256/384)
- **Claims validation** ✅ **FULLY IMPLEMENTED** - Issuer, audience, and expiration validation
- **User information embedding** ✅ **FULLY IMPLEMENTED** - Rich user metadata in tokens (ID, email, name, roles)
- **Permission system** ✅ **FULLY IMPLEMENTED** - Granular permissions embedded in JWT claims
- **JWT middleware integration** ✅ **FULLY IMPLEMENTED** - Seamless integration with authentication middleware
- **Token sources** ✅ **FULLY IMPLEMENTED** - Authorization header and query parameter support
- **Fallback authentication** ✅ **FULLY IMPLEMENTED** - API Key → OAuth → JWT authentication chain

#### Enhanced Authentication System ✅ **FULLY IMPLEMENTED**
- **Multi-type authentication framework** ✅ **FULLY IMPLEMENTED** - Unified middleware supporting all three types
- **Authentication type detection** ✅ **FULLY IMPLEMENTED** - Intelligent authentication method detection
- **Comprehensive error handling** ✅ **FULLY IMPLEMENTED** - Type-specific error responses and logging
- **Security enhancements** ✅ **FULLY IMPLEMENTED** - Algorithm validation and key requirements

### Enhanced - Improvements

#### JWT Configuration & Security
- **JWT configuration structure** ✅ **FULLY IMPLEMENTED** - Complete JwtConfig with validation
- **Security validation** ✅ **FULLY IMPLEMENTED** - Minimum key length and algorithm validation
- **Configuration examples** ✅ **FULLY IMPLEMENTED** - Production-ready configuration samples
- **Environment support** ✅ **FULLY IMPLEMENTED** - Environment variable configuration support

#### Documentation & Examples
- **JWT documentation** ✅ **FULLY IMPLEMENTED** - Complete authentication guide updates
- **Working examples** ✅ **FULLY IMPLEMENTED** - JWT example with token generation and usage
- **Algorithm matrix** ✅ **FULLY IMPLEMENTED** - Detailed algorithm support documentation
- **Security best practices** ✅ **FULLY IMPLEMENTED** - JWT security recommendations

### Testing - Quality Assurance

#### Comprehensive JWT Testing ✅ **FULLY IMPLEMENTED**
- **JWT integration tests** ✅ **10 tests** - Complete JWT authentication testing
- **Token lifecycle testing** ✅ **FULLY IMPLEMENTED** - Generation, validation, and expiration
- **Middleware testing** ✅ **FULLY IMPLEMENTED** - JWT middleware integration validation
- **Permission testing** ✅ **FULLY IMPLEMENTED** - Granular permission checking
- **Error scenario testing** ✅ **FULLY IMPLEMENTED** - Comprehensive error handling validation

#### Updated Test Statistics
- **Total tests**: 260+ tests across all modules
- **Authentication tests**: 36 total (7 API Key + 19 OAuth + 10 JWT)
- **Success rate**: 100% pass rate across all test suites
- **JWT specific**: 10 comprehensive integration tests

## [0.2.5] - Complete OAuth 2.0 Authentication System

### Added - New Features

#### Complete OAuth 2.0 Authentication Implementation ✅ **FULLY IMPLEMENTED**
- **OAuth 2.0 provider integration** ✅ **FULLY IMPLEMENTED** - GitHub, Google, Microsoft Azure AD support
- **Authorization code flow** ✅ **FULLY IMPLEMENTED** - Complete OAuth 2.0 authorization flow
- **Token validation and refresh** ✅ **FULLY IMPLEMENTED** - Access token validation and refresh logic
- **User information retrieval** ✅ **FULLY IMPLEMENTED** - User profile data from OAuth providers
- **OAuth middleware integration** ✅ **FULLY IMPLEMENTED** - Seamless integration with authentication middleware
- **Provider-specific configuration** ✅ **FULLY IMPLEMENTED** - Flexible provider configuration system

#### Enhanced Authentication Framework ✅ **FULLY IMPLEMENTED**
- **Multi-provider support** ✅ **FULLY IMPLEMENTED** - Support for multiple OAuth providers simultaneously
- **Flexible redirect handling** ✅ **FULLY IMPLEMENTED** - Configurable redirect URIs and success pages
- **Scope management** ✅ **FULLY IMPLEMENTED** - Configurable OAuth scopes per provider
- **State parameter validation** ✅ **FULLY IMPLEMENTED** - CSRF protection with state validation

### Enhanced - Improvements

#### OAuth Configuration & Security
- **OAuth configuration structure** ✅ **FULLY IMPLEMENTED** - Complete OAuthConfig with provider settings
- **Security validation** ✅ **FULLY IMPLEMENTED** - Client secret validation and secure token handling
- **Provider validation** ✅ **FULLY IMPLEMENTED** - Supported provider validation and configuration
- **Redirect URI validation** ✅ **FULLY IMPLEMENTED** - Secure redirect URI validation

### Testing - Quality Assurance

#### Comprehensive OAuth Testing ✅ **FULLY IMPLEMENTED**
- **OAuth integration tests** ✅ **19 tests** - Complete OAuth authentication testing
- **Provider flow testing** ✅ **FULLY IMPLEMENTED** - Authorization flow validation
- **Token handling testing** ✅ **FULLY IMPLEMENTED** - Token validation and refresh testing
- **Error scenario testing** ✅ **FULLY IMPLEMENTED** - Comprehensive error handling validation

## [0.2.4] - Authentication & Security System

### Added - New Features

#### Authentication System Implementation ✅ **API KEY COMPLETE**
- **API Key authentication** ✅ **FULLY IMPLEMENTED** - Bearer token support with comprehensive validation, middleware, and testing
- **OAuth 2.0 authentication framework** ⚠️ **CONFIGURATION ONLY** - Structure and validation ready, but OAuth flow implementation needed
- **JWT authentication framework** ⚠️ **CONFIGURATION ONLY** - Structure and validation ready, but JWT validation implementation needed
- **Flexible authentication type selection** via configuration with backward compatibility
- **Granular permission-based access control** ✅ **FULLY IMPLEMENTED** - read/write/admin permissions per endpoint
- **API key management** ✅ **FULLY IMPLEMENTED** - expiration, active/inactive status, and metadata
- **Authentication middleware** ✅ **FULLY IMPLEMENTED** - HTTP request validation and detailed error responses
- **Comprehensive security features** ✅ **FULLY IMPLEMENTED** - audit logging and security best practices

#### Enhanced Configuration System ✅ **FULLY IMPLEMENTED**
- **Extended AuthConfig structure** with comprehensive authentication options for all types
- **AuthType enum** for authentication method selection (ApiKey, OAuth, JWT)
- **ApiKeyConfig structure** ✅ **FULLY IMPLEMENTED** - detailed key management and validation
- **OAuthConfig structure** ⚠️ **CONFIGURATION ONLY** - ready for OAuth implementation
- **JwtConfig structure** ⚠️ **CONFIGURATION ONLY** - ready for JWT implementation
- **ApiKeyEntry structure** with individual key configuration, permissions, and metadata
- **Backward compatibility** ensuring existing configurations work without changes

#### Security & Access Control ✅ **FULLY IMPLEMENTED (API KEY ONLY)**
- **Endpoint-specific permissions** with granular access control (API Key authentication):
  - `/health` - No authentication required (always accessible)
  - `/mcp/tools` - Requires `read` permission for tool discovery
  - `/mcp/call` - Requires `write` permission for tool execution
  - `/mcp/call/stream` - Requires `write` permission for streaming execution
- **Secure validation** with timing attack protection and comprehensive input validation
- **Comprehensive audit logging** for authentication events and security monitoring
- **Error handling** following security best practices with proper HTTP status codes
- **Note**: Security features currently apply only to API Key authentication

### Enhanced - Improvements

#### MCP Server Integration
- **All HTTP endpoints** now support optional authentication with seamless integration
- **Permission validation** integrated into all protected endpoints
- **Detailed error responses** for authentication and authorization failures
- **Security middleware** providing consistent authentication across all endpoints

#### Testing Infrastructure ✅ **FULLY IMPLEMENTED (API KEY ONLY)**
- **7 comprehensive API key authentication tests** covering all authentication scenarios:
  - Health check without authentication validation
  - Valid API key authentication testing
  - Invalid API key rejection testing
  - Missing API key handling validation
  - Permission-based access control testing
  - Admin vs read-only permission differentiation
  - Disabled authentication mode compatibility testing
- **Fixed compilation issues** across all test files with updated AuthConfig usage
- **Enhanced test coverage** to 250+ total tests across 14 comprehensive test suites
- **Note**: OAuth and JWT tests will be added when those authentication methods are implemented

### Documentation & Examples
- **Comprehensive authentication guide** (`docs/AUTHENTICATION.md`) with setup instructions
- **Example configuration file** (`examples/auth_config.yaml`) with practical examples
- **Security best practices** documentation for production deployments
- **Updated README.md** with authentication system features and current status
- **Updated TODO.md** reflecting Phase 3.3 completion and next steps

### Fixed - Bug Fixes
- **Test compilation issues** with AuthConfig structure updates across all test files
- **Deprecated method calls** updated (`remote_addr()` to `peer_addr()`)
- **Unused imports** cleaned up across multiple test files
- **Code quality improvements** with enhanced error handling and documentation

### Security - Security Improvements ✅ **FULLY IMPLEMENTED (API KEY ONLY)**
- **Authentication security** with secure API key validation and timing attack protection
- **Access control** with granular permission system preventing unauthorized access
- **Input validation** for all authentication data with comprehensive error handling
- **Audit logging** for security monitoring and compliance requirements
- **Note**: Security improvements currently apply only to API Key authentication

### Future Development - OAuth & JWT Implementation Needed
- **OAuth 2.0 Implementation**: Requires OAuth flow logic, provider integration, and token validation
- **JWT Implementation**: Requires JWT parsing, signature verification, and claims validation
- **Extended Testing**: OAuth and JWT test suites need development
- **Multi-Auth Middleware**: Extend middleware to handle multiple authentication types simultaneously

## [0.2.3] - Core Hybrid Routing & MCP Proxy Integration

### Added - New Features

#### GraphQL Agent Routing Implementation ✅ **COMPLETE**
- **Added GraphQL agent type** to routing system for exposing GraphQL endpoints as MCP tools
- **Implemented GraphQL client** with query and mutation execution support
- **Added comprehensive configuration parsing** for GraphQL endpoints (endpoint, query, variables, headers, timeout, operation_name)
- **Implemented advanced parameter substitution** with pure placeholder replacement for JSON values
- **Added runtime query support** allowing GraphQL queries to be provided in tool call arguments
- **Support for GraphQL mutations, subscriptions, and schema introspection** with comprehensive examples
- **Created comprehensive test suite** with 8 test cases covering parsing, validation, execution, and complex parameter substitution
- **Added GraphQL capability examples** demonstrating queries, mutations, dynamic queries, subscriptions, schema introspection, and multi-operation documents
- **Enhanced routing middleware** to support GraphQL agent type across all routing components
- **Zero endpoint changes required** - existing GraphQL services work as-is through YAML configuration

#### Enhanced Parameter Substitution System
- **Pure placeholder replacement** - When a string contains only `{{variable}}`, it's replaced with the actual JSON value (preserving data types)
- **Complex JSON structure support** - Arrays and objects are properly preserved during substitution
- **Mixed content substitution** - Strings with multiple placeholders work correctly
- **Improved type preservation** - Booleans, numbers, arrays, and objects maintain their JSON types

### Changed - Documentation & System Updates
- **Agent count updated** from eight to nine agent types across all documentation
- **Test count updated** from 102 to 117 total tests
- **Documentation consistency** - All files (README.md, DEVELOPMENT.md, TODO.md, capabilities/testing/README.md) updated
- **Enhanced routing system** to handle nine agent types with GraphQL support

### Technical Details
- **Added dependency**: `graphql_client = "0.13"` for GraphQL support
- **Extended AgentType enum** with GraphQL variant including all configuration options
- **Enhanced routing system** to parse and execute GraphQL configurations
- **Updated timeout configuration** to support GraphQL agent timeouts
- **Added GraphQL support** to middleware and retry systems

### Files Added
- `capabilities/graphql/graphql_services.yaml` - Comprehensive GraphQL capability examples
- `tests/graphql_agent_tests.rs` - Complete GraphQL agent test suite (8 tests)

### Files Modified
- `src/routing/agent_router.rs` - Added GraphQL agent parsing and execution logic
- `src/routing/types.rs` - Extended AgentType enum with GraphQL variant
- `src/routing/enhanced_router.rs` - Added GraphQL timeout support
- `Cargo.toml` - Added GraphQL client dependency and version bump to 0.2.3
- All documentation files updated for nine-agent consistency

## [0.2.2]

### Added - New Features

#### SSE Agent Routing Implementation ✅ **COMPLETE**
- **Added SSE agent type** to routing system for exposing Server-Sent Events endpoints as MCP tools
- **Implemented SSE client** with event stream handling and mock implementation for testing
- **Added comprehensive configuration parsing** for SSE endpoints (url, headers, timeout, max_events, event_filter)
- **Implemented parameter substitution** for SSE requests with support for dynamic URL, headers, and event filtering
- **Added SSE event filtering and aggregation** with configurable maximum event limits
- **Created comprehensive test suite** with 5 test cases covering parsing, validation, execution, and parameter substitution
- **Added SSE capability examples** demonstrating notification streams, stock prices, chat messages, system monitoring, and log streams
- **Enhanced routing middleware** to support SSE agent type across all routing components
- **Zero endpoint changes required** - existing SSE services work as-is through YAML configuration

## [0.2.1]

### Added - New Features

#### gRPC Agent Routing Implementation ✅ **COMPLETE**
- **Added gRPC agent type** to routing system for exposing gRPC endpoints as MCP tools
- **Implemented gRPC client** using tonic for connecting to external gRPC services
- **Added comprehensive configuration parsing** for gRPC endpoints (endpoint, service, method, headers, timeout, request_body)
- **Implemented parameter substitution** for gRPC requests with support for dynamic endpoint, headers, and request body templating
- **Added gRPC request/response handling** with mock implementation for testing (production requires proper protobuf definitions)
- **Created comprehensive test suite** with 5 test cases covering parsing, validation, execution, and parameter substitution
- **Added gRPC capability examples** demonstrating user service, order service, notification service, and health check patterns
- **Enhanced routing middleware** to support gRPC agent type across all routing components
- **Zero endpoint changes required** - existing gRPC services work as-is through YAML configuration

### Changed - Documentation & Organization

#### Enterprise Features Consolidation
- **Consolidated scattered Enterprise features** from Core phases into dedicated Enterprise sections
- **Removed Enterprise subsections** from Core phases (2.2.1, 2.3.1, 2.4.1, 3.1.1, 3.2.1) to maintain focus on essential functionality
- **Reorganized Enterprise Phase A**: Added Enterprise Routing & Performance (EA.3) and Enterprise Configuration Management (EA.4)
- **Enhanced Enterprise Phase B**: Added Enterprise MCP Integration & Architecture (EB.2) consolidating advanced MCP and routing features
- **Expanded Enterprise Phase C**: Added Advanced MCP Features & AI Integration (EC.1) with comprehensive prompt template and AI capabilities
- **Improved roadmap clarity** with clear separation between Core (essential) and Enterprise (advanced commercial) features
- **Better navigation** for users to distinguish between basic and enterprise-level functionality
- **Maintained logical grouping** of related Enterprise features by domain (Production, Integration, AI/Ecosystem)

#### Documentation Updates for gRPC Support
- **Updated all documentation** to reflect eight agent types (added gRPC, SSE, Database, MCP Proxy)
- **Enhanced README.md** with gRPC agent routing and updated test counts
- **Expanded DEVELOPMENT.md** with gRPC configuration examples and implementation details
- **Updated MCP_PROXY_DESIGN.md** with gRPC service examples and routing patterns
- **Enhanced capability documentation** to showcase gRPC integration patterns

## [0.2.0]

### Added - Phase 3.1: MCP Server Proxy Integration ✅ **MAJOR RELEASE**

#### MCP Client Implementation (`src/mcp/client.rs`)
- **Full MCP client implementation** with WebSocket and stdio transport support
- **Connection lifecycle management** with automatic reconnection and health monitoring
- **JSON-RPC 2.0 protocol compliance** with proper request/response handling
- **Connection state tracking** (Disconnected, Connecting, Connected, Reconnecting, Failed)
- **Timeout management** and comprehensive error handling
- **Tool execution** through remote servers with proper error propagation

#### MCP Server Registry (`src/mcp/discovery.rs`)
- **Multi-server management** for registering and discovering MCP servers
- **Server configuration validation** with endpoint and transport validation
- **Real-time connection status tracking** with health monitoring
- **Bulk operations** for connecting/disconnecting multiple servers
- **Server metadata management** with registration timestamps and status history

#### Tool Mapping System (`src/mcp/mapping.rs`)
- **Intelligent tool name mapping** between local and remote tool names
- **Automatic conflict resolution** with configurable strategies
- **Auto-generation of local names** with server-specific prefixes
- **Manual mapping rules** with comprehensive validation
- **Tool aggregation** from multiple servers with namespace management
- **Bidirectional mapping** (local-to-remote and remote-to-local)

#### MCP Proxy Manager (`src/mcp/proxy.rs`)
- **Centralized proxy coordination** managing all MCP proxy operations
- **Automated tool discovery** and registration from connected servers
- **Health checking** and connection management across all servers
- **Comprehensive status reporting** with detailed metrics and diagnostics
- **Tool aggregation** with unified interface for local and remote tools

#### Hybrid Tool Resolver (`src/mcp/hybrid_resolver.rs`)
- **Seamless integration** of local registry and remote MCP server tools
- **Intelligent tool resolution** with conflict detection and handling
- **Performance optimization** with caching and efficient tool lookup
- **Source metadata tracking** for tool provenance and debugging
- **Unified tool execution** routing to appropriate local or remote sources

#### Advanced Conflict Resolution (`src/routing/conflict_resolution.rs`)
- **Multiple resolution strategies**:
  - **LocalFirst**: Prioritize local tools over remote tools
  - **ProxyFirst**: Prioritize remote tools over local tools
  - **Prefix**: Add server prefixes to avoid naming conflicts
  - **Reject**: Reject conflicting tools with clear error messages
  - **FirstFound**: Use the first tool discovered
- **Configurable resolution policies** with per-tool customization
- **Conflict tracking and reporting** with detailed conflict information
- **Order preservation** for deterministic tool resolution

#### Enhanced MCP Server Integration
- **Updated McpServer** to include proxy manager and hybrid resolver
- **Unified tool listing** seamlessly combining local and remote tools
- **Transparent tool execution** with automatic routing to appropriate sources
- **Enhanced capability reporting** including proxy server capabilities
- **Backward compatibility** maintaining existing local tool functionality

#### Comprehensive Testing & Validation
- **76 unit tests** passing for core functionality validation
- **61 integration tests** across multiple test suites
- **Test coverage** for data structures, hybrid routing, conflict resolution
- **MCP server integration tests** with real-world scenarios
- **Configuration validation tests** ensuring robust setup
- **Performance and reliability testing** for production readiness

### Added - Phase 2.5: Testing & Quality Assurance (Previous)

#### Security Validation
- **Input sanitization** and attack prevention (9 tests)
- **SQL injection** prevention testing
- **XSS attack** prevention validation
- **Path traversal** attack prevention
- **Command injection** prevention testing

#### Configuration Validation
- **Comprehensive config validation** system (7 tests)
- **Environment variable** override testing
- **Security validation** for all configuration fields
- **Error handling** for invalid configurations

#### Test Coverage Analysis
- **Coverage reporting** and metrics (4 tests)
- **Test success rate** tracking (98.6% success rate)
- **Performance benchmarking** tools
- **Quality assurance** metrics

### Added - Phase 2.4: Configuration Validation System (Previous)

#### Validation Framework
- **Comprehensive validation** for all configuration fields
- **Environment variable** override support
- **Security validation** with input sanitization
- **Error reporting** with detailed validation messages

### Added - Phase 2.3: MCP Core Features (Previous)

#### MCP Resource Management
- **Read-only resource system** with URI validation
- **Provider architecture** for extensible resource types
- **File resource provider** with secure path handling
- **Resource metadata** and content management

#### MCP Prompt Templates
- **Template management** with argument substitution
- **Validation system** for prompt templates
- **Provider architecture** for multiple template sources
- **In-memory provider** for dynamic templates

#### MCP Logging System
- **RFC 5424 syslog compliance** with 8 severity levels
- **Rate limiting** (100 messages/minute per logger)
- **Thread-safe concurrent logging** with Arc<RwLock<T>>
- **Structured logging** with JSON formatting
- **Dynamic log level control** via HTTP endpoints

#### MCP Notifications
- **Event-driven notification system** with broadcast channels
- **Resource subscriptions** with URI-based filtering
- **List changed notifications** for resources and prompts
- **Custom notifications** and server status events
- **Statistics tracking** and subscription metrics

### Added - Phase 2.0: Agent Routing System (Previous)

#### Multi-Agent Support
- **Subprocess Agent**: Local command execution with environment control
- **HTTP Agent**: REST API calls with retry logic and authentication
- **LLM Agent**: Integration with OpenAI, Anthropic, and other AI services
- **WebSocket Agent**: Real-time bidirectional communication

#### Advanced Parameter Substitution
- **Handlebars-style templating** with conditionals and loops
- **Environment variable** substitution
- **Dynamic configuration** based on input parameters
- **Complex data transformation** capabilities

#### Production Features
- **Comprehensive error handling** with timeout management
- **Retry logic** with configurable backoff strategies
- **Router integration** into main MCP server
- **Testing capabilities** with four comprehensive capability files

### Added - Phase 1: Foundation & MVP (Previous)

#### MCP Server Implementation
- **WebSocket support** for real-time communication
- **Server-Sent Events** for legacy streaming
- **HTTP streaming** for progressive results
- **gRPC streaming** for high-performance binary communication

#### Capability Registry
- **Flexible YAML-based** configuration
- **Hot-reload capability** with file watching
- **Pattern matching** for capability discovery
- **Validation system** for capability definitions

#### Core Infrastructure
- **Rust-based implementation** with async/await
- **Comprehensive testing** with 233+ tests
- **Docker support** for containerized deployment
- **Kubernetes manifests** for orchestrated deployment

## [0.1.0]

### Added
- Initial project setup and foundation
- Basic MCP server implementation
- Capability registry system
- Core infrastructure and testing framework

---

## Version History Summary

- **v3.2.0** (2024-12-19): Phase 3.1 MCP Server Proxy Integration - Complete external server connectivity
- **v0.1.0** (2024-11-01): Initial project setup and foundation

## Upgrade Guide

### From v0.1.0 to v0.2.0

1. **Update configuration** to include MCP proxy settings (optional)
2. **Review new capabilities** for external MCP server connectivity
3. **Update dependencies** if using the library programmatically
4. **Test existing functionality** to ensure backward compatibility

### Configuration Changes

- Added `mcp_proxy` section for proxy configuration
- Added `mcp_servers` section for external server definitions
- Added `tool_mappings` section for custom tool name mappings
- Added `conflict_resolution` section for handling tool name conflicts
- Added `hybrid_routing` section for local/remote tool integration

### API Changes

- **New MCP proxy APIs** for server management and discovery
- **Enhanced tool listing** seamlessly combining local and remote tools
- **New status endpoints** for comprehensive proxy monitoring
- **Hybrid tool execution** with automatic routing to appropriate sources
- **Conflict resolution APIs** for managing tool name conflicts

### Breaking Changes

- **None** - Full backward compatibility maintained
- All existing local tool functionality preserved
- Configuration is backward compatible with optional proxy settings

---

## Contributing

See [README.md](README.md) for comprehensive development guidelines, detailed architecture, and project overview.
