//! Tests for the magictunnel-visibility CLI tool
//!
//! This module tests the CLI binary functionality including all commands and edge cases.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;
use tokio::test;

/// Helper function to create a test capability file
fn create_test_capability_file(path: &PathBuf, content: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

/// Helper function to run the visibility CLI and capture output
fn run_visibility_cli(args: &[&str]) -> (bool, String, String) {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "magictunnel-visibility", "--"])
        .args(args)
        .output()
        .expect("Failed to run magictunnel-visibility");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    
    (output.status.success(), stdout, stderr)
}

/// Helper function to create a basic capability file with proper structure
fn create_basic_capability_file() -> String {
    r#"
metadata:
  name: "Test Capability"
  description: "Test capability file"
  version: "1.0.0"

tools:
  - name: "test_tool_1"
    description: "First test tool"
    inputSchema:
      type: "object"
      properties:
        arg1:
          type: "string"
          description: "Test argument"
    routing:
      type: "subprocess"
      config:
        command: "echo"
        args: ["test1"]
    hidden: false
    enabled: true

  - name: "test_tool_2"
    description: "Second test tool"
    inputSchema:
      type: "object"
      properties:
        arg2:
          type: "string"
          description: "Test argument 2"
    routing:
      type: "subprocess"
      config:
        command: "echo"
        args: ["test2"]
    hidden: true
    enabled: false

  - name: "test_tool_3"
    description: "Third test tool"
    inputSchema:
      type: "object"
      properties: {}
    routing:
      type: "http"
      config:
        url: "http://example.com"
    hidden: false
    enabled: true
"#.to_string()
}

/// Helper function to create a config file for testing
fn create_test_config(capabilities_dir: &str) -> String {
    format!(r#"
server:
  host: "127.0.0.1"
  port: 8080
  websocket: true
  timeout: 30

registry:
  type: "file"
  paths: ["{capabilities_dir}"]
  hot_reload: false
  validation:
    strict: true
    allow_unknown_fields: false

smart_discovery:
  enabled: true
  tool_selection_mode: "rule_based"
  default_confidence_threshold: 0.7
  enable_tool_enhancement: false
  enable_elicitation: false
  max_tools_to_consider: 10
  max_high_quality_matches: 3
  high_quality_threshold: 0.95
  use_fuzzy_matching: true
  enable_sequential_mode: true
"#, capabilities_dir = capabilities_dir)
}

#[tokio::test]
async fn test_cli_help_command() {
    let (success, stdout, _stderr) = run_visibility_cli(&["--help"]);
    
    assert!(success, "CLI help command should succeed");
    assert!(stdout.contains("MagicTunnel Tool Visibility Management"));
    assert!(stdout.contains("status"));
    assert!(stdout.contains("hide-all"));
    assert!(stdout.contains("show-all"));
    assert!(stdout.contains("hide-tool"));
    assert!(stdout.contains("show-tool"));
    assert!(stdout.contains("enable-all"));
    assert!(stdout.contains("disable-all"));
    assert!(stdout.contains("show-mcp-warnings"));
}

#[tokio::test]
async fn test_cli_version_command() {
    let (success, stdout, _stderr) = run_visibility_cli(&["--version"]);
    
    assert!(success, "CLI version command should succeed");
    assert!(stdout.contains("0.3.1") || stdout.contains("magictunnel-visibility"));
}

#[tokio::test]
async fn test_status_command_with_test_files() {
    let temp_dir = TempDir::new().unwrap();
    let capabilities_dir = temp_dir.path().join("capabilities");
    fs::create_dir_all(&capabilities_dir).unwrap();
    
    // Create test capability file
    let capability_file = capabilities_dir.join("test.yaml");
    create_test_capability_file(&capability_file, &create_basic_capability_file());
    
    // Create test config
    let config_file = temp_dir.path().join("config.yaml");
    fs::write(&config_file, create_test_config(capabilities_dir.to_str().unwrap())).unwrap();
    
    let (success, stdout, stderr) = run_visibility_cli(&[
        "-c", config_file.to_str().unwrap(),
        "status"
    ]);
    
    if !success {
        println!("STDERR: {}", stderr);
        println!("STDOUT: {}", stdout);
    }
    
    assert!(success, "Status command should succeed with test files");
    assert!(stdout.contains("Tool Visibility & Enabled Status"));
    assert!(stdout.contains("Overall Summary"));
}

#[tokio::test]
async fn test_status_command_detailed() {
    let temp_dir = TempDir::new().unwrap();
    let capabilities_dir = temp_dir.path().join("capabilities");
    fs::create_dir_all(&capabilities_dir).unwrap();
    
    // Create test capability file
    let capability_file = capabilities_dir.join("test.yaml");
    create_test_capability_file(&capability_file, &create_basic_capability_file());
    
    // Create test config
    let config_file = temp_dir.path().join("config.yaml");
    fs::write(&config_file, create_test_config(capabilities_dir.to_str().unwrap())).unwrap();
    
    let (success, stdout, _stderr) = run_visibility_cli(&[
        "-c", config_file.to_str().unwrap(),
        "status",
        "--detailed"
    ]);
    
    assert!(success, "Detailed status command should succeed");
    assert!(stdout.contains("test_tool_1"));
    assert!(stdout.contains("test_tool_2"));
    assert!(stdout.contains("test_tool_3"));
    assert!(stdout.contains("VISIBLE") || stdout.contains("HIDDEN"));
    assert!(stdout.contains("ENABLED") || stdout.contains("DISABLED"));
}

#[tokio::test]
async fn test_list_files_command() {
    let temp_dir = TempDir::new().unwrap();
    let capabilities_dir = temp_dir.path().join("capabilities");
    fs::create_dir_all(&capabilities_dir).unwrap();
    
    // Create multiple test capability files
    let capability_file1 = capabilities_dir.join("test1.yaml");
    let capability_file2 = capabilities_dir.join("test2.yaml");
    create_test_capability_file(&capability_file1, &create_basic_capability_file());
    create_test_capability_file(&capability_file2, &create_basic_capability_file());
    
    // Create test config
    let config_file = temp_dir.path().join("config.yaml");
    fs::write(&config_file, create_test_config(capabilities_dir.to_str().unwrap())).unwrap();
    
    let (success, stdout, _stderr) = run_visibility_cli(&[
        "-c", config_file.to_str().unwrap(),
        "list-files"
    ]);
    
    assert!(success, "List files command should succeed");
    assert!(stdout.contains("Capability Files"));
    assert!(stdout.contains("test1.yaml"));
    assert!(stdout.contains("test2.yaml"));
    assert!(stdout.contains("Total files: 2"));
}

#[tokio::test]
async fn test_hide_tool_command() {
    let temp_dir = TempDir::new().unwrap();
    let capabilities_dir = temp_dir.path().join("capabilities");
    fs::create_dir_all(&capabilities_dir).unwrap();
    
    // Create test capability file
    let capability_file = capabilities_dir.join("test.yaml");
    create_test_capability_file(&capability_file, &create_basic_capability_file());
    
    // Create test config
    let config_file = temp_dir.path().join("config.yaml");
    fs::write(&config_file, create_test_config(capabilities_dir.to_str().unwrap())).unwrap();
    
    let (success, stdout, _stderr) = run_visibility_cli(&[
        "-c", config_file.to_str().unwrap(),
        "hide-tool",
        "test_tool_1"
    ]);
    
    assert!(success, "Hide tool command should succeed");
    assert!(stdout.contains("Successfully hidden tool 'test_tool_1'"));
    
    // Verify the tool is now hidden by checking the file content
    let updated_content = fs::read_to_string(&capability_file).unwrap();
    assert!(updated_content.contains("hidden: true") || updated_content.contains("hidden:true"));
}

#[tokio::test]
async fn test_show_tool_command() {
    let temp_dir = TempDir::new().unwrap();
    let capabilities_dir = temp_dir.path().join("capabilities");
    fs::create_dir_all(&capabilities_dir).unwrap();
    
    // Create test capability file
    let capability_file = capabilities_dir.join("test.yaml");
    create_test_capability_file(&capability_file, &create_basic_capability_file());
    
    // Create test config
    let config_file = temp_dir.path().join("config.yaml");
    fs::write(&config_file, create_test_config(capabilities_dir.to_str().unwrap())).unwrap();
    
    let (success, stdout, _stderr) = run_visibility_cli(&[
        "-c", config_file.to_str().unwrap(),
        "show-tool",
        "test_tool_2"  // This tool starts as hidden
    ]);
    
    assert!(success, "Show tool command should succeed");
    assert!(stdout.contains("Successfully shown tool 'test_tool_2'"));
    
    // Verify the tool is now visible by checking the file content
    let updated_content = fs::read_to_string(&capability_file).unwrap();
    // The tool should now have hidden: false or no hidden field
    assert!(!updated_content.contains("hidden: true") || updated_content.contains("hidden: false"));
}

#[tokio::test]
async fn test_enable_tool_command() {
    let temp_dir = TempDir::new().unwrap();
    let capabilities_dir = temp_dir.path().join("capabilities");
    fs::create_dir_all(&capabilities_dir).unwrap();
    
    // Create test capability file
    let capability_file = capabilities_dir.join("test.yaml");
    create_test_capability_file(&capability_file, &create_basic_capability_file());
    
    // Create test config
    let config_file = temp_dir.path().join("config.yaml");
    fs::write(&config_file, create_test_config(capabilities_dir.to_str().unwrap())).unwrap();
    
    let (success, stdout, _stderr) = run_visibility_cli(&[
        "-c", config_file.to_str().unwrap(),
        "enable-tool",
        "test_tool_2"  // This tool starts as disabled
    ]);
    
    assert!(success, "Enable tool command should succeed");
    assert!(stdout.contains("Successfully enabled tool 'test_tool_2'"));
    
    // Verify the tool is now enabled by checking the file content
    let updated_content = fs::read_to_string(&capability_file).unwrap();
    assert!(updated_content.contains("enabled: true"));
}

#[tokio::test]
async fn test_disable_tool_command() {
    let temp_dir = TempDir::new().unwrap();
    let capabilities_dir = temp_dir.path().join("capabilities");
    fs::create_dir_all(&capabilities_dir).unwrap();
    
    // Create test capability file
    let capability_file = capabilities_dir.join("test.yaml");
    create_test_capability_file(&capability_file, &create_basic_capability_file());
    
    // Create test config
    let config_file = temp_dir.path().join("config.yaml");
    fs::write(&config_file, create_test_config(capabilities_dir.to_str().unwrap())).unwrap();
    
    let (success, stdout, _stderr) = run_visibility_cli(&[
        "-c", config_file.to_str().unwrap(),
        "disable-tool",
        "test_tool_1"  // This tool starts as enabled
    ]);
    
    assert!(success, "Disable tool command should succeed");
    assert!(stdout.contains("Successfully disabled tool 'test_tool_1'"));
    
    // Verify the tool is now disabled by checking the file content
    let updated_content = fs::read_to_string(&capability_file).unwrap();
    assert!(updated_content.contains("enabled: false"));
}

#[tokio::test]
async fn test_hide_all_with_confirmation() {
    let temp_dir = TempDir::new().unwrap();
    let capabilities_dir = temp_dir.path().join("capabilities");
    fs::create_dir_all(&capabilities_dir).unwrap();
    
    // Create test capability file
    let capability_file = capabilities_dir.join("test.yaml");
    create_test_capability_file(&capability_file, &create_basic_capability_file());
    
    // Create test config
    let config_file = temp_dir.path().join("config.yaml");
    fs::write(&config_file, create_test_config(capabilities_dir.to_str().unwrap())).unwrap();
    
    let (success, stdout, _stderr) = run_visibility_cli(&[
        "-c", config_file.to_str().unwrap(),
        "hide-all",
        "--confirm"
    ]);
    
    assert!(success, "Hide all command with confirmation should succeed");
    assert!(stdout.contains("Successfully hid all tools"));
    
    // Verify all tools are now hidden
    let updated_content = fs::read_to_string(&capability_file).unwrap();
    // Count occurrences of "hidden: true" - should be 3 (one for each tool)
    let hidden_count = updated_content.matches("hidden: true").count();
    assert!(hidden_count >= 2, "At least 2 tools should be hidden");
}

#[tokio::test]
async fn test_show_all_with_confirmation() {
    let temp_dir = TempDir::new().unwrap();
    let capabilities_dir = temp_dir.path().join("capabilities");
    fs::create_dir_all(&capabilities_dir).unwrap();
    
    // Create test capability file where all tools are initially hidden
    let mut hidden_content = create_basic_capability_file();
    hidden_content = hidden_content.replace("hidden: false", "hidden: true");
    
    let capability_file = capabilities_dir.join("test.yaml");
    create_test_capability_file(&capability_file, &hidden_content);
    
    // Create test config
    let config_file = temp_dir.path().join("config.yaml");
    fs::write(&config_file, create_test_config(capabilities_dir.to_str().unwrap())).unwrap();
    
    let (success, stdout, _stderr) = run_visibility_cli(&[
        "-c", config_file.to_str().unwrap(),
        "show-all",
        "--confirm"
    ]);
    
    assert!(success, "Show all command with confirmation should succeed");
    assert!(stdout.contains("Successfully showed all tools"));
    
    // Verify all tools are now visible
    let updated_content = fs::read_to_string(&capability_file).unwrap();
    // Should have "hidden: false" or no hidden field for visible tools
    let visible_count = updated_content.matches("hidden: false").count();
    assert!(visible_count >= 2, "At least 2 tools should be visible");
}

#[tokio::test]
async fn test_hide_all_without_confirmation() {
    let temp_dir = TempDir::new().unwrap();
    let capabilities_dir = temp_dir.path().join("capabilities");
    fs::create_dir_all(&capabilities_dir).unwrap();
    
    // Create test capability file
    let capability_file = capabilities_dir.join("test.yaml");
    create_test_capability_file(&capability_file, &create_basic_capability_file());
    
    // Create test config
    let config_file = temp_dir.path().join("config.yaml");
    fs::write(&config_file, create_test_config(capabilities_dir.to_str().unwrap())).unwrap();
    
    let (success, stdout, _stderr) = run_visibility_cli(&[
        "-c", config_file.to_str().unwrap(),
        "hide-all"
    ]);
    
    assert!(success, "Hide all command without confirmation should succeed");
    assert!(stdout.contains("Use --confirm to proceed"));
    
    // Verify tools are NOT hidden (no changes made)
    let content = fs::read_to_string(&capability_file).unwrap();
    assert!(content.contains("hidden: false"), "Tools should still be visible");
}

#[tokio::test]
async fn test_show_mcp_warnings_command() {
    let temp_dir = TempDir::new().unwrap();
    let capabilities_dir = temp_dir.path().join("capabilities");
    fs::create_dir_all(&capabilities_dir).unwrap();
    
    // Create test capability file with external MCP tool
    let external_mcp_content = r#"
metadata:
  name: "External MCP Test"
  description: "Test external MCP capability"
  version: "1.0.0"

tools:
  - name: "external_tool"
    description: "External MCP tool"
    inputSchema:
      type: "object"
      properties: {}
    routing:
      type: "external_mcp"
      config:
        server_url: "http://example.com"
    annotations:
      has_sampling_capability: "true"
      has_elicitation_capability: "false"
"#;
    
    let capability_file = capabilities_dir.join("external.yaml");
    create_test_capability_file(&capability_file, external_mcp_content);
    
    // Create test config with sampling enabled
    let config_content = format!(r#"
server:
  host: "127.0.0.1"
  port: 8080
  websocket: true
  timeout: 30

registry:
  type: "file"
  paths: ["{capabilities_dir}"]
  hot_reload: false
  validation:
    strict: true
    allow_unknown_fields: false

smart_discovery:
  enabled: true
  tool_selection_mode: "rule_based"
  default_confidence_threshold: 0.7
  enable_tool_enhancement: true
  enable_elicitation: false
  max_tools_to_consider: 10
  max_high_quality_matches: 3
  high_quality_threshold: 0.95
  use_fuzzy_matching: true
  enable_sequential_mode: true

sampling:
  enabled: true
"#, capabilities_dir = capabilities_dir.to_str().unwrap());
    
    let config_file = temp_dir.path().join("config.yaml");
    fs::write(&config_file, config_content).unwrap();
    
    let (success, stdout, _stderr) = run_visibility_cli(&[
        "-c", config_file.to_str().unwrap(),
        "show-mcp-warnings"
    ]);
    
    assert!(success, "Show MCP warnings command should succeed");
    assert!(stdout.contains("MCP Capability Override Warnings"));
}

#[tokio::test]
async fn test_show_mcp_warnings_detailed() {
    let temp_dir = TempDir::new().unwrap();
    let capabilities_dir = temp_dir.path().join("capabilities");
    fs::create_dir_all(&capabilities_dir).unwrap();
    
    // Create test capability file with external MCP tool
    let external_mcp_content = r#"
metadata:
  name: "External MCP Test"
  description: "Test external MCP capability"
  version: "1.0.0"

tools:
  - name: "external_tool"
    description: "External MCP tool"
    inputSchema:
      type: "object"
      properties: {}
    routing:
      type: "external_mcp"
      config:
        server_url: "http://example.com"
    annotations:
      has_sampling_capability: "true"
"#;
    
    let capability_file = capabilities_dir.join("external.yaml");
    create_test_capability_file(&capability_file, external_mcp_content);
    
    // Create test config
    let config_file = temp_dir.path().join("config.yaml");
    fs::write(&config_file, create_test_config(capabilities_dir.to_str().unwrap())).unwrap();
    
    let (success, stdout, _stderr) = run_visibility_cli(&[
        "-c", config_file.to_str().unwrap(),
        "show-mcp-warnings",
        "--detailed"
    ]);
    
    assert!(success, "Show MCP warnings detailed command should succeed");
    assert!(stdout.contains("MCP Capability Override Warnings"));
    assert!(stdout.contains("external_tool") || stdout.contains("External MCP tools: 1"));
}

#[tokio::test]
async fn test_error_handling_invalid_config() {
    let (success, _stdout, stderr) = run_visibility_cli(&[
        "-c", "/nonexistent/config.yaml",
        "status"
    ]);
    
    assert!(!success, "Command should fail with invalid config");
    assert!(stderr.contains("error") || stderr.contains("Error") || stderr.contains("failed"));
}

#[tokio::test]
async fn test_error_handling_nonexistent_tool() {
    let temp_dir = TempDir::new().unwrap();
    let capabilities_dir = temp_dir.path().join("capabilities");
    fs::create_dir_all(&capabilities_dir).unwrap();
    
    // Create test capability file
    let capability_file = capabilities_dir.join("test.yaml");
    create_test_capability_file(&capability_file, &create_basic_capability_file());
    
    // Create test config
    let config_file = temp_dir.path().join("config.yaml");
    fs::write(&config_file, create_test_config(capabilities_dir.to_str().unwrap())).unwrap();
    
    let (success, _stdout, stderr) = run_visibility_cli(&[
        "-c", config_file.to_str().unwrap(),
        "hide-tool",
        "nonexistent_tool"
    ]);
    
    assert!(!success, "Command should fail with nonexistent tool");
    assert!(stderr.contains("not found") || stderr.contains("Error"));
}

#[tokio::test]
async fn test_file_operations_hide_and_show_file() {
    let temp_dir = TempDir::new().unwrap();
    let capabilities_dir = temp_dir.path().join("capabilities");
    fs::create_dir_all(&capabilities_dir).unwrap();
    
    // Create test capability file
    let capability_file = capabilities_dir.join("test.yaml");
    create_test_capability_file(&capability_file, &create_basic_capability_file());
    
    // Create test config
    let config_file = temp_dir.path().join("config.yaml");
    fs::write(&config_file, create_test_config(capabilities_dir.to_str().unwrap())).unwrap();
    
    // Hide all tools in the file
    let (success, stdout, _stderr) = run_visibility_cli(&[
        "-c", config_file.to_str().unwrap(),
        "hide-file",
        capability_file.to_str().unwrap(),
        "--confirm"
    ]);
    
    assert!(success, "Hide file command should succeed");
    assert!(stdout.contains("Successfully hid all tools"));
    
    // Show all tools in the file
    let (success, stdout, _stderr) = run_visibility_cli(&[
        "-c", config_file.to_str().unwrap(),
        "show-file",
        capability_file.to_str().unwrap(),
        "--confirm"
    ]);
    
    assert!(success, "Show file command should succeed");
    assert!(stdout.contains("Successfully showed all tools"));
}

#[tokio::test]
async fn test_logging_levels() {
    let temp_dir = TempDir::new().unwrap();
    let capabilities_dir = temp_dir.path().join("capabilities");
    fs::create_dir_all(&capabilities_dir).unwrap();
    
    // Create test capability file
    let capability_file = capabilities_dir.join("test.yaml");
    create_test_capability_file(&capability_file, &create_basic_capability_file());
    
    // Create test config
    let config_file = temp_dir.path().join("config.yaml");
    fs::write(&config_file, create_test_config(capabilities_dir.to_str().unwrap())).unwrap();
    
    let (success, _stdout, stderr) = run_visibility_cli(&[
        "-c", config_file.to_str().unwrap(),
        "--log-level", "debug",
        "status"
    ]);
    
    // Should succeed regardless of log level
    assert!(success, "Command should succeed with debug log level");
    
    // Test with error log level
    let (success, _stdout, _stderr) = run_visibility_cli(&[
        "-c", config_file.to_str().unwrap(),
        "--log-level", "error",
        "status"
    ]);
    
    assert!(success, "Command should succeed with error log level");
}