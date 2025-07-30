# Contributing to MCP Proxy

Thank you for your interest in contributing to MCP Proxy! This document provides guidelines and information for contributors.

## 🚀 Quick Start for Contributors

1. **Fork the repository** on GitHub
2. **Clone your fork** locally
3. **Create a feature branch** from `main`
4. **Make your changes** with tests
5. **Submit a pull request**

## 📋 Development Setup

### Prerequisites

- **Rust 1.70+**: [Install Rust](https://rustup.rs/)
- **Git**: For version control
- **Docker** (optional): For testing containerized deployments

### Local Development

```bash
# Clone your fork
git clone https://github.com/MagicBeansAI/magictunnel.git
cd magictunnel

# Install development tools
make install-tools

# Run tests
make test

# Run development checks
make dev-check
```

## 🧪 Testing

We use comprehensive testing to ensure code quality:

```bash
# Run all tests
cargo test

# Run specific test suites
make test-agent-router
make test-streaming
make test-mcp-server

# Run with coverage
cargo test --all-features

# Performance tests
make test-performance
```

### Test Guidelines

- **Write tests** for all new functionality
- **Update tests** when modifying existing code
- **Include integration tests** for new features
- **Test error conditions** and edge cases
- **Maintain test coverage** above 80%

## 📝 Code Style

We follow Rust best practices and use automated formatting:

```bash
# Format code
cargo fmt

# Check formatting
cargo fmt --check

# Run lints
cargo clippy

# Run all checks
make dev-check
```

### Style Guidelines

- **Use `rustfmt`** for consistent formatting
- **Follow Rust naming conventions**
- **Write clear, descriptive variable names**
- **Add documentation** for public APIs
- **Include examples** in documentation

## 🏗️ Architecture

Before contributing, please read:

- **[Complete Guide](README.md)** - Comprehensive overview with detailed architecture
- **[How to Run](how_to_run.md)** - Getting the project running
- **[TODO.md](TODO.md)** - Current roadmap and priorities

### Key Components

- **`src/proxy/`** - Core MCP proxy implementation
- **`src/registry/`** - Capability generators (GraphQL, OpenAPI, gRPC)
- **`src/agents/`** - Protocol-specific agents (HTTP, SSE, WebSocket)
- **`src/auth/`** - Authentication and authorization
- **`src/config/`** - Configuration management

## 🎯 Contribution Areas

### High Priority

- **Bug fixes** - Always welcome!
- **Documentation improvements** - Help others understand the project
- **Test coverage** - Improve reliability
- **Performance optimizations** - Make it faster
- **Example configurations** - Real-world use cases

### Feature Development

Check [TODO.md](TODO.md) for current priorities:

- **Phase 3**: Core functionality completion
- **Phase 4**: Open source launch preparation
- **Phase 5**: Advanced features (post-launch)

### Generator Development

We use **Test-Driven Development (TDD)** for capability generators:

1. **Create comprehensive test data** (GraphQL schemas, OpenAPI specs, etc.)
2. **Write failing tests** for the feature
3. **Implement the feature** to make tests pass
4. **Refactor and optimize**

## 📖 Documentation

### Code Documentation

- **Document public APIs** with `///` comments
- **Include examples** in documentation
- **Explain complex algorithms** with inline comments
- **Update README** for user-facing changes

### User Documentation

- **Update how_to_run.md** for setup changes
- **Add examples** to demonstrate new features
- **Update configuration documentation**
- **Include troubleshooting guides**

## 🐛 Bug Reports

When reporting bugs, please include:

- **Clear description** of the issue
- **Steps to reproduce** the problem
- **Expected vs actual behavior**
- **Environment details** (OS, Rust version, etc.)
- **Relevant logs** or error messages
- **Minimal reproduction case** if possible

## 💡 Feature Requests

For new features:

- **Check existing issues** to avoid duplicates
- **Describe the use case** and motivation
- **Propose an implementation approach**
- **Consider backward compatibility**
- **Discuss in GitHub Discussions** for large features

## 🔄 Pull Request Process

### Before Submitting

- [ ] **Tests pass** locally (`make test`)
- [ ] **Code is formatted** (`cargo fmt`)
- [ ] **Lints pass** (`cargo clippy`)
- [ ] **Documentation updated** if needed
- [ ] **Changelog entry** added (if applicable)

### PR Guidelines

- **Clear title** describing the change
- **Detailed description** of what and why
- **Link related issues** with "Fixes #123"
- **Small, focused changes** are preferred
- **Include tests** for new functionality

### Review Process

1. **Automated checks** must pass (CI/CD)
2. **Code review** by maintainers
3. **Address feedback** and update PR
4. **Final approval** and merge

## 🏷️ Commit Messages

Use conventional commit format:

```
type(scope): description

feat(auth): add JWT authentication support
fix(proxy): resolve connection timeout issue
docs(readme): update installation instructions
test(graphql): add comprehensive schema tests
```

Types: `feat`, `fix`, `docs`, `test`, `refactor`, `perf`, `chore`

## 📄 License

By contributing, you agree that your contributions will be licensed under the same license as the project (MIT/Apache 2.0).

## 🤝 Community

- **GitHub Discussions** - Ask questions, share ideas
- **Issues** - Bug reports and feature requests
- **Pull Requests** - Code contributions
- **Discord/Slack** - Real-time chat (coming soon)

## 🙏 Recognition

Contributors are recognized in:

- **CONTRIBUTORS.md** - All contributors listed
- **Release notes** - Major contributions highlighted
- **GitHub contributors** - Automatic recognition

---

**Questions?** Feel free to ask in GitHub Discussions or open an issue!
