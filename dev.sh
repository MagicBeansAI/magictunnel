#!/bin/bash
# Simple development script for MagicTunnel

set -e

# Check if release mode is requested
RELEASE_MODE=false
if [ "$1" = "--release" ] || [ "$1" = "-r" ]; then
    RELEASE_MODE=true
    shift
fi

if [ "$RELEASE_MODE" = true ]; then
    echo "🚀 MagicTunnel Release Mode Helper"
    echo "=================================="
else
    echo "🚀 MagicTunnel Development Helper"
    echo "================================="
fi

# Function to check if .env file exists and has OpenAI key
check_env() {
    if [ ! -f .env ]; then
        return 1
    fi
    
    if grep -q "OPENAI_API_KEY=sk-your-openai-key-here" .env; then
        return 1
    fi
    
    if ! grep -q "OPENAI_API_KEY=" .env || grep -q "OPENAI_API_KEY=$" .env; then
        return 1
    fi
    
    return 0
}

# Setup .env file if needed
if ! check_env; then
    echo "📝 Setting up .env file..."
    
    if [ ! -f .env ]; then
        echo "Creating .env from example..."
        cp .env.example .env
    fi
    
    echo ""
    echo "⚠️  Please edit .env file and set your OpenAI API key:"
    echo "   OPENAI_API_KEY=sk-your-actual-openai-key-here"
    echo ""
    echo "Then run this script again:"
    echo "  ./dev.sh           # Development mode"
    echo "  ./dev.sh --release # Release mode"
    exit 1
fi

echo "✅ .env file configured"

# Check if we need to build
if [ "$RELEASE_MODE" = true ]; then
    if [ ! -f target/release/magictunnel ] || [ src/main.rs -nt target/release/magictunnel ]; then
        echo "🔨 Building project in release mode..."
        cargo build --release
    fi
else
    if [ ! -f target/debug/magictunnel ] || [ src/main.rs -nt target/debug/magictunnel ]; then
        echo "🔨 Building project..."
        cargo build
    fi
fi

if [ "$RELEASE_MODE" = true ]; then
    echo "🚀 Starting MagicTunnel (Release) with Smart Discovery..."
    echo "   - OpenAI API configured"
    echo "   - Release mode optimizations enabled"
    echo "   - Info logging enabled"
    echo "   - Server will start on port 3001"
    echo ""
    
    # Run the application in release mode
    cargo run --bin magictunnel --release -- --config magictunnel-config.yaml --log-level info
else
    echo "🚀 Starting MagicTunnel with Smart Discovery..."
    echo "   - OpenAI API configured"
    echo "   - Debug logging enabled"
    echo "   - Server will start on port 3001"
    echo ""
    
    # Run the application in debug mode
    cargo run --bin magictunnel -- --config magictunnel-config.yaml --log-level debug
fi