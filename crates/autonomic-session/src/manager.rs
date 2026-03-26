//! Session lifecycle manager for Claude Code subprocess instances.
//!
//! `SessionManager` is the ONLY component that spawns Claude Code processes
//! (XD-001). It owns the full lifecycle: validate -> spawn -> monitor ->
//! complete. Stdout and stderr are consumed concurrently to prevent pipe
//! buffer deadlocks (XD-005).

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use autonomic_container::ContainerConfig;
use autonomic_core::SessionId;
use dashmap::DashMap;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::{Semaphore, broadcast};

use crate::command::build_container_config;
use crate::config::SessionConfig;
use crate::cost::CostTracker;
use crate::error::SessionError;
use crate::parser::{SessionOutcome, StreamMessage, classify_result, parse_line};

/// The lifecycle state of a session tracked by `SessionManager`.
#[derive(Debug, Clone)]
pub enum SessionState {
    /// The session subprocess is currently running.
    Running {
        /// OS process ID of the child.
        pid: u32,
        /// When the session was spawned.
        started_at: Instant,
    },
    /// The session completed (successfully, with error, or max-turns).
    Completed {
        /// The classified outcome from Claude Code's result message.
        outcome: SessionOutcome,
        /// When the session was spawned.
        started_at: Instant,
        /// When the session finished.
        finished_at: Instant,
    },
    /// The session failed before or during execution.
    Failed {
        /// Human-readable error description.
        error: String,
        /// When the session was spawned (None if it failed before spawning).
        started_at: Option<Instant>,
        /// When the failure was recorded.
        finished_at: Instant,
    },
    /// The session was killed because it exceeded its wall-clock timeout.
    TimedOut {
        /// When the session was spawned.
        started_at: Instant,
        /// When the timeout was enforced.
        finished_at: Instant,
        /// Total cost accumulated before the timeout.
        accumulated_cost_usd: f64,
    },
}

/// A completed session record with its full transcript and stderr capture.
#[derive(Debug)]
pub struct SessionRecord {
    /// The session identifier.
    pub session_id: SessionId,
    /// The final lifecycle state.
    pub state: SessionState,
    /// All parsed NDJSON messages from stdout.
    pub transcript: Vec<StreamMessage>,
    /// Raw stderr lines captured from the subprocess.
    pub stderr_lines: Vec<String>,
}

/// Lifecycle events emitted by `SessionManager` via broadcast channel.
#[derive(Debug, Clone)]
pub enum SessionEvent {
    /// A new session subprocess was spawned.
    Spawned {
        /// The session identifier.
        session_id: SessionId,
        /// The OS process ID.
        pid: u32,
    },
    /// A stream-json message was parsed from stdout.
    Message {
        /// The session identifier.
        session_id: SessionId,
        /// The parsed message.
        message: StreamMessage,
    },
    /// The session has finished (completed, failed, or timed out).
    Finished {
        /// The session identifier.
        session_id: SessionId,
        /// The final state.
        state: SessionState,
    },
}

/// Manages Claude Code session lifecycles.
///
/// This is the single point of Claude Code process spawning (XD-001).
/// All sessions are tracked in a concurrent map and lifecycle events
/// are broadcast to subscribers.
pub struct SessionManager {
    /// Active and recently-completed sessions.
    sessions: DashMap<SessionId, SessionState>,
    /// Concurrency limiter for simultaneous sessions.
    concurrency: Arc<Semaphore>,
    /// Global cost tracking with sliding-window budget.
    cost_tracker: Arc<CostTracker>,
    /// Maximum concurrent sessions (for error messages).
    max_concurrent: usize,
    /// Broadcast channel for lifecycle events.
    event_tx: broadcast::Sender<SessionEvent>,
    /// Path to the Claude Code CLI binary.
    claude_binary: PathBuf,
}

impl std::fmt::Debug for SessionManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionManager")
            .field("max_concurrent", &self.max_concurrent)
            .field("active_sessions", &self.sessions.len())
            .field("claude_binary", &self.claude_binary)
            .finish()
    }
}

impl SessionManager {
    /// Create a new `SessionManager`.
    ///
    /// - `max_concurrent`: maximum number of sessions that can run simultaneously.
    /// - `cost_tracker`: shared global cost tracker.
    /// - `claude_binary`: absolute path to the `claude` CLI executable.
    pub fn new(
        max_concurrent: usize,
        cost_tracker: Arc<CostTracker>,
        claude_binary: PathBuf,
    ) -> Self {
        let (event_tx, _) = broadcast::channel(256);
        Self {
            sessions: DashMap::new(),
            concurrency: Arc::new(Semaphore::new(max_concurrent)),
            cost_tracker,
            max_concurrent,
            event_tx,
            claude_binary,
        }
    }

    /// Run a complete session: validate, spawn, monitor, and return the record.
    ///
    /// This method builds the container configuration, spawns the subprocess
    /// directly (to enable streaming stdout/stderr monitoring per XD-005),
    /// and tracks cost and lifecycle state.
    #[tracing::instrument(skip(self, config), fields(session_id = %config.session_id))]
    pub async fn run_session(&self, config: &SessionConfig) -> Result<SessionRecord, SessionError> {
        let session_id = config.session_id.clone();

        // 1. Validate config.
        config.validate()?;

        // 2. Check global budget.
        if !self.cost_tracker.can_afford(config.cost_budget_usd) {
            return Err(SessionError::GlobalBudgetExhausted);
        }

        // 3. Acquire concurrency permit (non-blocking).
        let _permit =
            self.concurrency
                .try_acquire()
                .map_err(|_| SessionError::ConcurrencyLimitReached {
                    max: self.max_concurrent,
                })?;

        // 4. Build container config (assembles CLI args, env filtering).
        let container_config = build_container_config(config, &self.claude_binary);

        // 5. Spawn subprocess directly for streaming access (XD-005).
        //    ContainerRuntime::spawn is run-to-completion; we need piped handles
        //    for line-by-line monitoring with concurrent stdout/stderr consumption.
        let mut child = self.spawn_process(&container_config)?;
        let started_at = Instant::now();

        let pid = child.id().unwrap_or(0);

        // Record running state.
        self.sessions.insert(
            session_id.clone(),
            SessionState::Running { pid, started_at },
        );

        // Emit spawned event.
        let _ = self.event_tx.send(SessionEvent::Spawned {
            session_id: session_id.clone(),
            pid,
        });

        // 6. Take stdout/stderr handles.
        let stdout = child.stdout.take().ok_or(SessionError::NoStdout)?;
        let stderr = child.stderr.take().ok_or(SessionError::NoStderr)?;

        // 7. Monitor with concurrent stdout/stderr (XD-005).
        let (state, transcript, stderr_lines) = self
            .monitor_session(
                &session_id,
                &mut child,
                stdout,
                stderr,
                config.timeout,
                config.cost_budget_usd,
                started_at,
            )
            .await;

        // 8. Ensure child is reaped.
        let _ = child.wait().await;

        // 9. Record cost from outcome.
        let cost = match &state {
            SessionState::Completed { outcome, .. } => match outcome {
                SessionOutcome::Success { cost_usd, .. }
                | SessionOutcome::Error { cost_usd, .. }
                | SessionOutcome::MaxTurns { cost_usd, .. } => *cost_usd,
            },
            SessionState::TimedOut {
                accumulated_cost_usd,
                ..
            } => *accumulated_cost_usd,
            _ => 0.0,
        };
        if cost > 0.0 {
            self.cost_tracker.record_cost(&session_id, cost);
        }

        // 10. Update session state.
        self.sessions.insert(session_id.clone(), state.clone());

        // 11. Emit finished event.
        let _ = self.event_tx.send(SessionEvent::Finished {
            session_id: session_id.clone(),
            state: state.clone(),
        });

        // 12. Return SessionRecord.
        Ok(SessionRecord {
            session_id,
            state,
            transcript,
            stderr_lines,
        })
    }

    /// Spawn the subprocess with piped stdout/stderr for streaming monitoring.
    fn spawn_process(
        &self,
        config: &ContainerConfig,
    ) -> Result<tokio::process::Child, SessionError> {
        let mut cmd = tokio::process::Command::new(&config.executable);
        cmd.args(&config.args)
            .current_dir(&config.working_dir)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        // Environment filtering: clear inherited env, then apply config env.
        cmd.env_clear();
        for (key, value) in &config.env {
            cmd.env(key, value);
        }
        for key in &config.env_remove {
            cmd.env_remove(key);
        }

        tracing::debug!(
            executable = %config.executable.display(),
            working_dir = %config.working_dir.display(),
            "spawning claude process"
        );

        cmd.spawn().map_err(|e| {
            SessionError::SpawnFailed(autonomic_container::ContainerError::SpawnFailed(e))
        })
    }

    /// Core monitoring loop: consumes stdout and stderr concurrently (XD-005).
    ///
    /// Uses `tokio::select!` with three branches:
    /// - Deadline timeout
    /// - stdout line parsing (NDJSON)
    /// - stderr line buffering (rate-limit detection)
    #[allow(clippy::too_many_arguments)]
    async fn monitor_session(
        &self,
        session_id: &SessionId,
        child: &mut tokio::process::Child,
        stdout: tokio::process::ChildStdout,
        stderr: tokio::process::ChildStderr,
        timeout: std::time::Duration,
        cost_budget_usd: f64,
        started_at: Instant,
    ) -> (SessionState, Vec<StreamMessage>, Vec<String>) {
        let mut stdout_reader = BufReader::new(stdout).lines();
        let mut stderr_reader = BufReader::new(stderr).lines();
        let deadline = tokio::time::sleep(timeout);
        tokio::pin!(deadline);

        let mut transcript: Vec<StreamMessage> = Vec::new();
        let mut stderr_lines: Vec<String> = Vec::new();
        let mut final_outcome: Option<SessionOutcome> = None;
        let mut accumulated_cost: f64 = 0.0;
        let mut stdout_done = false;
        let mut stderr_done = false;

        loop {
            tokio::select! {
                _ = &mut deadline => {
                    // Timeout: terminate the child.
                    tracing::warn!(
                        %session_id,
                        elapsed = ?started_at.elapsed(),
                        "session timed out, terminating"
                    );
                    Self::terminate_child(child).await;
                    return (
                        SessionState::TimedOut {
                            started_at,
                            finished_at: Instant::now(),
                            accumulated_cost_usd: accumulated_cost,
                        },
                        transcript,
                        stderr_lines,
                    );
                }

                line = stdout_reader.next_line(), if !stdout_done => {
                    match line {
                        Ok(Some(line)) => {
                            match parse_line(&line) {
                                Ok(Some(msg)) => {
                                    // Track cost from each message.
                                    if let Some(cost) = msg.total_cost_usd {
                                        accumulated_cost = cost;
                                    }

                                    // Check per-session cost budget.
                                    if accumulated_cost > cost_budget_usd {
                                        tracing::warn!(
                                            %session_id,
                                            accumulated_cost,
                                            cost_budget_usd,
                                            "session cost budget exceeded, terminating"
                                        );
                                        Self::terminate_child(child).await;
                                        return (
                                            SessionState::Failed {
                                                error: format!(
                                                    "cost ${accumulated_cost:.4} exceeded budget ${cost_budget_usd:.4}"
                                                ),
                                                started_at: Some(started_at),
                                                finished_at: Instant::now(),
                                            },
                                            transcript,
                                            stderr_lines,
                                        );
                                    }

                                    // Emit message event.
                                    let _ = self.event_tx.send(SessionEvent::Message {
                                        session_id: session_id.clone(),
                                        message: msg.clone(),
                                    });

                                    // Check if this is a result message.
                                    if msg.message_type == "result" {
                                        final_outcome = Some(classify_result(&msg));
                                        transcript.push(msg);
                                        break;
                                    }

                                    transcript.push(msg);
                                }
                                Ok(None) => {
                                    // Empty line, skip.
                                }
                                Err(err) => {
                                    tracing::warn!(
                                        %session_id,
                                        error = %err,
                                        raw_line = %line,
                                        "failed to parse stream-json line, skipping"
                                    );
                                }
                            }
                        }
                        Ok(None) => {
                            // stdout EOF.
                            stdout_done = true;
                            if stderr_done {
                                break;
                            }
                        }
                        Err(e) => {
                            tracing::error!(
                                %session_id,
                                error = %e,
                                "error reading stdout"
                            );
                            stdout_done = true;
                            if stderr_done {
                                break;
                            }
                        }
                    }
                }

                line = stderr_reader.next_line(), if !stderr_done => {
                    match line {
                        Ok(Some(line)) => {
                            // Detect rate-limit indicators.
                            if line.contains("rate limit")
                                || line.contains("Rate limit")
                                || line.contains("429")
                            {
                                tracing::warn!(
                                    %session_id,
                                    stderr_line = %line,
                                    "rate limit indicator detected on stderr"
                                );
                            }
                            stderr_lines.push(line);
                        }
                        Ok(None) => {
                            // stderr EOF.
                            stderr_done = true;
                            if stdout_done {
                                break;
                            }
                        }
                        Err(e) => {
                            tracing::error!(
                                %session_id,
                                error = %e,
                                "error reading stderr"
                            );
                            stderr_done = true;
                            if stdout_done {
                                break;
                            }
                        }
                    }
                }
            }
        }

        // If we got a result message, return Completed state.
        if let Some(outcome) = final_outcome {
            return (
                SessionState::Completed {
                    outcome,
                    started_at,
                    finished_at: Instant::now(),
                },
                transcript,
                stderr_lines,
            );
        }

        // No result message: process crashed or was killed.
        // Get exit status from the child.
        let exit_status = child.try_wait().ok().flatten();
        let exit_code = exit_status.and_then(|s| s.code());

        #[cfg(unix)]
        let signal = {
            use std::os::unix::process::ExitStatusExt;
            exit_status.and_then(|s| s.signal())
        };
        #[cfg(not(unix))]
        let signal: Option<i32> = None;

        tracing::error!(
            %session_id,
            ?exit_code,
            ?signal,
            stderr_line_count = stderr_lines.len(),
            "process exited without result message"
        );

        let error_msg = if !stderr_lines.is_empty() {
            // Take last few lines of stderr for the error message.
            let tail: Vec<&str> = stderr_lines
                .iter()
                .rev()
                .take(5)
                .map(String::as_str)
                .collect();
            tail.into_iter().rev().collect::<Vec<_>>().join("\n")
        } else {
            format!("process exited with code {exit_code:?}, signal {signal:?}")
        };

        (
            SessionState::Failed {
                error: error_msg,
                started_at: Some(started_at),
                finished_at: Instant::now(),
            },
            transcript,
            stderr_lines,
        )
    }

    /// Gracefully terminate a child process: SIGTERM, wait 5s, then SIGKILL.
    async fn terminate_child(child: &mut tokio::process::Child) {
        let Some(pid) = child.id() else {
            // Process already exited.
            return;
        };

        // Send SIGTERM.
        #[cfg(unix)]
        {
            let nix_pid = nix::unistd::Pid::from_raw(pid as i32);
            if let Err(e) = nix::sys::signal::kill(nix_pid, nix::sys::signal::Signal::SIGTERM) {
                tracing::warn!(pid, error = %e, "failed to send SIGTERM");
            }
        }
        #[cfg(not(unix))]
        {
            let _ = child.kill().await;
            return;
        }

        // Wait up to 5 seconds for graceful shutdown.
        let graceful = tokio::time::timeout(std::time::Duration::from_secs(5), child.wait()).await;

        if graceful.is_err() {
            tracing::warn!(pid, "process did not exit after SIGTERM, sending SIGKILL");
            let _ = child.kill().await;
        }
    }

    /// Subscribe to session lifecycle events.
    pub fn subscribe(&self) -> broadcast::Receiver<SessionEvent> {
        self.event_tx.subscribe()
    }

    /// Look up the current state of a session.
    pub fn state(&self, session_id: &SessionId) -> Option<SessionState> {
        self.sessions.get(session_id).map(|entry| entry.clone())
    }

    /// List all tracked session IDs.
    pub fn active_sessions(&self) -> Vec<SessionId> {
        self.sessions
            .iter()
            .map(|entry| entry.key().clone())
            .collect()
    }

    /// Count sessions currently in the `Running` state.
    pub fn running_count(&self) -> usize {
        self.sessions
            .iter()
            .filter(|entry| matches!(entry.value(), SessionState::Running { .. }))
            .count()
    }

    /// Remove a session from the tracking map (archival).
    pub fn archive(&self, session_id: &SessionId) -> Option<SessionState> {
        self.sessions.remove(session_id).map(|(_, state)| state)
    }
}
