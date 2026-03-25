# State Management

All mutable state for Autonomic lives in `~/.autonomic/`, which is itself a git repository. Every mutation to files in this directory results in an atomic git commit, providing a complete audit trail and point-in-time recovery for the entire orchestrator state.

## Directory Structure

```txt
~/.autonomic/
  config.toml              # User configuration (model preferences, API keys ref, thresholds)
  state.sqlite             # Session state, metrics, rate budget tracking
  memory.sqlite            # Long-term memory store (embeddings, retrieval)
  projects/
    <project-hash>/
      context.toml         # Project-specific overrides, model tier preferences
      history.sqlite       # Per-project session history (not git-committed, too large)
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
   b. Restores SQLite from the tagged snapshot
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
projects/*/history.sqlite
*.sqlite-wal
*.sqlite-shm
*.tmp
```

The SQLite databases `state.sqlite` and `memory.sqlite` are committed, but only at snapshot boundaries (not on every write). Their WAL files are never committed.

## Git-Backed State

### Initialization

On first run, Autonomic initializes `~/.autonomic/` as a git repository:

```rust
use git2::{Repository, Signature};
use std::path::PathBuf;

fn init_state_repo(autonomic_dir: &Path) -> Result<Repository, git2::Error> {
    if autonomic_dir.join(".git").exists() {
        Repository::open(autonomic_dir)
    } else {
        let repo = Repository::init(autonomic_dir)?;

        // Create initial commit with empty tree
        let sig = autonomic_signature()?;
        let tree_id = {
            let mut index = repo.index()?;
            index.write_tree()?
        };
        let tree = repo.find_tree(tree_id)?;
        repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            "autonomic: initialize state repository",
            &tree,
            &[],
        )?;

        Ok(repo)
    }
}

fn autonomic_signature() -> Result<Signature<'static>, git2::Error> {
    Signature::now("autonomic", "autonomic@localhost")
}
```

### Auto-Commit on Mutation

Every mutation to a tracked file triggers an atomic commit. The commit message encodes the operation type for machine-readable history:

```rust
use git2::{IndexAddOption, Repository, Signature};

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
    repo: &Repository,
    paths: &[&Path],
    kind: CommitKind,
    message: &str,
) -> Result<git2::Oid, StateError> {
    let sig = autonomic_signature()?;
    let mut index = repo.index()?;

    for path in paths {
        // Path must be relative to the repo workdir
        let relative = path.strip_prefix(repo.workdir().unwrap())?;
        index.add_path(relative)?;
    }
    index.write()?;

    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;
    let head = repo.head()?.peel_to_commit()?;

    let full_message = format!("{}: {}", kind.prefix(), message);
    let oid = repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        &full_message,
        &tree,
        &[&head],
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
    repo: &Repository,
    tag_name: &str,
    message: &str,
) -> Result<git2::Oid, git2::Error> {
    let sig = autonomic_signature()?;
    let head = repo.head()?.peel_to_commit()?;
    let obj = head.as_object();
    repo.tag(tag_name, obj, &sig, message, false)
}

fn create_daily_snapshot(repo: &Repository) -> Result<git2::Oid, git2::Error> {
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let tag_name = format!("daily/{}", today);
    create_tag(repo, &tag_name, &format!("Daily snapshot {}", today))
}

fn create_evolution_snapshot(
    repo: &Repository,
    evolution_id: &str,
    phase: &str,
) -> Result<git2::Oid, git2::Error> {
    let tag_name = format!("evolution/{}-{}", evolution_id, phase);
    create_tag(
        repo,
        &tag_name,
        &format!("Evolution {} {}", evolution_id, phase),
    )
}
```

## SQLite Schemas

### state.sqlite

```sql
-- Schema version tracked via PRAGMA user_version
-- Current version: 1
PRAGMA user_version = 1;
PRAGMA journal_mode = WAL;
PRAGMA busy_timeout = 5000;
PRAGMA synchronous = NORMAL;
PRAGMA foreign_keys = ON;

-- Active and historical sessions
CREATE TABLE sessions (
    id              TEXT PRIMARY KEY,          -- ULID for time-sortable uniqueness
    project_hash    TEXT NOT NULL,
    started_at      TEXT NOT NULL,             -- ISO 8601
    ended_at        TEXT,
    status          TEXT NOT NULL DEFAULT 'running'
                    CHECK (status IN ('running', 'completed', 'failed', 'cancelled')),
    model_tier      TEXT NOT NULL
                    CHECK (model_tier IN ('haiku', 'sonnet', 'opus')),
    prompt_hash     TEXT,                      -- SHA-256 of the initial prompt
    tokens_input    INTEGER NOT NULL DEFAULT 0,
    tokens_output   INTEGER NOT NULL DEFAULT 0,
    cost_millicents INTEGER NOT NULL DEFAULT 0,
    exit_code       INTEGER,
    error_message   TEXT,
    metadata        TEXT                       -- JSON blob for extensibility
);

CREATE INDEX idx_sessions_project ON sessions(project_hash);
CREATE INDEX idx_sessions_started ON sessions(started_at);
CREATE INDEX idx_sessions_status ON sessions(status) WHERE status = 'running';

-- Time-series metrics for observability and evolution decisions
CREATE TABLE metrics (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp   TEXT NOT NULL,                -- ISO 8601
    session_id  TEXT REFERENCES sessions(id),
    metric_name TEXT NOT NULL,
    metric_value REAL NOT NULL,
    labels      TEXT,                         -- JSON key-value pairs
    UNIQUE(timestamp, session_id, metric_name)
);

CREATE INDEX idx_metrics_name_time ON metrics(metric_name, timestamp);
CREATE INDEX idx_metrics_session ON metrics(session_id);

-- Rate budget tracking: sliding window token usage
CREATE TABLE rate_budget (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    window_start    TEXT NOT NULL,            -- ISO 8601, start of 5-hour window
    window_end      TEXT NOT NULL,
    model_tier      TEXT NOT NULL
                    CHECK (model_tier IN ('haiku', 'sonnet', 'opus')),
    tokens_used     INTEGER NOT NULL DEFAULT 0,
    tokens_limit    INTEGER NOT NULL,
    requests_used   INTEGER NOT NULL DEFAULT 0,
    requests_limit  INTEGER NOT NULL,
    updated_at      TEXT NOT NULL
);

CREATE INDEX idx_rate_budget_window ON rate_budget(model_tier, window_start);

-- Budget allocation: what percentage of the window budget each subsystem gets
CREATE TABLE budget_allocations (
    model_tier      TEXT NOT NULL,
    subsystem       TEXT NOT NULL
                    CHECK (subsystem IN ('interactive', 'scheduled', 'evolution', 'monitoring')),
    percentage      REAL NOT NULL CHECK (percentage >= 0 AND percentage <= 100),
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
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Metric {
    id: i64,
    timestamp: chrono::DateTime<chrono::Utc>,
    session_id: Option<String>,
    metric_name: String,
    metric_value: f64,
    labels: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    repo: &Repository,
    target: &Path,
    content: &[u8],
    kind: CommitKind,
    message: &str,
) -> Result<git2::Oid, StateError> {
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
    repo: &Repository,
    path: &Path,
    kind: CommitKind,
    message: &str,
    mutate: impl FnOnce(&mut T) -> Result<(), StateError>,
) -> Result<git2::Oid, StateError> {
    let content = fs::read_to_string(path)?;
    let mut value: T = toml::from_str(&content)?;
    mutate(&mut value)?;
    let new_content = toml::to_string_pretty(&value)?;
    atomic_write_and_commit(repo, path, new_content.as_bytes(), kind, message)
}
```

### SQLite Mutations

SQLite writes do not go through the atomic file write path (SQLite has its own atomicity via WAL). Instead, after a batch of SQLite operations, the orchestrator checkpoints the WAL and then commits the database file:

```rust
fn checkpoint_and_commit(
    conn: &Connection,
    repo: &Repository,
    db_path: &Path,
    message: &str,
) -> Result<git2::Oid, StateError> {
    // Force a WAL checkpoint so all data is in the main DB file
    conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")?;
    commit_mutation(repo, &[db_path], CommitKind::Snapshot, message)
}
```

## Recovery Procedures

### Rollback Command

```txt
autonomic rollback --to <tag-or-commit>
```

The rollback command restores the `~/.autonomic/` directory to a previous state:

```rust
fn rollback(repo: &Repository, target: &str) -> Result<(), StateError> {
    // Resolve target: could be a tag name or commit hash
    let obj = repo.revparse_single(target)?;
    let commit = obj
        .peel_to_commit()
        .map_err(|_| StateError::InvalidTarget(target.to_string()))?;

    // Create a snapshot tag before rolling back (safety net)
    let now = chrono::Utc::now().format("%Y%m%d-%H%M%S").to_string();
    create_tag(
        repo,
        &format!("pre-rollback/{}", now),
        &format!("State before rollback to {}", target),
    )?;

    // Reset working tree to target commit
    let tree = commit.tree()?;
    repo.checkout_tree(tree.as_object(), Some(
        git2::build::CheckoutBuilder::new()
            .force()
            .remove_untracked(true),
    ))?;

    // Move HEAD to target commit, then create a new commit recording the rollback
    // (We do NOT reset HEAD; we create a forward commit to preserve history)
    let sig = autonomic_signature()?;
    let head = repo.head()?.peel_to_commit()?;
    let rollback_message = format!("rollback: restored state to {}", target);
    repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        &rollback_message,
        &tree,
        &[&head],
    )?;

    Ok(())
}
```

Key properties of rollback:

- A `pre-rollback/<timestamp>` tag is always created before any rollback, so the previous state is never lost.
- Rollback creates a **forward commit** rather than resetting HEAD, preserving full history.
- After rollback of TOML files, SQLite databases are rebuilt from the committed copies. Running sessions are terminated.

### Listing Recovery Points

```rust
fn list_tags(repo: &Repository) -> Result<Vec<TagInfo>, git2::Error> {
    let mut tags = Vec::new();
    repo.tag_foreach(|oid, name| {
        let name = String::from_utf8_lossy(name).to_string();
        // Strip "refs/tags/" prefix
        let short_name = name.strip_prefix("refs/tags/").unwrap_or(&name).to_string();
        tags.push(TagInfo {
            name: short_name,
            oid: oid.to_string(),
        });
        true
    })?;
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

1. Checkpoint all SQLite WALs (truncate mode).
2. `git add` the SQLite database files.
3. Commit with message `snapshot: daily YYYY-MM-DD`.
4. Tag as `daily/YYYY-MM-DD`.
5. Prune daily tags older than 30 days (keep weekly on Sundays, keep all milestone/evolution tags).

### Tag Pruning

```rust
fn prune_daily_tags(repo: &Repository, keep_days: u32) -> Result<u32, StateError> {
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

2. SQLite in WAL mode with a 5-second busy timeout. Even though there is a single writer, WAL mode allows concurrent readers (e.g., a CLI status command querying metrics while the orchestrator is writing):

```rust
fn open_state_db(path: &Path) -> Result<Connection, rusqlite::Error> {
    let conn = Connection::open(path)?;
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA busy_timeout = 5000;
         PRAGMA synchronous = NORMAL;
         PRAGMA foreign_keys = ON;"
    )?;
    Ok(conn)
}
```

### Read-Only CLI Access

Commands like `autonomic status` open the database in read-only mode:

```rust
fn open_state_db_readonly(path: &Path) -> Result<Connection, rusqlite::Error> {
    let conn = Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY
            | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    conn.execute_batch("PRAGMA busy_timeout = 1000;")?;
    Ok(conn)
}
```

## Migration Strategy

Schema versions are tracked using SQLite's `user_version` pragma. Migrations run on startup before any other database access.

```rust
const CURRENT_SCHEMA_VERSION: u32 = 1;

struct Migration {
    version: u32,
    description: &'static str,
    sql: &'static str,
}

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        description: "Initial schema",
        sql: include_str!("../migrations/001_initial.sql"),
    },
    // Future migrations appended here:
    // Migration {
    //     version: 2,
    //     description: "Add session tags",
    //     sql: include_str!("../migrations/002_session_tags.sql"),
    // },
];

fn migrate(conn: &Connection) -> Result<(), StateError> {
    let current: u32 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;

    if current > CURRENT_SCHEMA_VERSION {
        return Err(StateError::FutureSchema {
            found: current,
            supported: CURRENT_SCHEMA_VERSION,
        });
    }

    for migration in MIGRATIONS {
        if migration.version > current {
            conn.execute_batch(migration.sql)?;
            conn.pragma_update(None, "user_version", migration.version)?;
            tracing::info!(
                version = migration.version,
                description = migration.description,
                "Applied migration"
            );
        }
    }

    Ok(())
}
```

### Migration Safety

- Migrations run inside an implicit transaction per `execute_batch` call. If any statement fails, the entire migration is rolled back.
- The migration runner compares `user_version` against `CURRENT_SCHEMA_VERSION`. If the database is from a newer version of Autonomic, startup aborts with a clear error rather than corrupting data.
- Before running migrations, the orchestrator creates a git snapshot tagged `migration/v<from>-to-v<to>`.

## Backup

The entire `~/.autonomic/` directory is a git repository. Backup is trivially:

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

1. Runs `checkpoint_and_commit` for all SQLite databases.
2. Pushes all branches and tags to the configured remote.
3. Records success/failure in metrics.

## Consolidated State Manager

The `StateManager` struct is the single entry point for all state operations:

```rust
use git2::Repository;
use rusqlite::Connection;
use std::path::{Path, PathBuf};

struct StateManager {
    root: PathBuf,
    repo: Repository,
    state_db: Connection,
    memory_db: Connection,
    lock: StateLock,
}

impl StateManager {
    fn open(autonomic_dir: &Path) -> Result<Self, StateError> {
        let lock = StateLock::acquire(autonomic_dir)?;
        let repo = Repository::open(autonomic_dir)?;

        let state_db = open_state_db(&autonomic_dir.join("state.sqlite"))?;
        migrate(&state_db)?;

        let memory_db = open_state_db(&autonomic_dir.join("memory.sqlite"))?;
        // memory_db has its own migration chain

        Ok(StateManager {
            root: autonomic_dir.to_path_buf(),
            repo,
            state_db,
            memory_db,
            lock,
        })
    }

    fn config_path(&self) -> PathBuf {
        self.root.join("config.toml")
    }

    fn update_config(
        &self,
        mutate: impl FnOnce(&mut AutoномicConfig) -> Result<(), StateError>,
    ) -> Result<git2::Oid, StateError> {
        atomic_toml_update(
            &self.repo,
            &self.config_path(),
            CommitKind::Config,
            "update configuration",
            mutate,
        )
    }

    fn snapshot(&self, tag_name: &str, message: &str) -> Result<git2::Oid, StateError> {
        // Checkpoint databases first
        checkpoint_and_commit(
            &self.state_db,
            &self.repo,
            &self.root.join("state.sqlite"),
            "checkpoint state.sqlite for snapshot",
        )?;
        checkpoint_and_commit(
            &self.memory_db,
            &self.repo,
            &self.root.join("memory.sqlite"),
            "checkpoint memory.sqlite for snapshot",
        )?;
        create_tag(&self.repo, tag_name, message).map_err(Into::into)
    }

    fn rollback(&self, target: &str) -> Result<(), StateError> {
        rollback(&self.repo, target)
    }

    fn record_session(&self, session: &Session) -> Result<(), StateError> {
        self.state_db.execute(
            "INSERT INTO sessions (id, project_hash, started_at, status, model_tier, tokens_input, tokens_output, cost_millicents)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                session.id,
                session.project_hash,
                session.started_at.to_rfc3339(),
                format!("{:?}", session.status).to_lowercase(),
                format!("{:?}", session.model_tier).to_lowercase(),
                session.tokens_input,
                session.tokens_output,
                session.cost_millicents,
            ],
        )?;
        Ok(())
    }

    fn record_metric(&self, metric: &Metric) -> Result<(), StateError> {
        self.state_db.execute(
            "INSERT OR REPLACE INTO metrics (timestamp, session_id, metric_name, metric_value, labels)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                metric.timestamp.to_rfc3339(),
                metric.session_id,
                metric.metric_name,
                metric.metric_value,
                metric.labels.as_ref().map(|v| v.to_string()),
            ],
        )?;
        Ok(())
    }

    fn get_budget_usage(&self, tier: ModelTier) -> Result<BudgetUsage, StateError> {
        let now = chrono::Utc::now();
        let window_start = now - chrono::Duration::hours(5);

        let row = self.state_db.query_row(
            "SELECT COALESCE(SUM(tokens_used), 0), COALESCE(MAX(tokens_limit), 0),
                    COALESCE(SUM(requests_used), 0), COALESCE(MAX(requests_limit), 0)
             FROM rate_budget
             WHERE model_tier = ?1 AND window_start >= ?2",
            rusqlite::params![
                format!("{:?}", tier).to_lowercase(),
                window_start.to_rfc3339(),
            ],
            |row| {
                Ok(BudgetUsage {
                    tokens_used: row.get(0)?,
                    tokens_limit: row.get(1)?,
                    requests_used: row.get(2)?,
                    requests_limit: row.get(3)?,
                })
            },
        )?;

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
    Git(#[from] git2::Error),

    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

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
git2 = "0.19"
rusqlite = { version = "0.32", features = ["bundled"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
chrono = { version = "0.4", features = ["serde"] }
tempfile = "3"
thiserror = "2"
tracing = "0.1"
fs4 = "0.12"
```
