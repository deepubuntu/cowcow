use std::ffi::c_char;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::error;

/// Quality control metrics for audio recordings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[repr(C)]
pub struct QcMetrics {
    /// Signal-to-noise ratio in decibels
    pub snr_db: f32,
    /// Percentage of samples that are clipped
    pub clipping_pct: f32,
    /// Ratio of frames classified as speech by VAD
    pub vad_ratio: f32,
}

/// Audio processing errors
#[derive(Debug, Error)]
pub enum AudioError {
    #[error("Failed to open audio file: {0}")]
    FileOpen(#[from] std::io::Error),
    #[error("Invalid WAV format: {0}")]
    WavFormat(#[from] hound::Error),
    #[error("VAD processing failed: {0}")]
    VadError(String),
}

/// Audio processor for real-time quality control
pub struct AudioProcessor {
    sample_rate: u32,
    #[allow(dead_code)]
    channels: u16,
    vad: webrtc_vad::Vad,
    #[allow(dead_code)]
    buffer: Vec<f32>,
}

impl AudioProcessor {
    /// Create a new audio processor
    pub fn new(sample_rate: u32, channels: u16) -> Result<Self> {
        // Validate sample rate
        match sample_rate {
            8000 | 16000 | 32000 | 48000 => {}
            _ => return Err(anyhow::anyhow!("Unsupported sample rate: {}", sample_rate)),
        };

        let vad = webrtc_vad::Vad::new(sample_rate as i32)
            .map_err(|_| anyhow::anyhow!("Failed to create VAD instance"))?;
        Ok(Self {
            sample_rate,
            channels,
            vad,
            buffer: Vec::new(),
        })
    }

    /// Process a chunk of audio samples
    pub fn process_chunk(&mut self, samples: &[f32]) -> QcMetrics {
        // Calculate RMS
        let rms = self.calculate_rms(samples);

        // Detect clipping
        let clipping_pct = self.detect_clipping(samples);

        // Run VAD
        let vad_ratio = self.run_vad(samples);

        // Compute SNR (simplified)
        let snr_db = self.estimate_snr(rms, clipping_pct);

        QcMetrics {
            snr_db,
            clipping_pct,
            vad_ratio,
        }
    }

    /// Calculate RMS of audio samples
    fn calculate_rms(&self, samples: &[f32]) -> f32 {
        let sum_squares: f32 = samples.iter().map(|&x| x * x).sum();
        (sum_squares / samples.len() as f32).sqrt()
    }

    /// Detect percentage of clipped samples
    fn detect_clipping(&self, samples: &[f32]) -> f32 {
        let clipped = samples.iter().filter(|&&x| x.abs() >= 1.0).count();
        (clipped as f32 / samples.len() as f32) * 100.0
    }

    /// Run Voice Activity Detection
    fn run_vad(&mut self, samples: &[f32]) -> f32 {
        // Convert f32 samples to i16 for VAD
        let mut i16_samples = Vec::with_capacity(samples.len());
        for &sample in samples {
            i16_samples.push((sample * 32767.0) as i16);
        }

        // Process in 30ms frames
        let frame_size = (self.sample_rate as f32 * 0.03) as usize;
        let mut speech_frames = 0;
        let mut total_frames = 0;

        for chunk in i16_samples.chunks(frame_size) {
            if chunk.len() == frame_size {
                match self.vad.is_voice_segment(chunk) {
                    Ok(is_speech) => {
                        if is_speech {
                            speech_frames += 1;
                        }
                        total_frames += 1;
                    }
                    Err(_) => {
                        error!("VAD processing failed for frame");
                    }
                }
            }
        }

        if total_frames > 0 {
            (speech_frames as f32 / total_frames as f32) * 100.0
        } else {
            0.0
        }
    }

    /// Estimate SNR based on RMS and clipping
    fn estimate_snr(&self, rms: f32, clipping_pct: f32) -> f32 {
        // Simple SNR estimation based on RMS and clipping
        // This is a simplified model - real SNR calculation would be more complex
        let noise_floor = -60.0; // Typical noise floor in dB
        let signal_level = 20.0 * rms.log10();
        let noise_level = noise_floor + (clipping_pct * 0.1);
        signal_level - noise_level
    }
}

/// Analyze a WAV file and return QC metrics
///
/// # Safety
///
/// This function dereferences a raw pointer. The caller must ensure that:
/// - `path` is a valid pointer to a null-terminated C string
/// - The string pointed to by `path` is valid UTF-8 or UTF-8 compatible
/// - The pointer remains valid for the duration of the function call
#[no_mangle]
pub unsafe extern "C" fn analyze_wav(path: *const c_char) -> QcMetrics {
    let path_str = std::ffi::CStr::from_ptr(path)
        .to_string_lossy()
        .into_owned();

    match analyze_wav_internal(&path_str) {
        Ok(metrics) => metrics,
        Err(e) => {
            error!("Failed to analyze WAV file: {}", e);
            QcMetrics {
                snr_db: 0.0,
                clipping_pct: 100.0,
                vad_ratio: 0.0,
            }
        }
    }
}

fn analyze_wav_internal(path: &str) -> Result<QcMetrics> {
    let reader = hound::WavReader::open(path)?;
    let spec = reader.spec();

    let mut processor = AudioProcessor::new(spec.sample_rate, spec.channels)?;
    let mut all_samples = Vec::new();

    // Read all samples
    for sample in reader.into_samples::<i16>() {
        let sample = sample?;
        all_samples.push(sample as f32 / 32768.0);
    }

    // Process in chunks
    let chunk_size = (spec.sample_rate as f32 * 0.1) as usize; // 100ms chunks
    let mut metrics = Vec::new();

    for chunk in all_samples.chunks(chunk_size) {
        metrics.push(processor.process_chunk(chunk));
    }

    // Average the metrics
    let avg_metrics = QcMetrics {
        snr_db: metrics.iter().map(|m| m.snr_db).sum::<f32>() / metrics.len() as f32,
        clipping_pct: metrics.iter().map(|m| m.clipping_pct).sum::<f32>() / metrics.len() as f32,
        vad_ratio: metrics.iter().map(|m| m.vad_ratio).sum::<f32>() / metrics.len() as f32,
    };

    Ok(avg_metrics)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_processor() {
        let mut processor = AudioProcessor::new(16000, 1).unwrap();

        // Generate a test signal (sine wave)
        let mut samples = Vec::new();
        for i in 0..1600 {
            let t = i as f32 / 16000.0;
            samples.push((2.0 * std::f32::consts::PI * 440.0 * t).sin());
        }

        let metrics = processor.process_chunk(&samples);

        assert!(metrics.snr_db > 0.0);
        assert!(metrics.clipping_pct < 1.0);
        assert!(metrics.vad_ratio >= 0.0 && metrics.vad_ratio <= 100.0);
    }
}
