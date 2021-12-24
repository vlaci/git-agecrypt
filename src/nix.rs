use std::{path::Path, process};

use anyhow::{anyhow, Context, Result};

pub(crate) fn eval_file(file_path: impl AsRef<Path>) -> Result<serde_json::Value> {
    let output = process::Command::new("nix")
        .arg("eval")
        .arg("--experimental-features")
        .arg("nix-command flakes")
        .arg("--no-net")
        .arg("--impure")
        .arg("--json")
        .arg("--expr")
        .arg(format!("import {}", file_path.as_ref().to_string_lossy()))
        .output()
        .context("failed to execute nix")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("Failed to convert file to JSON."))
            .with_context(|| format!("File: {}", file_path.as_ref().to_string_lossy()))
            .with_context(|| format!("Output of nix eval: {}", stderr));
    }

    Ok(serde_json::from_slice(&output.stdout)?)
}
