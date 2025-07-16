use serde::Deserialize;

/// Main configuration struct (add new fields here when expanding)
#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct Settings {
    /// Listening port for the proxy (add new comments like this for future fields)
    pub listen_port: u16,
    
    /// Backend server URL to forward requests to
    pub backend_url: String,
    
    /// Logging configuration section
    pub logging: Logging,
    // Add new sections below following the same pattern:
    // pub new_section: NewSection,
}

/// Logging-specific settings (example of a config section)
#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct Logging {
    /// Path to log file
    pub log_file: String,
    
    /// Log level (trace, debug, info, warn, error)
    pub log_level: String,
}

impl Settings {
    /// Loads configuration from file
    /// # Arguments
    /// * `config_path` - Path to configuration file
    /// 
    /// # Example
    /// ```
    /// let settings = Settings::new("waf.toml").unwrap();
    /// ```
    pub fn new(config_path: &str) -> Result<Self, config::ConfigError> {
        let settings = config::Config::builder()
            // Explicitly specify TOML format
            .add_source(config::File::new(config_path, config::FileFormat::Toml))
            .build()?;

        settings.try_deserialize()
    }

    /// Validates configuration values
    pub fn validate(&self) -> Result<(), String> {
        if self.listen_port == 0 {
            return Err("Port cannot be 0".to_string());
        }
        
        if !["trace", "debug", "info", "warn", "error"].contains(&self.logging.log_level.as_str()) {
            return Err("Invalid log level".to_string());
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    
    #[test]
    fn test_config_loading() {
        // Create temporary config file with .toml extension
        let config_content = r#"
            listen_port = 8080
            backend_url = "http://localhost:3000"
            
            [logging]
            log_file = "/var/log/waf-proxy.log"
            log_level = "warn"
        "#;
        
        let tmp_file = NamedTempFile::new().unwrap();
        let tmp_path = tmp_file.path().to_str().unwrap().to_string() + ".toml";
        fs::write(&tmp_path, config_content).unwrap();
        
        // Test loading
        let settings = Settings::new(&tmp_path).unwrap();
        
        assert_eq!(settings.listen_port, 8080);
        assert_eq!(settings.backend_url, "http://localhost:3000");
        assert_eq!(settings.logging.log_file, "/var/log/waf-proxy.log");
        assert_eq!(settings.logging.log_level, "warn");
        
        // Clean up
        fs::remove_file(&tmp_path).unwrap();
    }

    #[test]
    fn test_config_validation() {
        let valid_settings = Settings {
            listen_port: 8080,
            backend_url: "http://localhost:3000".to_string(),
            logging: Logging {
                log_file: "/var/log/waf-proxy.log".to_string(),
                log_level: "warn".to_string(),
            },
        };
        
        assert!(valid_settings.validate().is_ok());
        
        // Test invalid cases
        let mut invalid_port = valid_settings.clone();
        invalid_port.listen_port = 0;
        assert!(invalid_port.validate().is_err());
        
        let mut invalid_log_level = valid_settings.clone();
        invalid_log_level.logging.log_level = "invalid".to_string();
        assert!(invalid_log_level.validate().is_err());
    }

    #[test]
    fn test_missing_config() {
        let result = Settings::new("nonexistent.toml");
        assert!(result.is_err());
    }

    // Add new test cases here when adding new fields
    // #[test]
    // fn test_new_feature() { ... }
}