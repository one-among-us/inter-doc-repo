use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub server: ServerConfig,
    pub jwt: JwtConfig,
    pub google_oauth: GoogleOAuthConfig,
    pub google_drive: GoogleDriveConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JwtConfig {
    pub secret: String,
    pub expiry_hours: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GoogleOAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub allowed_email_domain: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GoogleDriveConfig {
    pub service_account_key_path: String,
    pub shared_drive_id: String,
    pub documents_index_file_id: String,
}

impl Config {
    pub fn load(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }
}
