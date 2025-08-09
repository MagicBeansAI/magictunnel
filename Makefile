# MCP Proxy Development Makefile

.PHONY: help build build-release test test-verbose test-agent-router test-data-structures test-streaming test-performance test-grpc test-integration test-mcp-server test-registry test-capabilities check fmt fmt-check clippy clippy-all clean run run-config run-release-openai run-release-env run-dev-env setup-env dev dev-check watch-test watch-run docs audit update install-tools

# Default target
help:
	@echo "MCP Proxy Development Commands:"
	@echo ""
	@echo "Build & Run:"
	@echo "  build                - Build the project"
	@echo "  build-release        - Build for release"
	@echo "  build-release-semantic - Build with semantic search env vars"
	@echo "  run                  - Run the proxy with default config"
	@echo "  run-config           - Run with config.yaml"
	@echo "  run-release-openai   - Run release with OpenAI API key"
	@echo "  run-release-env      - Run release mode with .env file support"
	@echo "  run-dev-env          - Run in dev mode with .env file support"
	@echo "  dev                  - Run in development mode with debug logging"
	@echo ""
	@echo "Smart Discovery Run Options (Real Semantic Search):"
	@echo "  run-release-ollama   - Ollama server (RECOMMENDED for local development)"
	@echo "  run-release-semantic - OpenAI embeddings (RECOMMENDED for production)"
	@echo "  run-release-external - Custom embedding API"
	@echo ""
	@echo "Smart Discovery Run Options (Hash Fallbacks - Testing Only):"
	@echo "  run-release-local    - Hash fallback (all-MiniLM-L6-v2)"
	@echo "  run-release-hq       - Hash fallback (all-mpnet-base-v2)"
	@echo ""
	@echo "Embedding Pre-generation (Real Semantic Embeddings):"
	@echo "  pregenerate-embeddings-ollama  - Ollama server (RECOMMENDED for local dev)"
	@echo "  pregenerate-embeddings-openai  - OpenAI API (RECOMMENDED for production)"
	@echo "  pregenerate-embeddings-external - Custom embedding API"
	@echo ""
	@echo "Embedding Pre-generation (Hash Fallbacks - Testing Only):"
	@echo "  pregenerate-embeddings-local   - Hash fallback (all-MiniLM-L6-v2)"
	@echo "  pregenerate-embeddings-hq      - Hash fallback (all-mpnet-base-v2)"
	@echo "  pregenerate-embeddings         - Uses config model (may be fallback)"
	@echo ""
	@echo "Environment Support:"
	@echo "  ENV=production make run-release-env  - Use .env.production"
	@echo "  ENV=staging make run-release-env     - Use .env.staging"
	@echo "  ENV=development make run-dev-env     - Use .env.development"
	@echo ""
	@echo "Testing:"
	@echo "  test                  - Run all tests"
	@echo "  test-verbose          - Run tests with output"
	@echo "  test-agent-router     - Run agent router tests"
	@echo "  test-data-structures  - Run data structure tests"
	@echo "  test-streaming        - Run streaming protocol tests"
	@echo "  test-performance      - Run performance tests"
	@echo "  test-grpc             - Run gRPC integration tests"
	@echo "  test-integration      - Run integration tests"
	@echo "  test-mcp-server       - Run MCP server tests"
	@echo "  test-registry         - Run registry service tests"
	@echo "  test-capabilities     - Run capability file validation"
	@echo ""
	@echo "Code Quality:"
	@echo "  check         - Run cargo check"
	@echo "  fmt           - Format code with rustfmt"
	@echo "  fmt-check     - Check code formatting"
	@echo "  clippy        - Run clippy lints"
	@echo "  clippy-all    - Run clippy with all features"
	@echo "  dev-check     - Run full development check (fmt + clippy + test + build)"
	@echo ""
	@echo "Development:"
	@echo "  watch-test    - Watch for changes and run tests"
	@echo "  watch-run     - Watch for changes and run proxy"
	@echo "  docs          - Generate and open documentation"
	@echo "  clean         - Clean build artifacts"
	@echo ""
	@echo "Maintenance:"
	@echo "  install-tools - Install development tools"
	@echo "  setup-env     - Set up .env file for development"
	@echo "  audit         - Check for security vulnerabilities"
	@echo "  update        - Update dependencies"

# Build the project
build:
	cargo build

# Build for release
build-release:
	export MAGICTUNNEL_ENV=development cargo build --release && sleep 2 && cp target/release/magictunnel . && cp target/release/magictunnel-supervisor .

# Update version across all files
update-version:
	cargo run --bin version-manager -- update

# Check version consistency
check-version:
	cargo run --bin version-manager -- check

# Set new version
set-version:
	@if [ -z "$(VERSION)" ]; then echo "Usage: make set-version VERSION=x.y.z"; exit 1; fi
	cargo run --bin version-manager -- set $(VERSION)

# Build for release with semantic search environment variables
build-release-ollama:
	MAGICTUNNEL_SEMANTIC_MODEL="ollama:nomic-embed-text" \
	MAGICTUNNEL_DISABLE_SEMANTIC="false" \
	OLLAMA_BASE_URL="http://localhost:11434" \
	cargo build --release && sleep 2 && cp target/release/magictunnel . && cp target/release/magictunnel-supervisor .

# Run all tests
test:
	cargo test

# Run tests with output
test-verbose:
	cargo test -- --nocapture

# Run cargo check
check:
	cargo check

# Format code
fmt:
	cargo fmt

# Check formatting
fmt-check:
	cargo fmt -- --check

# Run clippy
clippy:
	cargo clippy -- -D warnings

# Run clippy with all features
clippy-all:
	cargo clippy --all-features -- -D warnings

# Clean build artifacts
clean:
	cargo clean

# Run the proxy
run-release-wo-config:
	cargo run --bin magictunnel --release

# Run in development mode
run-dev-wo-config:
	cargo run --bin magictunnel -- --log-level debug

# Run in development mode
run-dev:
	cargo run --bin magictunnel -- --log-level debug --config magictunnel-config.yaml

# Run with custom config
run-release:
	MAGICTUNNEL_ENV=development \
	cargo run --bin magictunnel --release -- --config magictunnel-config.yaml --log-level info

# Run with OpenAI API key for smart discovery
run-release-openai:
	@if [ -z "$(OPENAI_API_KEY)" ]; then \
		echo "Error: OPENAI_API_KEY environment variable is not set"; \
		echo "Usage: make run-release-openai OPENAI_API_KEY=sk-your-key-here"; \
		exit 1; \
	fi
	OPENAI_API_KEY="$(OPENAI_API_KEY)" \
	cargo run --bin magictunnel -- --config magictunnel-config.yaml --log-level info

# Run with semantic search environment variables override (OpenAI)
run-release-semantic:
	@if [ -z "$(OPENAI_API_KEY)" ]; then \
		echo "Error: OPENAI_API_KEY environment variable is not set for OpenAI embeddings"; \
		echo "Usage: make run-release-semantic OPENAI_API_KEY=sk-your-key-here"; \
		exit 1; \
	fi
	@echo "🧠 Starting MagicTunnel with OpenAI semantic search..."
	@echo "   - Model: openai:text-embedding-3-small"
	@echo "   - Semantic search: enabled"
	OPENAI_API_KEY="$(OPENAI_API_KEY)" \
	MAGICTUNNEL_SEMANTIC_MODEL="openai:text-embedding-3-small" \
	MAGICTUNNEL_DISABLE_SEMANTIC="false" \
	cargo run --bin magictunnel --release -- --config magictunnel-config.yaml --log-level info

# Run with local transformer models (fallback embeddings - limited functionality)
run-release-local:
	@echo "⚠️  Starting MagicTunnel with fallback embeddings (LIMITED FUNCTIONALITY)..."
	@echo "   - Model: all-MiniLM-L6-v2 (384-dim, hash-based fallback)"
	@echo "   - WARNING: Uses deterministic fallback, not real semantic embeddings"
	@echo "   - RECOMMENDED: Use 'make run-release-ollama' instead for real embeddings"
	MAGICTUNNEL_SEMANTIC_MODEL="all-MiniLM-L6-v2" \
	MAGICTUNNEL_DISABLE_SEMANTIC="false" \
	cargo run --bin magictunnel --release -- --config magictunnel-config.yaml --log-level info

# Run with high-quality local model (fallback embeddings - limited functionality)
run-release-hq:
	@echo "⚠️  Starting MagicTunnel with fallback embeddings (LIMITED FUNCTIONALITY)..."
	@echo "   - Model: all-mpnet-base-v2 (768-dim, hash-based fallback)"
	@echo "   - WARNING: Uses deterministic fallback, not real semantic embeddings"
	@echo "   - RECOMMENDED: Use 'make run-release-ollama' instead for real embeddings"
	MAGICTUNNEL_SEMANTIC_MODEL="all-mpnet-base-v2" \
	MAGICTUNNEL_DISABLE_SEMANTIC="false" \
	cargo run --bin magictunnel --release -- --config magictunnel-config.yaml --log-level info

# Run with Ollama (local LLM server)
run-release-ollama:
	@echo "🧠 Starting MagicTunnel with Ollama..."
	@echo "   - Model: Uses your local Ollama server"
	@echo "   - Make sure Ollama is running with an embedding model!"
	OLLAMA_BASE_URL="http://localhost:11434" \
	MAGICTUNNEL_SEMANTIC_MODEL="ollama:nomic-embed-text" \
	MAGICTUNNEL_DISABLE_SEMANTIC="false" \
	cargo run --bin magictunnel --release -- --config magictunnel-config.yaml --log-level info

# Run with custom external API
run-release-external:
	@if [ -z "$(EMBEDDING_API_URL)" ]; then \
		echo "Error: EMBEDDING_API_URL environment variable is not set"; \
		echo "Usage: make run-release-external EMBEDDING_API_URL=http://your-server:8080"; \
		exit 1; \
	fi
	@echo "🧠 Starting MagicTunnel with external embedding API..."
	@echo "   - API URL: $(EMBEDDING_API_URL)"
	@echo "   - Custom embedding service"
	EMBEDDING_API_URL="$(EMBEDDING_API_URL)" \
	MAGICTUNNEL_SEMANTIC_MODEL="external:api" \
	MAGICTUNNEL_DISABLE_SEMANTIC="false" \
	cargo run --bin magictunnel --release -- --config magictunnel-config.yaml --log-level info

# Pre-generate embeddings for all enabled capabilities
pregenerate-embeddings:
	@if [ -z "$(OPENAI_API_KEY)" ]; then \
		echo "Error: OPENAI_API_KEY environment variable is not set for embedding generation"; \
		echo "Usage: make pregenerate-embeddings OPENAI_API_KEY=sk-your-key-here"; \
		exit 1; \
	fi
	@echo "🧠 Pre-generating embeddings for all enabled capabilities..."
	@echo "   - Model: configured in magictunnel-config.yaml"
	@echo "   - This will make server startup much faster!"
	OPENAI_API_KEY="$(OPENAI_API_KEY)" \
	cargo run --bin magictunnel --release -- --config magictunnel-config.yaml --log-level info --pregenerate-embeddings

# Pre-generate embeddings with OpenAI model override
pregenerate-embeddings-openai:
	@if [ -z "$(OPENAI_API_KEY)" ]; then \
		echo "Error: OPENAI_API_KEY environment variable is not set for OpenAI embeddings"; \
		echo "Usage: make pregenerate-embeddings-openai OPENAI_API_KEY=sk-your-key-here"; \
		exit 1; \
	fi
	@echo "🧠 Pre-generating embeddings with OpenAI model override..."
	@echo "   - Model: openai:text-embedding-3-small (environment override)"
	@echo "   - This will make server startup much faster!"
	OPENAI_API_KEY="$(OPENAI_API_KEY)" \
	MAGICTUNNEL_SEMANTIC_MODEL="openai:text-embedding-3-small" \
	MAGICTUNNEL_DISABLE_SEMANTIC="false" \
	cargo run --bin magictunnel --release -- --config magictunnel-config.yaml --log-level info --pregenerate-embeddings

# Pre-generate embeddings with fallback models (limited functionality)
pregenerate-embeddings-local:
	@echo "⚠️  Pre-generating fallback embeddings (LIMITED FUNCTIONALITY)..."
	@echo "   - Model: all-MiniLM-L6-v2 (384-dim, hash-based fallback)"
	@echo "   - WARNING: Uses deterministic fallback, not real semantic embeddings"
	@echo "   - RECOMMENDED: Use 'make pregenerate-embeddings-ollama' instead"
	MAGICTUNNEL_SEMANTIC_MODEL="all-MiniLM-L6-v2" \
	MAGICTUNNEL_DISABLE_SEMANTIC="false" \
	cargo run --bin magictunnel --release -- --config magictunnel-config.yaml --log-level info --pregenerate-embeddings

# Pre-generate embeddings with fallback models (limited functionality)
pregenerate-embeddings-hq:
	@echo "⚠️  Pre-generating fallback embeddings (LIMITED FUNCTIONALITY)..."
	@echo "   - Model: all-mpnet-base-v2 (768-dim, hash-based fallback)"
	@echo "   - WARNING: Uses deterministic fallback, not real semantic embeddings"
	@echo "   - RECOMMENDED: Use 'make pregenerate-embeddings-ollama' instead"
	MAGICTUNNEL_SEMANTIC_MODEL="all-mpnet-base-v2" \
	MAGICTUNNEL_DISABLE_SEMANTIC="false" \
	cargo run --bin magictunnel --release -- --config magictunnel-config.yaml --log-level info --pregenerate-embeddings

# Pre-generate embeddings with Ollama (local LLM server)
pregenerate-embeddings-ollama:
	@echo "🧠 Pre-generating embeddings with Ollama..."
	@echo "   - Model: Uses your local Ollama server"
	@echo "   - Make sure Ollama is running with an embedding model!"
	OLLAMA_BASE_URL="http://localhost:11434" \
	MAGICTUNNEL_SEMANTIC_MODEL="ollama:nomic-embed-text" \
	MAGICTUNNEL_DISABLE_SEMANTIC="false" \
	cargo run --bin magictunnel --release -- --config magictunnel-config.yaml --log-level info --pregenerate-embeddings

# Pre-generate embeddings with custom external API
pregenerate-embeddings-external:
	@if [ -z "$(EMBEDDING_API_URL)" ]; then \
		echo "Error: EMBEDDING_API_URL environment variable is not set"; \
		echo "Usage: make pregenerate-embeddings-external EMBEDDING_API_URL=http://your-server:8080"; \
		exit 1; \
	fi
	@echo "🧠 Pre-generating embeddings with external API..."
	@echo "   - API URL: $(EMBEDDING_API_URL)"
	@echo "   - Custom embedding service"
	EMBEDDING_API_URL="$(EMBEDDING_API_URL)" \
	MAGICTUNNEL_SEMANTIC_MODEL="external:api" \
	MAGICTUNNEL_DISABLE_SEMANTIC="false" \
	cargo run --bin magictunnel --release -- --config magictunnel-config.yaml --log-level info --pregenerate-embeddings


# Run in release mode with .env file support
run-release-env:
	@if [ ! -f .env ] && [ ! -f .env.development ] && [ ! -f .env.production ]; then \
		echo "🚨 No .env files found!"; \
		echo ""; \
		echo "📝 To get started:"; \
		echo "  1. Copy the example: cp .env.example .env"; \
		echo "  2. Edit .env and set your OPENAI_API_KEY"; \
		echo "  3. Run: make run-release-env"; \
		echo ""; \
		echo "Or use environment-specific files:"; \
		echo "  make run-release-env ENV=production"; \
		echo "  make run-release-env ENV=staging"; \
		echo ""; \
		exit 1; \
	fi
	@echo "🚀 Starting MagicTunnel (Release) with .env configuration..."
	@echo "   - Building in release mode..."
	$(if $(ENV),MAGICTUNNEL_ENV=$(ENV)) cargo run --bin magictunnel --release -- --config magictunnel-config.yaml --log-level info

# Run in development mode with .env file support
run-dev-env:
	@if [ ! -f .env ] && [ ! -f .env.development ] && [ ! -f .env.production ]; then \
		echo "🚨 No .env files found!"; \
		echo ""; \
		echo "📝 To get started:"; \
		echo "  1. Copy the example: cp .env.example .env"; \
		echo "  2. Edit .env and set your OPENAI_API_KEY"; \
		echo "  3. Run: make run-dev-env"; \
		echo ""; \
		echo "Or use environment-specific files:"; \
		echo "  make run-dev-env ENV=development"; \
		echo "  make run-dev-env ENV=staging"; \
		echo ""; \
		exit 1; \
	fi
	@echo "🚀 Starting MagicTunnel with .env configuration..."
	$(if $(ENV),MAGICTUNNEL_ENV=$(ENV)) cargo run --bin magictunnel -- --config magictunnel-config.yaml --log-level debug

# Set up .env file for development
setup-env:
	@if [ -f .env ]; then \
		echo "✅ .env file already exists"; \
	else \
		echo "📝 Creating .env file from example..."; \
		cp .env.example .env; \
		echo "✅ .env file created!"; \
		echo ""; \
		echo "⚠️  IMPORTANT: Edit .env file and set your OpenAI API key:"; \
		echo "   OPENAI_API_KEY=sk-your-actual-openai-key-here"; \
		echo ""; \
		echo "🚀 Then run: make run-dev-env"; \
	fi

# Install development tools
install-tools:
	rustup component add rustfmt clippy

# Full development check (format, clippy, test, build)
dev-check: fmt clippy test build
	@echo "✅ All development checks passed!"

# Watch for changes and run tests
watch-test:
	cargo watch -x test

# Watch for changes and run the proxy
watch-run:
	cargo watch -x "run -- --log-level debug"

# Run specific test suite
test-agent-router:
	cargo test --test agent_router_test

test-data-structures:
	cargo test --test data_structures_test

test-streaming:
	cargo test --test streaming_protocols_test

test-performance:
	cargo test --test performance_test

test-grpc:
	cargo test --test grpc_integration_test

test-integration:
	cargo test --test integration_test

test-mcp-server:
	cargo test --test mcp_server_test

test-registry:
	cargo test --test registry_service_test

# Run capability file validation
test-capabilities:
	cargo test --test yaml_parsing_test

# Generate documentation
docs:
	cargo doc --open

# Check for security vulnerabilities
audit:
	cargo audit

# Update dependencies
update:
	cargo update
