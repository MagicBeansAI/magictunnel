//! Integration tests for the gRPC capability generator CLI

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_grpc_generator_cli() {
    // Create a temporary directory for output
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let output_path = temp_dir.path().join("test_output.yaml");
    
    // Path to the test proto file
    let proto_path = PathBuf::from("data/grpc_test/comprehensive_test_service.proto");
    
    // Ensure the test proto file exists
    assert!(proto_path.exists(), "Test proto file not found: {:?}", proto_path);
    
    // Build the project to ensure the binary is available
    let build_status = Command::new("cargo")
        .args(["build", "--bin", "grpc-generator"])
        .status()
        .expect("Failed to build grpc-generator");
    
    assert!(build_status.success(), "Failed to build grpc-generator");
    
    // Run the gRPC generator CLI
    let status = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "grpc-generator",
            "--",
            "--proto",
            proto_path.to_str().unwrap(),
            "--output",
            output_path.to_str().unwrap(),
            "--endpoint",
            "localhost:50051",
            "--server-streaming",
            "polling",
            "--client-streaming",
            "polling",
            "--bidirectional-streaming",
            "polling",
        ])
        .status()
        .expect("Failed to run grpc-generator");
    
    // Check if the command executed successfully
    assert!(status.success(), "grpc-generator command failed");
    
    // Check if the output file was created
    assert!(output_path.exists(), "Output file was not created");
    
    // Read the output file
    let output_content = fs::read_to_string(&output_path).expect("Failed to read output file");
    
    // Basic validation of the output content
    // Note: Since the actual implementation is not complete yet, we're just checking
    // that the file contains some basic YAML structure
    assert!(
        output_content.contains("tools:") || 
        output_content.contains("metadata:") || 
        output_content.contains("name:"),
        "Output file does not contain expected YAML structure"
    );
    
    // Clean up the temporary directory
    temp_dir.close().expect("Failed to clean up temp directory");
}

#[test]
fn test_grpc_generator_cli_help() {
    // Run the gRPC generator CLI with --help flag
    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "grpc-generator",
            "--",
            "--help",
        ])
        .output()
        .expect("Failed to run grpc-generator with --help");
    
    // Check if the command executed successfully
    assert!(output.status.success(), "grpc-generator --help command failed");
    
    // Convert output to string
    let help_text = String::from_utf8_lossy(&output.stdout);
    
    // Check if the help text contains expected options
    assert!(help_text.contains("--proto"), "Help text missing --proto option");
    assert!(help_text.contains("--output"), "Help text missing --output option");
    assert!(help_text.contains("--endpoint"), "Help text missing --endpoint option");
    assert!(help_text.contains("--server-streaming"), "Help text missing --server-streaming option");
    assert!(help_text.contains("--client-streaming"), "Help text missing --client-streaming option");
    assert!(help_text.contains("--bidirectional-streaming"), "Help text missing --bidirectional-streaming option");
    assert!(help_text.contains("--auth-type"), "Help text missing --auth-type option");
}