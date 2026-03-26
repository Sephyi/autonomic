# Evolution Engine — Architecture Specification

**Status**: Draft
**Last updated**: 2026-03-25
**Research basis**: SICA, AgentDevel, GEA, DGM, DGM-Hyperagent, MARIA OS, Instar EvolutionManager
**Detailed paper analyses**: `../research/*.md`

## 1. Design Philosophy

Self-improvement converges only when outcomes are verifiable (SICA). Scaffold improvements help scaffold problems — hooks, rules, routing, memory — but cannot substitute for model capability (SICA: zero gain on reasoning tasks). The evolution engine targets the **configuration surface** of the orchestrator, not the model itself.

Three foundational insights from the research:

1. **Archive > Linear** (GEA, DGM): Maintaining diverse variants and evolving them as a group dramatically outperforms linear single-path improvement. GEA: SWE-bench 20% -> 71% in 30 iterations (archive) vs DGM: 20% -> 50% in 60 iterations (linear).
2. **Gate on regressions, not aggregate** (AgentDevel): A change that improves 5 cases but regresses 3 is fundamentally different from one that improves 5 with zero regressions. Flip-centered gating reduces bad releases from 14.8% to 3.1%.
3. **Bound the modifications** (MARIA OS): Without formal bounds, self-modification can cycle or diverge. A Lyapunov energy function that must strictly decrease per step guarantees convergence in N_max = floor(V(M_0) / epsilon) steps.

## 2. Configuration Surface

The evolution engine modifies the orchestrator's **configuration surface** — the set of files and settings that control behavior:

```rust
/// Everything the evolution engine can propose changes to.
pub struct ConfigSurface {
    /// Global CLAUDE.md rules (user-level, ~/.claude/CLAUDE.md sections)
    pub global_rules: Vec<Rule>,
    /// Project-specific CLAUDE.md rules
    pub project_rules: HashMap<ProjectId, Vec<Rule>>,
    /// Hook scripts (shell/python, registered in settings.json)
    pub hooks: Vec<HookDefinition>,
    /// Model routing table (task_type -> model_tier mapping)
    pub model_routing: ModelRoutingTable,
    /// Memory config (decay rates, token budgets, assembly triggers)
    pub memory_config: MemoryConfig,
    /// Scheduler config (job definitions, cron expressions, priority)
    pub scheduler_config: SchedulerConfig,
    /// Context assembly config (what to inject at session start/compaction)
    pub context_assembly: ContextAssemblyConfig,
}
```

**Formal representation** (from ARTEMIS): The configuration surface maps to `C = (P, T, M, Theta)` — Prompts (global_rules, project_rules), Tools (hooks), Memory (memory_config, context_assembly), Theta (numeric parameters in scheduler_config, model_routing thresholds, decay rates). Evolution uses semantic GA with LLM-ensemble mutation/crossover for NL components (P, T descriptions), and Bayesian optimization for numeric components (Theta). Hierarchical evaluation: cheap LLM scoring of proposals before expensive probe battery runs (10-36% improvement across four agent systems, ARTEMIS).

### 2.1 Modification Frontier (from MARIA OS)

The frontier defines what CAN vs CANNOT be modified. The frontier itself is frozen — the evolution engine cannot expand its own capabilities.

**Frozen (immutable from evolution)**:

- The modification frontier definition itself
- Safety predicates (command-guard blocklist, permission modes)
- The evolution engine's core loop logic
- The energy/convergence function
- Audit trail structure and integrity checks
- Git commit/tag mechanism
- Rate budget emergency reserve (5%)

Frozen files are enforced at the filesystem level (`chmod 444`), not just in logic. The watchdog process verifies permissions on every startup. This is Structure > Willpower applied to the evolution engine itself — prompt-level 'don't modify X' is insufficient (Rust self-evolving agent research).

**Modifiable (within bounds)**:

- Hook scripts: content, timing thresholds, patterns
- CLAUDE.md rules: add, remove, reword, reorder
- Model routing: task-type classification, tier assignment, thresholds
- Memory: decay rates, token budgets, assembly triggers, category weights
- Scheduler: job intervals, priority levels, model tiers per job
- Context assembly: what gets injected, token allocation per source

**Restricted (requires human approval regardless of governance level)**:

- Adding new safety exceptions
- Changing permission modes
- Modifying rate budget allocation ratios
- Enabling/disabling the evolution engine itself

## 3. Archive Architecture (from GEA + DGM)

### 3.1 The Variant Archive

Instead of one config with proposed sidecars, the system maintains an **append-only archive** of configuration variants:

```rust
pub struct VariantArchive {
    /// All variants ever created. Never pruned. Append-only.
    pub variants: Vec<ConfigVariant>,
    /// Currently deployed variant ID
    pub active_variant: VariantId,
    /// Performance + novelty scores per variant
    pub scores: HashMap<VariantId, VariantScore>,
}

pub struct ConfigVariant {
    pub id: VariantId,               // ULID, monotonically increasing
    pub parent_id: Option<VariantId>, // Which variant was this derived from?
    pub ancestors: Vec<VariantId>,    // Full lineage (for cross-pollination tracking)
    pub surface: ConfigSurface,       // The actual configuration
    pub creation_reason: String,      // What triggered this variant
    pub created_at: DateTime<Utc>,
    pub source_project: Option<ProjectId>, // Which project's experience seeded this
    pub probe_results: Option<ProbeResults>, // Test battery outcomes
    pub status: VariantStatus,        // Candidate | Testing | Active | Retired | Failed
}

pub struct VariantScore {
    pub performance: f64,    // Aggregate probe task success rate
    pub novelty: f64,        // KNN distance in capability space (from GEA)
    pub composite: f64,      // alpha * performance + (1-alpha) * novelty
    pub child_count: u32,    // How many children derived from this variant
    pub regression_rate: f64, // P2F / (passes + epsilon) from AgentDevel
}
```

### 3.2 Variant Selection (from GEA + DGM)

When the evolution engine needs a parent variant to derive a new one from:

```
Algorithm: SelectParent(archive, alpha=0.7)

0. Precondition: the seed variant (variant-0) MUST have probe_results
   initialized during bootstrapping. If no variants have probe_results,
   return variant-0 as default parent. (Gemini review fix)
1. For each variant v in archive where v.probe_results is Some:
   a. performance(v) = v.probe_results.success_rate
   b. novelty(v) = KNN_distance(v.capability_vector, archive, k=5)
   c. exploration_bonus(v) = 1.0 / (v.child_count + 1)  // DGM: inverse child count
   d. composite(v) = alpha * performance(v) + (1-alpha) * novelty(v) + 0.1 * exploration_bonus(v)
2. Select parent via softmax over composite scores (temperature=1.0)
3. Return selected variant
```

**Capability vector** (from GEA): A binary vector where each dimension represents a probe task. `1` = variant passes that probe, `0` = fails. KNN distance in this space measures how differently two variants behave — high distance = high novelty.

**Why novelty matters**: Without novelty pressure, the archive converges to local optima. With it, the system explores diverse improvement paths. GEA's best agent drew from 17 unique ancestors — it couldn't have found that path through linear improvement.

## 4. Evolution Cycle

### 4.1 Phase 1: DISCOVER (Continuous, Passive)

Every session contributes experience traces to a shared pool, regardless of project.

```rust
pub struct ExperienceTrace {
    pub session_id: String,
    pub project_id: ProjectId,
    pub variant_id: VariantId,      // Which config variant was active
    pub timestamp: DateTime<Utc>,
    pub duration_ms: u64,
    pub model_used: ModelTier,
    pub task_type: TaskType,
    pub outcome: TaskOutcome,        // Success | Failure | Partial
    pub tools_used: Vec<String>,
    pub hooks_triggered: Vec<HookTrace>,  // Which hooks fired, exit codes, timing
    pub rules_referenced: Vec<String>,    // Which CLAUDE.md rules were cited
    pub errors: Vec<ErrorTrace>,
    pub cost_usd: f64,
    pub compaction_count: u32,       // How many compactions occurred
}

pub struct HookTrace {
    pub hook_name: String,
    pub event: String,        // PreToolUse, PostToolUse, SessionStart, etc.
    pub exit_code: i32,
    pub duration_ms: u64,
    pub blocked: bool,        // Did it block an action?
}
```

Collection mechanism: PostToolUse and Stop hooks append structured JSON to `~/.autonomic/traces/<session_id>.jsonl`.

### 4.2 Phase 2: ANALYZE (Scheduled, Cross-Project)

A scheduled analysis session processes the experience trace pool. **Implementation-blind** (from AgentDevel): the analyzer receives only traces, scores, and a rubric — never the actual config content being evaluated.

```
Algorithm: AnalyzeTraces(traces, rubric)

Input:
  traces: last N experience traces across all projects
  rubric: evaluation criteria (hook effectiveness, rule compliance, routing accuracy, etc.)

Output:
  diagnostics: list of identified patterns with severity

Process:
1. Aggregate traces by variant_id
2. For each metric in rubric:
   a. Compute per-variant statistics (mean, variance, trend)
   b. Identify outliers (>2 sigma from population mean)
   c. Classify: improving, stable, declining, anomalous
3. Pattern detection:
   a. Hook never triggers in 30+ sessions -> "redundant_hook"
   b. Rule never referenced in 30+ sessions -> "unused_rule"
   c. Same error pattern 3+ times across sessions -> "recurring_error"
   d. Model X consistently 2x faster for task_type Y -> "routing_opportunity"
   e. Hook blocks >20% of actions -> "overly_aggressive_hook"
   f. Post-compaction metrics significantly worse -> "compaction_recovery_gap"
4. Return diagnostics with severity (critical/high/medium/low)
   DO NOT include the config content or propose fixes
   (diagnosis and repair are separate — AgentDevel principle)
```

**Uncertainty-guided selective adaptation** (from TT-SI): The analysis phase does not propose changes uniformly across all task categories. It computes failure rates per task type via Relative Softmax Scoring over probe results, and focuses evolution proposals on the highest-failure categories. TT-SI demonstrated +5.48% improvement with selective focus vs +1.04% with uniform adaptation — a 5x difference in evolution efficiency. In practice: if Rust async tasks fail 30% of the time but TypeScript tasks fail 5%, evolution proposals should overwhelmingly target the Rust async configuration.

**Cost**: Haiku-tier, <$0.05 per analysis run. Runs weekly by default, triggered earlier if critical patterns detected.

### 4.3 Phase 3: PROPOSE (From Diagnostics to Variants)

A separate session (can be different model tier) receives diagnostics and proposes config changes. This is the **repair** step, explicitly separated from diagnosis.

**Clarification: Blind Analyzer vs Sighted Proposer** (Gemini review finding): The **analyzer** (Phase 2) is implementation-blind — it sees only traces, scores, and rubric. The **proposer** (Phase 3) DOES have read access to the current `ConfigVariant` because it must generate valid diffs. The separation is intentional: the critic that identifies problems should not see the implementation (prevents rationalization), but the engineer that fixes problems must see the code. This mirrors AgentDevel's architecture where diagnosis and repair are separate roles.

```rust
pub struct EvolutionProposal {
    pub id: ProposalId,             // EVO-001, EVO-002, ...
    pub parent_variant: VariantId,  // Selected via SelectParent algorithm
    pub diagnostics: Vec<Diagnostic>, // What triggered this proposal
    pub changes: Vec<ConfigChange>,  // Exact diffs to the config surface
    pub rationale: String,           // Why these changes address the diagnostics
    pub expected_improvement: String, // Quantified prediction
    pub measurement_plan: MeasurementPlan, // How to verify effectiveness
    pub scope: ProposalScope,        // Safe | Restricted | Mixed
    pub created_at: DateTime<Utc>,
}

pub struct ConfigChange {
    pub target: ConfigTarget,        // Which part of the config surface
    pub operation: ChangeOp,         // Add | Remove | Modify | Reorder
    pub before: Option<String>,      // Previous content (for diff)
    pub after: String,               // New content
}

pub struct MeasurementPlan {
    pub metric: String,              // Which metric to track
    pub direction: Direction,        // Increase | Decrease
    pub threshold: f64,              // Minimum improvement to accept
    pub regression_limit: f64,       // Maximum P2F rate to accept (from AgentDevel)
    pub verification_windows: Vec<Duration>, // 1h, 24h, 72h (from MARIA OS)
}
```

### 4.4 Phase 4: TEST (Flip-Centered Gating)

Every proposal is tested against a **probe task battery** before deployment. The probe battery is project-specific: a set of representative tasks that exercise the configuration surface.

```
Algorithm: FlipCenteredGate(current_variant, proposed_variant, probe_battery)

Input:
  current: currently active ConfigVariant
  proposed: the new ConfigVariant from the proposal
  probes: list of ProbeTask (standardized test scenarios)

Output:
  decision: Accept | Reject
  flip_report: detailed P2P/P2F/F2P/F2F classification

Process:
1. Run probe_battery against current_variant -> results_current
2. Run probe_battery against proposed_variant -> results_proposed
3. For each probe task i:
   a. current_pass = results_current[i].success
   b. proposed_pass = results_proposed[i].success
   c. Classify:
      - P2P: current_pass AND proposed_pass     (stable pass)
      - P2F: current_pass AND NOT proposed_pass  (REGRESSION)
      - F2P: NOT current_pass AND proposed_pass  (improvement)
      - F2F: NOT current_pass AND NOT proposed_pass (stable fail)
4. Compute regression rate: rho_P2F = |P2F| / (|P2P| + |P2F| + epsilon)
5. Compute improvement rate: rho_F2P = |F2P| / (|F2F| + |F2P| + epsilon)
6. Gate decision (ALL must hold):
   a. rho_P2F < max_regression_rate (default: 0.02 = 2%)
   b. |F2P| > 0 OR proposal is cost-neutral  (must improve something or save cost)
   c. Intent alignment: F2P improvements correspond to proposal's stated rationale
   d. No critical probe tasks in P2F set (some probes are marked critical = zero-regression)
   e. Energy check: V(proposed) < V(current) - epsilon (MARIA OS convergence)
7. Return Accept if all gates pass, Reject otherwise
```

**Probe task design**: Each project defines 10-30 probe tasks covering:
- Compilation/build success
- Test suite pass rate
- Hook timing benchmarks
- Specific scenarios that have caused past failures
- Edge cases from the error trace history

Probe tasks themselves evolve — when a new failure mode is discovered, a probe task is added for it. Probe tasks are NEVER removed (only retired with a reason).

### 4.5 Phase 5: DEPLOY (Timed Verification)

Deployment uses MARIA OS's timed verification windows, not a simple "N sessions" check:

```
Algorithm: TimedDeploy(proposal, proposed_variant)

1. Create new variant in archive with status = Testing
2. Git commit: "evolution: testing EVO-{id}"
3. Git tag: evolution/evo-{id}-testing
4. Switch active_variant to proposed_variant

Verification Window 1 (1 hour):
  - Run probe battery again in production context
  - Check: no critical regressions
  - If FAIL: auto-rollback, mark variant as Failed, git tag: evo-{id}-rollback-1h

Verification Window 2 (24 hours):
  - Aggregate all session traces from the last 24h
  - Compare against pre-deployment baseline (last 7 days of traces)
  - Check: no metric decline >5%, no new error patterns
  - If FAIL: auto-rollback, mark variant as Failed, git tag: evo-{id}-rollback-24h

Verification Window 3 (72 hours):
  - Aggregate all session traces from the last 72h
  - Check: targeted metric improved as predicted
  - Check: no regression in any tracked metric >3%
  - If PASS: mark variant as Active, git tag: evo-{id}-deployed
  - If FAIL: auto-rollback, mark variant as Failed, git tag: evo-{id}-rollback-72h

On any rollback:
  - Switch active_variant back to previous
  - Log failure in learning registry with root cause analysis
  - The failed variant stays in the archive (DGM: never prune)
  - Create or update capability gap entry if underlying problem persists
```

### 4.6 Phase 6: CROSS-PROJECT EVOLUTION (from GEA)

This is the primary evolution mechanism, not a secondary feature.

```
Algorithm: GroupEvolutionRound(projects, archive, experience_pool)

Input:
  projects: all registered projects
  archive: the variant archive
  experience_pool: all experience traces from all projects

Process:
1. Aggregate experience traces across all projects
2. Reflection (Haiku or Sonnet tier):
   - Input: aggregated traces (implementation-blind — no config content)
   - Output: evolution directives (high-level guidance, not specific diffs)
   - Example: "Hook timing shows compaction-recovery takes >500ms in
     Rust projects due to cargo build check — consider lazy validation"
3. For each project p:
   a. Select parent variant from archive (SelectParent algorithm)
   b. Generate project-specific patches from shared directives
   c. Each project interprets directives in its own context
   d. Create new variant: inherits parent's config, applies patches
   e. Test via FlipCenteredGate
   f. If accepted: add to archive, deploy via TimedDeploy
   g. If rejected: add to archive as Failed (data point for future selection)
4. Update archive scores (performance + novelty for all variants)
```

**Why this works**: Project A might discover that a particular hook pattern eliminates a class of errors. The reflection module captures this as a directive. Project B interprets the directive in its own context — the specific hook code differs, but the pattern transfers. GEA showed improvements are **framework-level** and transfer across model families.

**Future: Tool Synthesis** (from MARIA OS SEAA): In later phases, cross-project evolution can go beyond modifying existing configuration to synthesizing entirely new tools. SEAA's 4-stage pipeline (Design -> Implement -> Validate in sandbox -> Register) achieved 87.2% first-attempt validation with 0 production rollbacks. When the evolution engine detects a recurring error pattern for which no existing hook exists, it could synthesize a new hook script, validate it in a sandboxed probe run, and register it — rather than only proposing modifications to existing hooks.

## 5. Energy Function and Convergence (from MARIA OS)

### 5.1 Energy Function

```
V(M) = w_regression * L_regression(M)
     + w_hook_overhead * L_hook_overhead(M)
     + w_unused_rules * L_unused_rules(M)
     + w_error_rate * L_error_rate(M)
     + w_routing_waste * L_routing_waste(M)
     + w_memory_staleness * L_memory_staleness(M)

where:
  L_regression(M) = average P2F rate across last K probe runs
  L_hook_overhead(M) = total hook execution time / session time
  L_unused_rules(M) = count of rules not referenced in N sessions / total rules
  L_error_rate(M) = recurring errors / total sessions
  L_routing_waste(M) = cost of suboptimal routing / cost of optimal routing
  L_memory_staleness(M) = average age of top-10 memory entries at assembly time
```

### 5.2 Convergence Guarantee

Each accepted modification must satisfy: `V(M_{t+1}) < V(M_t) - epsilon`

Upper bound on total modifications: `N_max = floor(V(M_0) / epsilon)`

With typical initial energy ~100 and epsilon = 0.5, this gives N_max = 200 modifications before the system has converged to a stable configuration. In practice, most systems converge in 50-100 modifications (MARIA OS observation).

After convergence, the evolution engine enters **maintenance mode**: it continues collecting traces and monitoring for drift, but does not propose new changes unless energy increases (indicating environmental change — new project, new model version, etc.).

## 6. Governance Profiles (from Instar + MARIA OS)

| Profile | Safe Changes | Restricted Changes | Analysis Mode | Evolution Rounds |
| --- | --- | --- | --- | --- |
| Cautious | Queue for review | Queue for review | Passive (log only) | Manual trigger only |
| Supervised | Auto-deploy via TimedDeploy | Queue for review | Contextual | Weekly, manual confirm |
| Collaborative | Auto-deploy via TimedDeploy | Recommend, auto-deploy after 48h | Proactive | Weekly, auto-trigger |
| Autonomous | Auto-deploy via TimedDeploy | Auto-deploy via TimedDeploy | Proactive + experimental | Continuous when budget allows |

**Rubber-stamp detection** (from Instar's TrustElevationTracker):
- Track approval latency and modification rate for queued proposals
- If 10+ consecutive approvals in <5 seconds each: suggest governance upgrade
- If acceptance rate >85% over rolling 20 proposals: surface upgrade recommendation
- If rejection rate >50%: suggest governance downgrade

## 7. Two-Phase Operation (from GEA)

### 7.1 Evolution Mode (R&D)

- Active when new projects are added or after significant environmental changes
- Higher token budget allocated to evolution (30% instead of 15%)
- Multiple concurrent variant evaluations
- Aggressive exploration (lower alpha = more novelty weight)
- Runs until energy function converges or N_max reached

### 7.2 Production Mode (Steady State)

- Active after convergence
- Single deployed variant, standard inference cost
- Evolution engine monitors for drift (energy increase)
- If energy increases >10% from converged value: switch back to Evolution Mode
- Minimal token cost (weekly Haiku analysis only)

## 8. Scope of Self-Improvement (Explicit Limits)

Based on SICA's finding that scaffold improvements cannot substitute for model capability:

**CAN improve** (scaffold-shaped):

- Hook effectiveness (timing, patterns, false positive rates)
- Rule compliance (which rules are followed, which are ignored)
- Model routing accuracy (which model is best for which task type)
- Memory relevance (which entries are useful, which are stale)
- Context assembly quality (what to inject, how much, in what order)
- Scheduler efficiency (job timing, priority allocation)
- Error recovery patterns (common failures and their fixes)

**CANNOT improve** (model-capability-shaped):

- Code architecture quality
- Reasoning depth or accuracy
- Planning sophistication
- Creative problem-solving
- Domain expertise
- Natural language understanding

The evolution engine should never propose changes that claim to improve model-capability-shaped outcomes. If diagnostics identify a model-capability limitation, it creates a **capability gap entry** (not an evolution proposal) noting that the limitation is model-bound and may resolve with future model updates.

## 9. Data Schema (PostgreSQL)

```sql
-- Variant archive
CREATE TABLE variants (
    id TEXT PRIMARY KEY,          -- ULID
    parent_id TEXT REFERENCES variants(id),
    surface_version INTEGER NOT NULL DEFAULT 1, -- Schema version for forward compat
    surface_json TEXT NOT NULL,   -- Serialized ConfigSurface (versioned, see note below)
    creation_reason TEXT NOT NULL,
    source_project TEXT,
    status TEXT NOT NULL DEFAULT 'candidate',
    -- CHECK(status IN ('candidate','testing','active','retired','failed'))
    created_at TEXT NOT NULL,     -- ISO 8601
    deployed_at TEXT,
    retired_at TEXT
);

-- NOTE (Gemini review fix): surface_json uses versioned serialization.
-- surface_version tracks the ConfigSurface schema version. When deserializing
-- old variants, the loader applies forward migrations (add missing fields with
-- defaults). Old variants are NEVER rewritten — the migration happens at read time.
-- This prevents archive corruption as the ConfigSurface struct evolves.

-- Variant ancestry (for cross-pollination tracking)
CREATE TABLE variant_ancestors (
    variant_id TEXT REFERENCES variants(id),
    ancestor_id TEXT REFERENCES variants(id),
    depth INTEGER NOT NULL,      -- 1 = parent, 2 = grandparent, etc.
    PRIMARY KEY (variant_id, ancestor_id)
);

-- Probe task definitions
CREATE TABLE probe_tasks (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    description TEXT NOT NULL,
    task_prompt TEXT NOT NULL,
    expected_outcome TEXT NOT NULL, -- JSON: what success looks like
    is_critical INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    retired_at TEXT               -- NULL = active
);

-- Probe results per variant
CREATE TABLE probe_results (
    variant_id TEXT REFERENCES variants(id),
    probe_id TEXT REFERENCES probe_tasks(id),
    passed INTEGER NOT NULL,      -- 0 or 1
    duration_ms INTEGER,
    cost_usd REAL,
    output_summary TEXT,
    run_at TEXT NOT NULL,
    PRIMARY KEY (variant_id, probe_id, run_at)
);

-- Flip classifications
CREATE TABLE flip_reports (
    proposal_id TEXT NOT NULL,
    probe_id TEXT REFERENCES probe_tasks(id),
    classification TEXT NOT NULL, -- P2P, P2F, F2P, F2F
    current_variant TEXT REFERENCES variants(id),
    proposed_variant TEXT REFERENCES variants(id),
    run_at TEXT NOT NULL
);

-- Experience traces
CREATE TABLE experience_traces (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    variant_id TEXT REFERENCES variants(id),
    model_used TEXT NOT NULL,
    task_type TEXT,
    outcome TEXT NOT NULL,
    duration_ms INTEGER,
    cost_usd REAL,
    compaction_count INTEGER DEFAULT 0,
    trace_json TEXT NOT NULL,     -- Full ExperienceTrace serialized
    created_at TEXT NOT NULL
);

-- Evolution proposals
CREATE TABLE proposals (
    id TEXT PRIMARY KEY,          -- EVO-001, etc.
    parent_variant TEXT REFERENCES variants(id),
    proposed_variant TEXT REFERENCES variants(id),
    diagnostics_json TEXT NOT NULL,
    changes_json TEXT NOT NULL,
    rationale TEXT NOT NULL,
    measurement_plan_json TEXT NOT NULL,
    scope TEXT NOT NULL,          -- safe, restricted, mixed
    status TEXT NOT NULL DEFAULT 'proposed',
    -- CHECK(status IN ('proposed','testing','deployed','rejected','rolled_back'))
    created_at TEXT NOT NULL,
    resolved_at TEXT
);

-- Learning registry
CREATE TABLE learnings (
    id TEXT PRIMARY KEY,          -- LRN-001, etc.
    category TEXT NOT NULL,
    content TEXT NOT NULL,
    source_proposal TEXT REFERENCES proposals(id),
    source_project TEXT,
    applied INTEGER NOT NULL DEFAULT 0,
    applied_to TEXT,              -- Which proposal applied this learning
    created_at TEXT NOT NULL
);

-- Capability gaps
CREATE TABLE capability_gaps (
    id TEXT PRIMARY KEY,          -- GAP-001, etc.
    severity TEXT NOT NULL,       -- critical, high, medium, low
    category TEXT NOT NULL,
    description TEXT NOT NULL,
    discovery_context TEXT,
    proposed_solution TEXT,
    status TEXT NOT NULL DEFAULT 'identified',
    -- CHECK(status IN ('identified','addressed','wont_fix','model_bound'))
    created_at TEXT NOT NULL,
    resolved_at TEXT
);

-- Energy function history (for convergence tracking)
CREATE TABLE energy_history (
    variant_id TEXT REFERENCES variants(id),
    energy REAL NOT NULL,
    components_json TEXT NOT NULL, -- Per-component breakdown
    measured_at TEXT NOT NULL
);
```

## 10. Implementation Notes

### 10.1 Probe Task Battery Cost

Each probe run costs tokens. With 20 probe tasks per project and 3 projects:
- Per variant evaluation: ~60 probe runs
- At Haiku tier (~$0.001/probe): ~$0.06 per variant evaluation
- At Sonnet tier (~$0.01/probe): ~$0.60 per variant evaluation
- Use Haiku for routine probes, Sonnet for critical probes only

### 10.2 Archive Growth

The archive is append-only but growth is bounded by evolution rate:
- At 1 variant/week (supervised mode): ~52 variants/year
- At 1 variant/day (autonomous mode): ~365 variants/year
- Each variant's ConfigSurface is ~10-50KB serialized
- Total archive: ~2-18MB/year — negligible

### 10.3 Git Storage

Every variant creation, deployment, and rollback is a git commit. The git history IS the audit trail. Tags provide fast navigation to important states.

### 10.4 Bootstrapping

On first run, the archive contains a single **seed variant** derived from the operator's existing configuration. This is variant-0, the root of all evolution. Its probe results establish the baseline that all future variants are measured against.

## 11. Watchdog Integration

The evolution engine operates within a two-process architecture (from Rust self-evolving agent research). The watchdog is a separate lightweight binary that:

1. Monitors daemon health via HTTP health endpoint
2. Tracks the last known-good git tag (updated after every successful 72h verification)
3. If the daemon crashes within 5 minutes of an evolution deployment: auto-reverts to last known-good tag and restarts
4. If the daemon crashes without recent evolution: normal restart (launchd KeepAlive)
5. Verifies filesystem permissions on frozen files at every startup

This prevents self-bricking: a daemon that evolves a broken configuration cannot permanently break itself. The watchdog is deliberately simple (<500 lines) and NEVER modified by the evolution engine — it is outside the modification frontier.
