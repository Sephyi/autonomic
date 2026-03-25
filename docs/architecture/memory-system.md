# Memory System — Architecture Specification

**Status**: Draft
**Last updated**: 2026-03-26
**Replaces**: Mem0 + Qdrant + Ollama (slow, imprecise, ignored in practice)

## 1. Design Rationale

### 1.1 Why Not Mem0/Qdrant/Ollama

The operator's experience with Mem0:
- Ollama embedding generation is slow (~200ms per entry), blocking the pipeline
- Qdrant requires a separate server process (Podman container)
- Vector similarity is imprecise for exact recall ("what was the decision about X?")
- Claude Code's native MEMORY.md auto-memory was used instead, making Mem0 redundant
- The system was "basically always ignored" in practice

### 1.2 Design Principles

1. **Containerized storage** — PostgreSQL runs in a Podman container alongside the daemon; no host-level database install required
2. **Sub-millisecond queries** — tsvector keyword search is instant for 10K entries
3. **Let the LLM judge relevance** — provide keyword-matched candidates, let Claude reason about which are relevant to the current task
4. **Structured over unstructured** — typed entries with categories, scopes, and metadata vs flat markdown
5. **Decay prevents bloat** — entries lose relevance over time; stale entries are retired, not manually pruned
6. **Bidirectional sync with MEMORY.md** — Claude Code's native auto-memory is ingested; the orchestrator generates project-specific MEMORY.md from the store

## 2. Storage Layer

### 2.1 PostgreSQL + tsvector

PostgreSQL 17 running in a Podman container (managed by `podman compose`). The database is accessed via `sqlx` with compile-time checked queries and an async connection pool.

```sql
-- Core memory entries
CREATE TABLE memory_entries (
    id TEXT PRIMARY KEY,              -- ULID (sortable, unique)
    content TEXT NOT NULL,            -- The actual knowledge
    category TEXT NOT NULL,           -- Decision, Pattern, Gotcha, Preference, Tool, Error, Lesson, Ephemeral
    scope_type TEXT NOT NULL,         -- global, project, language
    scope_value TEXT,                 -- NULL for global; project name or language name otherwise
    tags JSONB NOT NULL DEFAULT '[]', -- JSON array of strings
    source_session TEXT,              -- Session ID that created this entry
    source_project TEXT,              -- Project that created this entry
    helpful_count INTEGER NOT NULL DEFAULT 0,
    misleading_count INTEGER NOT NULL DEFAULT 0,
    decay_rate DOUBLE PRECISION NOT NULL, -- Per-category default, overridable
    retirement_policy TEXT NOT NULL DEFAULT 'auto', -- auto | manual_only
    created_at TIMESTAMPTZ NOT NULL,
    last_accessed TIMESTAMPTZ NOT NULL, -- Updated on every retrieval
    retired_at TIMESTAMPTZ,           -- NULL = active; set when retired

    -- tsvector column for full-text search (auto-generated)
    search_vector TSVECTOR GENERATED ALWAYS AS (
        setweight(to_tsvector('english', content), 'A') ||
        setweight(to_tsvector('english', coalesce(tags::text, '')), 'B')
    ) STORED
);

-- GIN index for full-text search (replaces FTS5 virtual table)
CREATE INDEX idx_memory_search ON memory_entries USING GIN (search_vector);

-- Standard indexes for common queries
CREATE INDEX idx_memory_scope ON memory_entries(scope_type, scope_value);
CREATE INDEX idx_memory_category ON memory_entries(category);
CREATE INDEX idx_memory_active ON memory_entries(retired_at) WHERE retired_at IS NULL;
CREATE INDEX idx_memory_project ON memory_entries(source_project);
```

**Note on pgvector**: The PostgreSQL container includes the `pgvector` extension, resolving OQ-002 (semantic search path). While not active yet — tsvector keyword search remains the primary retrieval mechanism — `CREATE EXTENSION vector;` enables future semantic search with embedding vectors alongside the existing keyword approach. No external embedding server required; embeddings can be generated in-process or via a lightweight ONNX model.

## 3. Memory Entry Types

```rust
pub enum MemoryCategory {
    /// Architectural choices with rationale.
    /// Example: "Chose containerized Postgres for state — concurrent access, tsvector FTS, pgvector-ready"
    /// Decay rate: 0.01 (near-permanent — decisions rarely become irrelevant)
    Decision,

    /// Recurring code/workflow patterns.
    /// Example: "For Rust FFI: always spawn_blocking, never hold mutex across await"
    /// Decay rate: 0.03 (stable — patterns change slowly)
    Pattern,

    /// Non-obvious behaviors, footguns, surprises.
    /// Example: "Metal crashes with concurrent ONNX sessions — serialize GPU access"
    /// Decay rate: 0.02 (may be fixed in future versions, but dangerous to forget too early)
    Gotcha,

    /// Operator workflow preferences discovered through interaction.
    /// Example: "Prefers conventional commits, one concern per commit"
    /// Decay rate: 0.01 (preferences are stable)
    Preference,

    /// Tool/MCP/hook/library discoveries.
    /// Example: "context7 MCP gives better API docs than raw web search"
    /// Decay rate: 0.05 (tools evolve, new ones appear)
    Tool,

    /// Error patterns and their fixes.
    /// Example: "CLAUDECODE=1 env var blocks nested SDK — filter from subprocess env"
    /// Decay rate: 0.08 (errors get fixed, workarounds become unnecessary)
    Error,

    /// Session post-mortems, retrospectives.
    /// Example: "Spent 2h debugging TSIG because response HMAC wasn't verified — add verification first"
    /// Decay rate: 0.10 (lessons fade as patterns are internalized)
    Lesson,

    /// Session-specific notes, temporary context.
    /// Example: "Currently working on Phase 2 of vox-scribe audio pipeline"
    /// Decay rate: 0.20 (aggressive — these are transient by nature)
    Ephemeral,
}
```

## 4. Context Assembly

### 4.1 Assembly Algorithm

Runs at session start (via SessionStart hook) and after compaction (via compact hook).

```
Algorithm: AssembleContext(prompt, project_id, token_budget=4096)

Input:
  prompt: the user's first message or current task description
  project_id: which project this session is for
  token_budget: maximum tokens to inject (default 4096)

Output:
  context_block: formatted text for hook stdout injection

Process:
1. EXTRACT keywords from prompt:
   - Strip stop words (the, a, an, is, are, was, etc.)
   - Take up to 8 unique terms
   - Split compound identifiers (e.g., "spawn_blocking" -> "spawn", "blocking")

2. QUERY tsvector with keywords (OR semantics via plainto_tsquery):
   - SELECT *, ts_rank(search_vector, query) AS rank
     FROM memory_entries,
          plainto_tsquery('english', '{keywords}') query
     WHERE search_vector @@ query
     AND retired_at IS NULL
     AND (scope_type = 'global'
          OR (scope_type = 'project' AND scope_value = project_id)
          OR (scope_type = 'language' AND scope_value = project_language))
     ORDER BY rank DESC
     LIMIT 30       -- candidate pool

   All queries use sqlx compile-time checked macros (`sqlx::query!` / `sqlx::query_as!`),
   eliminating runtime SQL errors.

3. SCORE each candidate:
   For each entry e:
     fts_rank = normalized ts_rank (0.0 to 1.0)
     usefulness = (e.helpful_count + 1) / (e.helpful_count + e.misleading_count + 2)  // Laplace smoothing
     days_since_access = (now - e.last_accessed).days
     decay_factor = exp(-e.decay_rate * days_since_access)
     category_boost = match e.category {
         Decision | Gotcha => 1.3,  // High value for architectural context
         Pattern | Preference => 1.1,
         _ => 1.0,
     }
     score = fts_rank * usefulness * decay_factor * category_boost

4. SORT by score descending

5. PACK within token_budget using tiered rendering:
   tier_1 (top 3): full content + category + tags
     Format: "## {category}: {first_line}\n{content}\nTags: {tags}"
     ~200-400 tokens each
   tier_2 (next 5): one-line summary + category
     Format: "- [{category}] {first_sentence_of_content}"
     ~30-50 tokens each
   tier_3 (remaining): tags only
     Format: "Also relevant: {tag1}, {tag2}, ..."
     ~10 tokens per entry
   Stop packing when token_budget reached

6. ORDER output with primacy/recency bias (from Instar's Playbook):
   - Decisions and Gotchas at the START (primacy position)
   - Patterns and Preferences at the END (recency position)
   - Everything else in the middle
   (LLMs attend most to first and last items in context)

7. UPDATE last_accessed for all entries included in output

8. RETURN formatted context block wrapped in delimiters:
   "=== SESSION CONTEXT (from memory, {N} entries, ~{T} tokens) ===\n"
   {formatted entries}
   "\n=== END SESSION CONTEXT ==="
```

### 4.2 Token Estimation

Simple heuristic: 1 token ≈ 4 characters for English text. No need for a tokenizer library.

```rust
fn estimate_tokens(text: &str) -> usize {
    // Conservative estimate: 1 token per 3.5 chars (slightly overestimates)
    (text.len() as f64 / 3.5).ceil() as usize
}
```

### 4.3 Assembly Trigger Points

| Trigger | What's Injected | Token Budget |
| --- | --- | --- |
| SessionStart (startup) | Keyword-matched entries + project context | 4096 |
| SessionStart (compact) | Identity + top entries + task context | 6144 (larger — recovering from loss) |
| SessionStart (resume) | Delta since last session (new entries only) | 2048 |

## 5. Decay and Retirement

### 5.1 Decay Formula

Relevance score at query time:

```
relevance(entry) = usefulness_ratio * exp(-decay_rate * days_since_accessed) * category_boost
```

Entries are NOT modified by decay — the score is computed at query time. This means an entry that hasn't been accessed in 90 days with decay_rate=0.10 has:
- `exp(-0.10 * 90)` = `exp(-9)` ≈ 0.0001 — effectively invisible in search results

### 5.2 Retirement Policy

```
Algorithm: RetireStaleEntries(capacity_threshold=1000)

Runs: weekly (scheduled job)

1. Count active entries (retired_at IS NULL)
2. If count < capacity_threshold: return (no action needed)
3. For each active entry:
   a. Compute current relevance score
   b. If relevance < 0.01 AND retirement_policy = 'auto':
      - Set retired_at = now
      - Do NOT delete — retired entries remain queryable with explicit flag
4. If still over capacity after retirement:
   a. Sort remaining by relevance ascending
   b. Retire lowest until count <= capacity_threshold * 0.8
5. Log retirement summary to learning registry
```

### 5.3 Manual-Only Entries

Entries with `retirement_policy = 'manual_only'` never auto-retire. Use for:
- Safety-critical gotchas ("NEVER log speaker embeddings — GDPR Art. 9")
- Fundamental architectural decisions
- Operator preferences that should persist indefinitely

## 6. MEMORY.md Bidirectional Sync

### 6.1 Store -> MEMORY.md (Generation)

For each registered project, the orchestrator generates a project-specific MEMORY.md:

```
Algorithm: GenerateMemoryMd(project_id)

1. Query all active entries where:
   scope = 'global' OR scope = ('project', project_id) OR scope = ('language', project_language)
2. Group by category
3. Sort within each category by score descending
4. Render as markdown:

   # {Project Name} Memory
   > Auto-generated by Autonomic. Do not edit directly — changes are ingested.
   > Last sync: {timestamp}

   ## Decisions
   - {decision entries, newest first, max 10}

   ## Patterns
   - {pattern entries, highest score first, max 10}

   ## Gotchas
   - {gotcha entries, highest score first, max 10}

   ## Recent Lessons
   - {lesson entries, last 5 only}

5. Write to {project_dir}/.claude/MEMORY.md (atomic write via tmp + rename)
```

**When**: Before every session start for that project. Cost: zero (file generation, no LLM).

### 6.2 MEMORY.md -> Store (Ingestion)

Claude Code's auto-memory writes to MEMORY.md. The orchestrator ingests these:

```
Algorithm: IngestMemoryMd(project_id, memory_md_path)

1. Read current MEMORY.md content
2. Diff against last known content (stored in PostgreSQL metadata table)
3. For each new/changed section:
   a. Parse into individual entries (split by bullet points or headers)
   b. For each entry:
      - Check if already exists in store (tsvector exact match on content)
      - If new: create MemoryEntry with:
        category = inferred from section header or content keywords
        scope = ('project', project_id)
        source_session = current session ID
        decay_rate = category default
      - If modified: update existing entry's content, reset last_accessed
4. Store new content hash as last known state
```

**When**: After every session end (Stop hook). Cost: zero (text parsing, no LLM).

### 6.3 Conflict Resolution

The PostgreSQL store is the **source of truth**. MEMORY.md is a projection.

- Orchestrator writes to MEMORY.md -> Claude Code reads it
- Claude Code's auto-memory appends to MEMORY.md -> orchestrator ingests additions
- If conflict: orchestrator's version wins on next generation cycle
- Operator can always manually add entries via CLI: `autonomic memory add "content" --category pattern --scope global`

## 7. Usefulness Tracking

### 7.1 Marking Useful/Misleading

Two mechanisms:

**Explicit** (operator via CLI):
```bash
autonomic memory helpful <id>
autonomic memory misleading <id>
```

**Implicit** (from experience traces):
- If an entry was injected at session start AND the session succeeded: increment helpful
- If an entry was injected AND the session had errors related to the entry's topic: increment misleading
- Heuristic: match error keywords against entry tags

### 7.2 Impact on Scoring

```
usefulness_ratio = (helpful + 1) / (helpful + misleading + 2)
```

**Laplace smoothing** (Gemini review fix): The `+1/+2` ensures new entries (0 helpful, 0 misleading) get a neutral score of 1/2 = 0.5, not zero. Without this, new memories would have score 0 and never surface — the system would ignore everything it learns until manually marked helpful. After first helpful mark: (1+1)/(1+0+2) = 2/3 ≈ 0.667. After 10 helpful, 0 misleading: 11/(10+0+2) = 0.917. (Codex review correction: initial example was mathematically wrong.)

An entry with 3 helpful and 7 misleading: 4/(3+7+2) = 0.33 — deprioritized but not removed.

## 8. Auto-Capture (from OpenClaw)

After significant sessions (not routine), the orchestrator evaluates whether to create memory entries automatically:

```
Algorithm: AutoCapture(session_trace)

Triggers (must meet at least one):
  - Session resolved an error that took >5 minutes
  - Session made an architectural decision (detected by keywords: "decided", "chose", "because")
  - Session discovered a new tool or pattern
  - Session encountered a gotcha ("actually", "turns out", "didn't expect", "counterintuitive")
  - Session modified hooks or configuration

Process:
1. Extract candidate entries from session transcript (Haiku tier, <$0.01)
2. For each candidate:
   a. Check for duplicates in store (tsvector search)
   b. If novel: create entry with appropriate category and scope
   c. If similar to existing: update existing entry's content (fresher version)
3. Mark new entries with source_session for provenance
```

**Cost**: ~$0.01 per significant session. Not every session triggers auto-capture.

## 9. CLI Interface

```bash
# Add entries
autonomic memory add "content" --category decision --scope global
autonomic memory add "content" --category gotcha --project vox-scribe --tags "rust,async"

# Search
autonomic memory search "spawn_blocking async"
autonomic memory search "TSIG" --project bind9-sdk

# Browse
autonomic memory list --category decision --limit 20
autonomic memory list --project vox-scribe --active-only

# Usefulness
autonomic memory helpful <id>
autonomic memory misleading <id>

# Maintenance
autonomic memory retire-stale          # Run retirement algorithm
autonomic memory stats                 # Entry counts, category breakdown, decay stats
autonomic memory export --format json  # Full export for backup

# Sync
autonomic memory sync <project>        # Force bidirectional sync with MEMORY.md
```

## 10. Performance Targets

| Operation | Target | Mechanism |
| --- | --- | --- |
| tsvector keyword search (10K entries) | < 1ms | PostgreSQL GIN index |
| Context assembly (30 candidates) | < 10ms | Score + sort + format |
| Full assembly pipeline | < 50ms | Query + score + render + write |
| Entry creation | < 5ms | Single INSERT + tsvector GENERATED column |
| MEMORY.md generation | < 20ms | Query + render + atomic write |
| MEMORY.md ingestion | < 50ms | Read + diff + parse + INSERTs |
| Retirement scan (10K entries) | < 100ms | Single pass with score computation |
