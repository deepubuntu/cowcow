use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{error, info, warn};

use crate::config::{Config, Credentials};

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub token_type: String,
    pub api_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterResponse {
    pub id: u64,
    pub username: String,
    pub email: String,
    pub api_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenBalance {
    pub balance: u32,
    pub total_earned: u32,
    pub total_spent: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenTransaction {
    pub id: String,
    pub transaction_type: String,
    pub amount: i32,
    pub balance: u32,
    pub date: DateTime<Utc>,
    pub notes: String,
}

pub struct AuthClient {
    client: Client,
    config: Config,
}

impl AuthClient {
    pub fn new(config: Config) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.api.timeout_secs))
            .build()
            .unwrap();

        Self { client, config }
    }

    pub async fn login(&self, username: String, password: String) -> Result<Credentials> {
        let login_url = format!("{}/auth/token", self.config.api.endpoint);

        let form_data = [("username", username.clone()), ("password", password)];

        info!("Attempting login for user: {}", username);

        let response = self
            .client
            .post(&login_url)
            .form(&form_data)
            .send()
            .await
            .with_context(|| format!("Failed to send login request to {login_url}"))?;

        if response.status().is_success() {
            let login_response: LoginResponse = response
                .json()
                .await
                .context("Failed to parse login response")?;

            let expires_at = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                + (24 * 60 * 60); // 24 hours

            let credentials = Credentials {
                access_token: Some(login_response.access_token),
                api_key: Some(login_response.api_key),
                username: Some(username),
                expires_at: Some(expires_at),
            };

            credentials.save(&self.config)?;
            info!("Login successful");

            Ok(credentials)
        } else {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            error!("Login failed: {}", error_text);
            Err(anyhow::anyhow!("Login failed: {}", error_text))
        }
    }

    pub async fn register(&self, username: String, email: String, password: String) -> Result<()> {
        let register_url = format!("{}/auth/users", self.config.api.endpoint);

        let register_request = RegisterRequest {
            username: username.clone(),
            email,
            password,
        };

        info!("Attempting registration for user: {}", username);

        let response = self
            .client
            .post(&register_url)
            .json(&register_request)
            .send()
            .await
            .with_context(|| format!("Failed to send registration request to {register_url}"))?;

        if response.status().is_success() {
            let _register_response: RegisterResponse = response
                .json()
                .await
                .context("Failed to parse registration response")?;

            info!("Registration successful for user: {}", username);
            Ok(())
        } else {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            error!("Registration failed: {}", error_text);
            Err(anyhow::anyhow!("Registration failed: {}", error_text))
        }
    }

    pub async fn check_auth(&self) -> Result<Credentials> {
        // Try to load existing credentials
        if let Some(credentials) = Credentials::load(&self.config)? {
            if credentials.is_valid() {
                info!("Using existing valid credentials");
                return Ok(credentials);
            } else {
                warn!("Existing credentials are expired");
            }
        }

        // No valid credentials found, need to authenticate
        Err(anyhow::anyhow!(
            "No valid credentials found. Please login first."
        ))
    }

    pub async fn logout(&self) -> Result<()> {
        Credentials::clear(&self.config)?;
        info!("Logged out successfully");
        Ok(())
    }

    pub async fn health_check(&self) -> Result<()> {
        let response = self
            .client
            .get(&format!("{}/health", self.config.api.endpoint))
            .send()
            .await
            .context("Failed to connect to server")?;

        if response.status().is_success() {
            info!("Server health check passed");
            Ok(())
        } else {
            error!("Server health check failed: {}", response.status());
            Err(anyhow::anyhow!("Server health check failed"))
        }
    }

    pub async fn get_token_balance(&self) -> Result<TokenBalance> {
        let credentials = self.check_auth().await?;

        let response = self
            .client
            .get(&format!("{}/tokens/balance", self.config.api.endpoint))
            .bearer_auth(credentials.access_token.context("No access token")?)
            .send()
            .await
            .context("Failed to get token balance")?;

        if response.status().is_success() {
            let balance = response
                .json::<TokenBalance>()
                .await
                .context("Failed to parse token balance response")?;
            Ok(balance)
        } else {
            error!("Failed to get token balance: {}", response.status());
            Err(anyhow::anyhow!("Failed to get token balance"))
        }
    }

    pub async fn get_token_history(&self, days: u32) -> Result<Vec<TokenTransaction>> {
        let credentials = self.check_auth().await?;

        let response = self
            .client
            .get(&format!("{}/tokens/history", self.config.api.endpoint))
            .bearer_auth(credentials.access_token.context("No access token")?)
            .query(&[("days", days)])
            .send()
            .await
            .context("Failed to get token history")?;

        if response.status().is_success() {
            let history = response
                .json::<Vec<TokenTransaction>>()
                .await
                .context("Failed to parse token history response")?;
            Ok(history)
        } else {
            error!("Failed to get token history: {}", response.status());
            Err(anyhow::anyhow!("Failed to get token history"))
        }
    }
}

pub fn prompt_for_credentials() -> Result<(String, String)> {
    use std::io::{self, Write};

    print!("Username: ");
    io::stdout().flush()?;
    let mut username = String::new();
    io::stdin().read_line(&mut username)?;
    let username = username.trim().to_string();

    print!("Password: ");
    io::stdout().flush()?;
    let password = rpassword::read_password()?;

    Ok((username, password))
}

pub fn prompt_for_registration() -> Result<(String, String, String)> {
    use std::io::{self, Write};

    print!("Username: ");
    io::stdout().flush()?;
    let mut username = String::new();
    io::stdin().read_line(&mut username)?;
    let username = username.trim().to_string();

    print!("Email: ");
    io::stdout().flush()?;
    let mut email = String::new();
    io::stdin().read_line(&mut email)?;
    let email = email.trim().to_string();

    print!("Password: ");
    io::stdout().flush()?;
    let password = rpassword::read_password()?;

    Ok((username, email, password))
}
