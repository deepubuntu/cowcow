use std::path::Path;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use cowcow_core::{AudioProcessor, QcMetrics};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use indicatif::{ProgressBar, ProgressStyle};
use sqlx::sqlite::SqlitePool;
use sqlx::Row;
use tokio::sync::mpsc;
use tracing::{error, info};
use uuid::Uuid;

mod auth;
mod config;
mod upload;

use auth::{prompt_for_credentials, prompt_for_registration, AuthClient};
use config::Config;
use upload::UploadClient;

/// Cowcow CLI - Offline-first data collection for low-resource languages
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Record audio with quality control
    Record {
        /// Language code (e.g., "sw" for Swahili)
        #[arg(short, long)]
        lang: String,

        /// Recording duration in seconds (optional)
        #[arg(short, long)]
        duration: Option<u32>,

        /// Prompt text to read
        #[arg(short, long)]
        prompt: Option<String>,
    },

    /// Upload queued recordings
    Upload {
        /// Force upload even if QC metrics are poor
        #[arg(short, long)]
        force: bool,
    },

    /// Show recording statistics
    Stats,

    /// Check system health
    Doctor,

    /// Export recordings to a directory
    Export {
        /// Export format (json, wav, or both)
        #[arg(short, long)]
        format: String,

        /// Destination directory
        #[arg(short, long)]
        dest: PathBuf,

        /// Filter by language code (e.g., "en", "sw")
        #[arg(long)]
        lang: Option<String>,

        /// Filter by upload status (uploaded, pending, failed)
        #[arg(long)]
        status: Option<String>,

        /// Minimum SNR in dB
        #[arg(long)]
        min_snr: Option<f32>,

        /// Maximum clipping percentage
        #[arg(long)]
        max_clipping: Option<f32>,

        /// Minimum VAD ratio
        #[arg(long)]
        min_vad: Option<f32>,

        /// Export recordings from this many days ago
        #[arg(long, default_value = "30")]
        days: u32,
    },

    /// Authentication commands
    Auth {
        #[command(subcommand)]
        command: AuthCommands,
    },

    /// Configuration commands
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },

    /// Token management commands
    Tokens {
        #[command(subcommand)]
        command: TokensCommands,
    },
}

#[derive(Subcommand)]
enum AuthCommands {
    /// Login with username and password
    Login,

    /// Register a new account
    Register,

    /// Logout (clear stored credentials)
    Logout,

    /// Show current authentication status
    Status,
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Show current configuration
    Show,

    /// Set configuration values
    Set {
        /// Configuration key (e.g., "api.endpoint")
        key: String,

        /// Configuration value
        value: String,
    },

    /// Reset configuration to defaults
    Reset,
}

#[derive(Subcommand)]
enum TokensCommands {
    /// Show current token balance
    Balance,

    /// Show token transaction history
    History {
        /// Show transactions from this many days ago
        #[arg(short, long, default_value = "30")]
        days: u32,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Parse command line arguments
    let cli = Cli::parse();

    // Load configuration
    let config = Config::load()?;
    config.validate()?;

    match cli.command {
        Commands::Record {
            lang,
            duration,
            prompt,
        } => {
            let db = init_db(&config).await?;
            record_audio(&lang, duration, prompt, &db, &config).await?;
        }
        Commands::Upload { force } => {
            let db = init_db(&config).await?;
            upload_recordings(force, &db, &config).await?;
        }
        Commands::Stats => {
            let db = init_db(&config).await?;
            show_stats(&db).await?;
        }
        Commands::Doctor => {
            check_health(&config).await?;
        }
        Commands::Export {
            format,
            dest,
            lang,
            status,
            min_snr,
            max_clipping,
            min_vad,
            days,
        } => {
            let db = init_db(&config).await?;
            export_recordings(
                format,
                dest,
                lang,
                status,
                min_snr,
                max_clipping,
                min_vad,
                days,
                &db,
            )
            .await?;
        }
        Commands::Auth { command } => {
            handle_auth_command(command, &config).await?;
        }
        Commands::Config { command } => {
            handle_config_command(command, &config).await?;
        }
        Commands::Tokens { command } => {
            handle_tokens_command(command, &config).await?;
        }
    }

    Ok(())
}

async fn init_db(config: &Config) -> Result<SqlitePool> {
    let db_path = config.database_path();

    // Create directory if it doesn't exist
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Create recordings directory
    let recordings_dir = config.recordings_dir();
    std::fs::create_dir_all(&recordings_dir)?;

    let pool = SqlitePool::connect(&format!("sqlite:{}?mode=rwc", db_path.display())).await?;

    // Create tables if they don't exist
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS recordings (
            id TEXT PRIMARY KEY,
            lang TEXT NOT NULL,
            prompt TEXT,
            qc_metrics TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            uploaded_at INTEGER,
            wav_path TEXT NOT NULL
        );
        
        CREATE TABLE IF NOT EXISTS upload_queue (
            recording_id TEXT PRIMARY KEY,
            attempts INTEGER NOT NULL,
            last_attempt INTEGER,
            FOREIGN KEY (recording_id) REFERENCES recordings(id)
        );
        "#,
    )
    .execute(&pool)
    .await?;

    Ok(pool)
}

async fn record_audio(
    lang: &str,
    duration: Option<u32>,
    prompt: Option<String>,
    db: &SqlitePool,
    config: &Config,
) -> Result<()> {
    info!("Starting recording for language: {}", lang);

    // Initialize audio device
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .context("No input device available")?;

    let config_audio = cpal::StreamConfig {
        channels: config.audio.channels,
        sample_rate: cpal::SampleRate(config.audio.sample_rate),
        buffer_size: cpal::BufferSize::Default,
    };

    // Create audio processor
    let mut processor = AudioProcessor::new(config.audio.sample_rate, config.audio.channels)?;

    // Create channels for audio processing
    let (tx, mut rx) = mpsc::channel(32); // Smaller buffer for better flow control

    // Start recording stream
    let stream = device.build_input_stream(
        &config_audio,
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            // Use try_send but with error handling
            match tx.try_send(data.to_vec()) {
                Ok(()) => {} // Success
                Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                    // Channel is full - this is normal under high load, just drop this chunk
                }
                Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                    // Receiver dropped - stop trying to send
                }
            }
        },
        move |err| {
            error!("Audio stream error: {}", err);
        },
        None,
    )?;

    stream.play()?;

    // Create output directory
    let output_dir = config.recordings_dir().join(lang);
    std::fs::create_dir_all(&output_dir)?;

    // Generate unique ID for this recording
    let recording_id = Uuid::new_v4();
    let wav_path = output_dir.join(format!("{recording_id}.wav"));

    // Create WAV writer
    let spec = hound::WavSpec {
        channels: config.audio.channels,
        sample_rate: config.audio.sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(&wav_path, spec)?;

    // Process audio data
    let mut metrics = Vec::new();
    let _start_time = std::time::Instant::now();
    let duration = duration.map(|d| Duration::from_secs(d as u64));

    // Track actual audio duration based on samples processed
    let mut total_samples_processed = 0u64;
    let samples_per_second = config.audio.sample_rate as u64;

    // Silence detection parameters
    let silence_threshold_secs = 5.0; // Stop after 5 seconds of silence
    let mut silence_start_samples = None::<u64>; // Track when silence started

    // Create progress bar
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} Recording... {msg}")
            .unwrap(),
    );

    // Display prompt if provided
    if let Some(prompt_text) = &prompt {
        println!("\nPlease read the following text:");
        println!("\"{prompt_text}\"");
        println!("Press Enter to start recording...");
        std::io::stdin().read_line(&mut String::new())?;
    }

    // Give user time to prepare
    println!("Get ready to speak...");
    for i in (1..=3).rev() {
        println!("Starting in {i}...");
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    println!("🎙️  RECORDING NOW!");
    loop {
        // Use timeout to avoid infinite waiting
        let timeout_result = tokio::time::timeout(
            Duration::from_millis(10), // Shorter timeout for more responsive processing
            rx.recv(),
        )
        .await;

        match timeout_result {
            Ok(Some(samples)) => {
                // Process chunk
                let chunk_metrics = processor.process_chunk(&samples);
                metrics.push(chunk_metrics.clone());

                // Write samples to WAV file
                for &sample in &samples {
                    writer.write_sample((sample * 32767.0) as i16)?;
                }

                // Update total samples processed
                total_samples_processed += samples.len() as u64;

                // Calculate actual audio duration based on samples processed
                let actual_duration = Duration::from_secs_f64(
                    total_samples_processed as f64 / samples_per_second as f64,
                );

                // Silence detection logic
                // Calculate RMS of the current chunk
                let rms = {
                    let sum_squares: f32 = samples.iter().map(|&x| x * x).sum();
                    (sum_squares / samples.len() as f32).sqrt()
                };

                // Consider voice activity if either VAD detects it OR RMS is above threshold
                let vad_threshold = 0.01; // VAD ratio threshold (1%)
                let rms_threshold = 0.005; // RMS level threshold (adjusted to 0.005 for better voice sensitivity)
                let has_voice_activity =
                    chunk_metrics.vad_ratio > vad_threshold || rms > rms_threshold;

                if has_voice_activity {
                    // Voice detected - reset silence timer
                    silence_start_samples = None;
                } else {
                    // No voice detected - track silence duration
                    if silence_start_samples.is_none() {
                        // Start tracking silence from this chunk
                        silence_start_samples =
                            Some(total_samples_processed - samples.len() as u64);
                    }
                }

                // Check if we should stop due to silence
                let mut stop_reason = None;
                if let Some(silence_start) = silence_start_samples {
                    let silence_duration_samples = total_samples_processed - silence_start;
                    let silence_duration_secs =
                        silence_duration_samples as f64 / samples_per_second as f64;

                    if silence_duration_secs >= silence_threshold_secs {
                        stop_reason =
                            Some(format!("Silence detected for {silence_duration_secs:.1}s"));
                    }
                }

                // Check duration based on actual audio processed (not wall clock time)
                if stop_reason.is_none() {
                    if let Some(dur) = duration {
                        if actual_duration >= dur {
                            stop_reason = Some(format!(
                                "Duration reached: {actual_duration:.2?} (actual audio duration)"
                            ));
                        }
                    }
                }

                // Update progress with silence information
                let silence_info = if let Some(silence_start) = silence_start_samples {
                    let silence_duration_samples = total_samples_processed - silence_start;
                    let silence_duration_secs =
                        silence_duration_samples as f64 / samples_per_second as f64;
                    format!(" | Silence: {silence_duration_secs:.1}s")
                } else {
                    String::new()
                };

                let voice_activity_info = if has_voice_activity {
                    " | VOICE DETECTED"
                } else {
                    ""
                };

                pb.set_message(format!(
                    "SNR: {:.1} dB | Clipping: {:.1}% | VAD: {:.1}% | RMS: {:.4}{}{}",
                    chunk_metrics.snr_db,
                    chunk_metrics.clipping_pct,
                    chunk_metrics.vad_ratio,
                    rms,
                    silence_info,
                    voice_activity_info
                ));

                // Stop recording if conditions are met
                if let Some(reason) = stop_reason {
                    println!("{reason}");
                    break;
                }
            }
            Ok(None) => {
                println!("Channel closed");
                break;
            }
            Err(_) => {
                // Timeout - just continue the loop without checking duration
                // This ensures we only stop based on actual audio data processed
                continue;
            }
        }
    }

    writer.finalize()?;
    pb.finish_with_message("Recording complete!");

    // Calculate average metrics
    let avg_metrics = QcMetrics {
        snr_db: metrics.iter().map(|m| m.snr_db).sum::<f32>() / metrics.len() as f32,
        clipping_pct: metrics.iter().map(|m| m.clipping_pct).sum::<f32>() / metrics.len() as f32,
        vad_ratio: metrics.iter().map(|m| m.vad_ratio).sum::<f32>() / metrics.len() as f32,
    };

    // Display quality metrics
    println!("\nRecording Quality Metrics:");
    println!("  SNR: {:.1} dB", avg_metrics.snr_db);
    println!("  Clipping: {:.1}%", avg_metrics.clipping_pct);
    println!("  Voice Activity: {:.1}%", avg_metrics.vad_ratio);

    // Save to database
    sqlx::query(
        r#"
        INSERT INTO recordings (id, lang, prompt, qc_metrics, created_at, wav_path)
        VALUES (?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(recording_id.to_string())
    .bind(lang)
    .bind(prompt)
    .bind(serde_json::to_string(&avg_metrics)?)
    .bind(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64,
    )
    .bind(wav_path.to_string_lossy())
    .execute(db)
    .await?;

    // Add to upload queue
    sqlx::query(
        r#"
        INSERT INTO upload_queue (recording_id, attempts, last_attempt)
        VALUES (?, 0, 0)
        "#,
    )
    .bind(recording_id.to_string())
    .execute(db)
    .await?;

    info!("Recording saved: {}", wav_path.display());

    // Auto-upload if configured
    if config.storage.auto_upload {
        println!("Auto-uploading recording...");
        upload_recordings(false, db, config).await?;
    }

    Ok(())
}

async fn upload_recordings(force: bool, db: &SqlitePool, config: &Config) -> Result<()> {
    let auth_client = AuthClient::new(config.clone());
    let upload_client = UploadClient::new(config.clone());

    // Check authentication
    let credentials = match auth_client.check_auth().await {
        Ok(creds) => creds,
        Err(_) => {
            println!("Authentication required. Please login first.");
            println!("Run: cowcow auth login");
            return Ok(());
        }
    };

    // Upload pending recordings
    upload_client
        .upload_pending_recordings(db, &credentials, force)
        .await?;

    Ok(())
}

async fn show_stats(db: &SqlitePool) -> Result<()> {
    let stats = sqlx::query(
        r#"
        SELECT 
            COUNT(*) as total_recordings,
            COUNT(CASE WHEN uploaded_at IS NOT NULL THEN 1 END) as uploaded_recordings,
            COUNT(CASE WHEN uploaded_at IS NULL THEN 1 END) as pending_recordings
        FROM recordings
        "#,
    )
    .fetch_one(db)
    .await?;

    println!("📊 Recording Statistics");
    println!(
        "  Total recordings: {}",
        stats.get::<i64, _>("total_recordings")
    );
    println!("  Uploaded: {}", stats.get::<i64, _>("uploaded_recordings"));
    println!("  Pending: {}", stats.get::<i64, _>("pending_recordings"));

    Ok(())
}

async fn check_health(config: &Config) -> Result<()> {
    println!("🔍 System Health Check");

    // Check audio device
    let host = cpal::default_host();
    let device = host.default_input_device();
    println!(
        "  Audio device: {}",
        if device.is_some() { "✅" } else { "❌" }
    );

    // Check storage
    let storage_dir = config.data_dir();
    println!(
        "  Storage directory: {}",
        if storage_dir.exists() { "✅" } else { "❌" }
    );

    // Check database
    let db_path = config.database_path();
    println!("  Database: {}", if db_path.exists() { "✅" } else { "❌" });

    // Check server connection
    let auth_client = AuthClient::new(config.clone());
    match auth_client.health_check().await {
        Ok(_) => println!("  Server connection: ✅"),
        Err(_) => println!("  Server connection: ❌"),
    }

    // Check authentication
    match auth_client.check_auth().await {
        Ok(_) => println!("  Authentication: ✅"),
        Err(_) => println!("  Authentication: ❌"),
    }

    Ok(())
}

async fn export_recordings(
    format: String,
    dest: PathBuf,
    lang: Option<String>,
    status: Option<String>,
    min_snr: Option<f32>,
    max_clipping: Option<f32>,
    min_vad: Option<f32>,
    days: u32,
    db: &SqlitePool,
) -> Result<()> {
    use std::fs;

    // Create destination directory if it doesn't exist
    fs::create_dir_all(&dest).context("Failed to create destination directory")?;

    // Build query with filters
    let mut query = String::from("SELECT * FROM recordings WHERE 1=1");
    let mut params: Vec<String> = Vec::new();

    // Language filter
    if let Some(lang_filter) = &lang {
        query.push_str(" AND lang = ?");
        params.push(lang_filter.clone());
    }

    // Status filter
    match status.as_deref() {
        Some("uploaded") => {
            query.push_str(" AND uploaded_at IS NOT NULL");
        }
        Some("pending") => {
            query.push_str(" AND uploaded_at IS NULL");
        }
        Some("failed") => {
            query.push_str(" AND id IN (SELECT recording_id FROM upload_queue WHERE attempts > 0)");
        }
        _ => {}
    }

    // Date filter
    let start_timestamp = chrono::Utc::now().timestamp() - (days as i64 * 24 * 60 * 60);
    query.push_str(" AND created_at >= ?");
    params.push(start_timestamp.to_string());

    query.push_str(" ORDER BY created_at DESC");

    // Execute query
    let mut query_builder = sqlx::query_as::<
        _,
        (
            String,
            String,
            Option<String>,
            String,
            i64,
            Option<i64>,
            String,
        ),
    >(&query);

    for param in &params {
        query_builder = query_builder.bind(param);
    }

    let recordings = query_builder
        .fetch_all(db)
        .await
        .context("Failed to fetch recordings")?;

    // Filter by QC metrics
    let mut filtered_recordings = Vec::new();
    for recording in recordings {
        let qc_metrics: serde_json::Value =
            serde_json::from_str(&recording.3).context("Failed to parse QC metrics")?;

        let snr = qc_metrics
            .get("snr_db")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as f32;
        let clipping = qc_metrics
            .get("clipping_pct")
            .and_then(|v| v.as_f64())
            .unwrap_or(100.0) as f32;
        let vad = qc_metrics
            .get("vad_ratio")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as f32;

        // Apply QC filters
        if let Some(min_snr_val) = min_snr {
            if snr < min_snr_val {
                continue;
            }
        }

        if let Some(max_clipping_val) = max_clipping {
            if clipping > max_clipping_val {
                continue;
            }
        }

        if let Some(min_vad_val) = min_vad {
            if vad < min_vad_val {
                continue;
            }
        }

        filtered_recordings.push(recording);
    }

    if filtered_recordings.is_empty() {
        println!("No recordings found matching the specified criteria.");
        return Ok(());
    }

    println!(
        "Found {} recordings matching criteria",
        filtered_recordings.len()
    );

    // Export based on format
    match format.as_str() {
        "json" => {
            export_json(&filtered_recordings, &dest).await?;
        }
        "wav" => {
            export_wav(&filtered_recordings, &dest).await?;
        }
        "both" => {
            export_json(&filtered_recordings, &dest).await?;
            export_wav(&filtered_recordings, &dest).await?;
        }
        _ => {
            return Err(anyhow::anyhow!(
                "Invalid format. Use 'json', 'wav', or 'both'"
            ));
        }
    }

    println!("✅ Export completed to: {}", dest.display());
    Ok(())
}

async fn export_json(
    recordings: &[(
        String,
        String,
        Option<String>,
        String,
        i64,
        Option<i64>,
        String,
    )],
    dest: &Path,
) -> Result<()> {
    use std::fs::File;
    use std::io::Write;

    let json_path = dest.join("recordings.json");
    let mut file = File::create(&json_path).context("Failed to create JSON file")?;

    writeln!(file, "[")?;

    for (i, recording) in recordings.iter().enumerate() {
        let qc_metrics: serde_json::Value = serde_json::from_str(&recording.3)?;

        let record = serde_json::json!({
            "id": recording.0,
            "lang": recording.1,
            "prompt": recording.2,
            "qc_metrics": qc_metrics,
            "created_at": recording.4,
            "uploaded_at": recording.5,
            "wav_path": recording.6
        });

        if i == recordings.len() - 1 {
            writeln!(file, "  {}", serde_json::to_string_pretty(&record)?)?;
        } else {
            writeln!(file, "  {},", serde_json::to_string_pretty(&record)?)?;
        }
    }

    writeln!(file, "]")?;
    println!("📄 JSON export: {}", json_path.display());
    Ok(())
}

async fn export_wav(
    recordings: &[(
        String,
        String,
        Option<String>,
        String,
        i64,
        Option<i64>,
        String,
    )],
    dest: &Path,
) -> Result<()> {
    use std::fs;

    let wav_dir = dest.join("recordings");
    fs::create_dir_all(&wav_dir).context("Failed to create WAV directory")?;

    let mut copied_files = 0;

    for recording in recordings {
        let source_path = Path::new(&recording.6);
        if source_path.exists() {
            let filename = format!("{}_{}.wav", recording.1, recording.0);
            let dest_path = wav_dir.join(&filename);

            fs::copy(source_path, &dest_path).context("Failed to copy WAV file")?;
            copied_files += 1;
        }
    }

    println!(
        "🎵 WAV export: {} files copied to {}",
        copied_files,
        wav_dir.display()
    );
    Ok(())
}

async fn handle_auth_command(command: AuthCommands, config: &Config) -> Result<()> {
    let auth_client = AuthClient::new(config.clone());

    match command {
        AuthCommands::Login => {
            let (username, password) = prompt_for_credentials()?;
            match auth_client.login(username, password).await {
                Ok(_) => println!("✅ Login successful!"),
                Err(e) => println!("❌ Login failed: {e}"),
            }
        }
        AuthCommands::Register => {
            let (username, email, password) = prompt_for_registration()?;
            match auth_client.register(username, email, password).await {
                Ok(_) => println!("✅ Registration successful! You can now login."),
                Err(e) => println!("❌ Registration failed: {e}"),
            }
        }
        AuthCommands::Logout => {
            auth_client.logout().await?;
            println!("✅ Logged out successfully");
        }
        AuthCommands::Status => match auth_client.check_auth().await {
            Ok(creds) => {
                println!("✅ Authenticated");
                if let Some(username) = creds.username {
                    println!("  Username: {username}");
                }
                if let Some(expires_at) = creds.expires_at {
                    let expires =
                        chrono::DateTime::from_timestamp(expires_at as i64, 0).unwrap_or_default();
                    println!("  Expires: {}", expires.format("%Y-%m-%d %H:%M:%S"));
                }
            }
            Err(_) => println!("❌ Not authenticated"),
        },
    }

    Ok(())
}

async fn handle_config_command(command: ConfigCommands, config: &Config) -> Result<()> {
    match command {
        ConfigCommands::Show => {
            let config_toml = toml::to_string_pretty(config)?;
            println!("📁 Current Configuration:");
            println!("{config_toml}");
        }
        ConfigCommands::Set { key, value } => {
            let mut config_copy = config.clone();
            match config_copy.set_value(&key, &value) {
                Ok(_) => {
                    config_copy.save()?;
                    println!("✅ Configuration updated: {} = {}", key, value);
                }
                Err(e) => {
                    println!("❌ Failed to set configuration: {}", e);
                    println!("Available keys:");
                    for available_key in Config::get_available_keys() {
                        println!("  - {}", available_key);
                    }
                }
            }
        }
        ConfigCommands::Reset => {
            let default_config = Config::default();
            default_config.save()?;
            println!("✅ Configuration reset to defaults");
        }
    }

    Ok(())
}

async fn handle_tokens_command(command: TokensCommands, config: &Config) -> Result<()> {
    let auth_client = AuthClient::new(config.clone());

    match command {
        TokensCommands::Balance => {
            let balance = auth_client.get_token_balance().await?;
            println!("💰 Token Balance Summary:");
            println!("  Current Balance: {} tokens", balance.balance);
            println!("  Total Earned: {} tokens", balance.total_earned);
            println!("  Total Spent: {} tokens", balance.total_spent);
        }
        TokensCommands::History { days } => {
            let history = auth_client.get_token_history(days).await?;
            println!("📜 Token Transaction History (last {} days):", days);

            if history.is_empty() {
                println!("  No transactions found.");
            } else {
                for tx in history {
                    println!(
                        "  {} | {} | {:+} tokens | Balance: {} | {}",
                        tx.date.format("%Y-%m-%d %H:%M:%S"),
                        tx.transaction_type,
                        tx.amount,
                        tx.balance,
                        tx.notes
                    );
                }
            }
        }
    }

    Ok(())
}
