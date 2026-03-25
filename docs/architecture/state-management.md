# State Management

Autonomic state is split across two stores:

1. **Shared state** (sessions, metrics, traces, evolution data) lives in **PostgreSQL**, running in a Podman container. This enables concurrent multi-session writes and rich querying.
2. **Config state** (hooks, rules, agents, specs) stays in **git-backed files** under `~/.autonomic/`. Every mutation to tracked files results in an atomic git commit, providing a complete audit trail and point-in-time recovery.

PostgreSQL credentials are stored in `secrets.toml` (gitignored) and injected into the Podman compose environment. The compose file also supports Docker secrets for production deployments.

## Directory Structure

```txt
~/.autonomic/
  config.toml              # User configuration (model preferences, API keys ref, thresholds)
  compose.yaml             # Podman compose for PostgreSQL + agent containers
  secrets.toml             # Database credentials, API keys (NEVER committed)
  projects/
    <project-hash>/
      context.toml         # Project-specific overrides, model tier preferences
  evolution/
    strategies.toml        # Current active evolution strategies
    changelog.toml         # Record of all strategy mutations with timestamps
    pending/               # Staged evolution proposals awaiting approval
  hooks/
    pre-session.sh         # Optional user hooks
    post-session.sh
    on-error.sh
  schedules/
    jobs.toml              # Cron job definitions (see scheduler.md)
    state.toml             # Last-run timestamps, next-run calculations
  logs/
    autonomic.log          # Current rotating log (not git-committed)
    sessions/
      <session-id>.log     # Per-session logs (not git-committed)
```

### Watchdog Integration

The watchdog process (separate binary, `autonomic-watchdog`) integrates with state management:

1. **Permission verification**: On startup, watchdog checks that all files in the modification frontier's frozen list have `444` permissions. If any are writable, it sets them read-only and logs a warning.

2. **Last known-good tracking**: After every successful 72h evolution verification window, the watchdog records the current git tag as `last-known-good` in `~/.autonomic/watchdog-state.toml`.

3. **Crash rollback**: If the daemon crashes within 5 minutes of an evolution deployment (detected via PID file + git log timestamps), the watchdog:
   a. Runs `git checkout <last-known-good-tag>`
   b. Restores PostgreSQL from the latest `pg_dump` backup
   c. Logs the rollback event
   d. Restarts the daemon

4. **Health monitoring**: Polls `http://localhost:<port>/health` every 30 seconds. 3 consecutive failures trigger a restart (without rollback — only evolution-related crashes trigger rollback).

```toml
# ~/.autonomic/watchdog-state.toml
[watchdog]
last_known_good_tag = 'evolution/evo-042-deployed'
last_health_check = '2026-03-25T10:30:00Z'
consecutive_failures = 0
last_rollback = '2026-03-20T14:22:00Z'
last_rollback_reason = 'daemon crash within 5min of evo-039 deployment'
```

### Git Ignore Rules

Large or high-churn files are excluded from git tracking to keep the repository small and commits fast:

```txt
# ~/.autonomic/.gitignore
logs/
backups/              # pg_dump backups (large, managed separately)
*.tmp
secrets.toml          # NEVER committed — API keys, tokens, Postgres credentials
daemon-heartbeat      # Transient runtime state
state/sessions/*.pid  # Transient PID files
```

### Secret Management (Gemini Review Finding)

`config.toml` is git-tracked. API keys and tokens MUST NOT be placed in `config.toml`. Instead:

- **`secrets.toml`** (gitignored): Stores API keys for Codex, Gemini, and any other external services.
- **Environment variable substitution**: `config.toml` references secrets via `${env:CODEX_API_KEY}` syntax. Figment resolves these at load time.
- **macOS Keychain** (future): For maximum security, secrets can be stored in Keychain and retrieved at runtime via `security find-generic-password`.

```toml
# ~/.autonomic/secrets.toml (NEVER committed)
[database]
url = "postgresql://autonomic:changeme@localhost:5432/autonomic"

[external.codex]
api_key = "sk-..."

[external.gemini]
api_key = "AI..."

# Or in config.toml with env reference (committed safely):
# [database]
# url = "${env:DATABASE_URL}"
```

PostgreSQL credentials are also injected into the compose environment via `secrets.toml` or Docker/Podman compose secrets. The compose file references `secrets.toml` for the `POSTGRES_PASSWORD` environment variable.

## Git-Backed State

### Initialization

On first run, Autonomic initializes `~/.autonomic/` as a git repository:

```rust
use gix::ThreadSafeRepository;
use std::path::Path;

fn init_state_repo(autonomic_dir: &Path) -> Result<ThreadSafeRepository, gix::init::Error> {
    if autonomic_dir.join(".git").exists() {
        Ok(ThreadSafeRepository::open(autonomic_dir)?)
    } else {
        let repo = ThreadSafeRepository::init(autonomic_dir)?;

        // Create initial commit with empty tree
        let repo_mut = repo.to_thread_local();
        let sig = gix::actor::SignatureRef {
            name: "autonomic".into(),
            email: "autonomic@localhost".into(),
            time: gix::date::Time::now_local_or_utc(),
        };
        let empty_tree = repo_mut.write_object(&gix::objs::Tree::empty())?;
        repo_mut.commit(
            "HEAD",
            sig,
            sig,
            "autonomic: initialize state repository",
            empty_tree,
            gix::commit::NO_PARENT_IDS,
        )?;

        Ok(repo)
    }
}
```

### Auto-Commit on Mutation

Every mutation to a tracked file triggers an atomic commit. The commit message encodes the operation type for machine-readable history:

```rust
/// Commit categories embedded in commit messages for filtering.
#[derive(Debug, Clone, Copy)]
enum CommitKind {
    Config,
    Evolution,
    Schedule,
    Snapshot,
    Migration,
    Rollback,
}

impl CommitKind {
    fn prefix(self) -> &'static str {
        match self {
            Self::Config => "config",
            Self::Evolution => "evolution",
            Self::Schedule => "schedule",
            Self::Snapshot => "snapshot",
            Self::Migration => "migration",
            Self::Rollback => "rollback",
        }
    }
}

fn commit_mutation(
    repo: &gix::Repository,
    paths: &[&Path],
    kind: CommitKind,
    message: &str,
) -> Result<gix::ObjectId, StateError> {
    let workdir = repo.work_dir().ok_or(StateError::InvalidPath)?;
    let mut index = repo.open_index()?;

    for path in paths {
        let relative = path.strip_prefix(workdir)?;
        index.add_path(relative)?;
    }
    index.write(Default::default())?;

    let tree_id = index.write_tree()?;
    let head = repo.head_commit()?;

    let sig = gix::actor::SignatureRef {
        name: "autonomic".into(),
        email: "autonomic@localhost".into(),
        time: gix::date::Time::now_local_or_utc(),
    };
    let full_message = format!("{}: {}", kind.prefix(), message);
    let oid = repo.commit(
        "HEAD",
        sig,
        sig,
        &full_message,
        tree_id,
        [head.id()],
    )?;

    Ok(oid)
}
```

### Tagged Snapshots

Tags provide named recovery points. Three kinds of tags exist:

| Tag Pattern | Created By | Purpose |
| --- | --- | --- |
| `daily/YYYY-MM-DD` | Daily snapshot job | Routine recovery point |
| `milestone/<name>` | User or evolution system | Named stable states |
| `evolution/<id>` | Evolution engine | Pre/post evolution checkpoints |

```rust
fn create_tag(
    repo: &gix::Repository,
    tag_name: &str,
    message: &str,
) -> Result<gix::ObjectId, gix::tag::Error> {
    let head = repo.head_commit()?;
    let sig = gix::actor::SignatureRef {
        name: "autonomic".into(),
        email: "autonomic@localhost".into(),
        time: gix::date::Time::now_local_or_utc(),
    };
    repo.tag(tag_name, head.id(), gix::objs::Kind::Commit, sig, message, false)
}

fn create_daily_snapshot(repo: &gix::Repository) -> Result<gix::ObjectId, gix::tag::Error> {
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let tag_name = format!("daily/{}", today);
    create_tag(repo, &tag_name, &format!("Daily snapshot {}", today))
}

fn create_evolution_snapshot(
    repo: &gix::Repository,
    evolution_id: &str,
    phase: &str,
) -> Result<gix::ObjectId, gix::tag::Error> {
    let tag_name = format!("evolution/{}-{}", evolution_id, phase);
    create_tag(
        repo,
        &tag_name,
        &format!("Evolution {} {}", evolution_id, phase),
    )
}
```

## PostgreSQL Schemas

### Shared State (PostgreSQL)

Sessions, metrics, rate budget, and evolution traces live in PostgreSQL (containerized). Schema migrations are managed by sqlx-cli.

```sql
-- Active and historical sessions
CREATE TABLE sessions (
    id              TEXT PRIMARY KEY,          -- ULID for time-sortable uniqueness
    project_hash    TEXT NOT NULL,
    started_at      TIMESTAMPTZ NOT NULL,
    ended_at        TIMESTAMPTZ,
    status          TEXT NOT NULL DEFAULT 'running'
                    CHECK (status IN ('running', 'completed', 'failed', 'cancelled')),
    model_tier      TEXT NOT NULL
                    CHECK (model_tier IN ('haiku', 'sonnet', 'opus')),
    prompt_hash     TEXT,                      -- SHA-256 of the initial prompt
    tokens_input    BIGINT NOT NULL DEFAULT 0,
    tokens_output   BIGINT NOT NULL DEFAULT 0,
    cost_millicents BIGINT NOT NULL DEFAULT 0,
    exit_code       INTEGER,
    error_message   TEXT,
    metadata        JSONB                      -- JSON blob for extensibility
);

CREATE INDEX idx_sessions_project ON sessions(project_hash);
CREATE INDEX idx_sessions_started ON sessions(started_at);
CREATE INDEX idx_sessions_status ON sessions(status) WHERE status = 'running';

-- Time-series metrics for observability and evolution decisions
CREATE TABLE metrics (
    id          BIGSERIAL PRIMARY KEY,
    timestamp   TIMESTAMPTZ NOT NULL,
    session_id  TEXT REFERENCES sessions(id),
    metric_name TEXT NOT NULL,
    metric_value DOUBLE PRECISION NOT NULL,
    labels      JSONB,                         -- JSON key-value pairs
    UNIQUE(timestamp, session_id, metric_name)
);

CREATE INDEX idx_metrics_name_time ON metrics(metric_name, timestamp);
CREATE INDEX idx_metrics_session ON metrics(session_id);

-- Rate budget tracking: sliding window token usage
CREATE TABLE rate_budget (
    id              BIGSERIAL PRIMARY KEY,
    window_start    TIMESTAMPTZ NOT NULL,      -- Start of 5-hour window
    window_end      TIMESTAMPTZ NOT NULL,
    model_tier      TEXT NOT NULL
                    CHECK (model_tier IN ('haiku', 'sonnet', 'opus')),
    tokens_used     BIGINT NOT NULL DEFAULT 0,
    tokens_limit    BIGINT NOT NULL,
    requests_used   BIGINT NOT NULL DEFAULT 0,
    requests_limit  BIGINT NOT NULL,
    updated_at      TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_rate_budget_window ON rate_budget(model_tier, window_start);

-- Budget allocation: what percentage of the window budget each subsystem gets
CREATE TABLE budget_allocations (
    model_tier      TEXT NOT NULL,
    subsystem       TEXT NOT NULL
                    CHECK (subsystem IN ('interactive', 'scheduled', 'evolution', 'monitoring')),
    percentage      DOUBLE PRECISION NOT NULL CHECK (percentage >= 0 AND percentage <= 100),
    PRIMARY KEY (model_tier, subsystem)
);

-- Default allocations
INSERT INTO budget_allocations VALUES ('haiku',  'interactive', 20);
INSERT INTO budget_allocations VALUES ('haiku',  'scheduled',   40);
INSERT INTO budget_allocations VALUES ('haiku',  'evolution',   10);
INSERT INTO budget_allocations VALUES ('haiku',  'monitoring',  30);
INSERT INTO budget_allocations VALUES ('sonnet', 'interactive', 50);
INSERT INTO budget_allocations VALUES ('sonnet', 'scheduled',   25);
INSERT INTO budget_allocations VALUES ('sonnet', 'evolution',   15);
INSERT INTO budget_allocations VALUES ('sonnet', 'monitoring',  10);
INSERT INTO budget_allocations VALUES ('opus',   'interactive', 60);
INSERT INTO budget_allocations VALUES ('opus',   'scheduled',   15);
INSERT INTO budget_allocations VALUES ('opus',   'evolution',   20);
INSERT INTO budget_allocations VALUES ('opus',   'monitoring',   5);
```

### Rust Types for State

```rust
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
struct Session {
    id: String,
    project_hash: String,
    started_at: chrono::DateTime<chrono::Utc>,
    ended_at: Option<chrono::DateTime<chrono::Utc>>,
    status: SessionStatus,
    model_tier: ModelTier,
    prompt_hash: Option<String>,
    tokens_input: u64,
    tokens_output: u64,
    cost_millicents: u64,
    exit_code: Option<i32>,
    error_message: Option<String>,
    metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
enum SessionStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
enum ModelTier {
    Haiku,
    Sonnet,
    Opus,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
struct Metric {
    id: i64,
    timestamp: chrono::DateTime<chrono::Utc>,
    session_id: Option<String>,
    metric_name: String,
    metric_value: f64,
    labels: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
struct RateBudget {
    id: i64,
    window_start: chrono::DateTime<chrono::Utc>,
    window_end: chrono::DateTime<chrono::Utc>,
    model_tier: ModelTier,
    tokens_used: u64,
    tokens_limit: u64,
    requests_used: u64,
    requests_limit: u64,
    updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
struct BudgetAllocation {
    model_tier: ModelTier,
    subsystem: Subsystem,
    percentage: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
enum Subsystem {
    Interactive,
    Scheduled,
    Evolution,
    Monitoring,
}
```

## Atomic Operations

Every state mutation follows a strict sequence to prevent partial writes:

```txt
1. Write data to temporary file (same filesystem as target)
2. fsync the temporary file
3. Rename tmp file to target path (atomic on POSIX)
4. Git add + commit the changed file
```

### Implementation

```rust
use std::fs;
use std::io::Write;
use std::path::Path;
use tempfile::NamedTempFile;

/// Atomically write content to a file, then commit to git.
fn atomic_write_and_commit(
    repo: &gix::Repository,
    target: &Path,
    content: &[u8],
    kind: CommitKind,
    message: &str,
) -> Result<gix::ObjectId, StateError> {
    // 1. Write to temp file in the same directory (ensures same filesystem)
    let parent = target.parent().ok_or(StateError::InvalidPath)?;
    let mut tmp = NamedTempFile::new_in(parent)?;
    tmp.write_all(content)?;

    // 2. fsync to ensure durability
    tmp.as_file().sync_all()?;

    // 3. Atomic rename
    tmp.persist(target)?;

    // 4. Git commit
    commit_mutation(repo, &[target], kind, message)
}

/// Atomically update a TOML config file with a merge function.
fn atomic_toml_update<T: Serialize + for<'de> Deserialize<'de>>(
    repo: &gix::Repository,
    path: &Path,
    kind: CommitKind,
    message: &str,
    mutate: impl FnOnce(&mut T) -> Result<(), StateError>,
) -> Result<gix::ObjectId, StateError> {
    let content = fs::read_to_string(path)?;
    let mut value: T = toml::from_str(&content)?;
    mutate(&mut value)?;
    let new_content = toml::to_string_pretty(&value)?;
    atomic_write_and_commit(repo, path, new_content.as_bytes(), kind, message)
}
```

### PostgreSQL Mutations

Shared state writes go directly to PostgreSQL via the sqlx connection pool. All queries use compile-time checked macros (`sqlx::query!` / `sqlx::query_as!`). PostgreSQL provides its own ACID guarantees and MVCC concurrency — no WAL checkpointing or git-commit-per-write is needed for database state. Backups use `pg_dump` scheduled via the existing job system.

## Recovery Procedures

### Rollback Command

```txt
autonomic rollback --to <tag-or-commit>
```

The rollback command restores the `~/.autonomic/` directory to a previous state:

```rust
fn rollback(repo: &gix::Repository, target: &str) -> Result<(), StateError> {
    // Resolve target: could be a tag name or commit hash
    let target_id = repo.rev_parse_single(target)?
        .object()?
        .peel_to_kind(gix::objs::Kind::Commit)?
        .id;

    let target_commit = repo.find_commit(target_id)?;

    // Create a snapshot tag before rolling back (safety net)
    let now = chrono::Utc::now().format("%Y%m%d-%H%M%S").to_string();
    create_tag(
        repo,
        &format!("pre-rollback/{}", now),
        &format!("State before rollback to {}", target),
    )?;

    // Checkout the target tree into the working directory
    let target_tree = target_commit.tree()?;
    repo.checkout_tree(target_tree.id())?;

    // Create a forward commit recording the rollback (preserves history)
    let head = repo.head_commit()?;
    let sig = gix::actor::SignatureRef {
        name: "autonomic".into(),
        email: "autonomic@localhost".into(),
        time: gix::date::Time::now_local_or_utc(),
    };
    let rollback_message = format!("rollback: restored state to {}", target);
    repo.commit(
        "HEAD",
        sig,
        sig,
        &rollback_message,
        target_tree.id(),
        [head.id()],
    )?;

    Ok(())
}
```

Key properties of rollback:

- A `pre-rollback/<timestamp>` tag is always created before any rollback, so the previous state is never lost.
- Rollback creates a **forward commit** rather than resetting HEAD, preserving full history.
- After rollback of TOML files, running sessions are terminated. PostgreSQL state is restored separately via `pg_dump` backups if needed.

### Listing Recovery Points

```rust
fn list_tags(repo: &gix::Repository) -> Result<Vec<TagInfo>, gix::reference::iter::Error> {
    let mut tags = Vec::new();
    let refs = repo.references()?.tags()?;
    for reference in refs {
        let reference = reference?;
        let name = reference.name().shorten().to_string();
        let oid = reference.id().to_string();
        tags.push(TagInfo { name, oid });
    }
    Ok(tags)
}

#[derive(Debug)]
struct TagInfo {
    name: String,
    oid: String,
}
```

## Snapshot Strategy

### Daily Auto-Snapshot

A scheduled job (see `scheduler.md`) runs daily at 03:00 UTC:

1. Run `pg_dump` to create a PostgreSQL backup in `~/.autonomic/backups/`.
2. `git add` any changed config files.
3. Commit with message `snapshot: daily YYYY-MM-DD`.
4. Tag as `daily/YYYY-MM-DD`.
5. Prune daily tags older than 30 days (keep weekly on Sundays, keep all milestone/evolution tags).

### Tag Pruning

```rust
fn prune_daily_tags(repo: &gix::Repository, keep_days: u32) -> Result<u32, StateError> {
    let cutoff = chrono::Utc::now() - chrono::Duration::days(keep_days as i64);
    let mut pruned = 0u32;

    let tags = list_tags(repo)?;
    for tag in &tags {
        if let Some(date_str) = tag.name.strip_prefix("daily/") {
            let date = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
                .map_err(|_| StateError::InvalidTag(tag.name.clone()))?;

            // Keep Sunday snapshots as weekly retention
            if date.weekday() == chrono::Weekday::Sun {
                continue;
            }

            let dt = date.and_hms_opt(0, 0, 0).unwrap()
                .and_utc();
            if dt < cutoff {
                repo.tag_delete(&tag.name)?;
                pruned += 1;
            }
        }
    }

    Ok(pruned)
}
```

### Milestone Tags

Created explicitly by the user or by the evolution system when a significant change is accepted:

```txt
autonomic snapshot --tag milestone/v1-stable --message "First stable config"
```

### Evolution Tags

Created in pairs by the evolution engine:

- `evolution/<id>-pre`: State before applying an evolution strategy.
- `evolution/<id>-post`: State after applying (if accepted).

If the evolution is rejected, the post tag is not created and the system remains at the pre state.

## Concurrent Access

### Single-Writer Principle

The Autonomic orchestrator process is the **sole writer** to `~/.autonomic/`. This is enforced by:

1. A PID lock file at `~/.autonomic/.lock`:

```rust
use fs4::fs_std::FileExt;
use std::fs::OpenOptions;

struct StateLock {
    file: std::fs::File,
}

impl StateLock {
    fn acquire(autonomic_dir: &Path) -> Result<Self, StateError> {
        let lock_path = autonomic_dir.join(".lock");
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&lock_path)?;

        file.try_lock_exclusive()
            .map_err(|_| StateError::AlreadyRunning)?;

        // Write PID for diagnostics
        use std::io::Write;
        writeln!(&file, "{}", std::process::id())?;

        Ok(StateLock { file })
    }
}

impl Drop for StateLock {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}
```

2. PostgreSQL MVCC provides full concurrent read/write access. The sqlx connection pool handles multiple concurrent sessions without contention:

```rust
use sqlx::postgres::PgPoolOptions;

async fn create_pool(database_url: &str) -> Result<sqlx::PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await
}
```

### Read-Only CLI Access

Commands like `autonomic status` connect to the same PostgreSQL instance. PostgreSQL MVCC ensures reads never block writes and vice versa. The CLI uses the same connection pool with a lower connection limit:

```rust
async fn create_readonly_pool(database_url: &str) -> Result<sqlx::PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(2)
        .connect(database_url)
        .await
}
```

## Migration Strategy

Schema migrations are managed by `sqlx-cli` and tracked in the `migrations/` directory. Migrations run on startup before any other database access.

```rust
use sqlx::PgPool;

async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
}
```

Migration files follow the sqlx convention:

```txt
migrations/
  20260325000000_initial_schema.sql
  20260326000000_add_memory_entries.sql
  # Future migrations appended here
```

### Migration Safety

- Each migration runs inside an explicit transaction. If any statement fails, the entire migration is rolled back.
- sqlx tracks applied migrations in the `_sqlx_migrations` table. If the database has migrations from a newer version, startup aborts with a clear error.
- Before running migrations, the orchestrator creates a git snapshot tagged `migration/v<from>-to-v<to>` (for config state only; PostgreSQL state is backed up via `pg_dump`).

## Backup

Config state in `~/.autonomic/` is a git repository. Backup for config is trivially:

```bash
git clone ~/.autonomic/ /path/to/backup/autonomic-$(date +%Y%m%d)
```

For remote backup:

```bash
cd ~/.autonomic && git remote add backup <remote-url> && git push backup --all --tags
```

### Automated Backup Job

A scheduled job (see `scheduler.md`) can push to a configured remote:

```toml
# In config.toml
[backup]
enabled = true
remote = "git@github.com:user/autonomic-state-backup.git"
schedule = "0 4 * * *"  # Daily at 04:00 UTC
```

The backup job:

1. Runs `pg_dump` for the PostgreSQL database (via `podman exec`).
2. Commits any changed config files to git.
3. Pushes all branches and tags to the configured remote.
4. Records success/failure in metrics.

## Consolidated State Manager

The `StateManager` struct is the single entry point for all state operations:

```rust
use sqlx::PgPool;
use std::path::{Path, PathBuf};

struct StateManager {
    root: PathBuf,
    repo: gix::Repository,
    pool: PgPool,
    lock: StateLock,
}

impl StateManager {
    async fn open(autonomic_dir: &Path, database_url: &str) -> Result<Self, StateError> {
        let lock = StateLock::acquire(autonomic_dir)?;
        let repo = gix::open(autonomic_dir)?;

        let pool = create_pool(database_url).await?;
        run_migrations(&pool).await?;

        Ok(StateManager {
            root: autonomic_dir.to_path_buf(),
            repo,
            pool,
            lock,
        })
    }

    fn config_path(&self) -> PathBuf {
        self.root.join("config.toml")
    }

    fn update_config(
        &self,
        mutate: impl FnOnce(&mut AutonomicConfig) -> Result<(), StateError>,
    ) -> Result<gix::ObjectId, StateError> {
        atomic_toml_update(
            &self.repo,
            &self.config_path(),
            CommitKind::Config,
            "update configuration",
            mutate,
        )
    }

    async fn snapshot(&self, tag_name: &str, message: &str) -> Result<gix::ObjectId, StateError> {
        // pg_dump for PostgreSQL backup
        let backup_path = self.root.join("backups").join(format!("{}.sql", tag_name.replace('/', "-")));
        tokio::fs::create_dir_all(backup_path.parent().unwrap()).await?;
        // pg_dump executed via container: podman exec postgres pg_dump ...

        // Git commit config state
        commit_mutation(&self.repo, &[], CommitKind::Snapshot, &format!("snapshot for {}", tag_name))?;
        create_tag(&self.repo, tag_name, message).map_err(Into::into)
    }

    fn rollback(&self, target: &str) -> Result<(), StateError> {
        rollback(&self.repo, target)
    }

    async fn record_session(&self, session: &Session) -> Result<(), StateError> {
        sqlx::query!(
            r#"INSERT INTO sessions (id, project_hash, started_at, status, model_tier, tokens_input, tokens_output, cost_millicents)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"#,
            session.id,
            session.project_hash,
            session.started_at,
            format!("{:?}", session.status).to_lowercase(),
            format!("{:?}", session.model_tier).to_lowercase(),
            session.tokens_input,
            session.tokens_output,
            session.cost_millicents,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn record_metric(&self, metric: &Metric) -> Result<(), StateError> {
        sqlx::query!(
            r#"INSERT INTO metrics (timestamp, session_id, metric_name, metric_value, labels)
               VALUES ($1, $2, $3, $4, $5)
               ON CONFLICT (timestamp, session_id, metric_name) DO UPDATE
               SET metric_value = EXCLUDED.metric_value, labels = EXCLUDED.labels"#,
            metric.timestamp,
            metric.session_id,
            metric.metric_name,
            metric.metric_value,
            metric.labels.as_ref().map(|v| v.clone()),
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_budget_usage(&self, tier: ModelTier) -> Result<BudgetUsage, StateError> {
        let now = chrono::Utc::now();
        let window_start = now - chrono::Duration::hours(5);
        let tier_str = format!("{:?}", tier).to_lowercase();

        let row = sqlx::query_as!(
            BudgetUsage,
            r#"SELECT COALESCE(SUM(tokens_used), 0) as "tokens_used!",
                      COALESCE(MAX(tokens_limit), 0) as "tokens_limit!",
                      COALESCE(SUM(requests_used), 0) as "requests_used!",
                      COALESCE(MAX(requests_limit), 0) as "requests_limit!"
               FROM rate_budget
               WHERE model_tier = $1 AND window_start >= $2"#,
            tier_str,
            window_start,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }
}

#[derive(Debug)]
struct BudgetUsage {
    tokens_used: u64,
    tokens_limit: u64,
    requests_used: u64,
    requests_limit: u64,
}

impl BudgetUsage {
    /// Returns usage as a percentage (0.0 to 100.0).
    fn usage_percent(&self) -> f64 {
        if self.tokens_limit == 0 {
            return 100.0;
        }
        (self.tokens_used as f64 / self.tokens_limit as f64) * 100.0
    }
}
```

## Error Types

```rust
#[derive(Debug, thiserror::Error)]
enum StateError {
    #[error("Git error: {0}")]
    Git(#[from] gix::open::Error),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML serialization error: {0}")]
    TomlSer(#[from] toml::ser::Error),

    #[error("TOML deserialization error: {0}")]
    TomlDe(#[from] toml::de::Error),

    #[error("Invalid path")]
    InvalidPath,

    #[error("Another autonomic instance is already running")]
    AlreadyRunning,

    #[error("Invalid rollback target: {0}")]
    InvalidTarget(String),

    #[error("Invalid tag: {0}")]
    InvalidTag(String),

    #[error("Database schema version {found} is newer than supported {supported}")]
    FutureSchema { found: u32, supported: u32 },

    #[error("Path strip prefix error: {0}")]
    StripPrefix(#[from] std::path::StripPrefixError),

    #[error("Temp file persist error: {0}")]
    Persist(#[from] tempfile::PersistError),
}
```

## Crate Dependencies

```toml
[dependencies]
gix = { version = "0.68", features = ["blocking-network-client"] }
sqlx = { version = "0.8", features = ["runtime-tokio", "postgres", "chrono", "json"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
chrono = { version = "0.4", features = ["serde"] }
tempfile = "3"
thiserror = "2"
tracing = "0.1"
tokio = { version = "1", features = ["fs"] }
fs4 = "0.12"
```
