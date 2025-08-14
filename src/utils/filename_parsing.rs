//! Filename parsing utilities for enhancement storage
//!
//! Shared logic for parsing enhancement filenames with consistent
//! tool name extraction across the system.

use std::path::Path;

/// Result of parsing an enhancement filename
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedEnhancementFilename {
    /// Original filename
    pub filename: String,
    /// Extracted tool name
    pub tool_name: String,
    /// Date part (YYYYMMDD)
    pub date: String,
    /// Time part (HHMMSS)
    pub time: String,
    /// Unique ID part
    pub id: String,
    /// Full timestamp (date_time)
    pub timestamp: String,
}

/// Parse an enhancement filename to extract components
///
/// Expected format: `{tool_name}_{date}_{time}_{id}_enhanced.json`
/// Where tool_name can contain underscores.
///
/// # Arguments
/// * `filename` - The filename to parse
///
/// # Returns
/// `Some(ParsedEnhancementFilename)` if parsing succeeds, `None` if it fails
///
/// # Examples
/// ```
/// use magictunnel::utils::filename_parsing::parse_enhancement_filename;
/// 
/// let result = parse_enhancement_filename("enhanced_analyze_sentiment_20250813_052616_b7adcb6b_enhanced.json");
/// assert!(result.is_some());
/// let parsed = result.unwrap();
/// assert_eq!(parsed.tool_name, "enhanced_analyze_sentiment");
/// assert_eq!(parsed.date, "20250813");
/// assert_eq!(parsed.time, "052616");
/// assert_eq!(parsed.id, "b7adcb6b");
/// ```
pub fn parse_enhancement_filename(filename: &str) -> Option<ParsedEnhancementFilename> {
    // Check if it's an enhancement file
    if !filename.ends_with("_enhanced.json") {
        return None;
    }
    
    // Remove the suffix
    let name_without_suffix = &filename[..filename.len() - "_enhanced.json".len()];
    
    // Split from the end to find date, time and id (last 3 parts)
    // Using rsplitn(4, '_') gives us [id, time, date, tool_name] in reverse order
    let parts: Vec<&str> = name_without_suffix.rsplitn(4, '_').collect();
    
    if parts.len() < 4 {
        return None;
    }
    
    // parts[0] = id, parts[1] = time, parts[2] = date, parts[3] = tool_name (reversed order)
    let tool_name = parts[3].to_string();
    let date = parts[2].to_string();
    let time = parts[1].to_string();
    let id = parts[0].to_string();
    let timestamp = format!("{}_{}", date, time);
    
    // Validate date format (YYYYMMDD)
    if date.len() != 8 || !date.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    
    // Validate time format (HHMMSS)
    if time.len() != 6 || !time.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    
    // Validate ID (should be alphanumeric, typically 8 characters)
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_alphanumeric()) {
        return None;
    }
    
    Some(ParsedEnhancementFilename {
        filename: filename.to_string(),
        tool_name,
        date,
        time,
        id,
        timestamp,
    })
}

/// Parse enhancement filename from a path
///
/// Convenience function that extracts the filename from a path and parses it.
///
/// # Arguments
/// * `path` - Path to the enhancement file
///
/// # Returns
/// `Some(ParsedEnhancementFilename)` if parsing succeeds, `None` if it fails
pub fn parse_enhancement_filename_from_path(path: &Path) -> Option<ParsedEnhancementFilename> {
    let filename = path.file_name()?.to_str()?;
    parse_enhancement_filename(filename)
}

/// Check if a filename matches the enhancement pattern
///
/// Quick check without full parsing to see if a file is an enhancement file.
///
/// # Arguments
/// * `filename` - The filename to check
///
/// # Returns
/// `true` if the filename matches the enhancement pattern
pub fn is_enhancement_filename(filename: &str) -> bool {
    filename.ends_with("_enhanced.json") && parse_enhancement_filename(filename).is_some()
}

/// Generate an enhancement filename
///
/// Creates a properly formatted enhancement filename for storage.
///
/// # Arguments
/// * `tool_name` - Name of the tool
/// * `date` - Date string (YYYYMMDD)
/// * `time` - Time string (HHMMSS)
/// * `id` - Unique ID string
///
/// # Returns
/// Formatted enhancement filename
///
/// # Examples
/// ```
/// use magictunnel::utils::filename_parsing::generate_enhancement_filename;
/// 
/// let filename = generate_enhancement_filename("analyze_sentiment", "20250813", "052616", "b7adcb6b");
/// assert_eq!(filename, "analyze_sentiment_20250813_052616_b7adcb6b_enhanced.json");
/// ```
pub fn generate_enhancement_filename(tool_name: &str, date: &str, time: &str, id: &str) -> String {
    format!("{}_{}_{}_{}_enhanced.json", tool_name, date, time, id)
}

/// Extract tool name from enhancement filename (quick version)
///
/// Lightweight function to just extract the tool name without full parsing.
///
/// # Arguments
/// * `filename` - The enhancement filename
///
/// # Returns
/// `Some(tool_name)` if extraction succeeds, `None` if it fails
pub fn extract_tool_name(filename: &str) -> Option<String> {
    parse_enhancement_filename(filename).map(|parsed| parsed.tool_name)
}

/// Compare two enhancement filenames by timestamp
///
/// Returns ordering for sorting enhancement files by creation time.
///
/// # Arguments
/// * `filename1` - First filename to compare
/// * `filename2` - Second filename to compare
///
/// # Returns
/// `std::cmp::Ordering` for sorting (newer files first)
pub fn compare_enhancement_timestamps(filename1: &str, filename2: &str) -> std::cmp::Ordering {
    let parsed1 = parse_enhancement_filename(filename1);
    let parsed2 = parse_enhancement_filename(filename2);
    
    match (parsed1, parsed2) {
        (Some(p1), Some(p2)) => p2.timestamp.cmp(&p1.timestamp), // Newer first
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_parse_enhancement_filename_success() {
        let filename = "enhanced_analyze_sentiment_20250813_052616_b7adcb6b_enhanced.json";
        let result = parse_enhancement_filename(filename).unwrap();
        
        assert_eq!(result.filename, filename);
        assert_eq!(result.tool_name, "enhanced_analyze_sentiment");
        assert_eq!(result.date, "20250813");
        assert_eq!(result.time, "052616");
        assert_eq!(result.id, "b7adcb6b");
        assert_eq!(result.timestamp, "20250813_052616");
    }

    #[test]
    fn test_parse_enhancement_filename_complex_tool_name() {
        let filename = "read_file_filesystem_20250813_052616_b7adcb6b_enhanced.json";
        let result = parse_enhancement_filename(filename).unwrap();
        
        assert_eq!(result.tool_name, "read_file_filesystem");
        assert_eq!(result.date, "20250813");
        assert_eq!(result.time, "052616");
        assert_eq!(result.id, "b7adcb6b");
    }

    #[test]
    fn test_parse_enhancement_filename_failures() {
        // Wrong extension
        assert!(parse_enhancement_filename("tool_20250813_052616_abc_enhanced.txt").is_none());
        
        // Missing parts
        assert!(parse_enhancement_filename("tool_enhanced.json").is_none());
        
        // Invalid date
        assert!(parse_enhancement_filename("tool_invalid_052616_abc_enhanced.json").is_none());
        
        // Invalid time
        assert!(parse_enhancement_filename("tool_20250813_invalid_abc_enhanced.json").is_none());
        
        // Not enhancement file
        assert!(parse_enhancement_filename("regular_file.json").is_none());
    }

    #[test]
    fn test_parse_enhancement_filename_from_path() {
        let path = PathBuf::from("/some/path/tool_20250813_052616_abc_enhanced.json");
        let result = parse_enhancement_filename_from_path(&path).unwrap();
        
        assert_eq!(result.tool_name, "tool");
        assert_eq!(result.date, "20250813");
    }

    #[test]
    fn test_is_enhancement_filename() {
        assert!(is_enhancement_filename("tool_20250813_052616_abc_enhanced.json"));
        assert!(!is_enhancement_filename("tool_20250813_052616_abc_enhanced.txt"));
        assert!(!is_enhancement_filename("regular_file.json"));
        assert!(!is_enhancement_filename("tool_enhanced.json")); // Missing parts
    }

    #[test]
    fn test_generate_enhancement_filename() {
        let filename = generate_enhancement_filename("analyze_sentiment", "20250813", "052616", "b7adcb6b");
        assert_eq!(filename, "analyze_sentiment_20250813_052616_b7adcb6b_enhanced.json");
    }

    #[test]
    fn test_extract_tool_name() {
        assert_eq!(
            extract_tool_name("enhanced_analyze_sentiment_20250813_052616_b7adcb6b_enhanced.json").unwrap(),
            "enhanced_analyze_sentiment"
        );
        
        assert_eq!(
            extract_tool_name("complex_tool_name_20250813_052616_abc_enhanced.json").unwrap(),
            "complex_tool_name"
        );
        
        assert!(extract_tool_name("invalid_file.json").is_none());
    }

    #[test]
    fn test_compare_enhancement_timestamps() {
        let newer = "tool_20250814_120000_abc_enhanced.json";
        let older = "tool_20250813_120000_def_enhanced.json";
        
        // Newer should come first (Less means first in sort)
        assert_eq!(compare_enhancement_timestamps(newer, older), std::cmp::Ordering::Less);
        assert_eq!(compare_enhancement_timestamps(older, newer), std::cmp::Ordering::Greater);
        assert_eq!(compare_enhancement_timestamps(newer, newer), std::cmp::Ordering::Equal);
    }
}