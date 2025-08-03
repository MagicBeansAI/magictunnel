use std::env;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Compile protobuf files
    tonic_build::compile_protos("proto/mcp.proto")?;

    // Update version information in key files
    update_version_info();

    Ok(())
}

fn update_version_info() {
    // Get package info from Cargo.toml
    let pkg_name = env::var("CARGO_PKG_NAME").unwrap_or_else(|_| "magictunnel".to_string());
    let pkg_version = env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.3.0".to_string());

    // Update version in key files
    update_json_file("test-resources/info.json", &pkg_name, &pkg_version);
    update_config_file("config.yaml.template", &pkg_version);
    update_config_file("magictunnel-config.yaml", &pkg_version);
    update_test_file("tests/test_config_validation.rs", &pkg_name, &pkg_version);
    update_test_file("tests/mcp_external_tests.rs", &pkg_name, &pkg_version);
    update_source_file("src/mcp/server.rs", &pkg_name, &pkg_version);
    update_source_file("src/auth/oauth.rs", &pkg_name, &pkg_version);

    println!("cargo:rerun-if-changed=Cargo.toml");
}

fn update_json_file(file_path: &str, name: &str, version: &str) {
    if let Ok(content) = fs::read_to_string(file_path) {
        let updated = content
            .replace(r#""version": "0.2.49""#, &format!(r#""version": "{}""#, version))
            .replace(r#""version": "0.2.48""#, &format!(r#""version": "{}""#, version))
            .replace(r#""version": "0.2.47""#, &format!(r#""version": "{}""#, version))
            .replace(r#""name": "MagicTunnel""#, &format!(r#""name": "{}""#, name));

        if let Err(e) = fs::write(file_path, updated) {
            println!("cargo:warning=Failed to update {}: {}", file_path, e);
        }
    }
}

fn update_config_file(file_path: &str, version: &str) {
    if let Ok(content) = fs::read_to_string(file_path) {
        let updated = content
            .replace(r#"client_version: "0.2.49""#, &format!(r#"client_version: "{}""#, version))
            .replace(r#"client_version: "0.2.48""#, &format!(r#"client_version: "{}""#, version))
            .replace(r#"client_version: "0.2.47""#, &format!(r#"client_version: "{}""#, version))
            .replace(r#"client_version: "0.2.37""#, &format!(r#"client_version: "{}""#, version));

        if let Err(e) = fs::write(file_path, updated) {
            println!("cargo:warning=Failed to update {}: {}", file_path, e);
        }
    }
}

fn update_test_file(file_path: &str, name: &str, version: &str) {
    if let Ok(content) = fs::read_to_string(file_path) {
        let updated = content
            .replace(r#"client_version: "0.2.49".to_string()"#, &format!(r#"client_version: "{}".to_string()"#, version))
            .replace(r#"client_version: "0.2.48".to_string()"#, &format!(r#"client_version: "{}".to_string()"#, version))
            .replace(r#"client_version: "0.2.47".to_string()"#, &format!(r#"client_version: "{}".to_string()"#, version))
            .replace(r#"client_version: "0.2.14".to_string()"#, &format!(r#"client_version: "{}".to_string()"#, version))
            .replace(r#"client_name: "magictunnel".to_string()"#, &format!(r#"client_name: "{}".to_string()"#, name))
            .replace(r#"client_name: "magictunnel-test".to_string()"#, &format!(r#"client_name: "{}-test".to_string()"#, name));

        if let Err(e) = fs::write(file_path, updated) {
            println!("cargo:warning=Failed to update {}: {}", file_path, e);
        }
    }
}

fn update_source_file(file_path: &str, name: &str, version: &str) {
    if let Ok(content) = fs::read_to_string(file_path) {
        let updated = content
            .replace(r#""name": "magictunnel""#, &format!(r#""name": "{}""#, name))
            .replace(r#""service": "magictunnel""#, &format!(r#""service": "{}""#, name))
            .replace(r#""version": "0.2.49""#, &format!(r#""version": "{}""#, version))
            .replace(r#""version": "0.2.48""#, &format!(r#""version": "{}""#, version))
            .replace(r#""version": "0.2.47""#, &format!(r#""version": "{}""#, version))
            .replace(r#""magictunnel/0.2.49""#, &format!(r#""{}/{}""#, name, version))
            .replace(r#""magictunnel/0.2.48""#, &format!(r#""{}/{}""#, name, version))
            .replace(r#""magictunnel/0.2.47""#, &format!(r#""{}/{}""#, name, version));

        if let Err(e) = fs::write(file_path, updated) {
            println!("cargo:warning=Failed to update {}: {}", file_path, e);
        }
    }
}
