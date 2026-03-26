# Async Concurrency Patterns — Autonomic

## Runtime

tokio multi-threaded work-stealing. Never mix runtimes.

## Subprocess Management

Claude Code runs inside containers. The orchestrator manages the container lifecycle:

```rust
use tokio::process::Command;
use tokio::io::{AsyncBufReadExt, BufReader};

async fn run_session(config: &SessionConfig) -> Result<SessionOutput> {
    let mut child = Command::new("podman")
        .args(["run", "--rm", "-v", &mount_spec, /* ... */])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    let stdout = BufReader::new(child.stdout.take().unwrap());
    let stderr = BufReader::new(child.stderr.take().unwrap());

    // CRITICAL: consume both streams concurrently (XD-005)
    let mut stdout_lines = stdout.lines();
    let mut stderr_lines = stderr.lines();

    loop {
        tokio::select! {
            line = stdout_lines.next_line() => {
                match line? {
                    Some(line) => parse_ndjson(&line)?,
                    None => break,
                }
            }
            line = stderr_lines.next_line() => {
                if let Some(line) = line? {
                    handle_stderr(&line);
                }
            }
        }
    }

    let status = child.wait().await?;
    Ok(SessionOutput { status, /* ... */ })
}
```

## Timeout Enforcement

```rust
use tokio::time::{timeout, Duration};

let result = timeout(Duration::from_secs(1800), run_session(&config)).await;
match result {
    Ok(Ok(output)) => handle_success(output),
    Ok(Err(e)) => handle_error(e),
    Err(_) => handle_timeout(), // 30 minute timeout exceeded
}
```

## Channel Patterns

```rust
// Bounded command channel for inter-component communication
let (tx, mut rx) = tokio::sync::mpsc::channel::<OrchestratorCommand>(32);

// Watch channel for config broadcast (single-producer, multi-consumer)
let (config_tx, config_rx) = tokio::sync::watch::channel(initial_config);

// Oneshot for request-reply patterns
let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
```

## Structured Concurrency with JoinSet

```rust
use tokio::task::JoinSet;

let mut set = JoinSet::new();

// Spawn tasks — auto-aborted when JoinSet is dropped
for project in projects {
    set.spawn(run_health_check(project));
}

// Collect results
while let Some(result) = set.join_next().await {
    match result {
        Ok(Ok(health)) => report_healthy(health),
        Ok(Err(e)) => report_unhealthy(e),
        Err(join_err) => report_panic(join_err),
    }
}
```

## Blocking Operations

```rust
// gix operations may block — always spawn_blocking
let repo = tokio::task::spawn_blocking(move || {
    gix::open(&repo_path)
}).await??;

// sqlx is async-native — no spawn_blocking needed
let entries = sqlx::query_as!(MemoryEntry, "SELECT * FROM memory_entries WHERE ...")
    .fetch_all(&pool)
    .await?;
```

## Scheduler Integration

```rust
use croner::Cron;
use tokio::time::{sleep_until, Instant};

async fn scheduler_loop(jobs: Vec<JobDefinition>) {
    loop {
        let next = jobs.iter()
            .filter_map(|j| j.cron.find_next_occurrence().ok())
            .min();

        if let Some(next_time) = next {
            sleep_until(Instant::from_std(next_time)).await;
            // Execute due jobs
        }
    }
}
```

## Anti-Patterns (Never Do)

- `std::thread::sleep` — blocks the runtime. Use `tokio::time::sleep`.
- `std::process::Command` — blocks the runtime. Use `tokio::process::Command`.
- Unbounded channels — can cause OOM. Always use bounded.
- Holding `std::sync::Mutex` across `.await` — deadlock. Clone out first.
- `println!` / `eprintln!` — use `tracing::info!` / `tracing::error!`.
