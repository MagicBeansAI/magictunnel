#!/usr/bin/env python3
import json
import requests
import numpy as np
from typing import List, Tuple

def get_ollama_embedding(text: str) -> List[float]:
    """Get embedding from Ollama API"""
    response = requests.post(
        "http://localhost:11434/api/embeddings",
        json={
            "model": "nomic-embed-text",
            "prompt": text
        }
    )
    return response.json()["embedding"]

def cosine_similarity(a: List[float], b: List[float]) -> float:
    """Calculate cosine similarity between two vectors"""
    a_np = np.array(a)
    b_np = np.array(b)
    dot_product = np.dot(a_np, b_np)
    norm_a = np.linalg.norm(a_np)
    norm_b = np.linalg.norm(b_np)
    return dot_product / (norm_a * norm_b)

def search_embeddings(query: str, threshold: float = 0.7, top_k: int = 10, use_stored_embedding: str = None) -> List[Tuple[str, float, str, bool, bool]]:
    """Search for similar tools using embeddings (only enabled tools)"""
    
    # Load stored embeddings first
    print("Loading stored embeddings...")
    with open("data/embeddings/tool_embeddings.bin", "r") as f:
        stored_embeddings = json.load(f)
    
    if use_stored_embedding and use_stored_embedding in stored_embeddings:
        # Use an existing tool embedding as the query
        print(f"Using stored embedding for '{use_stored_embedding}' as query (simulating: '{query}')")
        query_embedding = stored_embeddings[use_stored_embedding]
    else:
        # Get query embedding from Ollama
        print(f"Getting embedding for query: '{query}'")
        query_embedding = get_ollama_embedding(query)
    
    # Load metadata
    with open("data/embeddings/tool_metadata.json", "r") as f:
        metadata = json.load(f)
    
    print(f"Found {len(stored_embeddings)} stored embeddings")
    print(f"Similarity threshold: {threshold}")
    
    # Calculate similarities - only for ENABLED tools (hidden doesn't matter)
    results = []
    above_threshold_count = 0
    enabled_count = 0
    total_count = 0
    
    for tool_name, tool_embedding in stored_embeddings.items():
        if tool_name in metadata:
            total_count += 1
            enabled = metadata[tool_name]["enabled"]
            
            if enabled:  # Only process enabled tools
                enabled_count += 1
                similarity = cosine_similarity(query_embedding, tool_embedding)
                description = metadata[tool_name]["description"]
                hidden = metadata[tool_name]["hidden"]
                
                if similarity >= threshold:
                    above_threshold_count += 1
                    results.append((tool_name, similarity, description, hidden, enabled))
    
    print(f"Total tools: {total_count}")
    print(f"Enabled tools: {enabled_count}")
    print(f"Enabled tools above threshold ({threshold}): {above_threshold_count}")
    
    # Sort by similarity (highest first)
    results.sort(key=lambda x: x[1], reverse=True)
    
    return results[:top_k]

if __name__ == "__main__":
    query = "ping google.com to check network connectivity and response time"
    
    print("=" * 80)
    print(f"Testing with stored embeddings (no Ollama variability)")
    print(f"Simulating query: '{query}'")
    print("=" * 80)
    
    try:
        # Test using check_network_connectivity's embedding as the "query"
        print("\n1. Using check_network_connectivity embedding as query:")
        results = search_embeddings(query, threshold=0.7, top_k=10, use_stored_embedding="check_network_connectivity")
        
        print(f"\nTop {len(results)} ENABLED matches (threshold 0.7):")
        print("-" * 80)
        if results:
            for i, (tool_name, similarity, description, hidden, enabled) in enumerate(results, 1):
                hidden_flag = " [HIDDEN]" if hidden else " [VISIBLE]"
                print(f"{i:2d}. {tool_name:<30} (similarity: {similarity:.4f}){hidden_flag}")
                print(f"    Description: {description}")
                print()
        else:
            print("❌ NO ENABLED MATCHES with threshold 0.7")
            
        # Test using ping_globalping's embedding as the "query"  
        print("\n" + "=" * 80)
        print("2. Using ping_globalping embedding as query:")
        print("=" * 80)
        
        ping_results = search_embeddings(query, threshold=0.7, top_k=5, use_stored_embedding="ping_globalping")
        if ping_results:
            for i, (tool_name, similarity, description, hidden, enabled) in enumerate(ping_results, 1):
                hidden_flag = " [HIDDEN]" if hidden else " [VISIBLE]"
                print(f"{i:2d}. {tool_name:<30} (similarity: {similarity:.4f}){hidden_flag}")
                print(f"    Description: {description}")
                print()
        else:
            print("❌ NO ENABLED MATCHES with threshold 0.7")
            
        # Test with actual Ollama embedding for comparison
        print("\n" + "=" * 80)
        print("3. Using fresh Ollama embedding (for comparison):")
        print("=" * 80)
        
        fresh_results = search_embeddings(query, threshold=0.6, top_k=5)
        if fresh_results:
            for i, (tool_name, similarity, description, hidden, enabled) in enumerate(fresh_results, 1):
                hidden_flag = " [HIDDEN]" if hidden else " [VISIBLE]"
                print(f"{i:2d}. {tool_name:<30} (similarity: {similarity:.4f}){hidden_flag}")
                print(f"    Description: {description}")
                print()
        else:
            print("❌ NO ENABLED MATCHES even with threshold 0.6")
            
    except Exception as e:
        print(f"Error: {e}")
        import traceback
        traceback.print_exc()