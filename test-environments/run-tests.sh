#!/bin/bash

set -e

echo "🧪 MagicTunnel OAuth 2.1 Test Runner"
echo "==================================="

# Load environment variables if .env exists
if [ -f ".env" ]; then
    echo "📋 Loading environment variables from .env"
    export $(cat .env | grep -v '^#' | xargs)
else
    echo "⚠️  No .env file found. Using system environment variables."
fi

# Test environment selection
ENVIRONMENT=${1:-local}

case $ENVIRONMENT in
    "local")
        echo "🏠 Running local tests..."
        cd .. && ./scripts/test-oauth-production.sh
        ;;
    
    "docker")
        echo "🐳 Running Docker tests..."
        cd docker
        docker-compose up -d
        sleep 30  # Wait for services to start
        
        # Run tests against Docker container
        docker-compose exec artillery artillery run /app/scripts/oauth-load-test.yml --target http://magictunnel:3001
        
        # Cleanup
        docker-compose down
        ;;
    
    "staging")
        echo "🎭 Running staging tests..."
        echo "Staging tests require manual setup. See docs/OAUTH_2_1_TESTING_GUIDE.md"
        ;;
    
    "all")
        echo "🌐 Running all test environments..."
        $0 local
        $0 docker
        ;;
    
    *)
        echo "❌ Unknown environment: $ENVIRONMENT"
        echo "Usage: $0 [local|docker|staging|all]"
        exit 1
        ;;
esac

echo "✅ Tests completed for environment: $ENVIRONMENT"
