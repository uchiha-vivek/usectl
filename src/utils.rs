use anyhow::{bail, Context, Result};
use std::fs;
use std::process::Command;

pub fn ensure_supported() -> Result<()> {
    if !(cfg!(target_os = "linux") || cfg!(target_os = "macos")) {
        bail!("usectl currently supports Linux and macOS only.");
    }

    Ok(())
}

pub fn read_file(path: &str) -> Result<String> {
    fs::read_to_string(path).with_context(|| format!("failed to read {path}"))
}

pub fn command_output(program: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .with_context(|| format!("failed to run {program}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("{program} failed: {}", stderr.trim());
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn format_bytes_from_kb(kb: u64) -> String {
    let bytes = kb as f64 * 1024.0;
    format_bytes(bytes)
}

pub fn format_bytes(bytes: f64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    const TB: f64 = GB * 1024.0;

    if bytes >= TB {
        format!("{:.1} TB", bytes / TB)
    } else if bytes >= GB {
        format!("{:.1} GB", bytes / GB)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes / MB)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes / KB)
    } else {
        format!("{:.0} B", bytes)
    }
}

pub fn format_percent(value: f64) -> String {
    format!("{value:.1}%")
}
