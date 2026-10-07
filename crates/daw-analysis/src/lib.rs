//! Spawn Julia offline analysis. Never call from the audio thread.

use daw_contracts::{LoudnessResult, SpectrumResult};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use tokio::process::Command;

#[derive(Debug, thiserror::Error)]
pub enum AnalysisError {
    #[error("julia failed: {0}")]
    Julia(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone)]
pub struct JuliaAnalyzer {
    project: PathBuf,
    julia: String,
}

impl JuliaAnalyzer {
    pub fn from_workspace_root(root: impl AsRef<Path>) -> Self {
        Self {
            project: root.as_ref().join("analysis"),
            julia: std::env::var("JULIA_BIN").unwrap_or_else(|_| "julia".into()),
        }
    }

    pub async fn spectrum(
        &self,
        samples: &[f32],
        sample_rate: f32,
    ) -> Result<SpectrumResult, AnalysisError> {
        let raw = self.invoke("spectrum", samples, sample_rate).await?;
        Ok(serde_json::from_value(raw)?)
    }

    pub async fn loudness(
        &self,
        samples: &[f32],
        sample_rate: f32,
    ) -> Result<LoudnessResult, AnalysisError> {
        let raw = self.invoke("loudness", samples, sample_rate).await?;
        Ok(serde_json::from_value(raw)?)
    }

    async fn invoke(
        &self,
        op: &str,
        samples: &[f32],
        sample_rate: f32,
    ) -> Result<serde_json::Value, AnalysisError> {
        let req = serde_json::json!({
            "op": op,
            "sample_rate": sample_rate,
            "samples": samples,
        });
        let script = self.project.join("run.jl");
        let mut child = Command::new(&self.julia)
            .arg("--startup-file=no")
            .arg("--project")
            .arg(&self.project)
            .arg(&script)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;
        use tokio::io::AsyncWriteExt;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(req.to_string().as_bytes()).await?;
            stdin.write_all(b"\n").await?;
        }
        let out = child.wait_with_output().await?;
        if !out.status.success() {
            return Err(AnalysisError::Julia(
                String::from_utf8_lossy(&out.stderr).into(),
            ));
        }
        let stdout = String::from_utf8_lossy(&out.stdout);
        let line = stdout.lines().last().unwrap_or("{}");
        Ok(serde_json::from_str(line)?)
    }
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct WireSpectrum {
    bands_hz: Vec<f32>,
    magnitudes_db: Vec<f32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_root_points_at_analysis() {
        let a = JuliaAnalyzer::from_workspace_root(".");
        assert!(a.project.ends_with("analysis"));
    }
}
