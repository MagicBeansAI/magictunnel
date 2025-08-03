# Testing Tools

Development and testing utilities for MagicTunnel functionality.

## Tools

### `test_search.py`

**Purpose**: Manual semantic search testing for development and debugging.

**Usage**:
```bash
python3 test_search.py [--query "search term"] [--model model_name]
```

**Parameters**:
- `--query`: Search term to test (interactive if not provided)
- `--model`: OpenAI embedding model to use (default: text-embedding-3-small)

**Features**:
- **Interactive Mode**: Enter search queries interactively
- **Embedding Generation**: Tests OpenAI embedding API
- **Similarity Scoring**: Calculates cosine similarity scores
- **Tool Matching**: Shows how search terms match tool descriptions
- **Performance Timing**: Measures embedding generation and search time

**Example Usage**:
```bash
# Interactive mode
python3 tools/testing/test_search.py

# Direct query
python3 tools/testing/test_search.py --query "ping google.com"

# Custom model
python3 tools/testing/test_search.py --query "file operations" --model text-embedding-ada-002
```

**Sample Output**:
```
🔍 Semantic Search Testing Tool
==============================

Query: "ping google.com"
Model: text-embedding-3-small

Generating embedding... (0.234s)
Searching tools... (0.012s)

Top 5 matches:
1. ping_host (score: 0.87)
   - Description: Ping a host to check connectivity
   - Category: networking

2. network_diagnostic (score: 0.73)
   - Description: Run network diagnostic tests
   - Category: networking

3. check_connectivity (score: 0.68)
   - Description: Check network connectivity
   - Category: monitoring

4. trace_route (score: 0.52)
   - Description: Trace network route to destination
   - Category: networking

5. dns_lookup (score: 0.44)
   - Description: Perform DNS lookup
   - Category: networking

Performance:
- Embedding generation: 234ms
- Tool search: 12ms
- Total: 246ms
```

### `test_rust_semantic.py`

**Purpose**: Test MagicTunnel's semantic search API and integration.

**Usage**:
```bash
python3 test_rust_semantic.py [--server url] [--query "search term"]
```

**Parameters**:
- `--server`: MagicTunnel server URL (default: http://localhost:3001)
- `--query`: Search term to test (interactive if not provided)

**Features**:
- **API Integration**: Tests live MagicTunnel semantic search API
- **Tool Discovery**: Tests smart tool discovery functionality
- **Parameter Mapping**: Tests parameter extraction and mapping
- **Confidence Scoring**: Shows confidence scores from the system
- **Full Pipeline**: Tests complete discovery → parameter mapping → execution

**Example Usage**:
```bash
# Test local server
python3 tools/testing/test_rust_semantic.py

# Test remote server
python3 tools/testing/test_rust_semantic.py --server http://staging.example.com:3001

# Direct query test
python3 tools/testing/test_rust_semantic.py --query "create a file called test.txt"
```

**Sample Output**:
```
🦀 MagicTunnel Semantic API Testing Tool
========================================

Server: http://localhost:3001
Status: ✅ Connected

Testing query: "create a file called test.txt"

1. Smart Tool Discovery
   Tool found: create_file
   Confidence: 0.94
   Discovery method: hybrid

2. Parameter Mapping
   Extracted parameters:
   - file_path: "test.txt"
   - content: ""
   - create_dirs: true

3. Tool Execution
   Status: Success
   Result: File created successfully
   Execution time: 45ms

4. Full Pipeline Performance
   Discovery: 234ms
   Parameter mapping: 67ms
   Execution: 45ms
   Total: 346ms

✅ All tests passed!
```

## Usage Scenarios

### Development Workflow

```bash
# 1. Test semantic search during development
python3 tools/testing/test_search.py --query "your test query"

# 2. Verify MagicTunnel integration
python3 tools/testing/test_rust_semantic.py --query "your test query"

# 3. Compare results
diff <(python3 tools/testing/test_search.py --query "test") \
     <(python3 tools/testing/test_rust_semantic.py --query "test")
```

### Performance Testing

```bash
# Test multiple queries for performance analysis
queries=(
    "ping google.com"
    "create a file"
    "read database records"
    "send http request"
    "process images"
)

for query in "${queries[@]}"; do
    echo "Testing: $query"
    python3 tools/testing/test_rust_semantic.py --query "$query" | grep "Total:"
done
```

### Batch Testing

```bash
# Test multiple queries from file
cat test_queries.txt | while read query; do
    echo "Query: $query"
    python3 tools/testing/test_search.py --query "$query"
    echo "---"
done
```

### Model Comparison

```bash
# Compare different embedding models
models=("text-embedding-3-small" "text-embedding-3-large" "text-embedding-ada-002")

for model in "${models[@]}"; do
    echo "Testing model: $model"
    python3 tools/testing/test_search.py --query "ping host" --model "$model"
done
```

## Integration with CI/CD

### Automated Testing

```yaml
# .github/workflows/semantic-tests.yml
name: Semantic Search Tests

on: [push, pull_request]

jobs:
  semantic-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      
      - name: Set up Python
        uses: actions/setup-python@v2
        with:
          python-version: '3.8'
          
      - name: Install dependencies
        run: |
          pip install requests numpy openai
          
      - name: Build MagicTunnel
        run: |
          cargo build --release
          
      - name: Start MagicTunnel
        run: |
          ./target/release/magictunnel --config test-config.yaml &
          sleep 10
          
      - name: Run semantic tests
        env:
          OPENAI_API_KEY: ${{ secrets.OPENAI_API_KEY }}
        run: |
          python3 tools/testing/test_rust_semantic.py --query "ping google.com"
          python3 tools/testing/test_rust_semantic.py --query "create file"
          python3 tools/testing/test_rust_semantic.py --query "read data"
```

### Performance Regression Testing

```bash
#!/bin/bash
# performance_regression_test.sh

# Baseline queries for performance testing
baseline_queries=(
    "ping google.com"
    "create a file called test.txt"
    "read user data from database"
    "send HTTP GET request"
    "list all files in directory"
)

# Performance thresholds (milliseconds)
MAX_DISCOVERY_TIME=500
MAX_TOTAL_TIME=1000

for query in "${baseline_queries[@]}"; do
    echo "Testing: $query"
    
    # Run test and extract timing
    result=$(python3 tools/testing/test_rust_semantic.py --query "$query")
    discovery_time=$(echo "$result" | grep "Discovery:" | grep -o '[0-9]*ms' | grep -o '[0-9]*')
    total_time=$(echo "$result" | grep "Total:" | grep -o '[0-9]*ms' | grep -o '[0-9]*')
    
    # Check thresholds
    if [ "$discovery_time" -gt "$MAX_DISCOVERY_TIME" ]; then
        echo "❌ Discovery time exceeded: ${discovery_time}ms > ${MAX_DISCOVERY_TIME}ms"
        exit 1
    fi
    
    if [ "$total_time" -gt "$MAX_TOTAL_TIME" ]; then
        echo "❌ Total time exceeded: ${total_time}ms > ${MAX_TOTAL_TIME}ms"
        exit 1
    fi
    
    echo "✅ Performance OK: Discovery ${discovery_time}ms, Total ${total_time}ms"
done
```

## Configuration

### Environment Variables

```bash
# Required for semantic search testing
export OPENAI_API_KEY="your-openai-api-key"

# Optional configuration
export SEMANTIC_MODEL="text-embedding-3-small"
export MAGICTUNNEL_SERVER="http://localhost:3001"
export TEST_TIMEOUT="30"
export DEBUG_SEMANTIC_TESTS="true"
```

### Test Configuration Files

```yaml
# test-config.yaml
smart_discovery:
  enabled: true
  default_mode: "hybrid"
  confidence_threshold: 0.5
  
  llm:
    provider: "openai"
    model: "gpt-4"
    temperature: 0.1
    
  semantic_search:
    enabled: true
    model: "text-embedding-3-small"
    similarity_threshold: 0.6
    
logging:
  level: "debug"
  semantic_search: true
  tool_discovery: true
```

## Advanced Usage

### Custom Test Scenarios

```python
# custom_test_scenario.py
import subprocess
import json

def test_discovery_accuracy():
    """Test discovery accuracy with known query-tool pairs"""
    
    test_cases = [
        ("ping google.com", "ping_host", 0.8),
        ("create file test.txt", "create_file", 0.9),
        ("read database users", "query_database", 0.7),
    ]
    
    for query, expected_tool, min_confidence in test_cases:
        result = subprocess.run([
            "python3", "tools/testing/test_rust_semantic.py",
            "--query", query
        ], capture_output=True, text=True)
        
        # Parse result
        if expected_tool in result.stdout:
            confidence = extract_confidence(result.stdout)
            if confidence >= min_confidence:
                print(f"✅ {query} → {expected_tool} (confidence: {confidence})")
            else:
                print(f"❌ {query} → {expected_tool} (low confidence: {confidence})")
        else:
            print(f"❌ {query} → {expected_tool} (not found)")

def extract_confidence(output):
    """Extract confidence score from test output"""
    import re
    match = re.search(r'Confidence: ([\d.]+)', output)
    return float(match.group(1)) if match else 0.0

if __name__ == "__main__":
    test_discovery_accuracy()
```

### Load Testing

```python
# load_test.py
import asyncio
import aiohttp
import time

async def load_test_semantic_api():
    """Load test the semantic API with concurrent requests"""
    
    queries = [
        "ping google.com",
        "create file",
        "read data",
        "send request",
        "list files"
    ]
    
    async def make_request(session, query):
        start_time = time.time()
        async with session.post(
            "http://localhost:3001/mcp/call",
            json={
                "name": "smart_tool_discovery",
                "arguments": {"request": query}
            }
        ) as response:
            await response.json()
            return time.time() - start_time
    
    async with aiohttp.ClientSession() as session:
        # Launch 50 concurrent requests
        tasks = []
        for i in range(50):
            query = queries[i % len(queries)]
            tasks.append(make_request(session, query))
        
        response_times = await asyncio.gather(*tasks)
        
        # Analysis
        avg_time = sum(response_times) / len(response_times)
        max_time = max(response_times)
        min_time = min(response_times)
        
        print(f"Load test results:")
        print(f"  Concurrent requests: 50")
        print(f"  Average response time: {avg_time:.3f}s")
        print(f"  Max response time: {max_time:.3f}s")
        print(f"  Min response time: {min_time:.3f}s")

if __name__ == "__main__":
    asyncio.run(load_test_semantic_api())
```

## Dependencies

```bash
# Python packages
pip install requests numpy openai aiohttp

# System requirements
python3 --version  # Python 3.7+
curl --version     # For API testing
```

## Troubleshooting

### Common Issues

1. **OpenAI API Key Missing**
   ```
   Error: OpenAI API key not configured
   Fix: Set OPENAI_API_KEY environment variable
   ```

2. **MagicTunnel Server Not Running**
   ```
   Error: Connection refused to localhost:3001
   Fix: Start MagicTunnel server before running tests
   ```

3. **Semantic Search Disabled**
   ```
   Error: Semantic search not enabled
   Fix: Enable semantic search in MagicTunnel configuration
   ```

4. **Low Confidence Scores**
   ```
   Issue: All tools have low confidence scores
   Fix: Check embedding model configuration and tool descriptions
   ```

### Debug Mode

```bash
# Enable debug output
export DEBUG_SEMANTIC_TESTS=true
python3 tools/testing/test_search.py --query "test"

# Verbose API testing
export RUST_LOG=debug
python3 tools/testing/test_rust_semantic.py --query "test"
```

### Performance Analysis

```bash
# Profile semantic search performance
python3 -m cProfile tools/testing/test_search.py --query "ping host"

# Memory usage analysis
python3 -m memory_profiler tools/testing/test_search.py --query "ping host"
```