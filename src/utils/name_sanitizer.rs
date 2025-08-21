use std::collections::HashSet;
use regex::Regex;
use rand::{thread_rng, Rng};
use once_cell::sync::Lazy;

/// Regex for sanitizing capability names - removes/replaces invalid characters
static NAME_SANITIZER_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[^a-zA-Z0-9\-_]").expect("Invalid regex pattern")
});

/// Sanitizes a capability name to ensure it follows our naming conventions:
/// - Only alphanumeric characters, dashes, and underscores
/// - No spaces or special characters
/// - Converts to lowercase
/// - Collapses multiple dashes/underscores into single ones
pub fn sanitize_capability_name(raw_name: &str) -> String {
    // Convert to lowercase and replace spaces with dashes
    let name = raw_name.to_lowercase().replace(' ', "-");
    
    // Remove or replace invalid characters
    let sanitized = NAME_SANITIZER_REGEX.replace_all(&name, "");
    
    // Collapse multiple dashes/underscores into single ones
    let collapsed = collapse_separators(&sanitized);
    
    // Remove leading/trailing separators
    let trimmed = collapsed.trim_matches(&['-', '_'][..]);
    
    // Ensure we have at least some content
    if trimmed.is_empty() {
        "unnamed-capability".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Collapses multiple consecutive dashes or underscores into single ones
fn collapse_separators(input: &str) -> String {
    let dash_regex = Regex::new(r"-+").unwrap();
    let underscore_regex = Regex::new(r"_+").unwrap();
    
    let collapsed_dashes = dash_regex.replace_all(input, "-");
    underscore_regex.replace_all(&collapsed_dashes, "_").to_string()
}

/// Ensures the capability name is unique by checking against existing names
/// and appending a random suffix if needed
pub fn ensure_unique_capability_name(
    sanitized_name: &str,
    existing_names: &HashSet<String>,
) -> String {
    if !existing_names.contains(sanitized_name) {
        return sanitized_name.to_string();
    }
    
    // Generate random suffixes until we find a unique name
    let mut rng = thread_rng();
    for _ in 0..100 { // Max 100 attempts to avoid infinite loop
        let suffix: u32 = rng.gen_range(1000..9999);
        let unique_name = format!("{}-{}", sanitized_name, suffix);
        
        if !existing_names.contains(&unique_name) {
            return unique_name;
        }
    }
    
    // Fallback: use timestamp-based suffix
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    format!("{}-{}", sanitized_name, timestamp)
}

/// Complete sanitization process: sanitize name and ensure uniqueness
pub fn sanitize_and_ensure_unique(
    raw_name: &str,
    existing_names: &HashSet<String>,
) -> String {
    let sanitized = sanitize_capability_name(raw_name);
    ensure_unique_capability_name(&sanitized, existing_names)
}

/// Sanitizes tool names within capabilities (similar logic but for individual tools)
pub fn sanitize_tool_name(raw_name: &str) -> String {
    // Similar to capability name but may have slightly different rules
    let name = raw_name.to_lowercase().replace(' ', "_");
    let sanitized = NAME_SANITIZER_REGEX.replace_all(&name, "");
    let collapsed = collapse_separators(&sanitized);
    let trimmed = collapsed.trim_matches(&['-', '_'][..]);
    
    if trimmed.is_empty() {
        "unnamed_tool".to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    
    #[test]
    fn test_sanitize_capability_name() {
        assert_eq!(sanitize_capability_name("Enhanced HTTP Client Tools"), "enhanced-http-client-tools");
        assert_eq!(sanitize_capability_name("AI-Enhanced Smart Discovery"), "ai-enhanced-smart-discovery");
        assert_eq!(sanitize_capability_name("Database Tools & Utils!"), "database-tools-utils");
        assert_eq!(sanitize_capability_name("Multiple   Spaces"), "multiple-spaces");
        assert_eq!(sanitize_capability_name("___dashes---and___underscores___"), "dashes-and_underscores");
        assert_eq!(sanitize_capability_name(""), "unnamed-capability");
        assert_eq!(sanitize_capability_name("123 Numbers"), "123-numbers");
    }
    
    #[test]
    fn test_ensure_unique_capability_name() {
        let mut existing = HashSet::new();
        existing.insert("enhanced-database-tools".to_string());
        existing.insert("enhanced-database-tools-1234".to_string());
        
        // Should return original if unique
        assert_eq!(ensure_unique_capability_name("new-capability", &existing), "new-capability");
        
        // Should append suffix if not unique
        let result = ensure_unique_capability_name("enhanced-database-tools", &existing);
        assert!(result.starts_with("enhanced-database-tools-"));
        assert_ne!(result, "enhanced-database-tools");
        assert_ne!(result, "enhanced-database-tools-1234");
    }
    
    #[test]
    fn test_sanitize_and_ensure_unique() {
        let mut existing = HashSet::new();
        existing.insert("enhanced-http-client-tools".to_string());
        
        let result = sanitize_and_ensure_unique("Enhanced HTTP Client Tools", &existing);
        assert!(result.starts_with("enhanced-http-client-tools-"));
        
        let result2 = sanitize_and_ensure_unique("New API Tools", &existing);
        assert_eq!(result2, "new-api-tools");
    }
    
    #[test]
    fn test_sanitize_tool_name() {
        assert_eq!(sanitize_tool_name("Get User Info"), "get_user_info");
        assert_eq!(sanitize_tool_name("execute-query"), "execute-query");
        assert_eq!(sanitize_tool_name("API Call Helper!"), "api_call_helper");
    }
}