#!/usr/bin/env python3
"""
Test semantic search directly through MagicTunnel's HTTP API
to compare with Python script results
"""
import json
import requests

def test_magictunnel_semantic_search():
    """Test MagicTunnel's smart discovery with semantic search"""
    
    query = "ping google.com to check network connectivity and response time"
    
    print("="*80)
    print(f"Testing MagicTunnel semantic search with query: '{query}'")
    print("="*80)
    
    # Test with different confidence thresholds
    thresholds = [0.5, 0.6, 0.7, 0.8]
    
    for threshold in thresholds:
        print(f"\n--- Testing with threshold {threshold} ---")
        
        try:
            response = requests.post(
                "http://localhost:3001/mcp/call",
                headers={"Content-Type": "application/json"},
                json={
                    "name": "smart_tool_discovery",
                    "arguments": {
                        "context": "User wants to test network connectivity to google.com, similar to running a ping command to check if the server is responding and measure latency",
                        "request": query,
                        "confidence_threshold": threshold,
                        "preferred_tools": ["ping_globalping"]
                    }
                },
                timeout=30
            )
            
            if response.status_code == 200:
                result = response.json()
                print(f"✅ Status: {response.status_code}")
                
                # Extract the actual results from the smart discovery response
                if "content" in result and isinstance(result["content"], list):
                    content = result["content"][0] if result["content"] else {}
                    if "text" in content:
                        text_content = content["text"]
                        print(f"Response: {text_content[:200]}...")
                        
                        # Try to extract semantic matches info
                        if "semantic matches found" in text_content.lower():
                            print("✅ Semantic search was used")
                        elif "0 semantic matches" in text_content.lower() or "no semantic matches" in text_content.lower():
                            print("❌ No semantic matches found")
                        else:
                            print("? Unknown semantic search result")
                    else:
                        print(f"Response structure: {json.dumps(result, indent=2)}")
                else:
                    print(f"Unexpected response format: {json.dumps(result, indent=2)}")
                    
            else:
                print(f"❌ Status: {response.status_code}")
                print(f"Error: {response.text}")
                
        except Exception as e:
            print(f"❌ Request failed: {e}")

    print("\n" + "="*80)
    print("Testing complete!")

if __name__ == "__main__":
    test_magictunnel_semantic_search()