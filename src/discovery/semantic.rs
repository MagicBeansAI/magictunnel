//! Semantic Search Module
//!
//! This module implements semantic search capabilities for tool discovery using
//! sentence transformers and persistent embedding storage.

use crate::error::{ProxyError, Result};
use crate::registry::types::ToolDefinition;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Configuration for semantic search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticSearchConfig {
    /// Whether semantic search is enabled
    pub enabled: bool,
    
    /// Embedding model name (e.g., "all-MiniLM-L6-v2")
    pub model_name: String,
    
    /// Minimum similarity threshold for semantic matches
    pub similarity_threshold: f64,
    
    /// Maximum number of semantic search results
    pub max_results: usize,
    
    /// Storage configuration
    pub storage: StorageConfig,
    
    /// Model configuration
    pub model: ModelConfig,
    
    /// Performance configuration
    pub performance: PerformanceConfig,
}

/// Storage configuration for persistent embeddings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Binary file for embeddings storage
    pub embeddings_file: PathBuf,
    
    /// JSON file for tool metadata
    pub metadata_file: PathBuf,
    
    /// JSON file for content hash validation
    pub hash_file: PathBuf,
    
    /// Number of backup files to maintain
    pub backup_count: usize,
    
    /// Automatically backup embeddings on updates
    pub auto_backup: bool,
    
    /// Enable compression for storage files
    pub compression: bool,
}

/// Model configuration for embeddings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// Directory to cache downloaded models
    pub cache_dir: PathBuf,
    
    /// Device to use: cpu, cuda, mps
    pub device: String,
    
    /// Maximum sequence length for embeddings
    pub max_sequence_length: usize,
    
    /// Batch size for embedding generation
    pub batch_size: usize,
    
    /// Normalize embeddings to unit vectors
    pub normalize_embeddings: bool,
}

/// Performance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Load embeddings only when needed
    pub lazy_loading: bool,
    
    /// In-memory cache size for embeddings
    pub embedding_cache_size: usize,
    
    /// Enable parallel embedding generation
    pub parallel_processing: bool,
    
    /// Number of worker threads for parallel processing
    pub worker_threads: usize,
}

impl Default for SemanticSearchConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            model_name: "all-MiniLM-L6-v2".to_string(),
            similarity_threshold: 0.7,
            max_results: 10,
            storage: StorageConfig {
                embeddings_file: PathBuf::from("./data/embeddings/tool_embeddings.bin"),
                metadata_file: PathBuf::from("./data/embeddings/tool_metadata.json"),
                hash_file: PathBuf::from("./data/embeddings/content_hashes.json"),
                backup_count: 3,
                auto_backup: true,
                compression: true,
            },
            model: ModelConfig {
                cache_dir: PathBuf::from("./data/models"),
                device: "cpu".to_string(),
                max_sequence_length: 512,
                batch_size: 32,
                normalize_embeddings: true,
            },
            performance: PerformanceConfig {
                lazy_loading: true,
                embedding_cache_size: 1000,
                parallel_processing: true,
                worker_threads: 4,
            },
        }
    }
}

/// Tool metadata for semantic search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetadata {
    /// Tool name
    pub name: String,
    
    /// Tool description
    pub description: String,
    
    /// Whether the tool is enabled
    pub enabled: bool,
    
    /// Whether the tool is hidden
    pub hidden: bool,
    
    /// Content hash for change detection
    pub content_hash: String,
    
    /// Timestamp when embedding was last updated
    pub last_updated: u64,
    
    /// Embedding dimensions
    pub embedding_dims: usize,
}

/// Semantic search result
#[derive(Debug, Clone)]
pub struct SemanticMatch {
    /// Tool name
    pub tool_name: String,
    
    /// Similarity score (0.0 to 1.0)
    pub similarity_score: f64,
    
    /// Whether the tool is enabled
    pub enabled: bool,
    
    /// Whether the tool is hidden
    pub hidden: bool,
}

/// Embedding storage for tools
#[derive(Debug)]
pub struct EmbeddingStorage {
    /// Tool embeddings (tool_name -> embedding vector)
    embeddings: HashMap<String, Vec<f32>>,
    
    /// Tool metadata
    metadata: HashMap<String, ToolMetadata>,
    
    /// Content hashes for change detection
    content_hashes: HashMap<String, String>,
    
    /// Whether the storage has been modified
    dirty: bool,
}

impl EmbeddingStorage {
    /// Create a new empty embedding storage
    pub fn new() -> Self {
        Self {
            embeddings: HashMap::new(),
            metadata: HashMap::new(),
            content_hashes: HashMap::new(),
            dirty: false,
        }
    }
    
    /// Add or update tool embedding
    pub fn add_tool_embedding(
        &mut self,
        tool_name: String,
        embedding: Vec<f32>,
        metadata: ToolMetadata,
    ) {
        self.embeddings.insert(tool_name.clone(), embedding);
        self.content_hashes.insert(tool_name.clone(), metadata.content_hash.clone());
        self.metadata.insert(tool_name, metadata);
        self.dirty = true;
    }
    
    /// Remove tool embedding
    pub fn remove_tool_embedding(&mut self, tool_name: &str) {
        self.embeddings.remove(tool_name);
        self.metadata.remove(tool_name);
        self.content_hashes.remove(tool_name);
        self.dirty = true;
    }
    
    /// Get tool embedding
    pub fn get_embedding(&self, tool_name: &str) -> Option<&Vec<f32>> {
        self.embeddings.get(tool_name)
    }
    
    /// Get tool metadata
    pub fn get_metadata(&self, tool_name: &str) -> Option<&ToolMetadata> {
        self.metadata.get(tool_name)
    }
    
    /// Get content hash for tool
    pub fn get_content_hash(&self, tool_name: &str) -> Option<&String> {
        self.content_hashes.get(tool_name)
    }
    
    /// Check if tool exists in storage
    pub fn contains_tool(&self, tool_name: &str) -> bool {
        self.embeddings.contains_key(tool_name)
    }
    
    /// Get all tool names
    pub fn get_tool_names(&self) -> Vec<String> {
        self.embeddings.keys().cloned().collect()
    }
    
    /// Get all enabled tools
    pub fn get_enabled_tools(&self) -> Vec<String> {
        self.metadata
            .iter()
            .filter(|(_, meta)| meta.enabled)
            .map(|(name, _)| name.clone())
            .collect()
    }
    
    /// Check if storage is dirty (needs saving)
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }
    
    /// Mark storage as clean (after saving)
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }
    
    /// Get storage statistics
    pub fn get_stats(&self) -> (usize, usize, usize) {
        let total = self.embeddings.len();
        let enabled = self.metadata.values().filter(|m| m.enabled).count();
        let hidden = self.metadata.values().filter(|m| m.hidden).count();
        (total, enabled, hidden)
    }
}

/// Semantic search service
pub struct SemanticSearchService {
    /// Configuration
    config: SemanticSearchConfig,
    
    /// Embedding storage
    pub storage: Arc<RwLock<EmbeddingStorage>>,
    
    /// Whether the model is loaded
    model_loaded: Arc<RwLock<bool>>,
}

impl SemanticSearchService {
    /// Create a new semantic search service
    pub fn new(config: SemanticSearchConfig) -> Self {
        Self {
            config,
            storage: Arc::new(RwLock::new(EmbeddingStorage::new())),
            model_loaded: Arc::new(RwLock::new(false)),
        }
    }
    
    /// Initialize the semantic search service
    pub async fn initialize(&self) -> Result<()> {
        if !self.config.enabled {
            info!("Semantic search is disabled");
            return Ok(());
        }
        
        info!("Initializing semantic search service with model: {}", self.config.model_name);
        
        // Create necessary directories
        self.create_directories().await?;
        
        // Load existing embeddings from storage
        self.load_embeddings().await?;
        
        // Initialize the embedding model (lazy loading if enabled)
        if !self.config.performance.lazy_loading {
            self.load_model().await?;
        }
        
        info!("Semantic search service initialized successfully");
        Ok(())
    }
    
    /// Create necessary directories for storage
    async fn create_directories(&self) -> Result<()> {
        let dirs = [
            self.config.storage.embeddings_file.parent(),
            self.config.storage.metadata_file.parent(),
            self.config.storage.hash_file.parent(),
            Some(self.config.model.cache_dir.as_path()),
        ];
        
        for dir in dirs.into_iter().flatten() {
            if !dir.exists() {
                tokio::fs::create_dir_all(dir).await
                    .map_err(|e| ProxyError::config(format!("Failed to create directory '{}': {}", dir.display(), e)))?;
                debug!("Created directory: {}", dir.display());
            }
        }
        
        Ok(())
    }
    
    /// Load embeddings from persistent storage
    async fn load_embeddings(&self) -> Result<()> {
        let mut storage = self.storage.write().await;
        
        // Load metadata
        if self.config.storage.metadata_file.exists() {
            let metadata_content = tokio::fs::read_to_string(&self.config.storage.metadata_file).await
                .map_err(|e| ProxyError::config(format!("Failed to read metadata file: {}", e)))?;
            
            let metadata: HashMap<String, ToolMetadata> = serde_json::from_str(&metadata_content)
                .map_err(|e| ProxyError::config(format!("Failed to parse metadata: {}", e)))?;
            
            storage.metadata = metadata;
            info!("Loaded {} tool metadata entries", storage.metadata.len());
        }
        
        // Load content hashes
        if self.config.storage.hash_file.exists() {
            let hash_content = tokio::fs::read_to_string(&self.config.storage.hash_file).await
                .map_err(|e| ProxyError::config(format!("Failed to read hash file: {}", e)))?;
            
            let hashes: HashMap<String, String> = serde_json::from_str(&hash_content)
                .map_err(|e| ProxyError::config(format!("Failed to parse hashes: {}", e)))?;
            
            storage.content_hashes = hashes;
            info!("Loaded {} content hashes", storage.content_hashes.len());
        }
        
        // Load embeddings (binary format)
        if self.config.storage.embeddings_file.exists() {
            let embeddings = self.load_embeddings_binary(&self.config.storage.embeddings_file).await?;
            storage.embeddings = embeddings;
            info!("Loaded {} tool embeddings", storage.embeddings.len());
        }
        
        storage.mark_clean();
        Ok(())
    }
    
    /// Reload embeddings from disk (for hot-reload)
    pub async fn reload_embeddings(&self) -> Result<()> {
        info!("🔥 Reloading embeddings from disk for hot-reload");
        self.load_embeddings().await
    }
    
    /// Load embeddings from binary file
    async fn load_embeddings_binary(&self, file_path: &Path) -> Result<HashMap<String, Vec<f32>>> {
        // For now, we'll use a simple JSON format for embeddings
        // In a production system, you'd want to use a more efficient binary format
        let content = tokio::fs::read_to_string(file_path).await
            .map_err(|e| ProxyError::config(format!("Failed to read embeddings file: {}", e)))?;
        
        let embeddings: HashMap<String, Vec<f32>> = serde_json::from_str(&content)
            .map_err(|e| ProxyError::config(format!("Failed to parse embeddings: {}", e)))?;
        
        Ok(embeddings)
    }
    
    /// Save embeddings to persistent storage
    pub async fn save_embeddings(&self) -> Result<()> {
        let storage = self.storage.read().await;
        
        if !storage.is_dirty() {
            debug!("Storage is clean, skipping save");
            return Ok(());
        }
        
        info!("Saving embeddings to persistent storage");
        
        // Create backups if enabled
        if self.config.storage.auto_backup {
            self.create_backups().await?;
        }
        
        // Save metadata
        let metadata_content = serde_json::to_string_pretty(&storage.metadata)
            .map_err(|e| ProxyError::config(format!("Failed to serialize metadata: {}", e)))?;
        
        tokio::fs::write(&self.config.storage.metadata_file, metadata_content).await
            .map_err(|e| ProxyError::config(format!("Failed to write metadata file: {}", e)))?;
        
        // Save content hashes
        let hash_content = serde_json::to_string_pretty(&storage.content_hashes)
            .map_err(|e| ProxyError::config(format!("Failed to serialize hashes: {}", e)))?;
        
        tokio::fs::write(&self.config.storage.hash_file, hash_content).await
            .map_err(|e| ProxyError::config(format!("Failed to write hash file: {}", e)))?;
        
        // Save embeddings
        self.save_embeddings_binary(&storage.embeddings).await?;
        
        info!("Embeddings saved successfully");
        Ok(())
    }
    
    /// Save embeddings to binary file
    async fn save_embeddings_binary(&self, embeddings: &HashMap<String, Vec<f32>>) -> Result<()> {
        // For now, we'll use JSON format for simplicity
        // In production, you'd want to use a more efficient binary format
        let content = serde_json::to_string_pretty(embeddings)
            .map_err(|e| ProxyError::config(format!("Failed to serialize embeddings: {}", e)))?;
        
        tokio::fs::write(&self.config.storage.embeddings_file, content).await
            .map_err(|e| ProxyError::config(format!("Failed to write embeddings file: {}", e)))?;
        
        Ok(())
    }
    
    /// Create backup files
    async fn create_backups(&self) -> Result<()> {
        let files = [
            &self.config.storage.embeddings_file,
            &self.config.storage.metadata_file,
            &self.config.storage.hash_file,
        ];
        
        for file in &files {
            if file.exists() {
                self.create_file_backup(file).await?;
            }
        }
        
        Ok(())
    }
    
    /// Create backup for a single file
    async fn create_file_backup(&self, file_path: &Path) -> Result<()> {
        let backup_path = file_path.with_extension(format!("{}.backup", 
            file_path.extension().and_then(|s| s.to_str()).unwrap_or("bak")));
        
        tokio::fs::copy(file_path, &backup_path).await
            .map_err(|e| ProxyError::config(format!("Failed to create backup for '{}': {}", file_path.display(), e)))?;
        
        debug!("Created backup: {}", backup_path.display());
        Ok(())
    }
    
    /// Load the embedding model
    async fn load_model(&self) -> Result<()> {
        info!("Initializing embedding model: {}", self.config.model_name);
        
        // Check if we're using an external API or local model
        match self.config.model_name.as_str() {
            name if name.starts_with("openai:") => {
                info!("Using OpenAI embeddings API");
                // Validate API key is available
                if std::env::var("OPENAI_API_KEY").is_err() {
                    warn!("OPENAI_API_KEY not found, embeddings may fail");
                }
            }
            name if name.starts_with("ollama:") => {
                info!("Using Ollama embedding service: {}", name);
                let ollama_url = std::env::var("OLLAMA_BASE_URL")
                    .unwrap_or_else(|_| "http://localhost:11434".to_string());
                info!("Ollama server URL: {}", ollama_url);
                // Test connectivity to Ollama server
                let client = reqwest::Client::new();
                if let Err(e) = client.get(format!("{}/api/version", ollama_url))
                    .timeout(std::time::Duration::from_secs(5))
                    .send()
                    .await {
                    warn!("Could not connect to Ollama server at {}: {}", ollama_url, e);
                }
            }
            name if name.starts_with("local:") => {
                info!("Using local embedding model: {}", name);
                // For local models, we'd initialize the model here
                // For now, this is a placeholder for local model loading
            }
            _ => {
                info!("Using built-in sentence transformer compatible model: {}", self.config.model_name);
                // For standard models like all-MiniLM-L6-v2, we'll use external APIs or local inference
            }
        }
        
        let mut model_loaded = self.model_loaded.write().await;
        *model_loaded = true;
        
        info!("Embedding model initialized successfully");
        Ok(())
    }
    
    /// Generate embedding for text using the configured model
    pub async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>> {
        // Ensure model is loaded
        if self.config.performance.lazy_loading {
            let model_loaded = self.model_loaded.read().await;
            if !*model_loaded {
                drop(model_loaded);
                self.load_model().await?;
            }
        }
        
        // Route to appropriate embedding method based on model configuration
        let embedding = match self.config.model_name.as_str() {
            name if name.starts_with("openai:") => {
                let model = name.strip_prefix("openai:").unwrap_or("text-embedding-3-small");
                self.generate_openai_embedding(text, model).await?
            }
            name if name.starts_with("ollama:") => {
                let model = name.strip_prefix("ollama:").unwrap_or("nomic-embed-text");
                self.generate_ollama_embedding(text, model).await?
            }
            name if name.starts_with("external:") => {
                self.generate_api_embedding(text, &std::env::var("EMBEDDING_API_URL")
                    .unwrap_or_else(|_| "http://localhost:8080".to_string())).await?
            }
            name if name.starts_with("local:") => {
                let model_path = name.strip_prefix("local:").unwrap_or("");
                self.generate_local_embedding(text, model_path).await?
            }
            _ => {
                // Default to sentence-transformer compatible approach
                self.generate_transformer_embedding(text).await?
            }
        };
        
        // Normalize if configured
        let final_embedding = if self.config.model.normalize_embeddings {
            self.normalize_embedding(embedding)
        } else {
            embedding
        };
        
        debug!("Generated {}-dimensional embedding for text: {}", final_embedding.len(), 
               if text.len() > 50 { format!("{}...", &text[..50]) } else { text.to_string() });
        
        Ok(final_embedding)
    }
    
    /// Generate embedding using OpenAI API
    async fn generate_openai_embedding(&self, text: &str, model: &str) -> Result<Vec<f32>> {
        use reqwest::Client;
        use serde_json::json;
        
        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| ProxyError::config("OPENAI_API_KEY environment variable not set"))?;
        
        let client = Client::new();
        let response = client
            .post("https://api.openai.com/v1/embeddings")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&json!({
                "input": text,
                "model": model,
                "encoding_format": "float"
            }))
            .timeout(std::time::Duration::from_secs(self.config.model.max_sequence_length as u64))
            .send()
            .await
            .map_err(|e| ProxyError::connection(format!("OpenAI API request failed: {}", e)))?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ProxyError::connection(format!("OpenAI API error: {}", error_text)));
        }
        
        let json: serde_json::Value = response.json().await
            .map_err(|e| ProxyError::connection(format!("Failed to parse OpenAI response: {}", e)))?;
        
        let embedding = json["data"][0]["embedding"].as_array()
            .ok_or_else(|| ProxyError::connection("Invalid OpenAI embedding response format"))?;
        
        let embedding: Vec<f32> = embedding.iter()
            .map(|v| v.as_f64().unwrap_or(0.0) as f32)
            .collect();
        
        Ok(embedding)
    }
    
    /// Generate embedding using local model (placeholder for local inference)
    async fn generate_local_embedding(&self, text: &str, _model_path: &str) -> Result<Vec<f32>> {
        // This is a placeholder for local model inference
        // In a production system, you would integrate with:
        // - Candle (Rust ML framework)
        // - ONNX Runtime
        // - Python subprocess calling sentence-transformers
        // - Local HTTP API (like Ollama)
        
        warn!("Local embedding generation not yet implemented, using fallback");
        self.generate_fallback_embedding(text).await
    }
    
    /// Generate embedding using Ollama API
    async fn generate_ollama_embedding(&self, text: &str, model: &str) -> Result<Vec<f32>> {
        use reqwest::Client;
        use serde_json::json;
        
        let ollama_url = std::env::var("OLLAMA_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:11434".to_string());
        
        let client = Client::new();
        let response = client
            .post(format!("{}/api/embeddings", ollama_url))
            .header("Content-Type", "application/json")
            .json(&json!({
                "model": model,
                "prompt": text
            }))
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| ProxyError::connection(format!("Ollama API request failed: {}", e)))?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ProxyError::connection(format!("Ollama API error: {}", error_text)));
        }
        
        let json: serde_json::Value = response.json().await
            .map_err(|e| ProxyError::connection(format!("Failed to parse Ollama response: {}", e)))?;
        
        let embedding = json["embedding"].as_array()
            .ok_or_else(|| ProxyError::connection("Invalid Ollama embedding response format"))?;
        
        let embedding: Vec<f32> = embedding.iter()
            .map(|v| v.as_f64().unwrap_or(0.0) as f32)
            .collect();
        
        Ok(embedding)
    }
    
    /// Generate embedding using transformer-compatible approach (via external API or local service)
    async fn generate_transformer_embedding(&self, text: &str) -> Result<Vec<f32>> {
        // Check if EMBEDDING_API_URL is configured for external service
        if let Ok(api_url) = std::env::var("EMBEDDING_API_URL") {
            return self.generate_api_embedding(text, &api_url).await;
        }
        
        // Fallback to deterministic embedding for testing/development
        warn!("No embedding service configured, using deterministic fallback for model: {}", self.config.model_name);
        self.generate_fallback_embedding(text).await
    }
    
    /// Generate embedding via external API service
    async fn generate_api_embedding(&self, text: &str, api_url: &str) -> Result<Vec<f32>> {
        use reqwest::Client;
        use serde_json::json;
        
        let client = Client::new();
        let response = client
            .post(format!("{}/embed", api_url))
            .header("Content-Type", "application/json")
            .json(&json!({
                "text": text,
                "model": self.config.model_name
            }))
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| ProxyError::connection(format!("Embedding API request failed: {}", e)))?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ProxyError::connection(format!("Embedding API error: {}", error_text)));
        }
        
        let json: serde_json::Value = response.json().await
            .map_err(|e| ProxyError::connection(format!("Failed to parse embedding API response: {}", e)))?;
        
        let embedding = json["embedding"].as_array()
            .ok_or_else(|| ProxyError::connection("Invalid embedding API response format"))?;
        
        let embedding: Vec<f32> = embedding.iter()
            .map(|v| v.as_f64().unwrap_or(0.0) as f32)
            .collect();
        
        Ok(embedding)
    }
    
    /// Generate deterministic fallback embedding for development/testing
    async fn generate_fallback_embedding(&self, text: &str) -> Result<Vec<f32>> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        // Use model-specific dimensions
        let dimensions = match self.config.model_name.as_str() {
            "all-MiniLM-L6-v2" => 384,
            "all-mpnet-base-v2" => 768,
            name if name.starts_with("openai:text-embedding-3-small") => 1536,
            name if name.starts_with("openai:text-embedding-3-large") => 3072,
            name if name.starts_with("ollama:") => 768, // nomic-embed-text is 768-dim
            name if name.starts_with("external:") => 768, // Default for external APIs
            _ => 384, // Default fallback
        };
        
        let mut embedding = vec![0.0f32; dimensions];
        
        // Create a more sophisticated deterministic embedding
        // This won't be as good as real embeddings but will be consistent
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        let base_hash = hasher.finish();
        
        // Use multiple hash seeds to fill the embedding vector
        for i in 0..dimensions {
            let mut item_hasher = DefaultHasher::new();
            (base_hash.wrapping_add(i as u64)).hash(&mut item_hasher);
            let hash_value = item_hasher.finish();
            
            // Convert hash to float in range [-1, 1]
            let normalized = ((hash_value % 2000) as f32 - 1000.0) / 1000.0;
            embedding[i] = normalized;
            
            // Add some text-based features
            if i < text.len() {
                let char_influence = (text.chars().nth(i).unwrap_or(' ') as u32 as f32) / 1000.0;
                embedding[i] = (embedding[i] + char_influence) / 2.0;
            }
        }
        
        // Add some semantic-like features based on text properties
        if dimensions > 10 {
            embedding[0] = text.len() as f32 / 1000.0; // Length feature
            embedding[1] = text.chars().filter(|c| c.is_uppercase()).count() as f32 / 100.0; // Uppercase feature
            embedding[2] = text.chars().filter(|c| c.is_numeric()).count() as f32 / 100.0; // Numeric feature
            embedding[3] = text.split_whitespace().count() as f32 / 100.0; // Word count feature
        }
        
        Ok(embedding)
    }
    
    /// Normalize embedding to unit vector
    fn normalize_embedding(&self, mut embedding: Vec<f32>) -> Vec<f32> {
        let norm = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for val in &mut embedding {
                *val /= norm;
            }
        }
        embedding
    }
    
    /// Search for similar tools using semantic similarity
    pub async fn search_similar_tools(&self, query: &str) -> Result<Vec<SemanticMatch>> {
        if !self.config.enabled {
            return Ok(Vec::new());
        }
        
        // Generate embedding for the query
        let query_embedding = self.generate_embedding(query).await?;
        
        let storage = self.storage.read().await;
        let mut matches = Vec::new();
        
        // Calculate similarity with all tool embeddings
        for (tool_name, tool_embedding) in &storage.embeddings {
            let similarity = self.calculate_cosine_similarity(&query_embedding, tool_embedding);
            
            if similarity >= self.config.similarity_threshold {
                if let Some(metadata) = storage.get_metadata(tool_name) {
                    matches.push(SemanticMatch {
                        tool_name: tool_name.clone(),
                        similarity_score: similarity,
                        enabled: metadata.enabled,
                        hidden: metadata.hidden,
                    });
                }
            }
        }
        
        // Sort by similarity score (highest first)
        matches.sort_by(|a, b| b.similarity_score.partial_cmp(&a.similarity_score).unwrap_or(std::cmp::Ordering::Equal));
        
        // Limit results
        matches.truncate(self.config.max_results);
        
        debug!("Found {} semantic matches for query: '{}'", matches.len(), query);
        Ok(matches)
    }
    
    /// Calculate cosine similarity between two embeddings
    fn calculate_cosine_similarity(&self, a: &[f32], b: &[f32]) -> f64 {
        if a.len() != b.len() {
            warn!("Embedding dimensions mismatch: {} vs {}", a.len(), b.len());
            return 0.0;
        }
        
        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        
        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }
        
        (dot_product / (norm_a * norm_b)) as f64
    }
    
    /// Generate content hash for a tool
    pub fn generate_content_hash(&self, tool_def: &ToolDefinition) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        tool_def.name.hash(&mut hasher);
        tool_def.description.hash(&mut hasher);
        tool_def.enabled.hash(&mut hasher);
        tool_def.hidden.hash(&mut hasher);
        
        format!("{:x}", hasher.finish())
    }
    
    /// Check if the service is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }
    
    /// Get service statistics
    pub async fn get_stats(&self) -> HashMap<String, serde_json::Value> {
        let storage = self.storage.read().await;
        let (total, enabled, hidden) = storage.get_stats();
        
        let mut stats = HashMap::new();
        stats.insert("enabled".to_string(), serde_json::Value::Bool(self.config.enabled));
        stats.insert("model_name".to_string(), serde_json::Value::String(self.config.model_name.clone()));
        stats.insert("total_embeddings".to_string(), serde_json::Value::Number(total.into()));
        stats.insert("enabled_tools".to_string(), serde_json::Value::Number(enabled.into()));
        stats.insert("hidden_tools".to_string(), serde_json::Value::Number(hidden.into()));
        stats.insert("similarity_threshold".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(self.config.similarity_threshold).unwrap()));
        stats.insert("storage_dirty".to_string(), serde_json::Value::Bool(storage.is_dirty()));
        
        stats
    }
}