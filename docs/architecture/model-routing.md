# Model Routing — Architecture Specification

**Status**: Draft
**Last updated**: 2026-03-25

## 1. Design Philosophy

The operator currently makes every model routing decision manually. Autonomic automates this with a two-layer system:

1. **Heuristic layer** (day 1): Rule-based routing from the operator's established patterns
2. **Empirical layer** (after data): Performance matrix built from measured outcomes overrides heuristics

Key constraint: external models (Codex, Gemini) are **recommended but never required**. The system is fully functional with Claude alone. Each external model has a fallback path.

From Morph's 15-agent benchmark: "The same model scores 17 problems apart in different agents. The scaffolding matters more than the model." Model routing is important, but it's secondary to the hook/memory/evolution infrastructure.

## 2. Available Models

### 2.1 Claude Models (via Claude Code CLI)

| Model | Strengths | Cost Tier | Context |
| --- | --- | --- | --- |
| Opus 4.6 | Architecture, complex planning, multi-concern | Highest | 200K or 1M |
| Sonnet 4.6 | Multi-file implementation, tool-heavy | Medium | 200K or 1M |
| Haiku 4.5 | Monitoring, analysis, cheap tasks | Lowest | 200K |

All share the same Claude Max rate limit (per-account).

### 2.2 External Models (via CLI subprocess)

| Model | CLI | Strengths | Rate Limit |
| --- | --- | --- | --- |
| Codex (gpt-5.3-codex) | `/opt/homebrew/bin/codex exec "..."` | Verification, code review | Separate (OpenAI) |
| Gemini 3 Pro | `/opt/homebrew/bin/gemini -p "..."` | Cross-verification, multi-concern analysis | Separate (Google) |

External models use their own rate limits — they don't consume Claude Max budget.

## 3. Task Classification

```rust
pub enum TaskType {
    /// High-level design decisions, system architecture, complex planning.
    /// Route to: Opus 4.6 (1M context if needed)
    Architecture,

    /// Multi-file changes, coordinated edits, complex refactoring.
    /// Route to: Opus 4.6 or Sonnet 4.6
    MultiFileImplementation,

    /// Single-file implementation, straightforward feature work.
    /// Route to: Sonnet 4.6
    StandardImplementation,

    /// Code review, plan verification, implementation audit.
    /// Route to: Codex (recommended) or Opus 4.6 (fallback)
    Verification,

    /// Cross-model verification for critical decisions.
    /// Route to: Gemini 3 Pro (recommended) + Codex, Opus fallback
    CrossVerification,

    /// Health checks, status monitoring, simple queries.
    /// Route to: Haiku 4.5
    Monitoring,

    /// Metric analysis, evolution engine analysis, pattern detection.
    /// Route to: Haiku 4.5 (cost-efficient for structured analysis)
    Analysis,

    /// Web search, documentation lookup, research tasks.
    /// Route to: Haiku 4.5 + MCP tools
    Research,

    /// Complex tasks requiring parallel agent coordination.
    /// Route to: Opus 4.6 1M with Agent Teams
    AgentTeam,
}
```

### 3.1 Heuristic Classification

```rust
pub fn classify_task(prompt: &str, project: &Project) -> TaskType {
    let lower = prompt.to_lowercase();

    // Agent Teams trigger (explicit or complexity-detected)
    if lower.contains("agent team") || lower.contains("parallel agents") {
        return TaskType::AgentTeam;
    }

    // Architecture signals
    if lower.contains("architect") || lower.contains("design")
        || lower.contains("plan") || lower.contains("restructure")
        || lower.contains("how should") || lower.contains("trade-off")
    {
        return TaskType::Architecture;
    }

    // Verification signals
    if lower.contains("review") || lower.contains("verify")
        || lower.contains("audit") || lower.contains("check")
    {
        if lower.contains("cross") || lower.contains("dialectic") {
            return TaskType::CrossVerification;
        }
        return TaskType::Verification;
    }

    // Monitoring signals
    if lower.contains("status") || lower.contains("health")
        || lower.contains("check") || lower.contains("monitor")
    {
        return TaskType::Monitoring;
    }

    // Research signals
    if lower.contains("search") || lower.contains("find")
        || lower.contains("research") || lower.contains("look up")
    {
        return TaskType::Research;
    }

    // Multi-file detection (heuristic: mentions multiple files/modules)
    let file_mentions = prompt.matches('/').count() + prompt.matches(".rs").count()
        + prompt.matches(".ts").count() + prompt.matches("crate").count();
    if file_mentions >= 3 {
        return TaskType::MultiFileImplementation;
    }

    TaskType::StandardImplementation
}
```

## 4. Routing Table

```rust
pub struct RoutingDecision {
    pub model: ModelTier,
    pub context_size: ContextSize, // Standard200K | Extended1M
    pub use_agent_teams: bool,
    pub external_verification: Option<ExternalModel>,
    pub fallback: ModelTier,
}

pub fn route(task_type: TaskType, budget: &RateBudget) -> RoutingDecision {
    match task_type {
        TaskType::Architecture => RoutingDecision {
            model: ModelTier::Opus,
            context_size: ContextSize::Extended1M, // architecture benefits from large context
            use_agent_teams: false,
            external_verification: None, // verification is a separate step
            fallback: ModelTier::Opus, // no downgrade for architecture
        },
        TaskType::MultiFileImplementation => RoutingDecision {
            model: if budget.remaining_percent() > 50.0 {
                ModelTier::Opus
            } else {
                ModelTier::Sonnet
            },
            context_size: ContextSize::Standard200K,
            use_agent_teams: false,
            external_verification: None,
            fallback: ModelTier::Sonnet,
        },
        TaskType::StandardImplementation => RoutingDecision {
            model: ModelTier::Sonnet,
            context_size: ContextSize::Standard200K,
            use_agent_teams: false,
            external_verification: None,
            fallback: ModelTier::Sonnet,
        },
        TaskType::Verification => RoutingDecision {
            model: ModelTier::Opus, // self-review fallback
            context_size: ContextSize::Standard200K,
            use_agent_teams: false,
            external_verification: Some(ExternalModel::Codex),
            fallback: ModelTier::Opus,
        },
        TaskType::CrossVerification => RoutingDecision {
            model: ModelTier::Opus,
            context_size: ContextSize::Standard200K,
            use_agent_teams: false,
            external_verification: Some(ExternalModel::GeminiAndCodex),
            fallback: ModelTier::Opus,
        },
        TaskType::Monitoring => RoutingDecision {
            model: ModelTier::Haiku,
            context_size: ContextSize::Standard200K,
            use_agent_teams: false,
            external_verification: None,
            fallback: ModelTier::Haiku,
        },
        TaskType::Analysis => RoutingDecision {
            model: ModelTier::Haiku,
            context_size: ContextSize::Standard200K,
            use_agent_teams: false,
            external_verification: None,
            fallback: ModelTier::Sonnet,
        },
        TaskType::Research => RoutingDecision {
            model: ModelTier::Haiku,
            context_size: ContextSize::Standard200K,
            use_agent_teams: false,
            external_verification: None,
            fallback: ModelTier::Sonnet,
        },
        TaskType::AgentTeam => RoutingDecision {
            model: ModelTier::Opus,
            context_size: ContextSize::Extended1M, // Agent Teams requires 1M
            use_agent_teams: true,
            external_verification: None,
            fallback: ModelTier::Opus, // fall back to single Opus without teams
        },
    }
}
```

## 5. Empirical Performance Matrix

### 5.1 Schema

```sql
CREATE TABLE model_performance (
    model TEXT NOT NULL,           -- opus, sonnet, haiku, codex, gemini
    task_type TEXT NOT NULL,       -- from TaskType enum
    outcome TEXT NOT NULL,         -- success, failure, partial
    tokens_used INTEGER,
    cost_usd REAL,
    duration_ms INTEGER,
    session_id TEXT,
    project_id TEXT,
    recorded_at TEXT NOT NULL,
    PRIMARY KEY (model, task_type, recorded_at)
);

-- Aggregated view for routing decisions
CREATE VIEW model_performance_summary AS
SELECT
    model,
    task_type,
    COUNT(*) as sample_count,
    SUM(CASE WHEN outcome = 'success' THEN 1 ELSE 0 END) * 1.0 / COUNT(*) as success_rate,
    AVG(tokens_used) as avg_tokens,
    AVG(cost_usd) as avg_cost,
    AVG(duration_ms) as avg_duration_ms
FROM model_performance
WHERE recorded_at > datetime('now', '-90 days')  -- Rolling 90-day window
GROUP BY model, task_type
HAVING COUNT(*) >= 5;  -- Minimum sample size for reliability
```

### 5.2 Override Logic

```rust
const MIN_SAMPLES_FOR_OVERRIDE: u32 = 20;

pub fn route_with_empirical(
    task_type: TaskType,
    budget: &RateBudget,
    performance: &PerformanceMatrix,
) -> RoutingDecision {
    // Start with heuristic
    let mut decision = route(task_type, budget);

    // Check if empirical data overrides the heuristic
    let candidates = performance.get_candidates(task_type);
    for candidate in candidates {
        if candidate.sample_count >= MIN_SAMPLES_FOR_OVERRIDE
            && candidate.success_rate > decision_success_rate(&decision, performance)
            && candidate.avg_cost <= decision_avg_cost(&decision, performance) * 1.5
        {
            // Empirical data says this model is better for this task type
            decision.model = candidate.model;
        }
    }

    decision
}
```

### 5.3 Learning Loop

After each session:
1. Record model, task_type, outcome, cost, duration to `model_performance` table
2. When a model-task pair reaches 20 samples, the empirical layer activates for that pair
3. The routing decision logs include which layer made the decision (heuristic vs empirical)
4. The evolution engine can observe routing effectiveness and propose routing table changes

## 6. External Model Dispatch

### 6.1 Codex

```rust
pub async fn dispatch_codex(prompt: &str, timeout: Duration) -> Result<ExternalResult> {
    let output = tokio::process::Command::new("/opt/homebrew/bin/codex")
        .arg("exec")
        .arg(prompt)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    let result = tokio::time::timeout(timeout, output.wait_with_output()).await??;

    Ok(ExternalResult {
        model: ExternalModel::Codex,
        output: String::from_utf8_lossy(&result.stdout).to_string(),
        exit_code: result.status.code().unwrap_or(-1),
        duration: start.elapsed(),
    })
}
```

### 6.2 Gemini

```rust
pub async fn dispatch_gemini(prompt: &str, timeout: Duration) -> Result<ExternalResult> {
    let output = tokio::process::Command::new("/opt/homebrew/bin/gemini")
        .arg("--model")
        .arg("gemini-3-pro-preview")
        .arg("-p")
        .arg(prompt)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    let result = tokio::time::timeout(timeout, output.wait_with_output()).await??;

    Ok(ExternalResult {
        model: ExternalModel::Gemini,
        output: String::from_utf8_lossy(&result.stdout).to_string(),
        exit_code: result.status.code().unwrap_or(-1),
        duration: start.elapsed(),
    })
}
```

### 6.3 Fallback Chain

```
Primary (external) -> Fallback (Claude)

Codex verification  -> Opus self-review
Gemini verification -> Opus self-review
Codex + Gemini cross-verify -> Opus self-review (single perspective)
```

External model unavailability is detected by: timeout (30s default), non-zero exit code, or empty output. On failure, the fallback is used immediately — no retry.

## 7. Verification Pipeline Integration

For important work, the orchestrator automatically schedules verification:

```
Phase: Pre-Implementation
  Task: "Review this plan before I start implementing"
  Route to: Codex (recommended) -> Opus fallback
  Gate: critical findings block implementation start

Phase: Post-Implementation (per logical unit)
  Task: "Review this implementation against the plan"
  Route to: Codex + Gemini (cross-verify, recommended) -> Opus fallback
  Gate: critical findings require fixes before next phase

Phase: Pre-Merge
  Task: "Final review of all changes"
  Route to: Codex (recommended) -> Opus fallback
  Gate: critical findings block merge
```

All gates are advisory when external models are unavailable — Opus self-review runs but cannot block (single-perspective review is inherently limited).

## 8. Cost Tracking

Every routing decision records:
- Model chosen (and why: heuristic vs empirical)
- Tokens consumed (from Claude Code's `total_cost_usd`)
- External model cost (estimated from output length)
- Whether fallback was used

This feeds both the rate budget manager and the evolution engine's model routing analysis.

## 9. Agent Teams Decision Logic

**CooperBench Warning**: Multi-agent collaboration reduces success by ~50% vs single-agent in empirical studies (94 papers, LLM-based MAS Design Patterns). Agent Teams should be the exception, not the default. Use only when: (a) work is genuinely parallelizable across independent files/modules, (b) no shared reasoning or coordination is needed, (c) budget allows the ~7x token cost. The orchestrator mediates all coordination — agents within a team should NOT communicate with each other directly.

Agent Teams (Opus 4.6 1M, experimental) use ~7x the tokens of a single session. Use only when:

```rust
pub fn should_use_agent_teams(task: &Task, budget: &RateBudget) -> bool {
    // Must be explicitly classified as AgentTeam task type
    if task.task_type != TaskType::AgentTeam {
        return false;
    }

    // Budget gate: must have >60% remaining (Agent Teams is expensive)
    if budget.remaining_percent() < 60.0 {
        return false;
    }

    // Complexity gate: task must involve 3+ files or modules
    if task.estimated_file_count < 3 {
        return false;
    }

    true
}
```

On budget constraint or complexity below threshold, fall back to single Opus session.
