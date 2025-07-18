use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(default)]  // This enables default values for missing fields
pub struct Settings {
    /// [REQUIRED] Listening port for the proxy
    pub listen_port: u16,
    
    /// [REQUIRED] Webapp server URL to forward requests to
    pub webapp_url: String,
    
    /// Logging configuration
    #[serde(default)]
    pub logging: Logging,

    /// Threading configuration
    #[serde(default)]
    pub threading: Threading,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub struct Logging {
    /// Path to log file (default: stdout)
    pub log_file: Option<PathBuf>,
    
    /// Log level (default: "info")
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub struct Threading {
    /// Worker count (default: 0 = auto-detect)
    #[serde(default)]
    pub workers: u16,
}

// Default implementations
impl Default for Settings {
    fn default() -> Self {
        Self {
            listen_port: 0,  // Still requires explicit setting
            webapp_url: String::new(),  // Still requires explicit setting
            logging: Logging::default(),
            threading: Threading::default(),
        }
    }
}

impl Default for Logging {
    fn default() -> Self {
        Self {
            log_file: None,
            log_level: default_log_level(),
        }
    }
}

impl Default for Threading {
    fn default() -> Self {
        Self {
            workers: 0,
        }
    }
}

// Helper for default values
fn default_log_level() -> String {
    "info".to_string()
}

impl Settings {
    pub fn new(config_path: &str) -> Result<Self, config::ConfigError> {
        let settings = config::Config::builder()
            .add_source(config::File::new(config_path, config::FileFormat::Toml))
            .add_source(config::Environment::with_prefix("APP"))
            .build()?;

        settings.try_deserialize()
    }

    pub fn validate(&self) -> Result<(), String> {
        // Port validation (0 is now allowed for OS-assigned ports)
        if self.listen_port == 0 {
            return Err("Port must be specified".to_string());
        }
        // Webapp URL validation
        if self.webapp_url.is_empty() {
            return Err("Webapp URL must be specified".to_string());
        }

        // Log level validation
        if !["trace", "debug", "info", "warn", "error"].contains(&self.logging.log_level.as_str()) {
            return Err("Invalid log level (must be trace/debug/info/warn/error)".to_string());
        }

        // Worker count validation (0 means auto-detect)
        if self.threading.workers > 0 && self.threading.workers > num_cpus::get() as u16 {
            return Err(format!(
                "Worker count cannot exceed physical CPU cores: {}",
                num_cpus::get()
            ));
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    // Helper to create minimal valid settings
    fn minimal_settings() -> Settings {
        Settings {
            listen_port: 8080,
            webapp_url: "http://localhost:3000".to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn test_minimal_config_loading() {
        let config_content = r#"
            listen_port = 8080
            webapp_url = "http://localhost:3000"
        "#;
        
        let tmp_file = NamedTempFile::new().unwrap();
        let tmp_path = tmp_file.path().to_str().unwrap().to_string() + ".toml";
        fs::write(&tmp_path, config_content).unwrap();
        
        let settings = Settings::new(&tmp_path).unwrap();
        
        assert_eq!(settings.listen_port, 8080);
        assert_eq!(settings.webapp_url, "http://localhost:3000");
        // Verify defaults
        assert_eq!(settings.logging.log_level, "info");
        assert_eq!(settings.threading.workers, 0);
        assert!(settings.logging.log_file.is_none());
        
        fs::remove_file(&tmp_path).unwrap();
    }

    #[test]
    fn test_full_config_loading() {
        let config_content = r#"
            listen_port = 9090
            webapp_url = "http://example.com"

            [threading]
            workers = 4
            
            [logging]
            log_file = "/var/log/waf.log"
            log_level = "debug"
        "#;
        
        let tmp_file = NamedTempFile::new().unwrap();
        let tmp_path = tmp_file.path().to_str().unwrap().to_string() + ".toml";
        fs::write(&tmp_path, config_content).unwrap();
        
        let settings = Settings::new(&tmp_path).unwrap();
        
        assert_eq!(settings.listen_port, 9090);
        assert_eq!(settings.webapp_url, "http://example.com");
        assert_eq!(settings.threading.workers, 4);
        assert_eq!(settings.logging.log_file.unwrap(), PathBuf::from("/var/log/waf.log"));
        assert_eq!(settings.logging.log_level, "debug");
        
        fs::remove_file(&tmp_path).unwrap();
    }

    #[test]
    fn test_config_default_values() {
        let settings = Settings::default();
        
        assert_eq!(settings.listen_port, 0);
        assert_eq!(settings.webapp_url, "");
        assert_eq!(settings.logging.log_level, "info");
        assert_eq!(settings.threading.workers, 0);
    }

    #[test]
    fn test_config_validation() {
        let mut settings = minimal_settings();
        assert!(settings.validate().is_ok());

        // Test invalid port
        settings.listen_port = 0;
        assert_eq!(settings.validate(), Err("Port must be specified".to_string()));
        settings.listen_port = 8080;

        // Test empty webapp URL
        settings.webapp_url = String::new();
        assert_eq!(settings.validate(), Err("Webapp URL must be specified".to_string()));
        settings.webapp_url = "http://valid".to_string();

        // Test invalid log level
        settings.logging.log_level = "invalid".to_string();
        assert!(settings.validate().is_err());
        settings.logging.log_level = "info".to_string();

        // Test excessive workers
        settings.threading.workers = 9999;
        assert!(settings.validate().is_err());
    }
}