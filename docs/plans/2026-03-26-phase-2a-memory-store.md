# Phase 2A: Memory Store Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the `autonomic-memory` crate: CRUD operations, tsvector search, decay scoring, context assembly, MEMORY.md bidirectional sync, usefulness tracking, and retirement.

**Architecture:** All memory storage uses PostgreSQL (tables already exist from Phase 0 migration). The `autonomic-memory` crate provides a `MemoryStore` struct wrapping a `PgPool` with typed methods for every operation. Context assembly is a pure-Rust scoring + packing algorithm that runs queries, scores candidates, and renders token-budgeted output. MEMORY.md sync uses content hashing to detect changes.

**Tech Stack:** Rust 2024 (1.94), sqlx (Postgres, compile-time checked queries), serde/serde_json, chrono, ulid, tokio

**Spec:** `docs/architecture/memory-system.md`

**Prereqs from Phase 1:** `SessionId`, `ProjectId`, `PgPool`, `create_pool`

**Key Constraints:**
- XD-002: All data goes to PostgreSQL, not ad-hoc files
- tsvector search < 1ms for 10K entries
- Context assembly < 50ms total pipeline

## File Map

```txt
MODIFY: crates/autonomic-memory/Cargo.toml          # Add dependencies
CREATE: crates/autonomic-memory/src/lib.rs           # Re-exports
CREATE: crates/autonomic-memory/src/types.rs         # MemoryCategory, MemoryScope, MemoryEntry
CREATE: crates/autonomic-memory/src/store.rs         # MemoryStore: CRUD + search + usefulness
CREATE: crates/autonomic-memory/src/scoring.rs       # Decay scoring, candidate ranking
CREATE: crates/autonomic-memory/src/assembly.rs      # Context assembly algorithm (keyword extract, score, pack, render)
CREATE: crates/autonomic-memory/src/sync.rs          # MEMORY.md generation and ingestion
CREATE: crates/autonomic-memory/src/retirement.rs    # Stale entry retirement algorithm
CREATE: crates/autonomic-memory/src/error.rs         # MemoryError enum

CREATE: infra/migrations/003_memory_indexes.sql      # Additional indexes (category, tsvector weights)
```

## Task 1: Types and Error (autonomic-memory)

**Files:**
- Create: `crates/autonomic-memory/src/error.rs`
- Create: `crates/autonomic-memory/src/types.rs`
- Modify: `crates/autonomic-memory/Cargo.toml`
- Modify: `crates/autonomic-memory/src/lib.rs`

- [ ] **Step 1: Update Cargo.toml**

```toml
[package]
name = "autonomic-memory"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
description = "PostgreSQL memory store with tsvector FTS, context assembly, and MEMORY.md sync"

[dependencies]
autonomic-core = { path = "../autonomic-core" }
chrono = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
sqlx = { workspace = true }
thiserror = { workspace = true }
tokio = { workspace = true }
tracing = { workspace = true }
ulid = { workspace = true }
sha2 = { workspace = true }

[dev-dependencies]
proptest = { workspace = true }

[lints]
workspace = true
```

- [ ] **Step 2: Create error.rs**

```rust
//! Error types for the memory subsystem.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum MemoryError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("invalid category: {0}")]
    InvalidCategory(String),

    #[error("invalid scope: {0}")]
    InvalidScope(String),

    #[error("entry not found: {0}")]
    NotFound(String),

    #[error("sync error: {0}")]
    Sync(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
```

- [ ] **Step 3: Create types.rs**

```rust
//! Core types for memory entries.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Memory entry categories with default decay rates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MemoryCategory {
    Decision,
    Pattern,
    Gotcha,
    Preference,
    Tool,
    Error,
    Lesson,
    Ephemeral,
}

impl MemoryCategory {
    /// Default decay rate per day for this category.
    pub fn default_decay_rate(self) -> f64 {
        match self {
            Self::Decision => 0.01,
            Self::Pattern => 0.03,
            Self::Gotcha => 0.02,
            Self::Preference => 0.01,
            Self::Tool => 0.05,
            Self::Error => 0.08,
            Self::Lesson => 0.10,
            Self::Ephemeral => 0.20,
        }
    }

    /// Category boost factor for scoring.
    pub fn boost(self) -> f64 {
        match self {
            Self::Decision | Self::Gotcha => 1.3,
            Self::Pattern | Self::Preference => 1.1,
            _ => 1.0,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Decision => "decision",
            Self::Pattern => "pattern",
            Self::Gotcha => "gotcha",
            Self::Preference => "preference",
            Self::Tool => "tool",
            Self::Error => "error",
            Self::Lesson => "lesson",
            Self::Ephemeral => "ephemeral",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "decision" => Some(Self::Decision),
            "pattern" => Some(Self::Pattern),
            "gotcha" => Some(Self::Gotcha),
            "preference" => Some(Self::Preference),
            "tool" => Some(Self::Tool),
            "error" => Some(Self::Error),
            "lesson" => Some(Self::Lesson),
            "ephemeral" => Some(Self::Ephemeral),
            _ => None,
        }
    }
}

/// Scope determines which sessions see this entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum MemoryScope {
    /// Visible to all sessions across all projects.
    Global,
    /// Visible only to sessions in the named project.
    Project(String),
    /// Visible to sessions in projects using this language.
    Language(String),
}

impl MemoryScope {
    pub fn scope_type(&self) -> &str {
        match self {
            Self::Global => "global",
            Self::Project(_) => "project",
            Self::Language(_) => "language",
        }
    }

    pub fn scope_value(&self) -> Option<&str> {
        match self {
            Self::Global => None,
            Self::Project(v) | Self::Language(v) => Some(v),
        }
    }
}

/// A memory entry as stored in PostgreSQL.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub content: String,
    pub category: MemoryCategory,
    pub scope: MemoryScope,
    pub tags: Vec<String>,
    pub source_session: Option<String>,
    pub source_project: Option<String>,
    pub helpful_count: i32,
    pub misleading_count: i32,
    pub decay_rate: f64,
    pub retirement_policy: RetirementPolicy,
    pub created_at: DateTime<Utc>,
    pub last_accessed: DateTime<Utc>,
    pub retired_at: Option<DateTime<Utc>>,
}

/// Whether an entry can be auto-retired or requires manual removal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetirementPolicy {
    Auto,
    ManualOnly,
}

impl RetirementPolicy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::ManualOnly => "manual_only",
        }
    }
}

/// Parameters for creating a new memory entry.
pub struct NewMemoryEntry {
    pub content: String,
    pub category: MemoryCategory,
    pub scope: MemoryScope,
    pub tags: Vec<String>,
    pub source_session: Option<String>,
    pub source_project: Option<String>,
    pub retirement_policy: RetirementPolicy,
}

/// A scored memory candidate from search results.
#[derive(Debug, Clone)]
pub struct ScoredEntry {
    pub entry: MemoryEntry,
    pub fts_rank: f64,
    pub score: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_decay_rates_are_positive() {
        for cat in [
            MemoryCategory::Decision,
            MemoryCategory::Pattern,
            MemoryCategory::Gotcha,
            MemoryCategory::Preference,
            MemoryCategory::Tool,
            MemoryCategory::Error,
            MemoryCategory::Lesson,
            MemoryCategory::Ephemeral,
        ] {
            assert!(cat.default_decay_rate() > 0.0, "{cat:?} has non-positive decay");
            assert!(cat.default_decay_rate() <= 1.0, "{cat:?} has decay > 1.0");
        }
    }

    #[test]
    fn category_roundtrip() {
        for cat in [
            MemoryCategory::Decision,
            MemoryCategory::Pattern,
            MemoryCategory::Gotcha,
        ] {
            let s = cat.as_str();
            let parsed = MemoryCategory::parse(s).unwrap();
            assert_eq!(cat, parsed);
        }
    }

    #[test]
    fn scope_type_and_value() {
        let global = MemoryScope::Global;
        assert_eq!(global.scope_type(), "global");
        assert_eq!(global.scope_value(), None);

        let project = MemoryScope::Project("vox-scribe".to_string());
        assert_eq!(project.scope_type(), "project");
        assert_eq!(project.scope_value(), Some("vox-scribe"));
    }
}
```

- [ ] **Step 4: Update lib.rs**

```rust
//! PostgreSQL memory store with tsvector FTS, context assembly, and MEMORY.md sync.

pub mod assembly;
pub mod error;
pub mod retirement;
pub mod scoring;
pub mod store;
pub mod sync;
pub mod types;

pub use error::MemoryError;
pub use store::MemoryStore;
pub use types::{MemoryCategory, MemoryEntry, MemoryScope, NewMemoryEntry, RetirementPolicy, ScoredEntry};
```

- [ ] **Step 5: Run cargo check**

```bash
SQLX_OFFLINE=true cargo check -p autonomic-memory
```

- [ ] **Step 6: Commit**

```bash
git add crates/autonomic-memory/
git commit -m "feat(memory): add types, error enum, and crate structure"
```

## Task 2: MemoryStore CRUD + Search

**Files:**
- Create: `crates/autonomic-memory/src/store.rs`

- [ ] **Step 1: Create store.rs with CRUD and tsvector search**

```rust
//! PostgreSQL-backed memory store with CRUD and tsvector search.

use chrono::Utc;
use sqlx::PgPool;
use ulid::Ulid;

use crate::error::MemoryError;
use crate::types::*;

/// Memory store backed by PostgreSQL.
#[derive(Debug, Clone)]
pub struct MemoryStore {
    pool: PgPool,
}

impl MemoryStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new memory entry. Returns the generated ID.
    pub async fn create(&self, entry: &NewMemoryEntry) -> Result<String, MemoryError> {
        let id = Ulid::new().to_string();
        let now = Utc::now();
        let category = entry.category.as_str();
        let scope_type = entry.scope.scope_type();
        let scope_value = entry.scope.scope_value().map(String::from);
        let tags = serde_json::to_value(&entry.tags).unwrap_or_default();
        let decay_rate = entry.category.default_decay_rate();
        let retirement = entry.retirement_policy.as_str();

        sqlx::query(
            r#"
            INSERT INTO memory_entries
                (id, content, category, scope_type, scope_value, tags,
                 source_session, source_project, decay_rate, retirement_policy,
                 created_at, last_accessed)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $11)
            "#,
        )
        .bind(&id)
        .bind(&entry.content)
        .bind(category)
        .bind(scope_type)
        .bind(&scope_value)
        .bind(&tags)
        .bind(&entry.source_session)
        .bind(&entry.source_project)
        .bind(decay_rate)
        .bind(retirement)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(id)
    }

    /// Get a single entry by ID.
    pub async fn get(&self, id: &str) -> Result<MemoryEntry, MemoryError> {
        let row = sqlx::query_as::<_, MemoryRow>(
            "SELECT * FROM memory_entries WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| MemoryError::NotFound(id.to_string()))?;

        Ok(row.into_entry())
    }

    /// Search active entries using tsvector full-text search.
    /// Returns up to `limit` entries ranked by ts_rank.
    pub async fn search(
        &self,
        keywords: &str,
        scope_filter: Option<&MemoryScope>,
        limit: i64,
    ) -> Result<Vec<(MemoryEntry, f64)>, MemoryError> {
        let scope_type = scope_filter.map(|s| s.scope_type().to_string());
        let scope_value = scope_filter.and_then(|s| s.scope_value().map(String::from));

        let rows = sqlx::query_as::<_, RankedMemoryRow>(
            r#"
            SELECT m.*, ts_rank(m.search_vector, query)::double precision AS rank
            FROM memory_entries m,
                 plainto_tsquery('english', $1) query
            WHERE m.search_vector @@ query
              AND m.retired_at IS NULL
              AND ($2::TEXT IS NULL OR m.scope_type = $2)
              AND ($3::TEXT IS NULL OR m.scope_value = $3 OR m.scope_type = 'global')
            ORDER BY rank DESC
            LIMIT $4
            "#,
        )
        .bind(keywords)
        .bind(&scope_type)
        .bind(&scope_value)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| (r.row.into_entry(), r.rank)).collect())
    }

    /// Mark an entry as helpful (increment helpful_count).
    pub async fn mark_helpful(&self, id: &str) -> Result<(), MemoryError> {
        let result = sqlx::query(
            "UPDATE memory_entries SET helpful_count = helpful_count + 1 WHERE id = $1",
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(MemoryError::NotFound(id.to_string()));
        }
        Ok(())
    }

    /// Mark an entry as misleading (increment misleading_count).
    pub async fn mark_misleading(&self, id: &str) -> Result<(), MemoryError> {
        let result = sqlx::query(
            "UPDATE memory_entries SET misleading_count = misleading_count + 1 WHERE id = $1",
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(MemoryError::NotFound(id.to_string()));
        }
        Ok(())
    }

    /// Update last_accessed timestamp for a batch of entry IDs.
    pub async fn touch(&self, ids: &[String]) -> Result<(), MemoryError> {
        if ids.is_empty() {
            return Ok(());
        }
        let now = Utc::now();
        // Use ANY($1) for batch update.
        sqlx::query(
            "UPDATE memory_entries SET last_accessed = $1 WHERE id = ANY($2)",
        )
        .bind(now)
        .bind(ids)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Expose the connection pool for modules that need direct SQL access (sync, retirement).
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Count active (non-retired) entries.
    pub async fn active_count(&self) -> Result<i64, MemoryError> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM memory_entries WHERE retired_at IS NULL",
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row.0)
    }

    /// List entries with optional category and scope filters.
    pub async fn list(
        &self,
        category: Option<MemoryCategory>,
        scope: Option<&MemoryScope>,
        active_only: bool,
        limit: i64,
    ) -> Result<Vec<MemoryEntry>, MemoryError> {
        let cat_str = category.map(|c| c.as_str().to_string());
        let scope_type = scope.map(|s| s.scope_type().to_string());
        let scope_value = scope.and_then(|s| s.scope_value().map(String::from));

        let rows = sqlx::query_as::<_, MemoryRow>(
            r#"
            SELECT * FROM memory_entries
            WHERE ($1::TEXT IS NULL OR category = $1)
              AND ($2::TEXT IS NULL OR scope_type = $2)
              AND ($3::TEXT IS NULL OR scope_value = $3)
              AND ($4::BOOL = FALSE OR retired_at IS NULL)
            ORDER BY created_at DESC
            LIMIT $5
            "#,
        )
        .bind(&cat_str)
        .bind(&scope_type)
        .bind(&scope_value)
        .bind(active_only)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.into_entry()).collect())
    }
}

/// Raw row from the memory_entries table.
#[derive(Debug, sqlx::FromRow)]
struct MemoryRow {
    id: String,
    content: String,
    category: String,
    scope_type: String,
    scope_value: Option<String>,
    tags: serde_json::Value,
    source_session: Option<String>,
    source_project: Option<String>,
    helpful_count: i32,
    misleading_count: i32,
    decay_rate: f64,
    retirement_policy: String,
    created_at: chrono::DateTime<Utc>,
    last_accessed: chrono::DateTime<Utc>,
    retired_at: Option<chrono::DateTime<Utc>>,
}

impl MemoryRow {
    fn into_entry(self) -> MemoryEntry {
        let category = MemoryCategory::parse(&self.category)
            .unwrap_or(MemoryCategory::Lesson);
        let scope = match self.scope_type.as_str() {
            "project" => MemoryScope::Project(self.scope_value.unwrap_or_default()),
            "language" => MemoryScope::Language(self.scope_value.unwrap_or_default()),
            _ => MemoryScope::Global,
        };
        let tags: Vec<String> = serde_json::from_value(self.tags).unwrap_or_default();
        let retirement_policy = if self.retirement_policy == "manual_only" {
            RetirementPolicy::ManualOnly
        } else {
            RetirementPolicy::Auto
        };

        MemoryEntry {
            id: self.id,
            content: self.content,
            category,
            scope,
            tags,
            source_session: self.source_session,
            source_project: self.source_project,
            helpful_count: self.helpful_count,
            misleading_count: self.misleading_count,
            decay_rate: self.decay_rate,
            retirement_policy,
            created_at: self.created_at,
            last_accessed: self.last_accessed,
            retired_at: self.retired_at,
        }
    }
}

/// Row with rank from tsvector search.
#[derive(Debug, sqlx::FromRow)]
struct RankedMemoryRow {
    #[sqlx(flatten)]
    row: MemoryRow,
    rank: f64,
}
```

- [ ] **Step 2: Run cargo check**

```bash
SQLX_OFFLINE=true cargo check -p autonomic-memory
```

- [ ] **Step 3: Commit**

```bash
git add crates/autonomic-memory/src/store.rs
git commit -m "feat(memory): add MemoryStore with CRUD and tsvector search"
```

## Task 3: Scoring Algorithm

**Files:**
- Create: `crates/autonomic-memory/src/scoring.rs`

- [ ] **Step 1: Create scoring.rs with decay, usefulness, and ranking**

```rust
//! Decay scoring and candidate ranking for memory retrieval.
//!
//! Score formula: ts_rank * usefulness_laplace * exp(-decay_rate * days) * category_boost

use chrono::Utc;

use crate::types::{MemoryEntry, ScoredEntry};

/// Estimate tokens from text (1 token ≈ 3.5 chars, conservative).
pub fn estimate_tokens(text: &str) -> usize {
    (text.len() as f64 / 3.5).ceil() as usize
}

/// Compute the usefulness ratio with Laplace smoothing.
/// Returns a value in (0, 1). New entries (0/0) score 0.5.
pub fn usefulness_ratio(helpful: i32, misleading: i32) -> f64 {
    (helpful as f64 + 1.0) / (helpful as f64 + misleading as f64 + 2.0)
}

/// Compute the decay factor for an entry.
/// Returns exp(-decay_rate * days_since_last_access).
pub fn decay_factor(entry: &MemoryEntry) -> f64 {
    let days = (Utc::now() - entry.last_accessed).num_seconds() as f64 / 86400.0;
    (-entry.decay_rate * days).exp()
}

/// Score a single entry given its tsvector rank.
pub fn score_entry(entry: &MemoryEntry, fts_rank: f64) -> f64 {
    let useful = usefulness_ratio(entry.helpful_count, entry.misleading_count);
    let decay = decay_factor(entry);
    let boost = entry.category.boost();

    fts_rank * useful * decay * boost
}

/// Score and rank a batch of search results.
pub fn rank_candidates(entries: Vec<(MemoryEntry, f64)>) -> Vec<ScoredEntry> {
    let mut scored: Vec<ScoredEntry> = entries
        .into_iter()
        .map(|(entry, fts_rank)| {
            let score = score_entry(&entry, fts_rank);
            ScoredEntry {
                entry,
                fts_rank,
                score,
            }
        })
        .collect();

    scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    scored
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usefulness_new_entry_is_neutral() {
        let ratio = usefulness_ratio(0, 0);
        assert!((ratio - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn usefulness_helpful_increases() {
        let r0 = usefulness_ratio(0, 0);
        let r1 = usefulness_ratio(5, 0);
        assert!(r1 > r0);
    }

    #[test]
    fn usefulness_misleading_decreases() {
        let r0 = usefulness_ratio(0, 0);
        let r1 = usefulness_ratio(0, 5);
        assert!(r1 < r0);
    }

    #[test]
    fn estimate_tokens_rough_accuracy() {
        let text = "This is a test sentence with about ten words in it.";
        let tokens = estimate_tokens(text);
        // ~52 chars / 3.5 ≈ 15 tokens. Actual GPT tokenization: ~12.
        assert!(tokens >= 10 && tokens <= 20);
    }
}
```

- [ ] **Step 2: Commit**

```bash
git add crates/autonomic-memory/src/scoring.rs
git commit -m "feat(memory): add decay scoring with usefulness, decay factor, and category boost"
```

## Task 4: Context Assembly Algorithm

**Files:**
- Create: `crates/autonomic-memory/src/assembly.rs`

- [ ] **Step 1: Create assembly.rs with full assembly pipeline**

```rust
//! Context assembly: keyword extraction, search, scoring, token-budgeted packing.
//!
//! Runs at session start and after compaction. Produces a formatted text block
//! injected into the agent's context via hook stdout.

use crate::scoring::{estimate_tokens, rank_candidates};
use crate::store::MemoryStore;
use crate::types::{MemoryCategory, MemoryScope, ScoredEntry};
use crate::MemoryError;

/// Stop words to filter from keyword extraction.
const STOP_WORDS: &[&str] = &[
    "the", "a", "an", "is", "are", "was", "were", "be", "been", "being",
    "have", "has", "had", "do", "does", "did", "will", "would", "could",
    "should", "may", "might", "can", "shall", "to", "of", "in", "for",
    "on", "with", "at", "by", "from", "as", "into", "through", "during",
    "before", "after", "above", "below", "between", "out", "off", "over",
    "under", "again", "further", "then", "once", "here", "there", "when",
    "where", "why", "how", "all", "each", "every", "both", "few", "more",
    "most", "other", "some", "such", "no", "nor", "not", "only", "own",
    "same", "so", "than", "too", "very", "just", "because", "but", "and",
    "or", "if", "while", "about", "up", "it", "its", "this", "that",
    "these", "those", "i", "me", "my", "we", "our", "you", "your", "he",
    "him", "his", "she", "her", "they", "them", "their", "what", "which",
    "who", "whom",
];

/// Extract up to 8 keywords from a prompt.
pub fn extract_keywords(prompt: &str) -> Vec<String> {
    let stop: std::collections::HashSet<&str> = STOP_WORDS.iter().copied().collect();
    let mut seen = std::collections::HashSet::new();
    let mut keywords = Vec::new();

    for word in prompt.split(|c: char| !c.is_alphanumeric() && c != '_') {
        let lower = word.to_lowercase();
        if lower.len() < 2 || stop.contains(lower.as_str()) {
            continue;
        }
        // Split compound identifiers (snake_case).
        for part in lower.split('_') {
            if part.len() >= 2 && !stop.contains(part) && seen.insert(part.to_string()) {
                keywords.push(part.to_string());
                if keywords.len() >= 8 {
                    return keywords;
                }
            }
        }
    }

    keywords
}

/// Assemble context for a session.
///
/// Returns a formatted text block and the list of entry IDs that were included
/// (for updating last_accessed).
pub async fn assemble_context(
    store: &MemoryStore,
    prompt: &str,
    project_id: Option<&str>,
    project_language: Option<&str>,
    token_budget: usize,
) -> Result<(String, Vec<String>), MemoryError> {
    let keywords = extract_keywords(prompt);
    if keywords.is_empty() {
        return Ok((String::new(), Vec::new()));
    }

    let keyword_str = keywords.join(" ");

    // Query candidates (up to 30) with scope filtering.
    let scope = project_id.map(|p| MemoryScope::Project(p.to_string()));
    let candidates = store.search(&keyword_str, scope.as_ref(), 30).await?;

    if candidates.is_empty() {
        return Ok((String::new(), Vec::new()));
    }

    // Score and rank.
    let ranked = rank_candidates(candidates);

    // Pack within token budget using tiered rendering.
    let (output, ids) = pack_with_tiers(&ranked, token_budget);

    if output.is_empty() {
        return Ok((String::new(), Vec::new()));
    }

    // Wrap in delimiters.
    let header = format!(
        "=== SESSION CONTEXT (from memory, {} entries, ~{} tokens) ===\n",
        ids.len(),
        estimate_tokens(&output),
    );
    let footer = "\n=== END SESSION CONTEXT ===";
    let full = format!("{header}{output}{footer}");

    Ok((full, ids))
}

/// Pack scored entries into a token-budgeted output using tiered rendering.
///
/// Tier 1 (top 3): full content + category + tags (~200-400 tokens each)
/// Tier 2 (next 5): one-line summary + category (~30-50 tokens each)
/// Tier 3 (remaining): tags only (~10 tokens each)
///
/// Entries are reordered with primacy/recency bias:
/// - Decisions and Gotchas at the START
/// - Patterns and Preferences at the END
/// - Everything else in the middle
fn pack_with_tiers(ranked: &[ScoredEntry], token_budget: usize) -> (String, Vec<String>) {
    let mut used_tokens = 0;
    let mut included: Vec<(String, &ScoredEntry, u8)> = Vec::new(); // (rendered, entry, tier)

    for (i, se) in ranked.iter().enumerate() {
        let (rendered, tier) = if i < 3 {
            // Tier 1: full content.
            let tags_str = if se.entry.tags.is_empty() {
                String::new()
            } else {
                format!("\nTags: {}", se.entry.tags.join(", "))
            };
            let first_line = se.entry.content.lines().next().unwrap_or(&se.entry.content);
            let rest: String = se.entry.content.lines().skip(1).collect::<Vec<_>>().join("\n");
            let text = if rest.is_empty() {
                format!(
                    "## {}: {}{}",
                    capitalize(se.entry.category.as_str()),
                    first_line,
                    tags_str,
                )
            } else {
                format!(
                    "## {}: {}\n{}{}",
                    capitalize(se.entry.category.as_str()),
                    first_line,
                    rest,
                    tags_str,
                )
            };
            (text, 1u8)
        } else if i < 8 {
            // Tier 2: one-line summary.
            let first_sentence = se.entry.content
                .split('.')
                .next()
                .unwrap_or(&se.entry.content);
            let text = format!(
                "- [{}] {}",
                capitalize(se.entry.category.as_str()),
                first_sentence.trim(),
            );
            (text, 2)
        } else {
            // Tier 3: tags only.
            if se.entry.tags.is_empty() {
                continue;
            }
            let text = format!("Also relevant: {}", se.entry.tags.join(", "));
            (text, 3)
        };

        let tokens = estimate_tokens(&rendered);
        if used_tokens + tokens > token_budget {
            break;
        }

        used_tokens += tokens;
        included.push((rendered, se, tier));
    }

    // Reorder: Decisions/Gotchas first, Patterns/Preferences last, rest middle.
    let mut start = Vec::new();
    let mut middle = Vec::new();
    let mut end = Vec::new();

    for (text, se, _tier) in &included {
        match se.entry.category {
            MemoryCategory::Decision | MemoryCategory::Gotcha => start.push(text.as_str()),
            MemoryCategory::Pattern | MemoryCategory::Preference => end.push(text.as_str()),
            _ => middle.push(text.as_str()),
        }
    }

    let parts: Vec<&str> = start.into_iter().chain(middle).chain(end).collect();
    let ids: Vec<String> = included.iter().map(|(_, se, _)| se.entry.id.clone()).collect();

    (parts.join("\n\n"), ids)
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().to_string() + c.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_keywords_filters_stop_words() {
        let kw = extract_keywords("the quick brown fox jumps over the lazy dog");
        assert!(!kw.contains(&"the".to_string()));
        assert!(kw.contains(&"quick".to_string()));
        assert!(kw.contains(&"brown".to_string()));
    }

    #[test]
    fn extract_keywords_splits_snake_case() {
        let kw = extract_keywords("spawn_blocking async runtime");
        assert!(kw.contains(&"spawn".to_string()));
        assert!(kw.contains(&"blocking".to_string()));
    }

    #[test]
    fn extract_keywords_limits_to_8() {
        let kw = extract_keywords("one two three four five six seven eight nine ten eleven twelve");
        assert!(kw.len() <= 8);
    }
}
```

- [ ] **Step 2: Commit**

```bash
git add crates/autonomic-memory/src/assembly.rs
git commit -m "feat(memory): add context assembly with keyword extraction, scoring, tiered packing"
```

## Task 5: MEMORY.md Sync

**Files:**
- Create: `crates/autonomic-memory/src/sync.rs`

- [ ] **Step 1: Create sync.rs with bidirectional MEMORY.md sync**

```rust
//! Bidirectional MEMORY.md sync between PostgreSQL and Claude Code's native format.
//!
//! - Store -> MEMORY.md: Generate project-specific MEMORY.md from store
//! - MEMORY.md -> Store: Ingest new entries from Claude Code's auto-memory

use std::path::Path;

use chrono::Utc;
use sha2::{Digest, Sha256};
use sqlx::PgPool;

use crate::store::MemoryStore;
use crate::types::*;
use crate::MemoryError;

/// Generate a MEMORY.md for a project from the memory store.
pub async fn generate_memory_md(
    store: &MemoryStore,
    project_id: &str,
    project_name: &str,
    output_path: &Path,
) -> Result<(), MemoryError> {
    let scope = MemoryScope::Project(project_id.to_string());
    let entries = store.list(None, Some(&scope), true, 100).await?;

    // Also include global entries.
    let global_entries = store.list(None, Some(&MemoryScope::Global), true, 50).await?;

    let mut all_entries = entries;
    all_entries.extend(global_entries);

    // Group by category.
    let mut decisions = Vec::new();
    let mut patterns = Vec::new();
    let mut gotchas = Vec::new();
    let mut lessons = Vec::new();
    let mut other = Vec::new();

    for entry in &all_entries {
        match entry.category {
            MemoryCategory::Decision => decisions.push(entry),
            MemoryCategory::Pattern => patterns.push(entry),
            MemoryCategory::Gotcha => gotchas.push(entry),
            MemoryCategory::Lesson => lessons.push(entry),
            _ => other.push(entry),
        }
    }

    let mut md = format!(
        "# {} Memory\n\n> Auto-generated by Autonomic. Do not edit directly — changes are ingested.\n> Last sync: {}\n\n",
        project_name,
        Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
    );

    if !decisions.is_empty() {
        md.push_str("## Decisions\n\n");
        for (i, e) in decisions.iter().take(10).enumerate() {
            md.push_str(&format!("{}. {}\n", i + 1, e.content.lines().next().unwrap_or(&e.content)));
        }
        md.push('\n');
    }

    if !patterns.is_empty() {
        md.push_str("## Patterns\n\n");
        for e in patterns.iter().take(10) {
            md.push_str(&format!("- {}\n", e.content.lines().next().unwrap_or(&e.content)));
        }
        md.push('\n');
    }

    if !gotchas.is_empty() {
        md.push_str("## Gotchas\n\n");
        for e in gotchas.iter().take(10) {
            md.push_str(&format!("- {}\n", e.content.lines().next().unwrap_or(&e.content)));
        }
        md.push('\n');
    }

    if !lessons.is_empty() {
        md.push_str("## Recent Lessons\n\n");
        for e in lessons.iter().take(5) {
            md.push_str(&format!("- {}\n", e.content.lines().next().unwrap_or(&e.content)));
        }
        md.push('\n');
    }

    // Atomic write: write to tmp, then rename.
    let tmp_path = output_path.with_extension("md.tmp");
    tokio::fs::write(&tmp_path, &md).await?;
    tokio::fs::rename(&tmp_path, output_path).await?;

    // Update sync metadata.
    let hash = hex_hash(&md);
    update_sync_hash(&store, project_id, &hash).await?;

    Ok(())
}

/// Ingest new entries from a MEMORY.md file into the store.
pub async fn ingest_memory_md(
    store: &MemoryStore,
    pool: &PgPool,
    project_id: &str,
    memory_md_path: &Path,
    session_id: Option<&str>,
) -> Result<usize, MemoryError> {
    let content = tokio::fs::read_to_string(memory_md_path).await?;
    let current_hash = hex_hash(&content);

    // Check if content has changed since last sync.
    let last_hash = get_sync_hash(pool, project_id).await?;
    if last_hash.as_deref() == Some(current_hash.as_str()) {
        return Ok(0); // No changes.
    }

    // Parse sections and entries from markdown.
    let new_entries = parse_memory_md(&content, project_id);
    let mut ingested = 0;

    for (text, category) in &new_entries {
        // Check for duplicates via content similarity.
        let existing = store.search(text, None, 1).await?;
        let is_duplicate = existing.iter().any(|(e, _)| e.content.trim() == text.trim());

        if !is_duplicate {
            let entry = NewMemoryEntry {
                content: text.clone(),
                category: *category,
                scope: MemoryScope::Project(project_id.to_string()),
                tags: Vec::new(),
                source_session: session_id.map(String::from),
                source_project: Some(project_id.to_string()),
                retirement_policy: RetirementPolicy::Auto,
            };
            store.create(&entry).await?;
            ingested += 1;
        }
    }

    // Update sync hash.
    update_sync_hash(store, project_id, &current_hash).await?;

    Ok(ingested)
}

/// Parse a MEMORY.md file into (content, category) pairs.
fn parse_memory_md(content: &str, _project_id: &str) -> Vec<(String, MemoryCategory)> {
    let mut entries = Vec::new();
    let mut current_category = MemoryCategory::Lesson;

    for line in content.lines() {
        let trimmed = line.trim();

        // Detect section headers.
        if trimmed.starts_with("## ") {
            let header = trimmed.trim_start_matches("## ").to_lowercase();
            current_category = match header.as_str() {
                "decisions" => MemoryCategory::Decision,
                "patterns" => MemoryCategory::Pattern,
                "gotchas" => MemoryCategory::Gotcha,
                "recent lessons" | "lessons" => MemoryCategory::Lesson,
                "preferences" => MemoryCategory::Preference,
                "tools" => MemoryCategory::Tool,
                "errors" => MemoryCategory::Error,
                _ => MemoryCategory::Lesson,
            };
            continue;
        }

        // Skip headers, empty lines, and auto-generated markers.
        if trimmed.is_empty()
            || trimmed.starts_with('#')
            || trimmed.starts_with('>')
            || trimmed.starts_with("Auto-generated")
        {
            continue;
        }

        // Extract bullet point or numbered content.
        let text = trimmed
            .trim_start_matches("- ")
            .trim_start_matches(|c: char| c.is_ascii_digit() || c == '.')
            .trim();

        if !text.is_empty() && text.len() > 5 {
            entries.push((text.to_string(), current_category));
        }
    }

    entries
}

fn hex_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let result = hasher.finalize();
    result.iter().map(|b| format!("{b:02x}")).collect::<String>()[..16].to_string()
}

async fn get_sync_hash(pool: &PgPool, project_id: &str) -> Result<Option<String>, MemoryError> {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT last_hash FROM memory_md_sync WHERE project_id = $1",
    )
    .bind(project_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| r.0))
}

async fn update_sync_hash(store: &MemoryStore, project_id: &str, hash: &str) -> Result<(), MemoryError> {
    let now = Utc::now();
    sqlx::query(
        r#"
        INSERT INTO memory_md_sync (project_id, last_hash, last_sync)
        VALUES ($1, $2, $3)
        ON CONFLICT (project_id) DO UPDATE SET last_hash = $2, last_sync = $3
        "#,
    )
    .bind(project_id)
    .bind(hash)
    .bind(now)
    .execute(&store.pool)
    .await?;

    Ok(())
}

// Note: pool() accessor is defined in store.rs, available here via `store.pool()`.
```

- [ ] **Step 2: Commit**

```bash
git add crates/autonomic-memory/src/sync.rs
git commit -m "feat(memory): add MEMORY.md bidirectional sync (generation + ingestion)"
```

## Task 6: Retirement Algorithm

**Files:**
- Create: `crates/autonomic-memory/src/retirement.rs`

- [ ] **Step 1: Create retirement.rs**

```rust
//! Stale entry retirement algorithm.
//!
//! Runs weekly (via scheduled job). Retires entries whose relevance score
//! has dropped below threshold. Entries are never deleted — only marked
//! with retired_at timestamp.

use chrono::Utc;

use crate::scoring::{decay_factor, usefulness_ratio};
use crate::store::MemoryStore;
use crate::types::RetirementPolicy;
use crate::MemoryError;

/// Retire stale entries that have decayed below the relevance threshold.
///
/// Returns the number of entries retired.
pub async fn retire_stale_entries(
    store: &MemoryStore,
    capacity_threshold: i64,
    relevance_threshold: f64,
) -> Result<usize, MemoryError> {
    let count = store.active_count().await?;
    if count < capacity_threshold {
        return Ok(0);
    }

    // Fetch all active auto-retirable entries.
    let entries = store.list(None, None, true, 10000).await?;
    let now = Utc::now();
    let mut to_retire = Vec::new();

    for entry in &entries {
        if entry.retirement_policy != RetirementPolicy::Auto {
            continue;
        }

        let useful = usefulness_ratio(entry.helpful_count, entry.misleading_count);
        let decay = decay_factor(entry);
        let relevance = useful * decay * entry.category.boost();

        if relevance < relevance_threshold {
            to_retire.push(entry.id.clone());
        }
    }

    // If still over capacity after threshold-based retirement, retire lowest-relevance entries.
    let target = (capacity_threshold as f64 * 0.8) as i64;
    let remaining = count - to_retire.len() as i64;
    if remaining > target {
        // Sort remaining auto-retirable entries by relevance, retire lowest until under target.
        let mut scored: Vec<(String, f64)> = entries
            .iter()
            .filter(|e| e.retirement_policy == RetirementPolicy::Auto && !to_retire.contains(&e.id))
            .map(|e| {
                let useful = usefulness_ratio(e.helpful_count, e.misleading_count);
                let decay = decay_factor(e);
                let relevance = useful * decay * e.category.boost();
                (e.id.clone(), relevance)
            })
            .collect();
        scored.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        let excess = (remaining - target) as usize;
        for (id, _) in scored.into_iter().take(excess) {
            to_retire.push(id);
        }
    }

    if to_retire.is_empty() {
        return Ok(0);
    }

    // Batch retire.
    sqlx::query(
        "UPDATE memory_entries SET retired_at = $1 WHERE id = ANY($2)",
    )
    .bind(now)
    .bind(&to_retire)
    .execute(store.pool())
    .await?;

    tracing::info!(
        retired = to_retire.len(),
        active_before = count,
        "retired stale memory entries"
    );

    Ok(to_retire.len())
}
```

- [ ] **Step 2: Commit**

```bash
git add crates/autonomic-memory/src/retirement.rs
git commit -m "feat(memory): add stale entry retirement algorithm"
```

## Task 7: Integration Verification

- [ ] **Step 1: Verify crate compiles**

```bash
SQLX_OFFLINE=true cargo check -p autonomic-memory
```

- [ ] **Step 2: Run clippy**

```bash
SQLX_OFFLINE=true cargo clippy -p autonomic-memory --all-targets -- -D warnings
```

- [ ] **Step 3: Run tests**

```bash
cargo test -p autonomic-memory
```

- [ ] **Step 4: Commit and tag**

```bash
git tag -a milestone/phase-2a -m "Phase 2A: Memory store — CRUD, tsvector search, scoring, assembly, sync, retirement"
```
