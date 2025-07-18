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

    /// TLS configuration
    #[serde(default)]
    pub tls: Tls,
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

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub struct Tls {
    #[serde(default)]
    pub tls_enabled: bool,

    #[serde(default)]
    pub tls_cert_path: Option<PathBuf>,

    #[serde(default)]
    pub tls_key_path: Option<PathBuf>,
}

// Default implementations
impl Default for Settings {
    fn default() -> Self {
        Self {
            listen_port: 0,  // Still requires explicit setting
            webapp_url: String::new(),  // Still requires explicit setting
            logging: Logging::default(),
            threading: Threading::default(),
            tls: Tls::default(),
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

impl Default for Tls {
    fn default() -> Self {
        Self {
            tls_enabled: false,
            tls_cert_path: None,
            tls_key_path: None,
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

        // TLS validation
        if self.tls.tls_enabled {
            if self.tls.tls_cert_path.is_none() || self.tls.tls_key_path.is_none() {
                return Err("TLS is enabled but certificate or key path is not set".to_string());
            }

            // TLS paths validation

            if let Some(cert_path) = &self.tls.tls_cert_path {
                if !cert_path.exists() {
                    return Err(format!("TLS certificate path does not exist: {}", cert_path.display()));
                }
            }

            if let Some(key_path) = &self.tls.tls_key_path {
                if !key_path.exists() {
                    return Err(format!("TLS key path does not exist: {}", key_path.display()));
                }
            }

        }
        
        Ok(())
    }
}