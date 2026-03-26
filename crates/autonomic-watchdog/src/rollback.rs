//! Crash rollback to last known-good state.
//!
//! The watchdog deliberately uses `git` CLI (via `tokio::process::Command`)
//! instead of `gix`. If `gix` or its transitive dependencies cause the daemon
//! to crash, the watchdog must still function. Dependency isolation is a safety
//! requirement, not a convenience trade-off.
//!
//! Rollback respects XD-008: only tracked files are restored via
//! `git checkout <tag> -- .`. We never run `git clean`, so gitignored files
//! (secrets.toml, WAL files, Podman volumes) survive rollback.

use std::path::Path;

use chrono::Utc;
use tokio::process::Command;

/// Roll back the state directory to the most recent `known-good/*` tag.
///
/// # Sequence
///
/// 1. Find the latest `known-good/*` tag by creator date.
/// 2. Create a pre-rollback snapshot tag `watchdog-rollback/{timestamp}`.
/// 3. Restore tracked files: `git checkout <tag> -- .` (XD-008 safe).
/// 4. Forward-commit with `--allow-empty` to record the rollback.
///
/// All git commands run with their working directory set to `state_dir`.
pub async fn rollback_to_known_good(state_dir: &Path) -> Result<(), String> {
    let tag = find_latest_known_good_tag(state_dir).await?;
    tracing::info!(tag = tag.as_str(), "rolling back to known-good tag");

    // Create a snapshot tag before rolling back so the pre-rollback state
    // is always recoverable.
    let timestamp = Utc::now().format("%Y%m%dT%H%M%SZ");
    let snapshot_tag = format!("watchdog-rollback/{timestamp}");

    run_git(
        state_dir,
        &["tag", &snapshot_tag],
        "create pre-rollback snapshot tag",
    )
    .await?;

    tracing::info!(
        snapshot = snapshot_tag.as_str(),
        "pre-rollback snapshot created"
    );

    // XD-008: checkout tracked files only. No `git clean` -- gitignored files
    // (secrets.toml, WAL files, container volumes) must survive.
    run_git(
        state_dir,
        &["checkout", &tag, "--", "."],
        "restore tracked files from known-good tag",
    )
    .await?;

    // Forward-commit to record the rollback event in history.
    let message = format!("watchdog: rollback to {tag} (from {snapshot_tag})");
    run_git(state_dir, &["add", "."], "stage restored files").await?;

    run_git(
        state_dir,
        &["commit", "--allow-empty", "-m", &message],
        "commit rollback",
    )
    .await?;

    tracing::info!("rollback committed successfully");
    Ok(())
}

/// Find the most recent `known-good/*` tag by creator date.
async fn find_latest_known_good_tag(state_dir: &Path) -> Result<String, String> {
    let output = Command::new("git")
        .args(["tag", "-l", "known-good/*", "--sort=-creatordate"])
        .current_dir(state_dir)
        .output()
        .await
        .map_err(|e| format!("failed to run git tag: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git tag failed: {stderr}"));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let tag = stdout
        .lines()
        .next()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "no known-good/* tags found".to_string())?;

    Ok(tag.to_string())
}

/// Run a git command in `state_dir`, returning an error with context on failure.
async fn run_git(state_dir: &Path, args: &[&str], context: &str) -> Result<(), String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(state_dir)
        .output()
        .await
        .map_err(|e| format!("{context}: failed to run git: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("{context}: git failed: {stderr}"));
    }

    Ok(())
}
