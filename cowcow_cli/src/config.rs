use anyhow::{Context, Result};
use dirs::home_dir;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub api: ApiConfig,
    pub storage: StorageConfig,
    pub audio: AudioConfig,
    pub upload: UploadConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    pub endpoint: String,
    pub timeout_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub data_dir: PathBuf,
    pub auto_upload: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    pub sample_rate: u32,
    pub channels: u16,
    pub min_snr_db: f32,
    pub max_clipping_pct: f32,
    pub min_vad_ratio: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadConfig {
    pub max_retries: u32,
    pub retry_delay_secs: u64,
    pub chunk_size: usize,
}

impl Default for Config {
    fn default() -> Self {
        let data_dir = home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".cowcow");

        Self {
            api: ApiConfig {
                endpoint: "http://localhost:8000".to_string(),
                timeout_secs: 30,
            },
            storage: StorageConfig {
                data_dir,
                auto_upload: false,
            },
            audio: AudioConfig {
                sample_rate: 16000,
                channels: 1,
                min_snr_db: 20.0,
                max_clipping_pct: 1.0,
                min_vad_ratio: 80.0,
            },
            upload: UploadConfig {
                max_retries: 3,
                retry_delay_secs: 2,
                chunk_size: 1024 * 1024, // 1MB chunks
            },
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if config_path.exists() {
            let content = fs::read_to_string(&config_path).with_context(|| {
                format!("Failed to read config file: {}", config_path.display())
            })?;

            let config: Config = toml::from_str(&content).context(format!(
                "Failed to parse config file: {}",
                config_path.display()
            ))?;

            info!("Loaded config from: {}", config_path.display());
            Ok(config)
        } else {
            info!("Config file not found, creating default config");
            let config = Config::default();
            config.save()?;
            Ok(config)
        }
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;

        // Create directory if it doesn't exist
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create config directory: {}", parent.display())
            })?;
        }

        let content = toml::to_string_pretty(self).context("Failed to serialize config to TOML")?;

        fs::write(&config_path, content)
            .with_context(|| format!("Failed to write config file: {}", config_path.display()))?;

        info!("Saved config to: {}", config_path.display());
        Ok(())
    }

    fn config_path() -> Result<PathBuf> {
        let config_dir = home_dir()
            .context("Could not find home directory")?
            .join(".cowcow");

        Ok(config_dir.join("config.toml"))
    }

    pub fn data_dir(&self) -> &PathBuf {
        &self.storage.data_dir
    }

    pub fn recordings_dir(&self) -> PathBuf {
        self.storage.data_dir.join("recordings")
    }

    pub fn database_path(&self) -> PathBuf {
        self.storage.data_dir.join("cowcow.db")
    }

    pub fn credentials_path(&self) -> PathBuf {
        self.storage.data_dir.join("credentials.json")
    }

    pub fn validate(&self) -> Result<()> {
        // Validate API endpoint
        if !self.api.endpoint.starts_with("http://") && !self.api.endpoint.starts_with("https://") {
            return Err(anyhow::anyhow!(
                "API endpoint must start with http:// or https://"
            ));
        }

        // Validate timeout
        if self.api.timeout_secs == 0 {
            return Err(anyhow::anyhow!("API timeout must be greater than 0"));
        }

        // Validate audio settings
        if self.audio.sample_rate == 0 {
            return Err(anyhow::anyhow!("Sample rate must be greater than 0"));
        }

        if self.audio.channels == 0 {
            return Err(anyhow::anyhow!("Channel count must be greater than 0"));
        }

        Ok(())
    }

    pub fn set_value(&mut self, key: &str, value: &str) -> Result<()> {
        match key {
            "api.endpoint" => {
                if !value.starts_with("http://") && !value.starts_with("https://") {
                    return Err(anyhow::anyhow!(
                        "API endpoint must start with http:// or https://"
                    ));
                }
                self.api.endpoint = value.to_string();
            }
            "api.timeout_secs" => {
                self.api.timeout_secs = value
                    .parse::<u64>()
                    .context("Invalid timeout value, must be a positive integer")?;
            }
            "storage.auto_upload" => {
                self.storage.auto_upload = value
                    .parse::<bool>()
                    .context("Invalid auto_upload value, must be true or false")?;
            }
            "audio.sample_rate" => {
                self.audio.sample_rate = value
                    .parse::<u32>()
                    .context("Invalid sample rate, must be a positive integer")?;
            }
            "audio.channels" => {
                self.audio.channels = value
                    .parse::<u16>()
                    .context("Invalid channel count, must be a positive integer")?;
            }
            "audio.min_snr_db" => {
                self.audio.min_snr_db = value
                    .parse::<f32>()
                    .context("Invalid SNR value, must be a number")?;
            }
            "audio.max_clipping_pct" => {
                self.audio.max_clipping_pct = value
                    .parse::<f32>()
                    .context("Invalid clipping percentage, must be a number between 0 and 100")?;
                if self.audio.max_clipping_pct < 0.0 || self.audio.max_clipping_pct > 100.0 {
                    return Err(anyhow::anyhow!(
                        "Clipping percentage must be between 0 and 100"
                    ));
                }
            }
            "audio.min_vad_ratio" => {
                self.audio.min_vad_ratio = value
                    .parse::<f32>()
                    .context("Invalid VAD ratio, must be a number between 0 and 1")?;
                if self.audio.min_vad_ratio < 0.0 || self.audio.min_vad_ratio > 1.0 {
                    return Err(anyhow::anyhow!("VAD ratio must be between 0 and 1"));
                }
            }
            "upload.max_retries" => {
                self.upload.max_retries = value
                    .parse::<u32>()
                    .context("Invalid max retries, must be a positive integer")?;
            }
            "upload.retry_delay_secs" => {
                self.upload.retry_delay_secs = value
                    .parse::<u64>()
                    .context("Invalid retry delay, must be a positive integer")?;
            }
            "upload.chunk_size" => {
                self.upload.chunk_size = value
                    .parse::<usize>()
                    .context("Invalid chunk size, must be a positive integer")?;
            }
            _ => {
                return Err(anyhow::anyhow!("Unknown configuration key: {}", key));
            }
        }

        // Validate the configuration after setting the value
        self.validate()?;

        Ok(())
    }

    pub fn get_available_keys() -> Vec<&'static str> {
        vec![
            "api.endpoint",
            "api.timeout_secs",
            "storage.auto_upload",
            "audio.sample_rate",
            "audio.channels",
            "audio.min_snr_db",
            "audio.max_clipping_pct",
            "audio.min_vad_ratio",
            "upload.max_retries",
            "upload.retry_delay_secs",
            "upload.chunk_size",
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credentials {
    pub access_token: Option<String>,
    pub api_key: Option<String>,
    pub username: Option<String>,
    pub expires_at: Option<u64>,
}

impl Credentials {
    pub fn load(config: &Config) -> Result<Option<Self>> {
        let creds_path = config.credentials_path();

        if creds_path.exists() {
            let content = fs::read_to_string(&creds_path).with_context(|| {
                format!("Failed to read credentials file: {}", creds_path.display())
            })?;

            let creds: Credentials = serde_json::from_str(&content).context(format!(
                "Failed to parse credentials file: {}",
                creds_path.display()
            ))?;

            Ok(Some(creds))
        } else {
            Ok(None)
        }
    }

    pub fn save(&self, config: &Config) -> Result<()> {
        let creds_path = config.credentials_path();

        // Create directory if it doesn't exist
        if let Some(parent) = creds_path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "Failed to create credentials directory: {}",
                    parent.display()
                )
            })?;
        }

        let content = serde_json::to_string_pretty(self)
            .context("Failed to serialize credentials to JSON")?;

        fs::write(&creds_path, content).with_context(|| {
            format!("Failed to write credentials file: {}", creds_path.display())
        })?;

        info!("Saved credentials to: {}", creds_path.display());
        Ok(())
    }

    pub fn is_valid(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();

            expires_at > now
        } else {
            false
        }
    }

    pub fn clear(config: &Config) -> Result<()> {
        let creds_path = config.credentials_path();

        if creds_path.exists() {
            fs::remove_file(&creds_path).with_context(|| {
                format!(
                    "Failed to remove credentials file: {}",
                    creds_path.display()
                )
            })?;
            info!("Cleared credentials");
        }

        Ok(())
    }
}
