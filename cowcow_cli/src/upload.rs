use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::fs;
use std::path::Path;
use tracing::{error, info, warn};

use crate::config::{Config, Credentials};

#[derive(Debug, Serialize, Deserialize)]
pub struct UploadRequest {
    pub recording_id: String,
    pub lang: String,
    pub qc_metrics: String,
    pub file_path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UploadResponse {
    pub status: String,
    pub tokens_awarded: u32,
    pub recording_id: String,
}

pub struct UploadClient {
    client: Client,
    config: Config,
}

impl UploadClient {
    pub fn new(config: Config) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.api.timeout_secs))
            .build()
            .unwrap();

        Self { client, config }
    }

    pub async fn upload_recording(
        &self,
        recording_id: &str,
        lang: &str,
        qc_metrics: &str,
        file_path: &Path,
        credentials: &Credentials,
    ) -> Result<UploadResponse> {
        let upload_url = format!("{}/recordings/upload", self.config.api.endpoint);

        // Read the audio file
        let file_data = fs::read(file_path)
            .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

        info!(
            "Uploading recording: {} ({} bytes)",
            recording_id,
            file_data.len()
        );

        // Create multipart form
        let form = reqwest::multipart::Form::new()
            .text("recording_id", recording_id.to_string())
            .text("lang", lang.to_string())
            .text("qc_metrics", qc_metrics.to_string())
            .text("file_path", file_path.to_string_lossy().to_string())
            .part(
                "file",
                reqwest::multipart::Part::bytes(file_data)
                    .file_name(file_path.file_name().unwrap().to_string_lossy().to_string())
                    .mime_str("audio/wav")?,
            );

        // Create progress bar
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} Uploading {msg}")
                .unwrap(),
        );
        pb.set_message(format!("recording {recording_id}"));

        let mut request = self.client.post(&upload_url);

        // Add authentication headers
        if let Some(access_token) = &credentials.access_token {
            request = request.bearer_auth(access_token);
        }

        if let Some(api_key) = &credentials.api_key {
            request = request.header("X-API-Key", api_key);
        }

        let response = request
            .multipart(form)
            .send()
            .await
            .with_context(|| format!("Failed to send upload request to {upload_url}"))?;

        pb.finish_with_message("Upload complete");

        if response.status().is_success() {
            let upload_response: UploadResponse = response
                .json()
                .await
                .context("Failed to parse upload response")?;

            info!(
                "Upload successful: {} tokens awarded",
                upload_response.tokens_awarded
            );
            Ok(upload_response)
        } else {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            error!("Upload failed: {}", error_text);
            Err(anyhow::anyhow!("Upload failed: {}", error_text))
        }
    }

    pub async fn upload_pending_recordings(
        &self,
        db: &SqlitePool,
        credentials: &Credentials,
        force: bool,
    ) -> Result<()> {
        // Get pending recordings from upload queue
        #[derive(sqlx::FromRow)]
        struct PendingRecording {
            id: String,
            lang: String,
            qc_metrics: String,
            wav_path: String,
            attempts: i64,
        }

        let pending_recordings = sqlx::query_as::<_, PendingRecording>(
            r#"
            SELECT 
                r.id,
                r.lang,
                r.qc_metrics,
                r.wav_path,
                uq.attempts
            FROM recordings r
            JOIN upload_queue uq ON r.id = uq.recording_id
            WHERE r.uploaded_at IS NULL
            ORDER BY r.created_at ASC
            "#,
        )
        .fetch_all(db)
        .await
        .context("Failed to fetch pending recordings")?;

        if pending_recordings.is_empty() {
            info!("No pending recordings to upload");
            return Ok(());
        }

        info!("Found {} pending recordings", pending_recordings.len());

        let mut successful_uploads = 0;
        let mut failed_uploads = 0;

        for recording in pending_recordings {
            let file_path = Path::new(&recording.wav_path);

            // Check if file exists
            if !file_path.exists() {
                warn!("File not found: {}, skipping", recording.wav_path);
                continue;
            }

            // Check quality metrics if not forcing
            if !force {
                if let Ok(metrics) =
                    serde_json::from_str::<serde_json::Value>(&recording.qc_metrics)
                {
                    if let Some(snr) = metrics.get("snr_db").and_then(|v| v.as_f64()) {
                        if snr < self.config.audio.min_snr_db as f64 {
                            warn!(
                                "Skipping recording {} due to low SNR: {:.1} dB",
                                recording.id, snr
                            );
                            continue;
                        }
                    }

                    if let Some(clipping) = metrics.get("clipping_pct").and_then(|v| v.as_f64()) {
                        if clipping > self.config.audio.max_clipping_pct as f64 {
                            warn!(
                                "Skipping recording {} due to high clipping: {:.1}%",
                                recording.id, clipping
                            );
                            continue;
                        }
                    }

                    if let Some(vad) = metrics.get("vad_ratio").and_then(|v| v.as_f64()) {
                        if vad < self.config.audio.min_vad_ratio as f64 {
                            warn!(
                                "Skipping recording {} due to low VAD ratio: {:.1}%",
                                recording.id, vad
                            );
                            continue;
                        }
                    }
                }
            }

            // Attempt upload with retry logic
            let mut attempts = recording.attempts;
            let mut success = false;

            while attempts < self.config.upload.max_retries as i64 && !success {
                match self
                    .upload_recording(
                        &recording.id,
                        &recording.lang,
                        &recording.qc_metrics,
                        file_path,
                        credentials,
                    )
                    .await
                {
                    Ok(_) => {
                        // Mark as uploaded
                        let now = chrono::Utc::now().timestamp();
                        sqlx::query!(
                            "UPDATE recordings SET uploaded_at = ? WHERE id = ?",
                            now,
                            recording.id
                        )
                        .execute(db)
                        .await
                        .context("Failed to update recording status")?;

                        // Remove from upload queue
                        sqlx::query!(
                            "DELETE FROM upload_queue WHERE recording_id = ?",
                            recording.id
                        )
                        .execute(db)
                        .await
                        .context("Failed to remove from upload queue")?;

                        successful_uploads += 1;
                        success = true;
                        info!("Successfully uploaded recording: {}", recording.id);
                    }
                    Err(e) => {
                        attempts += 1;
                        warn!(
                            "Upload attempt {} failed for {}: {}",
                            attempts, recording.id, e
                        );

                        // Update attempt count
                        let now = chrono::Utc::now().timestamp();
                        sqlx::query!(
                            "UPDATE upload_queue SET attempts = ?, last_attempt = ? WHERE recording_id = ?",
                            attempts,
                            now,
                            recording.id
                        )
                        .execute(db)
                        .await
                        .context("Failed to update upload queue")?;

                        if attempts < self.config.upload.max_retries as i64 {
                            // Wait before retrying
                            let delay = std::time::Duration::from_secs(
                                self.config.upload.retry_delay_secs * (attempts as u64),
                            );
                            info!("Retrying in {} seconds...", delay.as_secs());
                            tokio::time::sleep(delay).await;
                        }
                    }
                }
            }

            if !success {
                failed_uploads += 1;
                error!(
                    "Failed to upload recording after {} attempts: {}",
                    attempts, recording.id
                );
            }
        }

        info!(
            "Upload summary: {} successful, {} failed",
            successful_uploads, failed_uploads
        );
        Ok(())
    }
}
