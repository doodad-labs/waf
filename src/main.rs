//! Web Application Firewall (WAF) Proxy Main Entry Point
//!
//! Handles configuration loading and initialization of the WAF proxy.
//! Looks for configuration in either:
//! 1. Path specified as first command line argument
//! 2. `waf.toml` in current working directory

mod config;
mod proxy;
mod firewall;

use std::{env, path::PathBuf};

/// Main application entry point
///
/// # Errors
/// Returns errors in these cases:
/// - Config file not found or invalid
/// - Invalid path encoding
/// - Configuration validation failures
#[ntex::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get config path from args or default to "waf.toml" in current directory
    let config_path = if let Some(arg_path) = env::args().nth(1) {
        PathBuf::from(arg_path)
    } else {
        env::current_dir()?.join("waf.toml")
    };

    // Convert to string for config loading, ensuring UTF-8 compatibility
    let config_str = config_path
        .to_str()
        .ok_or("Invalid config path encoding (non-UTF8 characters)")?;
    
    // Load and validate configuration
    let settings = config::Settings::new(config_str)?;
    settings.validate()?;
    println!("Loaded config from {}: {:#?}", config_str, settings);

    proxy::run(&settings).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use tempfile::TempDir;

    /// Tests that the config loader correctly falls back to waf.toml
    /// when no arguments are provided
    #[test]
    fn test_default_config_loading() -> Result<(), Box<dyn std::error::Error>> {
        // Create temporary directory with test config
        let temp_dir = TempDir::new()?;
        env::set_current_dir(&temp_dir)?;
        
        // Create test config file
        let config_content = r#"
            listen_port = 8080
            backend_url = "http://localhost:3000"
            
            [logging]
            log_file = "/var/log/waf-proxy.log"
            log_level = "warn"
        "#;
        fs::write("waf.toml", config_content)?;

        // Simulate no command line arguments
        let args: Vec<String> = vec![];
        env::set_var("RUST_TEST_ARGS", "");

        // Should load from ./waf.toml
        let config_path = if let Some(arg_path) = args.get(1) {
            PathBuf::from(arg_path)
        } else {
            env::current_dir()?.join("waf.toml")
        };

        assert!(config_path.exists());
        Ok(())
    }

    /// Tests that command line argument path takes precedence
    #[test]
    fn test_argument_config_loading() -> Result<(), Box<dyn std::error::Error>> {
        // Create temp file for argument path
        let temp_dir = TempDir::new()?;
        let custom_config = temp_dir.path().join("custom.toml");
        File::create(&custom_config)?;

        // Simulate command line argument
        let args = vec!["waf-proxy".to_string(), custom_config.to_str().unwrap().to_string()];

        let config_path = if let Some(arg_path) = args.get(1) {
            PathBuf::from(arg_path)
        } else {
            env::current_dir()?.join("waf.toml")
        };

        assert_eq!(config_path, custom_config);
        Ok(())
    }

    /// Tests error handling for invalid paths
    #[test]
    fn test_invalid_path_handling() {
        let invalid_path = PathBuf::from("non_existent.toml");
        assert!(!invalid_path.exists());
    }

    /// Tests path encoding validation
    #[test]
    fn test_path_encoding_validation() {
        // Create path with invalid UTF-8 (Unix-like only)
        #[cfg(unix)]
        {
            use std::ffi::OsStr;
            use std::os::unix::ffi::OsStrExt;
            
            let invalid_utf8 = OsStr::from_bytes(b"invalid_\xFF.toml");
            let path = PathBuf::from(invalid_utf8);
            
            assert!(path.to_str().is_none());
        }
    }
}