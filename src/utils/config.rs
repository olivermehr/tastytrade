use crate::utils::logger::setup_logger_with_level;
use crate::{TastyTrade, TastyTradeError};
use pretty_simple_display::{DebugPretty, DisplaySimple};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::Path;

const BASE_DEMO_URL: &str = "https://api.cert.tastyworks.com";
const BASE_URL: &str = "https://api.tastyworks.com";

const WEBSOCKET_DEMO_URL: &str = "wss://streamer.cert.tastyworks.com";

const WEBSOCKET_URL: &str = "wss://streamer.tastyworks.com";

/// Configuration structure for the application
/// Handles environment variables and logger setup
#[derive(DebugPretty, DisplaySimple, Clone, Serialize, Deserialize)]
pub struct TastyTradeConfig {
    /// TastyTrade Oauth Client ID
    pub client_id: String,
    /// TastyTrade Oauth Client Secret
    #[serde(skip_serializing, default)]
    pub client_secret: String,
    /// TastyTrade Oauth Refresh Token
    #[serde(skip_serializing, default)]
    pub refresh_token: String,
    /// Whether to use demo/cert environment
    pub use_demo: bool,
    /// Log level: "INFO", "DEBUG", "WARN", "ERROR", "TRACE"
    pub log_level: String,
    /// Base URL for API requests
    pub base_url: String,
    /// Websocket URL.
    pub websocket_url: String,
}

impl Default for TastyTradeConfig {
    fn default() -> Self {
        Self {
            client_id: String::new(),
            client_secret: String::new(),
            refresh_token: String::new(),
            use_demo: false,
            log_level: "INFO".to_string(),
            base_url: BASE_URL.to_string(),
            websocket_url: WEBSOCKET_URL.to_string(),
        }
    }
}

impl TastyTradeConfig {
    /// Creates a new instance of the type by loading configuration or setup
    /// details from the environment.
    ///
    /// This function is a constructor that initializes the object by calling
    /// `from_env()`, which is expected to handle the process of reading and
    /// setting up values from the environment context (e.g., environment variables).
    ///
    /// # Returns
    /// A new instance of the type.
    ///
    pub fn new() -> Self {
        Self::from_env()
    }

    /// Initialize a new configuration from environment variables
    pub fn from_env() -> Self {
        dotenv::dotenv().ok();
        let client_id = env::var("TASTYTRADE_CLIENT_ID").unwrap_or_default();
        let client_secret = env::var("TASTYTRADE_CLIENT_SECRET").unwrap_or_default();
        let use_demo = env::var("TASTYTRADE_USE_DEMO")
            .unwrap_or_else(|_| "false".to_string())
            .parse()
            .unwrap_or(false);
        let log_level = env::var("LOGLEVEL").unwrap_or_else(|_| "INFO".to_string());
        let refresh_token = env::var("TASTYTRADE_REFRESH_TOKEN").unwrap_or_default();

        // Initialize logger with the specified log level
        setup_logger_with_level(&log_level);

        Self {
            client_id,
            client_secret,
            refresh_token,
            use_demo,
            log_level,
            base_url: if use_demo {
                BASE_DEMO_URL.to_string()
            } else {
                BASE_URL.to_string()
            },
            websocket_url: if use_demo {
                WEBSOCKET_DEMO_URL.to_string()
            } else {
                WEBSOCKET_URL.to_string()
            },
        }
    }

    /// Load configuration from a JSON file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, TastyTradeError> {
        let contents = fs::read_to_string(path)?;
        let config: TastyTradeConfig = serde_json::from_str(&contents)?;

        // Initialize logger with the log level from the config file
        setup_logger_with_level(&config.log_level);

        Ok(config)
    }

    /// Save configuration to a JSON file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), TastyTradeError> {
        let contents = serde_json::to_string_pretty(self)?;
        fs::write(path, contents)?;
        Ok(())
    }

    /// Check if the configuration has valid credentials
    pub fn has_valid_credentials(&self) -> bool {
        !self.client_id.is_empty()
            && !self.client_secret.is_empty()
            && !self.refresh_token.is_empty()
    }

    /// Creates a TastyTrade client from the configuration
    pub async fn create_client(&self) -> Result<TastyTrade, TastyTradeError> {
        if !self.has_valid_credentials() {
            "Missing TastyTrade credentials. Please set TASTYTRADE_USERNAME and TASTYTRADE_PASSWORD \
            environment variables or load from config file.".to_string();
        }

        let client = TastyTrade::login(self).await?;
        Ok(client)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;

    #[test]
    fn test_default_config() {
        let config = TastyTradeConfig::default();
        assert!(config.client_id.is_empty());
        assert!(config.client_secret.is_empty());
        assert!(config.refresh_token.is_empty());
        assert!(!config.use_demo);
        assert_eq!(config.log_level, "INFO");
    }

    #[test]
    #[serial]
    fn test_config_from_env() {
        // Set environment variables for testing
        unsafe {
            env::set_var("TASTYTRADE_CLIENT_ID", "test_client_id");
            env::set_var("TASTYTRADE_CLIENT_SECRET", "test_client_secret");
            env::set_var("TASTYTRADE_REFRESH_TOKEN", "test_refresh_token");
            env::set_var("TASTYTRADE_USE_DEMO", "true");
            env::set_var("LOGLEVEL", "DEBUG");
        }
        let config = TastyTradeConfig::from_env();
        assert_eq!(config.client_id, "test_client_id");
        assert_eq!(config.client_secret, "test_client_secret");
        assert_eq!(config.refresh_token, "test_refresh_token");
        assert!(config.use_demo);
        assert_eq!(config.base_url, BASE_DEMO_URL.to_string());
        assert_eq!(config.websocket_url, WEBSOCKET_DEMO_URL.to_string());

        unsafe {
            // Clean up environment
            env::remove_var("TASTYTRADE_CLIENT_ID");
            env::remove_var("TASTYTRADE_CLIENT_SECRET");
            env::remove_var("TASTYTRADE_REFRESH_TOKEN");
            env::remove_var("TASTYTRADE_USE_DEMO");
            env::remove_var("LOGLEVEL");
        }
    }

    #[test]
    fn test_has_valid_credentials() {
        let mut config = TastyTradeConfig::default();
        assert!(!config.has_valid_credentials());

        config.client_id = "client_id".to_string();
        assert!(!config.has_valid_credentials());

        config.client_secret = "client_secret".to_string();
        assert!(!config.has_valid_credentials());

        config.refresh_token = "refresh_token".to_string();
        assert!(config.has_valid_credentials());
    }

    #[test]
    fn test_serialize_deserialize() {
        let config = TastyTradeConfig {
            client_id: "test_client_id".to_string(),
            client_secret: "test_client_secret".to_string(),
            refresh_token: "test_refresh_token".to_string(),
            use_demo: true,
            log_level: "DEBUG".to_string(),
            base_url: BASE_DEMO_URL.to_string(),
            websocket_url: WEBSOCKET_DEMO_URL.to_string(),
        };

        let json = serde_json::to_string(&config).unwrap();

        // Client secret should be skipped during serialization
        assert!(!json.contains("test_client_secret"));
        // Refresh token should be skipped during serialization
        assert!(!json.contains("test_refresh_token"));

        // Create a new config with an empty client secret
        let mut deserialized: TastyTradeConfig = serde_json::from_str(&json).unwrap();

        // Manually set the client secret since it's not in the JSON
        deserialized.client_secret = "test_client_secret".to_string();
        // Manually set the refresh token since it's not in the JSON
        deserialized.refresh_token = "test_refresh_token".to_string();

        assert_eq!(config.client_id, deserialized.client_id);
        assert_eq!(config.client_secret, deserialized.client_secret);
        assert_eq!(config.refresh_token, deserialized.refresh_token);
        assert_eq!(config.use_demo, deserialized.use_demo);
        assert_eq!(config.log_level, deserialized.log_level);
    }

    #[test]
    #[serial]
    fn test_config_from_env_demo_false() {
        // Clean up any existing environment variables first
        unsafe {
            env::remove_var("TASTYTRADE_CLIENT_ID");
            env::remove_var("TASTYTRADE_CLIENT_SECRET");
            env::remove_var("TASTYTRADE_REFRESH_TOKEN");
            env::remove_var("TASTYTRADE_USE_DEMO");
            env::remove_var("LOGLEVEL");
        }

        // Set environment variables for testing
        unsafe {
            env::set_var("TASTYTRADE_CLIENT_ID", "test_client_id");
            env::set_var("TASTYTRADE_CLIENT_SECRET", "test_client_secret");
            env::set_var("TASTYTRADE_REFRESH_TOKEN", "test_refresh_token");
            env::set_var("TASTYTRADE_USE_DEMO", "false");
            env::set_var("LOGLEVEL", "DEBUG");
        }
        let config = TastyTradeConfig::from_env();
        assert_eq!(config.client_id, "test_client_id");
        assert_eq!(config.client_secret, "test_client_secret");
        assert_eq!(config.refresh_token, "test_refresh_token");
        assert!(!config.use_demo);
        assert_eq!(config.base_url, BASE_URL.to_string());
        assert_eq!(config.websocket_url, WEBSOCKET_URL.to_string());

        unsafe {
            // Clean up environment
            env::remove_var("TASTYTRADE_CLIENT_ID");
            env::remove_var("TASTYTRADE_CLIENT_SECRET");
            env::remove_var("TASTYTRADE_REFRESH_TOKEN");
            env::remove_var("TASTYTRADE_USE_DEMO");
            env::remove_var("LOGLEVEL");
        }
    }
}
