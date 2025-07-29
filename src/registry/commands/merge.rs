//! Capability Merger
//!
//! This module provides functionality to merge multiple capability files into one.
//!
//! # Overview
//!
//! The capability merger allows combining multiple capability files into a single file,
//! which is useful when:
//!
//! - Combining capabilities from different API sources (GraphQL, gRPC, OpenAPI)
//! - Merging capabilities from multiple services
//! - Creating a comprehensive capability file from smaller, focused files
//!
//! # Merge Strategies
//!
//! When merging files, duplicate tool names may be encountered. The merger supports
//! several strategies for handling these duplicates:
//!
//! - `KeepFirst`: Keep the first occurrence of each tool and discard later duplicates
//! - `KeepLast`: Keep the last occurrence of each tool, replacing earlier duplicates
//! - `Rename`: Keep all tools but rename duplicates with version suffixes (e.g., `tool_v1`, `tool_v2`)
//! - `Error`: Fail with an error when duplicates are found (default behavior)
//!
//! # Metadata Handling
//!
//! The merger also combines metadata from all input files:
//!
//! - Names: Uses the first non-empty name, or "merged_capabilities" if none found
//! - Descriptions: Concatenates all descriptions with separators
//! - Versions: Combines all versions with "+" separators
//! - Authors: Combines all authors with comma separators
//! - Tags: Combines all unique tags into a single set

use crate::error::{ProxyError, Result};
use crate::registry::types::{CapabilityFile, FileMetadata, ToolDefinition};
use std::collections::{HashMap, HashSet};

/// Capability Merger
///
/// Provides functionality to merge multiple capability files into one.
/// This is useful for combining capabilities from different sources or services.
///
/// # Example
///
/// ```
/// use magictunnel::registry::commands::merge::{CapabilityMerger, MergeStrategy};
/// use magictunnel::registry::types::CapabilityFile;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let merger = CapabilityMerger::new();
/// let file1 = CapabilityFile::new(vec![])?;
/// let file2 = CapabilityFile::new(vec![])?;
/// let merged_file = merger.merge(vec![file1, file2], MergeStrategy::Rename)?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct CapabilityMerger;

impl CapabilityMerger {
    /// Create a new capability merger
    pub fn new() -> Self {
        Self
    }

    /// Merge multiple capability files into one
    /// 
    /// This method merges multiple capability files into one, handling duplicates
    /// and conflicts according to the specified strategy.
    /// 
    /// # Arguments
    /// 
    /// * `files` - A vector of capability files to merge
    /// * `strategy` - The merge strategy to use
    /// 
    /// # Returns
    /// 
    /// A Result containing the merged capability file or an error
    pub fn merge(&self, files: Vec<CapabilityFile>, strategy: MergeStrategy) -> Result<CapabilityFile> {
        if files.is_empty() {
            return Err(ProxyError::validation("No files to merge".to_string()));
        }

        // Merge metadata
        let metadata = self.merge_metadata(&files);

        // Collect all tools
        let mut all_tools: Vec<ToolDefinition> = Vec::new();
        let mut tool_names: HashSet<String> = HashSet::new();
        let mut duplicates: HashMap<String, Vec<ToolDefinition>> = HashMap::new();

        // First pass: collect all tools and identify duplicates
        for file in &files {
            for tool in &file.tools {
                let name = tool.name.clone();
                
                if tool_names.contains(&name) {
                    // Add to duplicates
                    duplicates.entry(name.clone())
                        .or_insert_with(Vec::new)
                        .push(tool.clone());
                } else {
                    // Add to unique tools
                    tool_names.insert(name);
                    all_tools.push(tool.clone());
                }
            }
        }

        // Second pass: handle duplicates according to strategy
        for (name, dupe_tools) in duplicates {
            match strategy {
                MergeStrategy::KeepFirst => {
                    // Already added the first one, do nothing
                }
                MergeStrategy::KeepLast => {
                    // Remove the first one we added and add the last duplicate
                    if let Some(pos) = all_tools.iter().position(|t| t.name == name) {
                        all_tools.remove(pos);
                    }
                    all_tools.push(dupe_tools.last().unwrap().clone());
                }
                MergeStrategy::Rename => {
                    // Add all duplicates with renamed versions
                    for (i, tool) in dupe_tools.iter().enumerate() {
                        let mut renamed_tool = tool.clone();
                        renamed_tool.name = format!("{}_v{}", name, i + 1);
                        all_tools.push(renamed_tool);
                    }
                }
                MergeStrategy::Error => {
                    return Err(ProxyError::validation(
                        format!("Duplicate tool name found: {}", name)
                    ));
                }
            }
        }

        // Create merged file
        let merged_file = CapabilityFile {
            metadata: Some(metadata),
            tools: all_tools,
        };

        // Validate the merged file
        merged_file.validate()?;

        Ok(merged_file)
    }

    /// Merge metadata from multiple files
    /// 
    /// This method creates a new metadata object by combining information
    /// from all input files.
    fn merge_metadata(&self, files: &[CapabilityFile]) -> FileMetadata {
        let mut merged = FileMetadata::new();
        let mut descriptions = Vec::new();
        let mut versions = Vec::new();
        let mut authors = Vec::new();
        let mut all_tags = HashSet::new();

        // Collect metadata from all files
        for file in files {
            if let Some(ref metadata) = file.metadata {
                // Use the first non-empty name
                if merged.name.is_none() && metadata.name.is_some() {
                    merged.name = metadata.name.clone();
                }

                // Collect descriptions
                if let Some(ref desc) = metadata.description {
                    descriptions.push(desc.clone());
                }

                // Collect versions
                if let Some(ref ver) = metadata.version {
                    versions.push(ver.clone());
                }

                // Collect authors
                if let Some(ref author) = metadata.author {
                    authors.push(author.clone());
                }

                // Collect tags
                if let Some(ref tags) = metadata.tags {
                    for tag in tags {
                        all_tags.insert(tag.clone());
                    }
                }
            }
        }

        // Set merged name if not already set
        if merged.name.is_none() {
            merged.name = Some("merged_capabilities".to_string());
        }

        // Set merged description
        if !descriptions.is_empty() {
            merged.description = Some(descriptions.join(" | "));
        } else {
            merged.description = Some("Merged capability file".to_string());
        }

        // Set merged version
        if !versions.is_empty() {
            merged.version = Some(versions.join("+"));
        }

        // Set merged authors
        if !authors.is_empty() {
            merged.author = Some(authors.join(", "));
        }

        // Set merged tags
        if !all_tags.is_empty() {
            merged.tags = Some(all_tags.into_iter().collect());
        }

        merged
    }
}

/// Merge strategy for handling duplicate tools
///
/// When merging capability files, duplicate tool names may be encountered.
/// This enum defines different strategies for handling these duplicates.
///
/// # Variants
///
/// - `KeepFirst`: Keep the first occurrence of each tool and discard later duplicates
/// - `KeepLast`: Keep the last occurrence of each tool, replacing earlier duplicates
/// - `Rename`: Keep all tools but rename duplicates with version suffixes (e.g., `tool_v1`, `tool_v2`)
/// - `Error`: Fail with an error when duplicates are found (default behavior)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeStrategy {
    /// Keep the first occurrence of each tool
    KeepFirst,
    /// Keep the last occurrence of each tool
    KeepLast,
    /// Rename duplicate tools
    Rename,
    /// Return an error on duplicates
    Error,
}

impl Default for MergeStrategy {
    fn default() -> Self {
        MergeStrategy::Error
    }
}

impl Default for CapabilityMerger {
    fn default() -> Self {
        Self::new()
    }
}