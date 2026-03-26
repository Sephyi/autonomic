-- migrations/001_initial.sql
-- Autonomic initial schema

-- Enable extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- pgvector for semantic search (install separately if needed)
-- CREATE EXTENSION IF NOT EXISTS "vector";

-- Memory entries with full-text search
CREATE TABLE memory_entries (
    id TEXT PRIMARY KEY,
    content TEXT NOT NULL,
    category TEXT NOT NULL,
    scope_type TEXT NOT NULL,
    scope_value TEXT,
    tags JSONB NOT NULL DEFAULT '[]',
    source_session TEXT,
    source_project TEXT,
    helpful_count INTEGER NOT NULL DEFAULT 0,
    misleading_count INTEGER NOT NULL DEFAULT 0,
    decay_rate DOUBLE PRECISION NOT NULL,
    retirement_policy TEXT NOT NULL DEFAULT 'auto',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_accessed TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    retired_at TIMESTAMPTZ,
    search_vector TSVECTOR GENERATED ALWAYS AS (
        to_tsvector('english', content || ' ' || COALESCE(tags::text, ''))
    ) STORED
);

CREATE INDEX idx_memory_search ON memory_entries USING GIN (search_vector);
CREATE INDEX idx_memory_scope ON memory_entries (scope_type, scope_value);
CREATE INDEX idx_memory_active ON memory_entries (retired_at) WHERE retired_at IS NULL;

-- Sessions
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    model TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'running',
    cost_usd DOUBLE PRECISION,
    duration_ms BIGINT,
    container_id TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ
);

-- Experience traces
CREATE TABLE experience_traces (
    id TEXT PRIMARY KEY,
    session_id TEXT REFERENCES sessions(id),
    project_id TEXT NOT NULL,
    variant_id TEXT,
    model_used TEXT NOT NULL,
    task_type TEXT,
    outcome TEXT NOT NULL,
    duration_ms BIGINT,
    cost_usd DOUBLE PRECISION,
    compaction_count INTEGER DEFAULT 0,
    trace_json JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Model performance matrix
CREATE TABLE model_performance (
    model TEXT NOT NULL,
    task_type TEXT NOT NULL,
    outcome TEXT NOT NULL,
    tokens_used BIGINT,
    cost_usd DOUBLE PRECISION,
    duration_ms BIGINT,
    session_id TEXT,
    project_id TEXT,
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_model_perf ON model_performance (model, task_type);

-- Projects registry (FR-002)
CREATE TABLE projects (
    id TEXT PRIMARY KEY,
    path TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    language TEXT,
    last_activity TIMESTAMPTZ,
    last_session_cost_usd DOUBLE PRECISION,
    hook_count INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Metrics time-series (FR-005)
CREATE TABLE metrics (
    id BIGSERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL,
    session_id TEXT REFERENCES sessions(id),
    metric_name TEXT NOT NULL,
    metric_value DOUBLE PRECISION NOT NULL,
    labels JSONB,
    UNIQUE(timestamp, session_id, metric_name)
);

CREATE INDEX idx_metrics_name_time ON metrics(metric_name, timestamp);
CREATE INDEX idx_metrics_session ON metrics(session_id);

-- Rate budget tracking
CREATE TABLE rate_budget (
    id BIGSERIAL PRIMARY KEY,
    window_start TIMESTAMPTZ NOT NULL,
    window_end TIMESTAMPTZ NOT NULL,
    model_tier TEXT NOT NULL,
    tokens_used BIGINT NOT NULL DEFAULT 0,
    tokens_limit BIGINT NOT NULL,
    requests_used BIGINT NOT NULL DEFAULT 0,
    requests_limit BIGINT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_rate_budget_window ON rate_budget(model_tier, window_start);

-- Memory sync metadata (XD-009)
CREATE TABLE memory_md_sync (
    project_id TEXT PRIMARY KEY,
    last_hash TEXT NOT NULL,
    last_sync TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
