# MagicTunnel - Current Tasks & Future Roadmap

This document outlines current tasks and future development plans for MagicTunnel. For completed work and achievements, see [TODO_DONE.md](TODO_DONE.md).

## 🚀 Current Status

**MagicTunnel v0.3.5** - **Production Ready** with full MCP 2025-06-18 compliance and comprehensive test coverage

### ✅ Major Achievements Complete
- **MCP 2025-06-18 Full Compliance** - All specification requirements implemented
- **Enterprise-Grade Smart Discovery** - AI-powered tool discovery with sub-second responses
- **Comprehensive Web Dashboard** - Full management and monitoring interface
- **Network Protocol Gateway** - Multi-protocol MCP service integration
- **Advanced Security Framework** - Enterprise-grade security and access control
- **Complete OAuth 2.1 Authentication** - Full OAuth 2.1 with PKCE, Resource Indicators, and multi-provider support

📚 **[View Complete Achievement History](TODO_DONE.md)** - Detailed archive of all completed work

---

## 🔥 Current High Priority Tasks

### 1. OpenAPI Capability Generation Completion ⚠️ **IN PROGRESS**
**Status**: Partially complete - needs final implementation

**Remaining Work**:
- [ ] Complete OpenAPI 3.x specification parser
- [ ] Implement comprehensive schema-to-MCP mapping
- [ ] Add support for OpenAPI authentication schemes
- [ ] Integrate with unified CLI generator
- [ ] Add comprehensive test coverage

**Impact**: Complete API-to-tool generation pipeline for all major API formats

### 2. LLM Services Backend API Completion 🚨 **CRITICAL - BLOCKING**
**Status**: Missing generation APIs for prompts and resources - blocks UI development

**Critical Missing Endpoints**:
- [ ] **Prompt Generation APIs** (`src/web/dashboard.rs`)
  - [ ] `POST /dashboard/api/prompts/service/tools/{tool_name}/generate` - Generate prompts with LLM
  - [ ] `GET /dashboard/api/prompts/service/health` - Prompt generation service health
  - [ ] `POST /dashboard/api/prompts/service/batch/generate` - Batch prompt generation
- [ ] **Resource Generation APIs** (`src/web/dashboard.rs`)
  - [ ] `POST /dashboard/api/resources/service/tools/{tool_name}/generate` - Generate resources with LLM
  - [ ] `GET /dashboard/api/resources/service/health` - Resource generation service health
  - [ ] `POST /dashboard/api/resources/service/batch/generate` - Batch resource generation

**Implementation Required**:
- [ ] Add the 6 missing generation API endpoints to `src/web/dashboard.rs`
- [ ] Create handlers that call the LLM services for prompt/resource generation
- [ ] Add service health checks for prompt and resource generation
- [ ] Implement batch processing capabilities
- [ ] Add external MCP server content protection (similar to sampling/elicitation)

**Impact**: **BLOCKING** - Frontend LLM services UI cannot be implemented until these APIs exist

### 3. Frontend Accessibility Improvements 🎯 **HIGH PRIORITY**
**Status**: Required for production compliance

**Tasks**:
- [ ] Fix AlertsPanel component accessibility issues
- [ ] Add ARIA labels and roles throughout dashboard
- [ ] Implement keyboard navigation support
- [ ] Add screen reader compatibility
- [ ] Create accessibility testing pipeline
- [ ] WCAG 2.1 compliance validation

**Impact**: Ensure dashboard meets accessibility standards and regulations

### 4. LLM Services Frontend UI Implementation 🎨 **HIGH PRIORITY**
**Status**: Complete frontend UI implementation needed - 0% complete

**Dependencies**: Blocked by Task 2 (Missing Generation APIs)

**UI Components Needed**:
- [ ] **Provider Management UI** (`frontend/src/routes/llm-services/providers/+page.svelte`)
  - [ ] Provider health status dashboard (OpenAI, Anthropic, Ollama)
  - [ ] API key configuration interface
  - [ ] Provider testing and validation
  - [ ] Performance metrics display
  - [ ] Provider selection preferences

- [ ] **Sampling Service UI** (`frontend/src/routes/llm-services/sampling/+page.svelte`)
  - [ ] Tool enhancement interface (individual and batch)
  - [ ] Sampling request configuration
  - [ ] Content generation preview
  - [ ] Enhancement queue management
  - [ ] Result validation and approval workflow

- [ ] **Elicitation Service UI** (`frontend/src/routes/llm-services/elicitation/+page.svelte`)
  - [ ] Metadata extraction interface
  - [ ] Schema analysis tools
  - [ ] Parameter validation setup
  - [ ] Batch elicitation processing
  - [ ] Extracted metadata review

- [ ] **Prompt Management UI** (`frontend/src/routes/llm-services/prompts/+page.svelte`)
  - [ ] Prompt generation interface (requires missing APIs)
  - [ ] Prompt library browser
  - [ ] Template management system
  - [ ] Version control for prompts
  - [ ] Quality scoring and validation

- [ ] **Resource Management UI** (`frontend/src/routes/llm-services/resources/+page.svelte`)
  - [ ] Resource generation interface (requires missing APIs)
  - [ ] Resource browser with filtering
  - [ ] Content validation tools
  - [ ] External MCP resource fetching
  - [ ] Resource versioning system

- [ ] **Enhancement Pipeline UI** (`frontend/src/routes/llm-services/enhancements/+page.svelte`)
  - [ ] Pipeline status dashboard
  - [ ] Job queue management
  - [ ] Batch processing controls
  - [ ] Progress tracking
  - [ ] Error handling and retry mechanisms

**Navigation Updates Needed**:
- [ ] Add LLM Services section to main navigation (`frontend/src/lib/navigation.ts`)
- [ ] Create nested navigation for all 6 service pages
- [ ] Add appropriate icons and routing

**Advanced Features**:
- [ ] **Discovery Testing Interface** - Interactive tool discovery testing
- [ ] **Analytics Dashboard** - LLM usage metrics and enhancement success rates
- [ ] **Configuration Management** - Visual configuration editor

**Technical Requirements**:
- [ ] **SvelteKit Framework** integration (matches existing dashboard)
- [ ] **TypeScript** data models for LLM services
- [ ] **RESTful API** integration with dashboard endpoints
- [ ] **Real-time updates** via WebSocket/SSE
- [ ] **Error handling** and retry logic
- [ ] **Loading states** and progress indicators

**Estimated Effort**: 2-3 weeks after API dependencies resolved

**Impact**: Complete LLM services management interface for enhanced tool discovery system

### 5. Enhanced Versioning System 📋 **NEW - MEDIUM PRIORITY**
**Status**: Currently has basic versioning, needs comprehensive version management

**Objective**: Implement robust versioning for prompts, resources, and capability files

**Current State**:
- ✅ Capability YAML files have basic versioning (semver + MCP protocol version)
- ✅ Content storage has rudimentary version fields in StorageMetadata
- ❌ MCP Resource/PromptTemplate types lack version fields
- ❌ No automatic version management or migration support

**Tasks**:
- [ ] **Enhanced Resource/Prompt Versioning**
  - [ ] Add version fields to base MCP Resource and PromptTemplate types
  - [ ] Implement VersionedResource and VersionedPrompt wrapper types
  - [ ] Add schema_version field for evolution tracking
  - [ ] Include created_at/updated_at timestamps

- [ ] **Version Management Service**
  - [ ] Create centralized version tracking service
  - [ ] Implement automatic version increment on content changes
  - [ ] Add version conflict detection and resolution
  - [ ] Build version history maintenance system

- [ ] **Migration Framework**
  - [ ] Design schema evolution support system
  - [ ] Implement backward compatibility validation
  - [ ] Create migration path definitions and automation
  - [ ] Add version upgrade/downgrade capabilities

- [ ] **Version Control Integration**
  - [ ] Add content change detection algorithms
  - [ ] Implement version branching for parallel development
  - [ ] Create version tagging and release management
  - [ ] Build diff and merge capabilities for versioned content

**Expected Impact**: Robust version management across all MagicTunnel content types with migration support

### 6. MCP 2025-06-18 User Consent System Implementation 🔐 **FUTURE - REQUIRES DESIGN CLARIFICATION**
**Status**: ❌ REMOVED (August 5, 2025) - All consent implementation removed due to design confusion

**Background**: 
- Previous consent implementation was unclear about when/why consent is needed
- Confusion between consent for data processing vs. consent for showing requests to users
- Consent blocking operations incorrectly instead of checking at data processing layer
- Complete removal performed to establish clean foundation for future implementation

**Design Questions to Resolve** (Before Implementation):
1. **Consent Purpose Clarification**:
   - Is consent for external data processing (sending data to OpenAI/Anthropic APIs)?
   - Or is consent for showing elicitation/sampling requests to users?
   - When exactly should consent be required vs. optional?

2. **Data Processing vs Request Consent**:
   - **Sampling Service**: Uses external LLM APIs - needs data processing consent for API calls
   - **Elicitation Service**: Local data collection only - unclear if consent needed
   - Should consent be checked at API call level, not at service operation level?

3. **User Experience Considerations**:
   - Can't users choose to respond or not respond to elicitation/sampling requests?
   - When is explicit consent UI necessary vs. implicit through user interaction?
   - How to balance security with usability?

**Future Implementation Requirements** (After Design Clarification):
- [ ] **Consent System Architecture Design**
  - [ ] Define clear consent scope (data processing vs. operation approval)
  - [ ] Design consent flow for external LLM service data transmission
  - [ ] Determine if elicitation operations require consent (no external APIs used)
  - [ ] Create consent level hierarchy (None/Basic/Explicit/Informed)

- [ ] **Data Processing Consent Implementation**
  - [ ] Add consent checks before external API calls (OpenAI, Anthropic, Ollama)
  - [ ] Implement consent UI for LLM data transmission approval
  - [ ] Create consent caching with appropriate TTL for user convenience
  - [ ] Add consent timeout handling and fallback behavior

- [ ] **Consent Configuration System**
  - [ ] Design consent presets (global_once, session_based, tool_based, operation_based)
  - [ ] Implement consent modes for external server interactions (proxy, supplement, override)
  - [ ] Add consent audit logging for compliance and debugging
  - [ ] Create consent policy management for different data sensitivity levels

- [ ] **User Interface Components**
  - [ ] Design consent request UI components
  - [ ] Create consent history and management interface
  - [ ] Implement consent preference settings
  - [ ] Add consent status indicators and controls

**Files Previously Removed** (August 5, 2025):
- `src/security/consent.rs` - Complete consent engine implementation
- Consent-related structs from `src/security/config.rs` (ConsentLevel, ConsentPreset, ConsentMode, etc.)
- Consent integration from `src/mcp/sampling.rs` and `src/mcp/elicitation.rs`
- ConsentRequired error codes from sampling and elicitation types

**Impact**: Clean foundation established - requires design clarification before re-implementation

### 7. MCP 2025-06-18 Security Configuration System 🔐 **FUTURE - DESIGN CLARIFICATION NEEDED**
**Status**: ❌ REMOVED (August 5, 2025) - All MCP 2025 security configuration removed due to no implementation

**Background**:
- Complex security configuration structures existed (`Mcp2025SecurityConfig`, `McpCapabilityPermissionsConfig`, etc.) but were never actually used
- OAuth implementation uses its own separate `ResourceIndicatorsConfig`, not `McpResourceIndicatorsConfig`
- No MCP services (sampling, elicitation, roots) reference or check these security configurations
- Configuration without implementation - same problem as consent system

**Removed Configuration Structures** (August 5, 2025):
- `Mcp2025SecurityConfig` - Main MCP security configuration container
- `McpCapabilityPermissionsConfig` - Permission controls for sampling/elicitation capabilities
- `McpToolApprovalConfig` - Tool execution approval workflows
- `McpOAuth21Config` - OAuth 2.1 enhancements (duplicated existing OAuth config)
- `McpResourceIndicatorsConfig` - Resource indicators (duplicated existing OAuth config)
- `PermissionBehavior` enum - Permission behavior types (Allow/Deny/Ask)
- `ApprovalNotificationMethod` enum - Approval notification methods

**Design Questions to Resolve** (Before Implementation):
1. **Security vs Configuration Duplication**:
   - OAuth already has complete Resource Indicators implementation - why duplicate?
   - Should MCP security extend existing OAuth/RBAC systems or replace them?
   - What specific MCP 2025-06-18 security features are actually required?

2. **Implementation vs Configuration**:
   - Is complex security configuration needed if no services use it?
   - Should security be enforced at middleware level vs. service level?
   - What's the right balance between configurability and simplicity?

3. **Actual MCP 2025-06-18 Requirements**:
   - Which parts of MCP 2025-06-18 spec actually require security configuration?
   - Are there specific compliance requirements that need implementation?
   - Can existing security systems (OAuth, RBAC, allowlisting) cover MCP requirements?

**Future Implementation Options** (After Design Clarification):
- **Option 1: Extend Existing Systems** - Enhance OAuth/RBAC to cover MCP requirements
- **Option 2: MCP-Specific Security Layer** - Build targeted MCP security without duplication
- **Option 3: Minimal Implementation** - Only implement actually required MCP security features

**Files Previously Removed** (August 5, 2025):
- MCP 2025 security structs from `src/security/config.rs` (~200 lines)
- MCP 2025 exports from `src/security/mod.rs`
- MCP 2025 references from `src/bin/magictunnel-security.rs`

**Impact**: Eliminated configuration bloat - requires actual requirements analysis before re-implementation


### 5.1 Remove Non-MCP Security System ✅ **COMPLETED - August 4, 2025**
**Status**: ✅ COMPLETE - All non-MCP security system removed

**Objective**: ✅ ACHIEVED - Completely removed the non-MCP elicitation security system that created confusion

**Completed Tasks**:
- [x] **✅ Disable Non-MCP Security** - Set `enabled: false` in both ElicitationConfig instances
- [x] **✅ Remove ElicitationSecurityConfig struct** - Complete removal from codebase (`src/mcp/elicitation.rs`)
- [x] **✅ Remove security validation logic** - Removed all blocked_schema_patterns and blocked_field_names checks
- [x] **✅ Remove ElicitationPrivacyLevel restrictions** - Removed min_privacy_level enforcement
- [x] **✅ Update ElicitationConfig struct** - Removed security field entirely from configuration
- [x] **✅ Clean up security-related imports and dependencies** - Removed regex patterns and privacy checks
- [x] **✅ Update tests** - Removed security-related test cases (`test_security.rs`, `test_mcp_security_default.rs`)
- [x] **✅ Fix compilation issues** - Fixed missing `json!` macro imports and dependencies

**Impact**: ✅ ACHIEVED - Clean foundation established for proper MCP 2025-06-18 security implementation without interference

### 7. Client Capability Tracking & Elicitation Routing ✅ **COMPLETED - v0.3.7**
**Status**: ✅ COMPLETE - Full MCP 2025-06-18 client capability tracking implemented

**Completed Implementation**:

- [x] **✅ Enhanced Session Management** (`src/mcp/session.rs`)
  - [x] Added `client_capabilities: Option<ClientCapabilities>` to `ClientInfo` struct
  - [x] Created `ClientCapabilities` struct with sampling/elicitation support flags
  - [x] Updated `extract_client_info()` to parse capabilities from initialize request
  - [x] Store client capabilities in session for routing decisions

- [x] **✅ Client Capability Types** (`src/mcp/types/capabilities.rs` - NEW FILE)
  - [x] Defined `ClientCapabilities` struct matching MCP spec
  - [x] Defined `ElicitationCapability` with create/accept/reject/cancel flags
  - [x] Defined `SamplingCapability` with relevant flags
  - [x] Added serialization/deserialization support

- [x] **✅ Capability-Based Routing Logic** (`src/mcp/server.rs`)
  - [x] Updated `check_any_client_supports_elicitation()` to check client capabilities
  - [x] Implemented session iteration methods for capability checking
  - [x] Added capability validation before forwarding requests
  - [x] Enhanced error handling when clients lack elicitation capability

- [x] **✅ Transport Integration Verification** 
  - [x] Verified capability tracking works across stdio (Claude Desktop, Cursor)
  - [x] Verified capability tracking works across WebSocket connections
  - [x] Verified capability tracking works across HTTP connections 
  - [x] Verified capability tracking works across Streamable HTTP connections

- [x] **✅ Enhanced Tool Discovery Logic Fix** (`src/discovery/enhancement.rs`)
  - [x] Fixed tool discovery elicitation to only work when smart discovery is disabled
  - [x] Added smart discovery state tracking to enhancement pipeline
  - [x] Enhanced logic in `should_use_local_elicitation()` method
  - [x] Updated all call sites to pass smart discovery enabled state

**Impact**: ✅ ACHIEVED - Complete MCP 2025-06-18 compliance with capability-aware routing and logical elicitation behavior

### 8. Development Tools Integration 🛠️ **NEW - HIGH VALUE**
**Status**: Newly identified enhancement opportunity

**Objective**: Integrate development tools into existing workflows for automatic execution

**Tasks**:
- [ ] **Validation Tools Integration**
  - [ ] Add to `cargo test` as integration tests
  - [ ] Create pre-commit hooks for YAML validation
  - [ ] Integrate into CI/CD pipeline
  - [ ] Add `make validate` and `make validate-strict` targets
  - [ ] Setup file watchers for auto-validation

- [ ] **Migration Tools Integration**
  - [ ] Auto-migration during startup for legacy files
  - [ ] Version upgrade workflow integration
  - [ ] CI/CD auto-migration with PR creation
  - [ ] Development environment setup automation

- [ ] **Testing Tools Integration**
  - [ ] Continuous semantic search validation in `cargo test`
  - [ ] Performance benchmarking integration
  - [ ] Health check automation for production monitoring
  - [ ] API testing pipeline integration

- [ ] **Release Tools Integration**
  - [ ] Automated release pipeline integration
  - [ ] `make release VERSION=x.y.z` command creation
  - [ ] CI/CD release workflow automation
  - [ ] Git hooks for version management

**Integration Patterns**:
```bash
# Proposed make targets
make validate          # Run YAML validation
make migrate           # Migrate legacy files  
make test-semantic     # Run semantic search tests
make release VERSION=  # Full release workflow
```

**Impact**: Transform standalone scripts into integrated development workflow tools

---

## 🎯 Phase 4: Registry & OAuth2 Integration (Medium Priority)

### 4.1 MCP Registry Integration 📋 **FUTURE**
**Objective**: App Store experience for MCP servers

**Tasks**:
- [ ] **Core Registry Infrastructure**
  - [ ] Registry API client implementation
  - [ ] Server metadata and rating system
  - [ ] Dependency resolution system

- [ ] **Server Discovery & Search**
  - [ ] Visual server browser in dashboard
  - [ ] Advanced search and filtering
  - [ ] Category-based organization

- [ ] **One-Click Installation**
  - [ ] Automated server setup and configuration
  - [ ] Dependency installation automation
  - [ ] Configuration template system

**Expected Impact**: Reduce MCP server setup from 2-3 hours to 30 seconds

### 4.2 OAuth2 UI Integration 🔐 **MEDIUM PRIORITY**
**Objective**: Complete OAuth2 system with web dashboard management

**Current State**:
- ✅ **OAuth 2.1 Backend Complete** - Full implementation with PKCE, Resource Indicators, multi-provider support
- ✅ **Web API Endpoints** - Authorization, callback, and token validation endpoints
- ✅ **Configuration System** - Complete OAuth configuration framework
- ❌ **Dashboard UI Missing** - No web interface for OAuth management

**Remaining Tasks**:
- [ ] **OAuth Management UI**
  - [ ] OAuth provider configuration interface in dashboard
  - [ ] Token status and management interface
  - [ ] User OAuth session management
  - [ ] OAuth provider health monitoring UI

- [ ] **Enhanced Features**
  - [ ] Automatic token refresh implementation
  - [ ] OAuth provider registration wizard
  - [ ] Session timeout and management
  - [ ] OAuth audit logging and monitoring

- [ ] **Integration Improvements**
  - [ ] Single Sign-On (SSO) integration for dashboard access
  - [ ] OAuth-based MCP client authentication
  - [ ] Provider-specific optimization settings

**Expected Impact**: Complete OAuth2 management experience with web interface

---

## 🚀 Phase 5: Open Source & Community (Low Priority)

### 5.1 Open Source Preparation 📂 **FUTURE**
- [ ] License compliance review
- [ ] Security audit and cleanup
- [ ] Community contribution guidelines
- [ ] Release packaging and distribution

### 5.2 Community & Marketing Launch 📢 **FUTURE**
- [ ] Documentation website creation
- [ ] Video tutorials and demos
- [ ] Community discord/forum setup
- [ ] Blog posts and technical articles

---

## 🏢 Enterprise Phase: Advanced Features (Future)

### SaaS Service Features ❌ **FUTURE CONSIDERATION**
**Status**: Not currently planned for core product

**Potential Features** (if SaaS direction chosen):
- [ ] Multi-tenant architecture
- [ ] User management and authentication
- [ ] Billing and subscription management
- [ ] Customer experience features
- [ ] SaaS operations and monitoring
- [ ] Compliance and security frameworks
- [ ] SaaS APIs and integration

**Note**: These features would be for a hosted SaaS version of MagicTunnel, not the core open-source product.

---

## 📊 Success Metrics & Targets

### Current Version (v0.3.5) Targets
- ✅ **MCP 2025-06-23 Compliance**: 100% specification compliance
- ✅ **Performance**: Sub-second tool discovery responses
- ✅ **Reliability**: 99.9% uptime with graceful degradation
- ✅ **OAuth 2.1 Authentication**: Complete backend implementation
- 🎯 **Accessibility**: WCAG 2.1 AA compliance (in progress)
- 🎯 **Development Experience**: Integrated tool workflows (planned)

### Next Version (v0.4.0) Targets
- 🎯 **API Coverage**: 100% OpenAPI generation support
- 🎯 **Developer Productivity**: 50% reduction in development workflow time
- 🎯 **Tool Discovery**: 95% accuracy in natural language tool matching
- 🎯 **Registry Integration**: Basic MCP registry support

---

## 🚨 Risk Assessment

### Technical Risks
- **OpenAPI Complexity**: OpenAPI 3.x specification has many edge cases
  - *Mitigation*: Incremental implementation with comprehensive testing
- **Accessibility Standards**: WCAG compliance can be complex
  - *Mitigation*: Use established accessibility libraries and automated testing

### Resource Risks
- **Development Bandwidth**: Limited development resources for multiple features
  - *Mitigation*: Prioritize high-impact features and defer nice-to-have items

---

## 🔄 Development Workflow

### 1. Current Sprint Focus
1. **OpenAPI Generation Completion** (2-3 weeks)
2. **LLM Services Backend API Completion** (1 week) - **CRITICAL BLOCKING**
3. **LLM Services Frontend UI Implementation** (2-3 weeks after APIs)
4. **Accessibility Improvements** (1-2 weeks)

### 2. Next Sprint Planning
1. **Versioning System Implementation** (2-3 weeks)
2. **Registry Integration Planning** (research phase)
3. **OAuth2 UI Integration** (dashboard interface development)
4. **Community Preparation** (documentation and packaging)

### 3. Review and Adjustment
- Monthly roadmap review and priority adjustment
- Quarterly strategic planning and goal setting
- User feedback integration and feature prioritization

---

## 📞 Get Involved

### Current Priorities Need Help With:
1. **OpenAPI Edge Cases** - Complex schema mapping scenarios
2. **Accessibility Testing** - Screen reader and keyboard navigation testing
3. **Integration Testing** - Real-world MCP client compatibility testing

### Future Opportunities:
1. **MCP Registry Design** - Community input on registry requirements
2. **OAuth2 Dashboard UI** - Web interface for OAuth management and monitoring
3. **Enhanced Versioning** - Advanced version control and migration systems
4. **Documentation Improvements** - User guides and tutorials

---

**Last Updated**: August 4, 2025
**Next Review**: September 2025

For detailed implementation history and completed features, see [TODO_DONE.md](TODO_DONE.md).
