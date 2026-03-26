# Scheduler

The Autonomic scheduler runs cron-based recurring tasks with model-tier awareness and rate budget integration. It is responsible for health checks, evolution analysis, reflections, backups, and any user-defined periodic jobs. The scheduler is a subsystem of the orchestrator process, not a separate daemon.

## Job Definitions

Jobs are defined in `~/.autonomic/schedules/jobs.toml`. Each job specifies a cron expression, model tier, priority level, and the prompt/context to feed into a Claude Code session.

### TOML Schema

```toml
# ~/.autonomic/schedules/jobs.toml

# Global scheduler settings
[scheduler]
enabled = true
timezone = "UTC"
max_concurrent_jobs = 2
default_timeout_seconds = 300

# Built-in jobs ship with Autonomic. Users can override any field.

[jobs.daily_health_check]
description = "Verify all subsystems are operational"
cron = "0 6 * * *"                # Daily at 06:00 UTC
model_tier = "haiku"
priority = "critical"
timeout_seconds = 120
prompt = """
Perform a health check of the Autonomic system:
1. Verify PostgreSQL is accessible and schema is current
2. Check disk usage of ~/.autonomic/
3. Verify git repository integrity (git fsck --quick)
4. Report rate budget status for all tiers
5. Check for any stuck sessions (running > 1 hour)
Output a JSON summary with status: "healthy" | "degraded" | "unhealthy"
"""
context_files = ["config.toml"]
output_metric = "health_check.status"

[jobs.weekly_evolution_analysis]
description = "Analyze metrics and propose evolution strategies"
cron = "0 2 * * 0"               # Sundays at 02:00 UTC
model_tier = "sonnet"
priority = "normal"
timeout_seconds = 600
prompt = """
Analyze the past week of Autonomic metrics and session data:
1. Identify patterns in session success/failure rates
2. Analyze token usage efficiency across model tiers
3. Review evolution changelog for strategy effectiveness
4. Propose 0-3 concrete evolution strategies with expected impact
5. Flag any anomalies or degradation trends
Output structured JSON with: analysis, proposals[], anomalies[]
"""
context_files = ["config.toml", "evolution/strategies.toml", "evolution/changelog.toml"]
output_metric = "evolution.weekly_analysis"
pre_snapshot = true               # Create evolution snapshot before running

[jobs.daily_reflection]
description = "Summarize daily activity and learnings"
cron = "0 23 * * *"              # Daily at 23:00 UTC
model_tier = "haiku"
priority = "low"
timeout_seconds = 180
prompt = """
Summarize today's Autonomic activity:
1. Sessions run: count by model tier and status
2. Total tokens consumed vs. budget
3. Notable events or errors
4. Recommendations for tomorrow's scheduling
Output a concise JSON summary.
"""
output_metric = "daily.reflection"

[jobs.daily_snapshot]
description = "Create daily state snapshot and prune old tags"
cron = "0 3 * * *"               # Daily at 03:00 UTC
model_tier = "haiku"             # Minimal model usage — mostly internal ops
priority = "critical"
timeout_seconds = 60
builtin = true                   # Executed internally, not as a Claude session
output_metric = "snapshot.daily"

[jobs.backup]
description = "Push state to configured backup remote"
cron = "0 4 * * *"               # Daily at 04:00 UTC
model_tier = "haiku"
priority = "normal"
timeout_seconds = 120
builtin = true
enabled = false                  # Requires backup.remote in config.toml

[jobs.budget_recalibration]
description = "Adjust budget allocations based on usage patterns"
cron = "0 */6 * * *"            # Every 6 hours
model_tier = "haiku"
priority = "normal"
timeout_seconds = 90
prompt = """
Review rate budget usage over the last 6 hours:
1. Calculate actual usage per subsystem vs. allocated percentage
2. Identify any subsystem consistently over/under its allocation
3. Propose rebalanced allocations if drift > 10%
4. Check if any scheduled jobs were deferred and whether they should be prioritized
Output JSON with: current_allocations, proposed_allocations, deferred_jobs[]
"""
output_metric = "budget.recalibration"
```

### User-Defined Jobs

Users add custom jobs in the same file:

```toml
[jobs.my_project_check]
description = "Run test suite on my-project and report results"
cron = "0 8 * * 1-5"            # Weekdays at 08:00 UTC
model_tier = "sonnet"
priority = "normal"
timeout_seconds = 300
prompt = """
Navigate to ~/projects/my-project and:
1. Run the test suite
2. Analyze any failures
3. Report coverage delta since last run
"""
context_files = ["projects/abc123/context.toml"]
output_metric = "custom.my_project_check"
```

### Trigger Types (from Clawith's Aware System)

Beyond cron, the scheduler supports six trigger types:

| Type | Description | Example |
| --- | --- | --- |
| `cron` | Recurring schedule (croner expression) | `0 6 * * *` (daily 6am) |
| `once` | One-time future execution | `2026-04-01T09:00:00Z` |
| `interval` | Every N duration | `every 30m`, `every 2h` |
| `poll` | Check condition, act if true | Poll project build status every 10m |
| `on_message` | React to incoming CLI/API request | User sends task via `autonomic run` |
| `webhook` | External HTTP trigger on daemon API | CI pipeline calls `/api/trigger/<job>` |

```rust
pub enum Trigger {
    Cron(CronExpr),
    Once(DateTime<Utc>),
    Interval(Duration),
    Poll { check: PollCheck, interval: Duration },
    OnMessage { filter: Option<String> },
    Webhook { path: String, secret: Option<String> },
}
```

The scheduler is **hot-reloadable** (from OpenClaw): config changes detected via file watcher restart the scheduler without restarting the daemon.

### Probe Tasks

Lightweight probe jobs that check conditions without consuming significant tokens:

```toml
[jobs.api_health_probe]
description = "Verify Claude API is responsive"
cron = "*/15 * * * *"           # Every 15 minutes
model_tier = "haiku"
priority = "critical"
timeout_seconds = 30
probe = true                     # Probe: minimal token usage, just tests connectivity
prompt = "Respond with exactly: OK"
output_metric = "probe.api_health"
max_tokens = 10
```

## Rust Types

```rust
use chrono::{DateTime, Utc};
use croner::Cron;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

/// Priority determines whether a job runs when budget is constrained.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Priority {
    /// Always runs regardless of budget pressure.
    Critical,
    /// Deferred when budget usage exceeds 80%.
    Normal,
    /// Deferred when budget usage exceeds 60%.
    Low,
}

impl Priority {
    /// Returns the budget usage threshold (0.0-1.0) above which this priority is deferred.
    fn deferral_threshold(self) -> f64 {
        match self {
            Self::Critical => 1.0, // Never deferred
            Self::Normal => 0.80,
            Self::Low => 0.60,
        }
    }
}

/// Which model tier to use for this job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum ModelTier {
    Haiku,
    Sonnet,
    Opus,
}

impl ModelTier {
    /// Tokens per 5-hour window for each tier (Claude Max plan, March 2026 estimates).
    fn window_token_limit(self) -> u64 {
        match self {
            Self::Haiku => 5_000_000,   // ~5M tokens / 5h window
            Self::Sonnet => 2_000_000,  // ~2M tokens / 5h window
            Self::Opus => 500_000,      // ~500K tokens / 5h window
        }
    }
}

/// A job definition as parsed from TOML.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct JobDefinition {
    description: String,
    cron: String,
    model_tier: ModelTier,
    priority: Priority,
    #[serde(default = "default_timeout")]
    timeout_seconds: u64,
    #[serde(default)]
    prompt: Option<String>,
    #[serde(default)]
    context_files: Vec<String>,
    #[serde(default)]
    output_metric: Option<String>,
    #[serde(default)]
    pre_snapshot: bool,
    #[serde(default)]
    builtin: bool,
    #[serde(default = "default_true")]
    enabled: bool,
    #[serde(default)]
    probe: bool,
    #[serde(default)]
    max_tokens: Option<u32>,
}

fn default_timeout() -> u64 {
    300
}

fn default_true() -> bool {
    true
}

/// Parsed and validated job ready for scheduling.
struct ScheduledJob {
    name: String,
    definition: JobDefinition,
    cron: Cron,
    next_run: DateTime<Utc>,
}

/// The top-level TOML structure for schedules/jobs.toml.
#[derive(Debug, Serialize, Deserialize)]
struct JobsConfig {
    #[serde(default)]
    scheduler: SchedulerConfig,
    #[serde(default)]
    jobs: HashMap<String, JobDefinition>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SchedulerConfig {
    #[serde(default = "default_true")]
    enabled: bool,
    #[serde(default = "default_timezone")]
    timezone: String,
    #[serde(default = "default_max_concurrent")]
    max_concurrent_jobs: u32,
    #[serde(default = "default_timeout")]
    default_timeout_seconds: u64,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        SchedulerConfig {
            enabled: true,
            timezone: "UTC".to_string(),
            max_concurrent_jobs: 2,
            default_timeout_seconds: 300,
        }
    }
}

fn default_timezone() -> String {
    "UTC".to_string()
}

fn default_max_concurrent() -> u32 {
    2
}

/// Record of a job execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct JobExecution {
    job_name: String,
    started_at: DateTime<Utc>,
    ended_at: Option<DateTime<Utc>>,
    status: JobStatus,
    model_tier: ModelTier,
    tokens_used: u64,
    output: Option<String>,
    error: Option<String>,
    deferred: bool,
    defer_reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum JobStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Deferred,
    TimedOut,
}

/// Tracks token usage within the current 5-hour sliding window.
#[derive(Debug, Clone)]
struct QuotaTracker {
    windows: HashMap<ModelTier, WindowUsage>,
}

#[derive(Debug, Clone)]
struct WindowUsage {
    window_start: DateTime<Utc>,
    tokens_used: u64,
    tokens_limit: u64,
    requests_used: u64,
    requests_limit: u64,
    allocations: HashMap<Subsystem, f64>, // subsystem -> percentage
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
enum Subsystem {
    Interactive,
    Scheduled,
    Evolution,
    Monitoring,
}

impl WindowUsage {
    /// Returns overall usage as a fraction (0.0 to 1.0).
    fn usage_fraction(&self) -> f64 {
        if self.tokens_limit == 0 {
            return 1.0;
        }
        self.tokens_used as f64 / self.tokens_limit as f64
    }

    /// Returns the token budget remaining for the scheduled subsystem.
    fn scheduled_budget_remaining(&self) -> u64 {
        let pct = self.allocations.get(&Subsystem::Scheduled).copied().unwrap_or(25.0);
        let allocated = (self.tokens_limit as f64 * pct / 100.0) as u64;
        // Estimate how many tokens scheduled jobs have used (tracked separately)
        // For now, use overall usage as a conservative proxy
        allocated.saturating_sub(self.tokens_used * allocated / self.tokens_limit.max(1))
    }
}

impl QuotaTracker {
    fn new() -> Self {
        QuotaTracker {
            windows: HashMap::new(),
        }
    }

    /// Refresh window data from the state database.
    fn refresh(&mut self, state: &StateManager) -> Result<(), StateError> {
        for tier in &[ModelTier::Haiku, ModelTier::Sonnet, ModelTier::Opus] {
            let usage = state.get_budget_usage(*tier)?;
            let allocations = state.get_budget_allocations(*tier)?;

            self.windows.insert(*tier, WindowUsage {
                window_start: Utc::now() - chrono::Duration::hours(5),
                tokens_used: usage.tokens_used,
                tokens_limit: usage.tokens_limit,
                requests_used: usage.requests_used,
                requests_limit: usage.requests_limit,
                allocations,
            });
        }
        Ok(())
    }

    /// Check if a job should be deferred based on budget pressure.
    fn should_defer(&self, tier: ModelTier, priority: Priority) -> Option<String> {
        let window = self.windows.get(&tier)?;
        let usage = window.usage_fraction();
        let threshold = priority.deferral_threshold();

        if usage >= threshold {
            Some(format!(
                "Budget usage {:.0}% exceeds {:.0}% threshold for {:?} priority on {:?} tier",
                usage * 100.0,
                threshold * 100.0,
                priority,
                tier,
            ))
        } else {
            None
        }
    }

    /// Record token usage after a job completes.
    fn record_usage(
        &mut self,
        tier: ModelTier,
        tokens: u64,
        state: &StateManager,
    ) -> Result<(), StateError> {
        state.increment_budget_usage(tier, tokens)?;
        if let Some(window) = self.windows.get_mut(&tier) {
            window.tokens_used += tokens;
        }
        Ok(())
    }
}
```

## Cron Parsing with croner

The `croner` crate provides cron expression parsing compatible with standard 5-field cron syntax plus extensions (seconds, `L`, `W`, `#`).

```rust
use croner::Cron;
use chrono::{DateTime, Utc};

fn parse_cron(expression: &str) -> Result<Cron, SchedulerError> {
    Cron::new(expression)
        .parse()
        .map_err(|e| SchedulerError::InvalidCron {
            expression: expression.to_string(),
            error: e.to_string(),
        })
}

fn next_occurrence(cron: &Cron) -> Option<DateTime<Utc>> {
    cron.find_next_occurrence(&Utc::now(), false).ok()
}

/// Calculate all occurrences within a time range (for budget forecasting).
fn occurrences_in_range(
    cron: &Cron,
    start: &DateTime<Utc>,
    end: &DateTime<Utc>,
) -> Vec<DateTime<Utc>> {
    let mut times = Vec::new();
    let mut current = *start;
    while let Ok(next) = cron.find_next_occurrence(&current, false) {
        if next > *end {
            break;
        }
        times.push(next);
        current = next + chrono::Duration::seconds(1);
    }
    times
}
```

## Scheduler Loop

The scheduler runs as a `tokio` task within the orchestrator process. It wakes up every 30 seconds to check for due jobs, or is notified early by a channel when jobs are added/modified.

The scheduler maintains a unified next-fire queue across all trigger types. Cron and interval triggers compute their next fire time. Once triggers fire and self-remove. Poll triggers run their check function at their interval. OnMessage and webhook triggers are event-driven (not polled).

```rust
use tokio::sync::{mpsc, Notify};
use tokio::task::JoinHandle;
use std::sync::Arc;

struct Scheduler {
    state: Arc<StateManager>,
    jobs: Vec<ScheduledJob>,
    quota: QuotaTracker,
    config: SchedulerConfig,
    running_jobs: Vec<JoinHandle<JobExecution>>,
    notify: Arc<Notify>,
}

enum SchedulerCommand {
    Reload,
    RunNow(String),
    Shutdown,
}

impl Scheduler {
    fn new(state: Arc<StateManager>) -> Result<Self, SchedulerError> {
        let config = Self::load_config(&state)?;
        let jobs = Self::load_jobs(&state, &config)?;
        let mut quota = QuotaTracker::new();
        quota.refresh(&state)?;

        Ok(Scheduler {
            state,
            jobs,
            quota,
            config,
            running_jobs: Vec::new(),
            notify: Arc::new(Notify::new()),
        })
    }

    fn load_config(state: &StateManager) -> Result<SchedulerConfig, SchedulerError> {
        let path = state.root.join("schedules/jobs.toml");
        let content = std::fs::read_to_string(&path)?;
        let config: JobsConfig = toml::from_str(&content)?;
        Ok(config.scheduler)
    }

    fn load_jobs(
        state: &StateManager,
        config: &SchedulerConfig,
    ) -> Result<Vec<ScheduledJob>, SchedulerError> {
        let path = state.root.join("schedules/jobs.toml");
        let content = std::fs::read_to_string(&path)?;
        let jobs_config: JobsConfig = toml::from_str(&content)?;

        let mut scheduled = Vec::new();
        for (name, def) in &jobs_config.jobs {
            if !def.enabled {
                tracing::info!(job = %name, "Job disabled, skipping");
                continue;
            }

            let cron = parse_cron(&def.cron)?;
            let next_run = next_occurrence(&cron).ok_or_else(|| {
                SchedulerError::NoCronMatch(name.clone())
            })?;

            scheduled.push(ScheduledJob {
                name: name.clone(),
                definition: def.clone(),
                cron,
                next_run,
            });
        }

        // Sort by next_run so the soonest job is first
        scheduled.sort_by_key(|j| j.next_run);
        Ok(scheduled)
    }

    /// Main scheduler loop. Runs until shutdown signal.
    async fn run(
        &mut self,
        mut cmd_rx: mpsc::Receiver<SchedulerCommand>,
    ) -> Result<(), SchedulerError> {
        tracing::info!(
            job_count = self.jobs.len(),
            "Scheduler started"
        );

        loop {
            // Calculate sleep duration: time until next job or 30s max
            let sleep_duration = self.time_until_next_job()
                .unwrap_or(Duration::from_secs(30))
                .min(Duration::from_secs(30));

            tokio::select! {
                _ = tokio::time::sleep(sleep_duration) => {
                    self.tick().await?;
                }
                _ = self.notify.notified() => {
                    // Early wake: re-evaluate immediately
                    self.tick().await?;
                }
                cmd = cmd_rx.recv() => {
                    match cmd {
                        Some(SchedulerCommand::Reload) => {
                            self.reload()?;
                        }
                        Some(SchedulerCommand::RunNow(name)) => {
                            self.run_job_by_name(&name).await?;
                        }
                        Some(SchedulerCommand::Shutdown) | None => {
                            tracing::info!("Scheduler shutting down");
                            self.drain_running_jobs().await;
                            return Ok(());
                        }
                    }
                }
            }

            // Reap completed jobs
            self.reap_completed_jobs().await;
        }
    }

    fn time_until_next_job(&self) -> Option<Duration> {
        self.jobs.first().map(|j| {
            let now = Utc::now();
            if j.next_run <= now {
                Duration::from_secs(0)
            } else {
                (j.next_run - now).to_std().unwrap_or(Duration::from_secs(0))
            }
        })
    }

    /// Called every tick: find due jobs and execute them.
    async fn tick(&mut self) -> Result<(), SchedulerError> {
        let now = Utc::now();

        // Refresh quota data periodically
        self.quota.refresh(&self.state)?;

        // Collect due jobs
        let due_jobs: Vec<usize> = self.jobs.iter()
            .enumerate()
            .filter(|(_, j)| j.next_run <= now)
            .map(|(i, _)| i)
            .collect();

        for idx in due_jobs {
            let job = &self.jobs[idx];

            // Check concurrency limit
            if self.running_jobs.len() >= self.config.max_concurrent_jobs as usize {
                tracing::warn!(
                    job = %job.name,
                    "Concurrency limit reached, deferring"
                );
                continue;
            }

            // Check budget pressure
            if let Some(reason) = self.quota.should_defer(
                job.definition.model_tier,
                job.definition.priority,
            ) {
                tracing::info!(job = %job.name, reason = %reason, "Deferring job");
                self.record_deferral(&job.name, &reason).await?;

                // Apply adaptive scheduling: push next_run by one interval
                self.adapt_interval(idx);
                continue;
            }

            // Execute the job
            let handle = self.spawn_job(idx).await?;
            self.running_jobs.push(handle);

            // Advance to next occurrence
            if let Some(next) = self.jobs[idx].cron
                .find_next_occurrence(&now, false)
                .ok()
            {
                self.jobs[idx].next_run = next;
            }
        }

        // Re-sort jobs by next_run
        self.jobs.sort_by_key(|j| j.next_run);

        Ok(())
    }

    /// Spawn a job as an async task.
    async fn spawn_job(
        &self,
        job_idx: usize,
    ) -> Result<JoinHandle<JobExecution>, SchedulerError> {
        let job = &self.jobs[job_idx];
        let job_name = job.name.clone();
        let definition = job.definition.clone();
        let state = Arc::clone(&self.state);

        tracing::info!(
            job = %job_name,
            model_tier = ?definition.model_tier,
            priority = ?definition.priority,
            "Executing job"
        );

        // Create pre-snapshot if requested
        if definition.pre_snapshot {
            let tag = format!("evolution/{}-pre", job_name);
            state.snapshot(&tag, &format!("Pre-execution snapshot for {}", job_name))?;
        }

        let handle = tokio::spawn(async move {
            let started_at = Utc::now();

            let result = if definition.builtin {
                execute_builtin_job(&job_name, &state).await
            } else {
                execute_claude_session(&definition, &state).await
            };

            let ended_at = Utc::now();

            match result {
                Ok(output) => {
                    // Store output metric if configured
                    if let Some(metric_name) = &definition.output_metric {
                        let _ = store_job_metric(&state, metric_name, &output);
                    }

                    JobExecution {
                        job_name,
                        started_at,
                        ended_at: Some(ended_at),
                        status: JobStatus::Completed,
                        model_tier: definition.model_tier,
                        tokens_used: output.tokens_used,
                        output: Some(output.text),
                        error: None,
                        deferred: false,
                        defer_reason: None,
                    }
                }
                Err(e) => {
                    tracing::error!(job = %job_name, error = %e, "Job failed");
                    JobExecution {
                        job_name,
                        started_at,
                        ended_at: Some(ended_at),
                        status: if e.is_timeout() {
                            JobStatus::TimedOut
                        } else {
                            JobStatus::Failed
                        },
                        model_tier: definition.model_tier,
                        tokens_used: 0,
                        output: None,
                        error: Some(e.to_string()),
                        deferred: false,
                        defer_reason: None,
                    }
                }
            }
        });

        Ok(handle)
    }

    fn reload(&mut self) -> Result<(), SchedulerError> {
        self.config = Self::load_config(&self.state)?;
        self.jobs = Self::load_jobs(&self.state, &self.config)?;
        tracing::info!(job_count = self.jobs.len(), "Scheduler reloaded");
        Ok(())
    }

    /// Adaptive scheduling: when a job is deferred, extend its next run
    /// by one additional interval to reduce pressure.
    fn adapt_interval(&mut self, idx: usize) {
        let job = &mut self.jobs[idx];
        let current_next = job.next_run;

        // Find the occurrence after the current next_run
        if let Ok(after_next) = job.cron.find_next_occurrence(&current_next, false) {
            let interval = after_next - current_next;
            job.next_run = current_next + interval;
            tracing::info!(
                job = %job.name,
                next_run = %job.next_run,
                "Adapted interval due to budget pressure"
            );
        }
    }

    async fn record_deferral(
        &self,
        job_name: &str,
        reason: &str,
    ) -> Result<(), SchedulerError> {
        let metric = Metric {
            id: 0,
            timestamp: Utc::now(),
            session_id: None,
            metric_name: format!("scheduler.deferred.{}", job_name),
            metric_value: 1.0,
            labels: Some(serde_json::json!({ "reason": reason })),
        };
        self.state.record_metric(&metric)?;
        Ok(())
    }

    async fn reap_completed_jobs(&mut self) {
        let mut remaining = Vec::new();
        for handle in self.running_jobs.drain(..) {
            if handle.is_finished() {
                match handle.await {
                    Ok(execution) => {
                        tracing::info!(
                            job = %execution.job_name,
                            status = ?execution.status,
                            tokens = execution.tokens_used,
                            "Job completed"
                        );
                        // Record execution in database
                        let _ = self.record_execution(&execution);
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "Job task panicked");
                    }
                }
            } else {
                remaining.push(handle);
            }
        }
        self.running_jobs = remaining;
    }

    fn record_execution(&self, exec: &JobExecution) -> Result<(), SchedulerError> {
        self.state.state_db.execute(
            "INSERT INTO metrics (timestamp, metric_name, metric_value, labels)
             VALUES (?1, ?2, ?3, ?4)",
            sqlx::query!(
                exec.started_at.to_rfc3339(),
                format!("scheduler.execution.{}", exec.job_name),
                match exec.status {
                    JobStatus::Completed => 1.0,
                    JobStatus::Failed | JobStatus::TimedOut => 0.0,
                    _ => -1.0,
                },
                serde_json::to_string(&serde_json::json!({
                    "status": format!("{:?}", exec.status),
                    "tokens_used": exec.tokens_used,
                    "model_tier": format!("{:?}", exec.model_tier),
                    "duration_seconds": exec.ended_at.map(|e| (e - exec.started_at).num_seconds()),
                })).ok(),
            ],
        )?;
        Ok(())
    }

    async fn drain_running_jobs(&mut self) {
        for handle in self.running_jobs.drain(..) {
            let _ = handle.await;
        }
    }

    async fn run_job_by_name(&mut self, name: &str) -> Result<(), SchedulerError> {
        let idx = self.jobs.iter()
            .position(|j| j.name == name)
            .ok_or_else(|| SchedulerError::JobNotFound(name.to_string()))?;
        let handle = self.spawn_job(idx).await?;
        self.running_jobs.push(handle);
        Ok(())
    }
}
```

## Job Execution

### Claude Code Sessions

Non-builtin jobs spawn a Claude Code subprocess with the job's prompt and context:

```rust
use tokio::process::Command;

struct JobOutput {
    text: String,
    tokens_used: u64,
    exit_code: i32,
}

async fn execute_claude_session(
    def: &JobDefinition,
    state: &StateManager,
) -> Result<JobOutput, JobError> {
    let mut cmd = Command::new("claude");

    // Model selection based on tier
    let model_flag = match def.model_tier {
        ModelTier::Haiku => "--model=haiku",
        ModelTier::Sonnet => "--model=sonnet",
        ModelTier::Opus => "--model=opus",
    };
    cmd.arg(model_flag);

    // Non-interactive, print output
    cmd.arg("--print");

    // Max tokens for probes
    if let Some(max_tokens) = def.max_tokens {
        cmd.arg(format!("--max-tokens={}", max_tokens));
    }

    // Build the prompt with context
    let prompt = build_job_prompt(def, state)?;
    cmd.arg("--prompt").arg(&prompt);

    // Set working directory to autonomic root for context file access
    cmd.current_dir(&state.root);

    // Timeout
    let timeout = Duration::from_secs(def.timeout_seconds);

    let output = tokio::time::timeout(timeout, cmd.output())
        .await
        .map_err(|_| JobError::Timeout(def.timeout_seconds))?
        .map_err(JobError::Spawn)?;

    let text = String::from_utf8_lossy(&output.stdout).to_string();
    let exit_code = output.status.code().unwrap_or(-1);

    // Parse token usage from Claude's output metadata (stderr contains usage info)
    let stderr = String::from_utf8_lossy(&output.stderr);
    let tokens_used = parse_token_usage(&stderr).unwrap_or(0);

    if exit_code != 0 {
        return Err(JobError::NonZeroExit {
            code: exit_code,
            stderr: stderr.to_string(),
        });
    }

    Ok(JobOutput {
        text,
        tokens_used,
        exit_code,
    })
}

fn build_job_prompt(def: &JobDefinition, state: &StateManager) -> Result<String, JobError> {
    let mut prompt = String::new();

    // Include context files
    for ctx_file in &def.context_files {
        let path = state.root.join(ctx_file);
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            prompt.push_str(&format!(
                "<context file=\"{}\">\n{}\n</context>\n\n",
                ctx_file, content
            ));
        }
    }

    // Include recent metrics summary for analysis jobs
    if def.model_tier != ModelTier::Haiku || !def.probe {
        let metrics_summary = get_recent_metrics_summary(state)?;
        if !metrics_summary.is_empty() {
            prompt.push_str(&format!(
                "<context source=\"recent_metrics\">\n{}\n</context>\n\n",
                metrics_summary
            ));
        }
    }

    // The main prompt
    if let Some(ref p) = def.prompt {
        prompt.push_str(p);
    }

    Ok(prompt)
}

fn parse_token_usage(stderr: &str) -> Option<u64> {
    // Claude CLI outputs usage info on stderr in a known format
    // e.g., "Total tokens: 1234"
    for line in stderr.lines() {
        if let Some(rest) = line.strip_prefix("Total tokens: ") {
            return rest.trim().parse().ok();
        }
    }
    None
}
```

### Built-in Jobs

Built-in jobs execute internal Rust functions, not Claude sessions:

```rust
async fn execute_builtin_job(
    name: &str,
    state: &StateManager,
) -> Result<JobOutput, JobError> {
    match name {
        "daily_snapshot" => {
            // Checkpoint databases and create tagged snapshot
            state.snapshot(
                &format!("daily/{}", Utc::now().format("%Y-%m-%d")),
                &format!("Daily snapshot {}", Utc::now().format("%Y-%m-%d")),
            )?;

            // Prune old daily tags
            let pruned = prune_daily_tags(&state.repo, 30)?;

            Ok(JobOutput {
                text: format!("{{\"snapshot\": \"ok\", \"pruned_tags\": {}}}", pruned),
                tokens_used: 0,
                exit_code: 0,
            })
        }
        "backup" => {
            // Push to configured remote
            let config = state.load_config()?;
            let remote_url = config.backup.remote
                .ok_or(JobError::MissingConfig("backup.remote".to_string()))?;

            push_to_remote(&state.repo, &remote_url)?;

            Ok(JobOutput {
                text: "{\"backup\": \"ok\"}".to_string(),
                tokens_used: 0,
                exit_code: 0,
            })
        }
        _ => Err(JobError::UnknownBuiltin(name.to_string())),
    }
}

fn push_to_remote(repo: &Repository, url: &str) -> Result<(), JobError> {
    let mut remote = match repo.find_remote("backup") {
        Ok(r) => r,
        Err(_) => repo.remote("backup", url)?,
    };

    remote.push(
        &["refs/heads/*:refs/heads/*", "refs/tags/*:refs/tags/*"],
        None,
    )?;

    Ok(())
}
```

## Rate Budget Math

### 5-Hour Sliding Window

Claude's rate limits operate on a 5-hour sliding window. The scheduler must track and forecast usage within this window.

```rust
/// Rate budget constants per tier (Claude Max, estimated March 2026).
/// These are configurable in config.toml to adapt to plan changes.
struct RateLimits {
    haiku_tokens_per_window: u64,
    sonnet_tokens_per_window: u64,
    opus_tokens_per_window: u64,
}

impl Default for RateLimits {
    fn default() -> Self {
        RateLimits {
            haiku_tokens_per_window: 5_000_000,
            sonnet_tokens_per_window: 2_000_000,
            opus_tokens_per_window: 500_000,
        }
    }
}

/// Forecast token usage for the next window based on scheduled jobs.
fn forecast_window_usage(
    jobs: &[ScheduledJob],
    window_start: DateTime<Utc>,
    window_end: DateTime<Utc>,
) -> HashMap<ModelTier, u64> {
    let mut forecast: HashMap<ModelTier, u64> = HashMap::new();

    for job in jobs {
        if !job.definition.enabled {
            continue;
        }

        let occurrences = occurrences_in_range(
            &job.cron,
            &window_start,
            &window_end,
        );

        // Estimate tokens per execution based on history or defaults
        let estimated_tokens = estimate_job_tokens(&job.definition);

        let total = occurrences.len() as u64 * estimated_tokens;
        *forecast.entry(job.definition.model_tier).or_insert(0) += total;
    }

    forecast
}

/// Estimate token usage for a single job execution.
fn estimate_job_tokens(def: &JobDefinition) -> u64 {
    if def.probe {
        return 50; // Probes use minimal tokens
    }
    if def.builtin {
        return 0; // Built-in jobs don't use API tokens
    }

    // Default estimates by tier (conservative)
    match def.model_tier {
        ModelTier::Haiku => 2_000,
        ModelTier::Sonnet => 5_000,
        ModelTier::Opus => 10_000,
    }
}

/// Determine if a job can run without exceeding the subsystem's allocation.
fn can_afford_job(
    quota: &QuotaTracker,
    def: &JobDefinition,
) -> BudgetDecision {
    let tier = def.model_tier;
    let estimated = estimate_job_tokens(def);

    let Some(window) = quota.windows.get(&tier) else {
        return BudgetDecision::Allow; // No data, allow optimistically
    };

    let usage_fraction = window.usage_fraction();
    let threshold = def.priority.deferral_threshold();

    if usage_fraction >= threshold {
        return BudgetDecision::Defer(format!(
            "Usage {:.1}% >= {:.1}% threshold for {:?}",
            usage_fraction * 100.0,
            threshold * 100.0,
            def.priority,
        ));
    }

    // Check if this specific job would push us over the subsystem allocation
    let scheduled_remaining = window.scheduled_budget_remaining();
    if estimated > scheduled_remaining {
        return BudgetDecision::Defer(format!(
            "Estimated {} tokens exceeds scheduled budget remaining {}",
            estimated, scheduled_remaining,
        ));
    }

    BudgetDecision::Allow
}

enum BudgetDecision {
    Allow,
    Defer(String),
}
```

### Allocation Percentages

Default budget allocation across subsystems (configurable in `PostgreSQL` `budget_allocations` table):

| Tier | Interactive | Scheduled | Evolution | Monitoring |
| --- | --- | --- | --- | --- |
| Haiku | 20% | 40% | 10% | 30% |
| Sonnet | 50% | 25% | 15% | 10% |
| Opus | 60% | 15% | 20% | 5% |

Rationale:

- **Haiku**: Cheap tier, heavily used for scheduled monitoring and probes.
- **Sonnet**: Primary interactive tier, most budget reserved for user sessions.
- **Opus**: Expensive tier, reserved mainly for interactive decisions and evolution analysis.

### Adaptive Scheduling Example

When budget pressure is detected, the scheduler adjusts:

1. **60-80% usage**: Low-priority jobs deferred to next interval.
2. **80-95% usage**: Normal-priority jobs also deferred. Low-priority jobs skip entirely until next window.
3. **95%+ usage**: Only critical jobs run. Scheduler enters "conservation mode" and logs a warning.

```rust
fn compute_scheduling_mode(usage_fraction: f64) -> SchedulingMode {
    if usage_fraction >= 0.95 {
        SchedulingMode::Conservation
    } else if usage_fraction >= 0.80 {
        SchedulingMode::Constrained
    } else if usage_fraction >= 0.60 {
        SchedulingMode::Cautious
    } else {
        SchedulingMode::Normal
    }
}

#[derive(Debug, Clone, Copy)]
enum SchedulingMode {
    /// All jobs run normally.
    Normal,
    /// Low-priority jobs deferred.
    Cautious,
    /// Normal and low priority deferred.
    Constrained,
    /// Only critical jobs run. Warning logged.
    Conservation,
}
```

## Schedule State Tracking

Last-run timestamps and next-run calculations are persisted in `~/.autonomic/schedules/state.toml`:

```toml
# ~/.autonomic/schedules/state.toml
# Auto-generated by the scheduler. Do not edit manually.

[last_run]
daily_health_check = "2026-03-25T06:00:00Z"
weekly_evolution_analysis = "2026-03-22T02:00:00Z"
daily_reflection = "2026-03-24T23:00:00Z"
daily_snapshot = "2026-03-25T03:00:00Z"
budget_recalibration = "2026-03-25T12:00:00Z"

[next_run]
daily_health_check = "2026-03-26T06:00:00Z"
weekly_evolution_analysis = "2026-03-29T02:00:00Z"
daily_reflection = "2026-03-25T23:00:00Z"
daily_snapshot = "2026-03-26T03:00:00Z"
budget_recalibration = "2026-03-25T18:00:00Z"

[deferral_count]
daily_reflection = 0
budget_recalibration = 1
```

On startup, the scheduler reads this file to determine which jobs are overdue (should have run while Autonomic was not running) and executes them in priority order.

## Error Types

```rust
#[derive(Debug, thiserror::Error)]
enum SchedulerError {
    #[error("Invalid cron expression '{expression}': {error}")]
    InvalidCron { expression: String, error: String },

    #[error("No matching occurrence for job '{0}'")]
    NoCronMatch(String),

    #[error("Job not found: {0}")]
    JobNotFound(String),

    #[error("State error: {0}")]
    State(#[from] StateError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML error: {0}")]
    Toml(#[from] toml::de::Error),
}

#[derive(Debug, thiserror::Error)]
enum JobError {
    #[error("Job timed out after {0} seconds")]
    Timeout(u64),

    #[error("Failed to spawn process: {0}")]
    Spawn(std::io::Error),

    #[error("Job exited with code {code}: {stderr}")]
    NonZeroExit { code: i32, stderr: String },

    #[error("Unknown built-in job: {0}")]
    UnknownBuiltin(String),

    #[error("Missing config: {0}")]
    MissingConfig(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("State error: {0}")]
    State(#[from] StateError),

    #[error("Git error: {0}")]
    Git(#[from] gix::open::Error),
}

impl JobError {
    fn is_timeout(&self) -> bool {
        matches!(self, Self::Timeout(_))
    }
}
```

## Crate Dependencies

```toml
[dependencies]
croner = "2"
tokio = { version = "1", features = ["full"] }
chrono = { version = "0.4", features = ["serde"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
tracing = "0.1"
thiserror = "2"
sqlx = { version = "0.8", features = ["runtime-tokio", "postgres"] }
gix = { version = "0.81", default-features = false, features = ["revision"] }
```

## Integration with State Management

The scheduler interacts with the state management system (see `state-management.md`) in the following ways:

1. **Job config** (`schedules/jobs.toml`): Tracked by git. Changes trigger a `schedule: ...` commit.
2. **Schedule state** (`schedules/state.toml`): Tracked by git. Updated after every job execution.
3. **Metrics**: Written to `PostgreSQL` via `StateManager::record_metric`.
4. **Budget tracking**: Read from `PostgreSQL` `rate_budget` table; written back after each job.
5. **Snapshots**: The `daily_snapshot` built-in job calls `StateManager::snapshot`.
6. **Evolution tags**: Jobs with `pre_snapshot = true` create evolution tags via `StateManager`.

The scheduler never writes to PostgreSQL directly; it always goes through the `StateManager` API to maintain the single-writer invariant.
