OpenAI Codex v0.116.0 (research preview)
--------
workdir: /private/tmp/research
model: gpt-5.4
provider: openai
approval: never
sandbox: workspace-write [workdir, /tmp, $TMPDIR, /Users/sephyi/.codex/memories]
reasoning effort: xhigh
reasoning summaries: none
session id: 019d248c-05d1-7dc1-a193-55ac095c1e70
--------
user
Review the Autonomic project documentation for implementation readiness. The project is a Rust-based persistent, self-improving orchestrator for autonomous AI development.

Read ALL files in /private/tmp/research/PRD.md and /private/tmp/research/docs/architecture/ and /private/tmp/research/SOURCES.md

Then provide a structured review:
1. **Critical Issues** — Things that would cause implementation failure
2. **Design Concerns** — Potential problems worth reconsidering
3. **Missing Elements** — Gaps in coverage
4. **Strengths** — What's well-designed
5. **Specific Corrections** — Exact errors found (with file and section references)

Be thorough. Check algorithms, Rust types, SQL schemas, internal consistency between docs, and feasibility for a solo dev on macOS ARM.
mcp: github starting
mcp: fetch starting
mcp: filesystem starting
mcp: exa starting
mcp: context7 starting
mcp: mem0 starting
mcp: zai-mcp-server starting
mcp: zread starting
mcp: time starting
mcp: web-reader starting
mcp: web-search-prime starting
mcp: exa ready
mcp: context7 ready
mcp: time ready
mcp: fetch ready
mcp: github ready
mcp: zai-mcp-server ready
mcp: filesystem ready
mcp: mem0 ready
mcp: web-reader ready
mcp: zread ready
mcp: web-search-prime ready
mcp startup: ready: exa, context7, time, fetch, github, zai-mcp-server, filesystem, mem0, web-reader, zread, web-search-prime
codex
I’m reviewing the Autonomic docs for implementation readiness and will start by enumerating the exact files in scope, then read every requested document before checking consistency across algorithms, types, schemas, and platform assumptions.
exec
/bin/zsh -lc pwd in /private/tmp/research succeeded in 0ms:
/private/tmp/research
exec
/bin/zsh -lc 'rg --files /private/tmp/research/docs/architecture /private/tmp/research/PRD.md /private/tmp/research/SOURCES.md' in /private/tmp/research succeeded in 0ms:
/private/tmp/research/SOURCES.md
/private/tmp/research/PRD.md
/private/tmp/research/docs/architecture/session-management.md
/private/tmp/research/docs/architecture/memory-system.md
/private/tmp/research/docs/architecture/model-routing.md
/private/tmp/research/docs/architecture/hook-system.md
/private/tmp/research/docs/architecture/scheduler.md
/private/tmp/research/docs/architecture/state-management.md
/private/tmp/research/docs/architecture/evolution-engine.md
/private/tmp/research/docs/architecture/overview.md
codex
I’ve confirmed the full document set in scope. Next I’m reading each file and mapping sections so I can cross-check claims instead of reviewing each doc in isolation.
exec
/bin/zsh -lc 'wc -l /private/tmp/research/PRD.md /private/tmp/research/SOURCES.md /private/tmp/research/docs/architecture/*.md' in /private/tmp/research succeeded in 0ms:
     690 /private/tmp/research/PRD.md
     298 /private/tmp/research/SOURCES.md
     618 /private/tmp/research/docs/architecture/evolution-engine.md
    1427 /private/tmp/research/docs/architecture/hook-system.md
     426 /private/tmp/research/docs/architecture/memory-system.md
     409 /private/tmp/research/docs/architecture/model-routing.md
     294 /private/tmp/research/docs/architecture/overview.md
    1319 /private/tmp/research/docs/architecture/scheduler.md
    1290 /private/tmp/research/docs/architecture/session-management.md
    1007 /private/tmp/research/docs/architecture/state-management.md
    7778 total
exec
/bin/zsh -lc "for f in /private/tmp/research/PRD.md /private/tmp/research/SOURCES.md /private/tmp/research/docs/architecture/*.md; do printf '\\n===== %s =====\\n' \""'$f"; sed -n '"'1,260p' \""'$f"; done' in /private/tmp/research succeeded in 0ms:

===== /private/tmp/research/PRD.md =====
<!-- SPDX-FileCopyrightText: 2026 Sephyi <me@sephy.io> -->
<!-- SPDX-License-Identifier: LicenseRef-Proprietary -->

# Autonomic -- Product Requirements Document

**Version**: v0.3
**Date**: 2026-03-25
**Status**: Planning
**Author**: [Sephyi](https://github.com/Sephyi) + [Claude Opus 4.6](https://www.anthropic.com/news/claude-opus-4-6) + [Gemini 3 Pro](https://deepmind.google/technologies/gemini/) (reviewer)
**Edition**: Rust 2024 | **MSRV**: 1.94 | **Toolchain**: stable
**License**: LicenseRef-Proprietary | **REUSE compliant**
**Platform**: macOS 14+ (primary), Linux (secondary)

<details>
<summary>Changelog</summary>

| Version | Date | Summary |
| --- | --- | --- |
codex
I’ve confirmed the corpus is larger than a quick skim can cover. I’m indexing section headers and line ranges now so I can read every document in chunks with line references and cross-link inconsistencies precisely.
exec
/bin/zsh -lc "for f in /private/tmp/research/PRD.md /private/tmp/research/SOURCES.md /private/tmp/research/docs/architecture/*.md; do printf '%5s  %s\\n' \""'$(wc -l < "$f")" "$f"; done' in /private/tmp/research succeeded in 0ms:
     690  /private/tmp/research/PRD.md
     298  /private/tmp/research/SOURCES.md
     618  /private/tmp/research/docs/architecture/evolution-engine.md
    1427  /private/tmp/research/docs/architecture/hook-system.md
     426  /private/tmp/research/docs/architecture/memory-system.md
     409  /private/tmp/research/docs/architecture/model-routing.md
     294  /private/tmp/research/docs/architecture/overview.md
    1319  /private/tmp/research/docs/architecture/scheduler.md
    1290  /private/tmp/research/docs/architecture/session-management.md
    1007  /private/tmp/research/docs/architecture/state-management.md
exec
/bin/zsh -lc "for f in /private/tmp/research/PRD.md /private/tmp/research/SOURCES.md /private/tmp/research/docs/architecture/*.md; do printf '\\n===== %s =====\\n' \""'$f"; rg -n '"'"'^#{1,6} '"' \""'$f"; done' in /private/tmp/research succeeded in 0ms:

===== /private/tmp/research/PRD.md =====
4:# Autonomic -- Product Requirements Document
27:## 1. Vision
35:### 1.1 Core Principles
47:### 1.2 What This Is Not
55:### 1.3 Compatibility Policy
62:## 2. Competitive Landscape
64:### 2.1 Market Position
73:### 2.2 Unique Differentiators
83:### 2.3 Key Inspirations
104:## 3. Architecture
108:### 3.1 System Overview
130:### 3.2 Workspace Structure
151:### 3.3 Evolution Engine
194:### 3.4 Memory System
206:### 3.5 Session Management
212:### 3.6 Model Routing
218:### 3.7 Scheduling and Rate Budget
224:### 3.8 Hook System
tool filesystem.read_text_file({"path":"/private/tmp/research/docs/architecture/overview.md"})
filesystem.read_text_file({"path":"/private/tmp/research/docs/architecture/overview.md"}) failed in 3ms:
{
  "content": [
    {
      "type": "text",
      "text": "Access denied - path outside allowed directories: /private/tmp/research/docs/architecture/overview.md not in /Users/sephyi/Library/Mobile Documents/com~apple~CloudDocs/Development, /Users/sephyi/Library/Mobile Documents/com~apple~CloudDocs/Development"
    }
  ],
  "isError": true
}
codex
I’ve indexed the documents and the review will be cross-document, not file-by-file summary. Next I’m reading the architecture specs in full, then I’ll reconcile them against the PRD and source list before writing findings ordered by implementation risk.
Plan update
Structured review of the Autonomic documentation for implementation readiness.
  → Read all in-scope documents with line references
  • Cross-check algorithms, types, schemas, and platform assumptions for consistency and feasibility
  • Produce structured findings with exact file/section references
exec
/bin/zsh -lc "nl -ba /private/tmp/research/docs/architecture/memory-system.md | sed -n '1,460p'" in /private/tmp/researchexec
/bin/zsh -lc "nl -ba /private/tmp/research/docs/architecture/overview.md | sed -n '1,320p'" in /private/tmp/research succeeded in 0ms:
     1	# Memory System — Architecture Specification
     2	
     3	**Status**: Draft
     4	**Last updated**: 2026-03-25
     5	**Replaces**: Mem0 + Qdrant + Ollama (slow, imprecise, ignored in practice)
     6	
     7	## 1. Design Rationale
     8	
     9	### 1.1 Why Not Mem0/Qdrant/Ollama
    10	
    11	The operator's experience with Mem0:
    12	- Ollama embedding generation is slow (~200ms per entry), blocking the pipeline
    13	- Qdrant requires a separate server process (Podman container)
    14	- Vector similarity is imprecise for exact recall ("what was the decision about X?")
    15	- Claude Code's native MEMORY.md auto-memory was used instead, making Mem0 redundant
    16	- The system was "basically always ignored" in practice
    17	
    18	### 1.2 Design Principles
    19	
    20	1. **Zero external dependencies** — no separate server, no containers, no embedding models
 succeeded in 0ms:
     1	# Architecture Overview
     2	
     3	**Status**: Draft
     4	**Last updated**: 2026-03-25
     5	
     6	## 1. What Autonomic Is
     7	
     8	A persistent Rust daemon that replaces the human as the orchestration layer for AI-assisted software development. It manages multiple Claude Code instances across projects, routes tasks to optimal model tiers, enforces quality through structural hooks, accumulates knowledge across sessions, and evolves its own configuration based on measured outcomes.
     9	
    10	## 2. System Diagram
    11	
    12	```txt
    13	launchd
    14	  |
    15	  +-- autonomic-watchdog (~1MB, monitors daemon, crash rollback)
    16	  |
    17	  +-- autonomic daemon (Rust, axum, ~5MB RSS)
    18	  |
    19	  +------------------------------------------------------------+
    20	  |                     CORE SUBSYSTEMS                         |
exec
/bin/zsh -lc "nl -ba /private/tmp/research/docs/architecture/model-routing.md | sed -n '1,460p'" in /private/tmp/research succeeded in 0ms:
     1	# Model Routing — Architecture Specification
     2	
     3	**Status**: Draft
     4	**Last updated**: 2026-03-25
     5	
     6	## 1. Design Philosophy
     7	
     8	The operator currently makes every model routing decision manually. Autonomic automates this with a two-layer system:
     9	
    10	1. **Heuristic layer** (day 1): Rule-based routing from the operator's established patterns
    11	2. **Empirical layer** (after data): Performance matrix built from measured outcomes overrides heuristics
    12	
    13	Key constraint: external models (Codex, Gemini) are **recommended but never required**. The system is fully functional with Claude alone. Each external model has a fallback path.
    14	
    15	From Morph's 15-agent benchmark: "The same model scores 17 problems apart in different agents. The scaffolding matters more than the model." Model routing is important, but it's secondary to the hook/memory/evolution infrastructure.
    16	
    17	## 2. Available Models
    18	
    19	### 2.1 Claude Models (via Claude Code CLI)
    20	
exec
/bin/zsh -lc "nl -ba /private/tmp/research/docs/architecture/evolution-engine.md | sed -n '1,700p'" in /private/tmp/research succeeded in 0ms:
     1	# Evolution Engine — Architecture Specification
     2	
     3	**Status**: Draft
     4	**Last updated**: 2026-03-25
     5	**Research basis**: SICA, AgentDevel, GEA, DGM, DGM-Hyperagent, MARIA OS, Instar EvolutionManager
     6	**Detailed paper analyses**: `../research/*.md`
     7	
     8	## 1. Design Philosophy
     9	
    10	Self-improvement converges only when outcomes are verifiable (SICA). Scaffold improvements help scaffold problems — hooks, rules, routing, memory — but cannot substitute for model capability (SICA: zero gain on reasoning tasks). The evolution engine targets the **configuration surface** of the orchestrator, not the model itself.
    11	
    12	Three foundational insights from the research:
    13	
    14	1. **Archive > Linear** (GEA, DGM): Maintaining diverse variants and evolving them as a group dramatically outperforms linear single-path improvement. GEA: SWE-bench 20% -> 71% in 30 iterations (archive) vs DGM: 20% -> 50% in 60 iterations (linear).
    15	2. **Gate on regressions, not aggregate** (AgentDevel): A change that improves 5 cases but regresses 3 is fundamentally different from one that improves 5 with zero regressions. Flip-centered gating reduces bad releases from 14.8% to 3.1%.
    16	3. **Bound the modifications** (MARIA OS): Without formal bounds, self-modification can cycle or diverge. A Lyapunov energy function that must strictly decrease per step guarantees convergence in N_max = floor(V(M_0) / epsilon) steps.
    17	
    18	## 2. Configuration Surface
    19	
    20	The evolution engine modifies the orchestrator's **configuration surface** — the set of files and settings that control behavior:
exec
/bin/zsh -lc "nl -ba /private/tmp/research/docs/architecture/session-management.md | sed -n '341,700p'" in /private/tmp/researchexec
/bin/zsh -lc "nl -ba /private/tmp/research/docs/architecture/session-management.md | sed -n '1,340p'" in /private/tmp/researchexec
/bin/zsh -lc "nl -ba /private/tmp/research/docs/architecture/session-management.md | sed -n '701,1040p'" in /private/tmp/research succeeded in 0ms:
   341	fn classify_result(msg: &StreamMessage) -> SessionOutcome {
   342	    match msg.subtype.as_deref() {
   343	        Some("success") => SessionOutcome::Success {
   344	            result_text: msg.result.clone(),
   345	            cost_usd: msg.total_cost_usd.unwrap_or(0.0),
   346	            duration_ms: msg.duration_ms.unwrap_or(0),
   347	            claude_session_id: msg.session_id.clone(),
   348	        },
   349	        Some("error") => SessionOutcome::Error {
   350	            error: msg.error.clone().unwrap_or_else(|| "unknown error".into()),
   351	            cost_usd: msg.total_cost_usd.unwrap_or(0.0),
   352	        },
   353	        Some("max_turns") => SessionOutcome::MaxTurns {
   354	            result_text: msg.result.clone(),
   355	            cost_usd: msg.total_cost_usd.unwrap_or(0.0),
   356	            claude_session_id: msg.session_id.clone(),
   357	        },
   358	        other => SessionOutcome::Error {
   359	            error: format!("Unknown result subtype: {:?}", other),
   360	            cost_usd: msg.total_cost_usd.unwrap_or(0.0),
 succeeded in 0ms:
     1	# Session Management Architecture
     2	
     3	## 1. Overview and Design Rationale
     4	
     5	The `autonomic-session` crate owns the full lifecycle of Claude Code subprocess instances. It is the only component that spawns `claude` processes, parses their output, and reports status back to the orchestrator.
     6	
     7	### Design Goals
     8	
     9	| Goal | Mechanism |
    10	| --- | --- |
    11	| Structured observability | `--output-format stream-json` gives typed NDJSON on stdout |
    12	| Cost governance | Per-session and global budget tracking from `total_cost_usd` in result messages |
    13	| Parallel safety | Each session runs against an isolated git worktree |
    14	| Crash resilience | Sessions are resumable via `--resume <id>` after unexpected termination |
    15	| Nesting prevention | `CLAUDECODE=1` is stripped from the subprocess environment to allow spawning |
    16	| Least privilege | `--permission-mode acceptEdits` with explicit `--allowedTools` whitelist |
    17	
    18	### Non-Goals
    19	
    20	- The session manager does not decide *what* to run. Task decomposition and prompt construction live in the orchestrator layer above.
 succeeded in 0ms:
   701	    SessionConfig {
   702	        session_id: SessionId::new(),
   703	        prompt: continuation_prompt.to_string(),
   704	        continue_recent: true,
   705	        resume_session_id: None,
   706	        // Carry forward all other settings from the original config
   707	        working_dir: original.working_dir.clone(),
   708	        model: original.model.clone(),
   709	        system_prompt: original.system_prompt.clone(),
   710	        allowed_tools: original.allowed_tools.clone(),
   711	        timeout: original.timeout,
   712	        cost_budget_usd: original.cost_budget_usd,
   713	        extra_env: original.extra_env.clone(),
   714	        agent_teams: original.agent_teams,
   715	        worktree: original.worktree.clone(),
   716	    }
   717	}
   718	```
   719	
   720	**CRITICAL: Orphan Process Prevention** (Gemini review finding): If the daemon crashes or is killed (SIGKILL), child Claude Code processes are detached and continue running — burning rate budget silently. Mitigations:
exec
/bin/zsh -lc "nl -ba /private/tmp/research/docs/architecture/session-management.md | sed -n '1041,1320p'" in /private/tmp/research succeeded in 0ms:
  1041	                        // The process likely crashed.
  1042	                        break;
  1043	                    }
  1044	                }
  1045	            }
  1046	        }
  1047	    }
  1048	
  1049	    // Determine final state
  1050	    match final_outcome {
  1051	        Some(outcome) => SessionState::Completed {
  1052	            outcome,
  1053	            started_at,
  1054	            finished_at: Instant::now(),
  1055	        },
  1056	        None => {
  1057	            // No result message received; process crashed or was killed
  1058	            let status = child.wait().await.ok();
  1059	            let exit_code = status.and_then(|s| s.code());
  1060	            #[cfg(unix)]
codex
I’m through the core session/evolution docs. The next pass is the scheduler, hooks, and state layer, because most likely implementation blockers are emerging at the subsystem boundaries rather than inside any single module.
exec
/bin/zsh -lc "nl -ba /private/tmp/research/docs/architecture/scheduler.md | sed -n '361,760p'" in /private/tmp/researchexec
/bin/zsh -lc "nl -ba /private/tmp/research/docs/architecture/scheduler.md | sed -n '1,360p'" in /private/tmp/research succeeded in 0ms:
   361	    /// Returns overall usage as a fraction (0.0 to 1.0).
   362	    fn usage_fraction(&self) -> f64 {
   363	        if self.tokens_limit == 0 {
   364	            return 1.0;
   365	        }
   366	        self.tokens_used as f64 / self.tokens_limit as f64
   367	    }
   368	
   369	    /// Returns the token budget remaining for the scheduled subsystem.
   370	    fn scheduled_budget_remaining(&self) -> u64 {
   371	        let pct = self.allocations.get(&Subsystem::Scheduled).copied().unwrap_or(25.0);
   372	        let allocated = (self.tokens_limit as f64 * pct / 100.0) as u64;
   373	        // Estimate how many tokens scheduled jobs have used (tracked separately)
   374	        // For now, use overall usage as a conservative proxy
   375	        allocated.saturating_sub(self.tokens_used * allocated / self.tokens_limit.max(1))
   376	    }
   377	}
   378	
   379	impl QuotaTracker {
   380	    fn new() -> Self {
 succeeded in 0ms:
     1	# Scheduler
     2	
     3	The Autonomic scheduler runs cron-based recurring tasks with model-tier awareness and rate budget integration. It is responsible for health checks, evolution analysis, reflections, backups, and any user-defined periodic jobs. The scheduler is a subsystem of the orchestrator process, not a separate daemon.
     4	
     5	## Job Definitions
     6	
     7	Jobs are defined in `~/.autonomic/schedules/jobs.toml`. Each job specifies a cron expression, model tier, priority level, and the prompt/context to feed into a Claude Code session.
     8	
     9	### TOML Schema
    10	
    11	```toml
    12	# ~/.autonomic/schedules/jobs.toml
    13	
    14	# Global scheduler settings
    15	[scheduler]
    16	enabled = true
    17	timezone = "UTC"
    18	max_concurrent_jobs = 2
    19	default_timeout_seconds = 300
    20	
exec
/bin/zsh -lc "nl -ba /private/tmp/research/docs/architecture/scheduler.md | sed -n '761,1160p'" in /private/tmp/research succeeded in 0ms:
   761	    }
   762	
   763	    /// Adaptive scheduling: when a job is deferred, extend its next run
   764	    /// by one additional interval to reduce pressure.
   765	    fn adapt_interval(&mut self, idx: usize) {
   766	        let job = &mut self.jobs[idx];
   767	        let current_next = job.next_run;
   768	
   769	        // Find the occurrence after the current next_run
   770	        if let Ok(after_next) = job.cron.find_next_occurrence(&current_next, false) {
   771	            let interval = after_next - current_next;
   772	            job.next_run = current_next + interval;
   773	            tracing::info!(
   774	                job = %job.name,
   775	                next_run = %job.next_run,
   776	                "Adapted interval due to budget pressure"
   777	            );
   778	        }
   779	    }
   780	
exec
/bin/zsh -lc "nl -ba /private/tmp/research/docs/architecture/scheduler.md | sed -n '1161,1360p'" in /private/tmp/research succeeded in 0ms:
  1161	| Haiku | 20% | 40% | 10% | 30% |
  1162	| Sonnet | 50% | 25% | 15% | 10% |
  1163	| Opus | 60% | 15% | 20% | 5% |
  1164	
  1165	Rationale:
  1166	
  1167	- **Haiku**: Cheap tier, heavily used for scheduled monitoring and probes.
  1168	- **Sonnet**: Primary interactive tier, most budget reserved for user sessions.
  1169	- **Opus**: Expensive tier, reserved mainly for interactive decisions and evolution analysis.
  1170	
  1171	### Adaptive Scheduling Example
  1172	
  1173	When budget pressure is detected, the scheduler adjusts:
  1174	
  1175	1. **60-80% usage**: Low-priority jobs deferred to next interval.
  1176	2. **80-95% usage**: Normal-priority jobs also deferred. Low-priority jobs skip entirely until next window.
  1177	3. **95%+ usage**: Only critical jobs run. Scheduler enters "conservation mode" and logs a warning.
  1178	
  1179	```rust
  1180	fn compute_scheduling_mode(usage_fraction: f64) -> SchedulingMode {
exec
/bin/zsh -lc "nl -ba /private/tmp/research/docs/architecture/state-management.md | sed -n '1,360p'" in /private/tmp/researchexec
/bin/zsh -lc "nl -ba /private/tmp/research/docs/architecture/state-management.md | sed -n '361,720p'" in /private/tmp/research succeeded in 0ms:
     1	# State Management
     2	
     3	All mutable state for Autonomic lives in `~/.autonomic/`, which is itself a git repository. Every mutation to files in this directory results in an atomic git commit, providing a complete audit trail and point-in-time recovery for the entire orchestrator state.
     4	
     5	## Directory Structure
     6	
     7	```txt
     8	~/.autonomic/
     9	  config.toml              # User configuration (model preferences, API keys ref, thresholds)
    10	  state.sqlite             # Session state, metrics, rate budget tracking
    11	  memory.sqlite            # Long-term memory store (embeddings, retrieval)
    12	  projects/
    13	    <project-hash>/
    14	      context.toml         # Project-specific overrides, model tier preferences
    15	      history.sqlite       # Per-project session history (not git-committed, too large)
    16	  evolution/
    17	    strategies.toml        # Current active evolution strategies
    18	    changelog.toml         # Record of all strategy mutations with timestamps
    19	    pending/               # Staged evolution proposals awaiting approval
    20	  hooks/
 succeeded in 0ms:
   361	    Running,
   362	    Completed,
   363	    Failed,
   364	    Cancelled,
   365	}
   366	
   367	#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
   368	enum ModelTier {
   369	    Haiku,
   370	    Sonnet,
   371	    Opus,
   372	}
   373	
   374	#[derive(Debug, Clone, Serialize, Deserialize)]
   375	struct Metric {
   376	    id: i64,
   377	    timestamp: chrono::DateTime<chrono::Utc>,
   378	    session_id: Option<String>,
   379	    metric_name: String,
   380	    metric_value: f64,
exec
/bin/zsh -lc "nl -ba /private/tmp/research/docs/architecture/state-management.md | sed -n '721,1080p'" in /private/tmp/research succeeded in 0ms:
   721	        description: "Initial schema",
   722	        sql: include_str!("../migrations/001_initial.sql"),
   723	    },
   724	    // Future migrations appended here:
   725	    // Migration {
   726	    //     version: 2,
   727	    //     description: "Add session tags",
   728	    //     sql: include_str!("../migrations/002_session_tags.sql"),
   729	    // },
   730	];
   731	
   732	fn migrate(conn: &Connection) -> Result<(), StateError> {
   733	    let current: u32 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
   734	
   735	    if current > CURRENT_SCHEMA_VERSION {
   736	        return Err(StateError::FutureSchema {
   737	            found: current,
   738	            supported: CURRENT_SCHEMA_VERSION,
   739	        });
   740	    }
exec
/bin/zsh -lc "nl -ba /private/tmp/research/docs/architecture/hook-system.md | sed -n '341,700p'" in /private/tmp/researchexec
/bin/zsh -lc "nl -ba /private/tmp/research/docs/architecture/hook-system.md | sed -n '1,340p'" in /private/tmp/researchexec
/bin/zsh -lc "nl -ba /private/tmp/research/docs/architecture/hook-system.md | sed -n '701,1060p'" in /private/tmp/research succeeded in 0ms:
   341	echo "Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
   342	
   343	exit 0
   344	```
   345	
   346	#### Registration
   347	
   348	```json
   349	{
   350	  "hooks": {
   351	    "SessionStart": [
   352	      {
   353	        "matcher": ".*",
   354	        "hooks": [
   355	          {
   356	            "type": "command",
   357	            "command": "bash ~/.autonomic/hooks/compaction-recovery.sh",
   358	            "timeout": 10
   359	          }
   360	        ]
 succeeded in 0ms:
     1	# Autonomic Hook System Architecture
     2	
     3	## 1. Overview: Structure > Willpower
     4	
     5	A 10-line hook is a guarantee. A 1000-line prompt is a wish.
     6	
     7	Claude Code hooks provide structural enforcement of behaviors that natural language instructions cannot guarantee. No matter how explicit a CLAUDE.md instruction is, an LLM can drift, ignore, or "forget" it after compaction. A hook is code that runs outside the model's inference loop. It cannot be forgotten. It cannot be negotiated with. It either passes or it blocks.
     8	
     9	### Concrete contrast
    10	
    11	**Instruction-based approach (wish):**
    12	
    13	```markdown
    14	# CLAUDE.md
    15	IMPORTANT: Never run `rm -rf` on any directory. Always run rustfmt after editing Rust files.
    16	```
    17	
    18	The agent follows this 95% of the time. After compaction, it drops to 60%. Under complex multi-step reasoning, it occasionally "decides" the formatting step is unnecessary. One `rm -rf /` is all it takes.
    19	
    20	**Hook-based approach (guarantee):**
 succeeded in 0ms:
   701	        print(f"command-guard: failed to parse stdin: {e}", file=sys.stderr)
   702	        sys.exit(1)
   703	
   704	    tool_name = input_data.get("tool_name", "")
   705	    if tool_name != "Bash":
   706	        sys.exit(0)
   707	
   708	    command = input_data.get("tool_input", {}).get("command", "")
   709	    if not command:
   710	        sys.exit(0)
   711	
   712	    result = check_command(command)
   713	    if result is None:
   714	        sys.exit(0)
   715	
   716	    message, level = result
   717	
   718	    # Emit structured deny output
   719	    output = {
   720	        "hookSpecificOutput": {
exec
/bin/zsh -lc 'nl -ba /private/tmp/research/docs/architecture/PRD.md 2>/dev/null || true' in /private/tmp/researchexec
/bin/zsh -lc "nl -ba /private/tmp/research/docs/architecture/hook-system.md | sed -n '1061,1460p'" in /private/tmp/research succeeded in 0ms:

 succeeded in 0ms:
  1061	
  1062	```txt
  1063	autonomic uninstall <project-dir>
  1064	  │
  1065	  ├─ 1. Remove Autonomic hook entries from <project>/.claude/settings.json
  1066	  ├─ 2. Remove <project>/.autonomic/ directory
  1067	  └─ 3. Optionally remove global hooks (--global flag)
  1068	```
  1069	
  1070	## 7. Hook Evolution
  1071	
  1072	The evolution engine is Autonomic's mechanism for self-improvement. It analyzes experience traces (collected by session-metrics.sh and other instrumentation) to identify recurring failure patterns, then proposes new hooks to prevent those failures structurally.
  1073	
  1074	### Evolution pipeline
  1075	
  1076	```txt
  1077	Experience Traces (JSONL)
  1078	  │
  1079	  ├─ Pattern Detection
  1080	  │   Scan traces for recurring errors, repeated tool blocks,
exec
/bin/zsh -lc "nl -ba /private/tmp/research/PRD.md | sed -n '1,240p'" in /private/tmp/research succeeded in 0ms:
     1	<!-- SPDX-FileCopyrightText: 2026 Sephyi <me@sephy.io> -->
     2	<!-- SPDX-License-Identifier: LicenseRef-Proprietary -->
     3	
     4	# Autonomic -- Product Requirements Document
     5	
     6	**Version**: v0.3
     7	**Date**: 2026-03-25
     8	**Status**: Planning
     9	**Author**: [Sephyi](https://github.com/Sephyi) + [Claude Opus 4.6](https://www.anthropic.com/news/claude-opus-4-6) + [Gemini 3 Pro](https://deepmind.google/technologies/gemini/) (reviewer)
    10	**Edition**: Rust 2024 | **MSRV**: 1.94 | **Toolchain**: stable
    11	**License**: LicenseRef-Proprietary | **REUSE compliant**
    12	**Platform**: macOS 14+ (primary), Linux (secondary)
    13	
    14	<details>
    15	<summary>Changelog</summary>
    16	
    17	| Version | Date | Summary |
    18	| --- | --- | --- |
    19	| 0.3 | 2026-03-25 | Second research wave: ARTEMIS, TT-SI, MAS design patterns, Agent Skills Standard, ClaudeClaw, Clawith, Rust self-evolving agent, MARIA OS SEAA, MAS Orchestration Survey, Agyn, Awesome AI Agents 2026 (9 additional research docs, 197KB total). Key additions: daemon+watchdog two-process architecture, filesystem-level modification frontier, 6 trigger types beyond cron, SKILL.md as capability format, ARTEMIS config formalization, uncertainty-guided selective adaptation, CooperBench agent collaboration warning, system prompt re-injection on --resume constraint. 22 files, 560KB total documentation. |
    20	| 0.2 | 2026-03-25 | Major revision after full paper analysis (SICA, AgentDevel, GEA, DGM, DGM-Hyperagent, MARIA OS, Godel Agent — all read as full HTML from arxiv). Evolution engine redesigned: linear sidecar replaced by archive-based group evolution (GEA), flip-centered gating (AgentDevel), Lyapunov convergence bounds (MARIA OS), timed verification windows, implementation-blind analysis, cross-project evolution as primary mechanism. Architecture docs created (7 files, 218KB). Research docs created (5 files, 108KB). |
exec
/bin/zsh -lc "nl -ba /private/tmp/research/PRD.md | sed -n '241,480p'" in /private/tmp/research succeeded in 0ms:
   241	
   242	#### FR-001: Daemon Process with Watchdog
   243	
   244	**Priority**: P0 | **Phase**: 1
   245	
   246	Two-process architecture: daemon (main binary) + watchdog (lightweight monitor). Both managed by launchd. Watchdog prevents self-bricking from evolution failures.
   247	
   248	**Acceptance Criteria**:
   249	- [ ] Daemon binary starts via launchd Launch Agent plist
   250	- [ ] Watchdog binary starts via separate launchd plist, monitors daemon health
   251	- [ ] Watchdog auto-reverts to last known-good git tag if daemon crashes after evolution deployment
   252	- [ ] Daemon auto-restarts within 10s of crash (KeepAlive)
   253	- [ ] Structured tracing to `~/.autonomic/logs/daemon.log`
   254	- [ ] HTTP API on localhost (axum) for CLI communication
   255	- [ ] PID file at `~/.autonomic/daemon.pid`
   256	- [ ] Graceful shutdown on SIGTERM (drain active sessions)
   257	- [ ] Frozen files enforced at filesystem level (`chmod 444`) — watchdog verifies on startup
   258	
   259	#### FR-002: Project Registry
   260	
exec
/bin/zsh -lc "nl -ba /private/tmp/research/PRD.md | sed -n '481,760p'" in /private/tmp/research succeeded in 0ms:
   481	Staged deployment with auto-rollback at each verification window.
   482	
   483	**Acceptance Criteria**:
   484	- [ ] Window 1 (1h): run probe battery in production context
   485	- [ ] Window 2 (24h): aggregate session traces, compare to 7-day baseline
   486	- [ ] Window 3 (72h): check targeted metric improvement, no regressions > 3%
   487	- [ ] Auto-rollback at any failing window
   488	- [ ] Git tag on each stage: `evo-{id}-testing`, `evo-{id}-deployed`, `evo-{id}-rollback-{window}`
   489	
   490	#### FR-019: Energy Function and Convergence
   491	
   492	**Priority**: P1 | **Phase**: 4
   493	
   494	Lyapunov energy tracking to guarantee convergence.
   495	
   496	**Acceptance Criteria**:
   497	- [ ] Energy function: weighted sum of loss metrics (regression, hook overhead, unused rules, error rate, routing waste, memory staleness)
   498	- [ ] Each accepted modification must decrease energy by >= epsilon
   499	- [ ] `energy_history` table tracks energy per variant
   500	- [ ] System enters maintenance mode after convergence (monitor for drift only)
exec
/bin/zsh -lc "nl -ba /private/tmp/research/SOURCES.md | sed -n '1,340p'" in /private/tmp/research succeeded in 0ms:
     1	# Autonomic Research Sources
     2	
     3	**Date**: 2026-03-25
     4	**Research method**: 11 parallel research agents + direct codebase analysis
     5	
     6	## Primary Sources (Direct Code Analysis)
     7	
     8	### Instar
     9	
    10	- **Repository**: https://github.com/JKHeadley/instar
    11	- **Version analyzed**: v0.24.2
    12	- **License**: MIT
    13	- **Author**: JKHeadley (SageMindAI)
    14	- **Impact**: Core architectural inspiration -- "Structure > Willpower" principle, compaction recovery hooks, sidecar mutation pattern, evolution engine (EvolutionManager, AutonomousEvolution, AutonomyProfileManager, TrustElevationTracker), graduated governance, LLM-supervised execution tiers, token-budgeted context assembly with decay, Playbook system, safety architecture (ExternalOperationGate, AdaptiveTrust), memory architecture (SemanticMemory, TopicMemory, WorkingMemoryAssembler)
    15	- **Documentation site**: https://instar.sh
    16	- **Files read**: README.md, CLAUDE.md, package.json, `.claude/settings.json`, all hook templates (`src/templates/hooks/*`), scaffold templates, all skills (`skills/*/SKILL.md`), `docs/LLM-SUPERVISED-EXECUTION.md`, `docs/CURRENT-STATUS.md`, `src/data/http-hook-templates.ts`, `src/core/EvolutionManager.ts`, `src/core/AutonomousEvolution.ts`, `src/core/AutonomyProfileManager.ts`, `src/core/AdaptiveTrust.ts`, `src/core/TrustElevationTracker.ts`, `src/core/AdaptationValidator.ts`, `src/core/ExternalOperationGate.ts`, `src/core/TrustRecovery.ts`, `src/core/RelationshipManager.ts`, `src/memory/*`, `src/knowledge/*`, `src/security/*`, `src/monitoring/*`, `playbook-scripts/*`, `docs/PROP-memory-architecture.md`, `docs/PLAYBOOK-*.md`, `docs/THREADLINE*.md`, `docs/UX-AND-AGENT-AGENCY-STANDARD.md`
    17	
    18	### OpenClaw
    19	
    20	- **Repository**: https://github.com/openclaw/openclaw
codex
I’ve finished the full document read. I’m doing one last consistency pass now against the highest-risk interfaces: Claude CLI contract, scheduler/state boundaries, hook storage/layout, and anything that claims atomic recovery.
exec
/bin/zsh -lc "which codex && codex --help | sed -n '1,220p'" in /private/tmp/research succeeded in 0ms:
/opt/homebrew/bin/codex
WARNING: proceeding, even though we could not update PATH: Operation not permitted (os error 1)
Codex CLI

If no subcommand is specified, options will be forwarded to the interactive CLI.

Usage: codex [OPTIONS] [PROMPT]
       codex [OPTIONS] <COMMAND> [ARGS]

Commands:
  exec        Run Codex non-interactively [aliases: e]
  review      Run a code review non-interactively
  login       Manage login
  logout      Remove stored authentication credentials
  mcp         Manage external MCP servers for Codex
  mcp-server  Start Codex as an MCP server (stdio)
  app-server  [experimental] Run the app server or related tooling
  app         Launch the Codex desktop app (downloads the macOS installer if missing)
  completion  Generate shell completion scripts
  sandbox     Run commands within a Codex-provided sandbox
exec
/bin/zsh -lc "which gemini && gemini --help | sed -n '1,220p'" in /private/tmp/researchexec
/bin/zsh -lc "which claude && claude --help | sed -n '1,220p'" in /private/tmp/research succeeded in 395ms:
Usage: claude [options] [command] [prompt]

Claude Code - starts an interactive session by default, use -p/--print for
non-interactive output

Arguments:
  prompt                                            Your prompt

Options:
  --add-dir <directories...>                        Additional directories to allow tool access to
  --agent <agent>                                   Agent for the current session. Overrides the 'agent' setting.
  --agents <json>                                   JSON object defining custom agents (e.g. '{"reviewer": {"description": "Reviews code", "prompt": "You are a code reviewer"}}')
  --allow-dangerously-skip-permissions              Enable bypassing all permission checks as an option, without it being enabled by default. Recommended only for sandboxes with no internet access.
  --allowedTools, --allowed-tools <tools...>        Comma or space-separated list of tool names to allow (e.g. "Bash(git:*) Edit")
  --append-system-prompt <prompt>                   Append a system prompt to the default system prompt
  --bare                                            Minimal mode: skip hooks, LSP, plugin sync, attribution, auto-memory, background prefetches, keychain reads, and CLAUDE.md auto-discovery. Sets CLAUDE_CODE_SIMPLE=1. Anthropic auth is strictly ANTHROPIC_API_KEY or apiKeyHelper via --settings (OAuth and keychain are never read). 3P providers (Bedrock/Vertex/Foundry) use their own credentials. Skills still resolve via /skill-name. Explicitly provide context via: --system-prompt[-file], --append-system-prompt[-file], --add-dir (CLAUDE.md dirs), --mcp-config, --settings, --agents, --plugin-dir.
  --betas <betas...>                                Beta headers to include in API requests (API key users only)
  --brief                                           Enable SendUserMessage tool for agent-to-user communication
  --chrome                                          Enable Claude in Chrome integration
  -c, --continue                                    Continue the most recent conversation in the current directory
exec
/bin/zsh -lc "(command -v gtimeout >/dev/null && gtimeout 5 gemini --help || timeout 5 gemini --help) | sed -n '1,220p'" in /private/tmp/research succeeded in 9.92s:

codex
I’ve identified several cross-document mismatches. I’m confirming the exact line references now so the review can separate hard blockers from softer design tradeoffs.
exec
/bin/zsh -lc "rg -n \"--prompt|max-tokens|Total tokens:|append-system-prompt|system prompt|SessionId\\(|Uuid::now_v7|ulid|AutoномicConfig|state_db.execute|never writes to SQLite directly|init_state_repo|Repository::open\\(|remove_untracked|edition 2021|L2|override\" /private/tmp/research/PRD.md /private/tmp/research/docs/architecture/*.md" in /private/tmp/research exited 2 in 0ms:
rg: unrecognized flag --prompt|max-tokens|Total tokens:|append-system-prompt|system prompt|SessionId\(|Uuid::now_v7|ulid|AutoномicConfig|state_db.execute|never writes to SQLite directly|init_state_repo|Repository::open\(|remove_untracked|edition 2021|L2|override
exec
/bin/zsh -lc 'rg -n "15% orchestrator|55% active work|15% scheduled|10% external|5% emergency|budget allocation" /private/tmp/research/PRD.md /private/tmp/research/docs/architecture/*.md' in /private/tmp/researchexec
/bin/zsh -lc 'rg -n "memory/projects|MEMORY.md|memory.sqlite|jsonl|experience_traces|current-task.json|project-specific MEMORY|Relevant Memory|memory store" /private/tmp/research/docs/architecture/memory-system.md /private/tmp/research/docs/architecture/hook-system.md /private/tmp/research/docs/architecture/evolution-engine.md /private/tmp/research/PRD.md' in /private/tmp/research succeeded in 0ms:
/private/tmp/research/docs/architecture/evolution-engine.md:73:- Modifying rate budget allocation ratios
/private/tmp/research/docs/architecture/scheduler.md:95:description = "Adjust budget allocations based on usage patterns"
/private/tmp/research/docs/architecture/scheduler.md:1157:Default budget allocation across subsystems (configurable in `state.sqlite` `budget_allocations` table):
/private/tmp/research/PRD.md:222:Six trigger types (adapted from Clawith's Aware system): `cron` (recurring schedule via croner), `once` (one-time future execution), `interval` (every N minutes/hours), `poll` (check condition, act if true), `on_message` (react to incoming request), `webhook` (external HTTP trigger). Priority levels (critical/normal/low). Model tier per job. Rate budget: 15% orchestrator, 55% active work, 15% scheduled, 10% external, 5% emergency reserve. Adaptive: degrade gracefully when budget is tight.
 succeeded in 0ms:
/private/tmp/research/docs/architecture/memory-system.md:15:- Claude Code's native MEMORY.md auto-memory was used instead, making Mem0 redundant
/private/tmp/research/docs/architecture/memory-system.md:25:6. **Bidirectional sync with MEMORY.md** — Claude Code's native auto-memory is ingested; the orchestrator generates project-specific MEMORY.md from the store
/private/tmp/research/docs/architecture/memory-system.md:31:Single SQLite database at `~/.autonomic/memory.sqlite` in WAL mode.
/private/tmp/research/docs/architecture/memory-system.md:267:## 6. MEMORY.md Bidirectional Sync
/private/tmp/research/docs/architecture/memory-system.md:269:### 6.1 Store -> MEMORY.md (Generation)
/private/tmp/research/docs/architecture/memory-system.md:271:For each registered project, the orchestrator generates a project-specific MEMORY.md:
/private/tmp/research/docs/architecture/memory-system.md:298:5. Write to {project_dir}/.claude/MEMORY.md (atomic write via tmp + rename)
/private/tmp/research/docs/architecture/memory-system.md:303:### 6.2 MEMORY.md -> Store (Ingestion)
/private/tmp/research/docs/architecture/memory-system.md:305:Claude Code's auto-memory writes to MEMORY.md. The orchestrator ingests these:
/private/tmp/research/docs/architecture/memory-system.md:310:1. Read current MEMORY.md content
/private/tmp/research/docs/architecture/memory-system.md:311:2. Diff against last known content (stored in memory.sqlite metadata table)
/private/tmp/research/docs/architecture/memory-system.md:329:The SQLite store is the **source of truth**. MEMORY.md is a projection.
/private/tmp/research/docs/architecture/memory-system.md:331:- Orchestrator writes to MEMORY.md -> Claude Code reads it
/private/tmp/research/docs/architecture/memory-system.md:332:- Claude Code's auto-memory appends to MEMORY.md -> orchestrator ingests additions
/private/tmp/research/docs/architecture/memory-system.md:413:autonomic memory sync <project>        # Force bidirectional sync with MEMORY.md
/private/tmp/research/docs/architecture/memory-system.md:424:| MEMORY.md generation | < 20ms | Query + render + atomic write |
/private/tmp/research/docs/architecture/memory-system.md:425:| MEMORY.md ingestion | < 50ms | Read + diff + parse + INSERTs |
/private/tmp/research/docs/architecture/evolution-engine.md:169:Collection mechanism: PostToolUse and Stop hooks append structured JSON to `~/.autonomic/traces/<session_id>.jsonl`.
/private/tmp/research/docs/architecture/evolution-engine.md:516:CREATE TABLE experience_traces (
/private/tmp/research/PRD.md:204:**MEMORY.md sync**: Bidirectional. Orchestrator generates per-project MEMORY.md from store. Claude Code auto-memory writes ingested back. SQLite is source of truth.
exec
/bin/zsh -lc 'rg -n -e "--prompt" -e "max-tokens" -e "Total tokens:" -e "append-system-prompt" -e "system prompt" /private/tmp/research/PRD.md /private/tmp/research/docs/architecture/*.md' in /private/tmp/research succeeded in 0ms:
/private/tmp/research/docs/architecture/overview.md:217:| DEC-018 | System prompt re-injection on resume | --append-system-prompt doesn't persist | ClaudeClaw source analysis |
/private/tmp/research/docs/architecture/scheduler.md:898:        cmd.arg(format!("--max-tokens={}", max_tokens));
/private/tmp/research/docs/architecture/scheduler.md:903:    cmd.arg("--prompt").arg(&prompt);
/private/tmp/research/docs/architecture/scheduler.md:973:    // e.g., "Total tokens: 1234"
/private/tmp/research/docs/architecture/scheduler.md:975:        if let Some(rest) = line.strip_prefix("Total tokens: ") {
/private/tmp/research/docs/architecture/session-management.md:54:    /// Optional system prompt override. Passed via `--system-prompt`.
/private/tmp/research/docs/architecture/session-management.md:55:    /// When `None`, Claude Code uses its built-in system prompt.
/private/tmp/research/docs/architecture/session-management.md:726:**CRITICAL: System prompt does NOT persist across --resume** (discovered from ClaudeClaw source code analysis). The `--append-system-prompt` flag does not persist across session resumes. The orchestrator MUST re-inject the full system context (identity, project context, memory assembly) on every `--resume` call. This is not optional — without re-injection, resumed sessions lose all orchestrator context.
/private/tmp/research/PRD.md:19:| 0.3 | 2026-03-25 | Second research wave: ARTEMIS, TT-SI, MAS design patterns, Agent Skills Standard, ClaudeClaw, Clawith, Rust self-evolving agent, MARIA OS SEAA, MAS Orchestration Survey, Agyn, Awesome AI Agents 2026 (9 additional research docs, 197KB total). Key additions: daemon+watchdog two-process architecture, filesystem-level modification frontier, 6 trigger types beyond cron, SKILL.md as capability format, ARTEMIS config formalization, uncertainty-guided selective adaptation, CooperBench agent collaboration warning, system prompt re-injection on --resume constraint. 22 files, 560KB total documentation. |
/private/tmp/research/PRD.md:188:**Config Space Formalization** (from ARTEMIS): The configuration surface is formalized as `C = (P, T, M, Theta)` — Prompts (CLAUDE.md rules, system prompts), Tools (hooks, scripts), Memory (decay rates, assembly config), and Theta (numeric parameters: token budgets, thresholds, intervals). Evolution uses semantic GA with LLM-ensemble mutation/crossover for NL components (P), and standard optimization for numeric components (Theta). Hierarchical evaluation: cheap LLM scoring before expensive probe battery runs.
/private/tmp/research/PRD.md:210:Claude Code spawned via `tokio::process::Command` with `--print --output-format stream-json`. Completion via `message.type == "result"`. Cost via `total_cost_usd`. `CLAUDECODE=1` filtered from env. **Critical constraint** (discovered from ClaudeClaw source): system prompt does NOT persist across `--resume` calls — the orchestrator must re-inject system context on every session resume. Agent Teams (Opus 4.6 1M) available for complex tasks with cost threshold gate (~7x tokens). **CooperBench warning**: agents achieve ~50% lower success collaborating vs solo — Agent Teams should be used only for truly parallelizable work with orchestrator-mediated coordination, never for collaborative reasoning.
/private/tmp/research/PRD.md:285:- [ ] Re-inject system prompt on every `--resume` call (it does NOT persist across resumes — ClaudeClaw finding)
/private/tmp/research/PRD.md:685:| DEC-018 | System prompt re-injected on every --resume | ClaudeClaw source code confirms --append-system-prompt does NOT persist across resumes. Must re-inject context explicitly. | 2026-03-25 |
exec
/bin/zsh -lc "rg -n -e \"SessionId\\(\" -e \"Uuid::now_v7\" -e \"ulid\" -e \"AutoномicConfig\" -e \"state_db.execute\" -e \"never writes to SQLite directly\" -e \"init_state_repo\" -e \"Repository::open\\(\" -e \"remove_untracked\" -e \"edition 2021\" -e \"L2\" -e \"override\" /private/tmp/research/PRD.md /private/tmp/research/docs/architecture/*.md" in /private/tmp/research succeeded in 0ms:
/private/tmp/research/docs/architecture/state-management.md:14:      context.toml         # Project-specific overrides, model tier preferences
/private/tmp/research/docs/architecture/state-management.md:108:fn init_state_repo(autonomic_dir: &Path) -> Result<Repository, git2::Error> {
/private/tmp/research/docs/architecture/state-management.md:110:        Repository::open(autonomic_dir)
/private/tmp/research/docs/architecture/state-management.md:519:            .remove_untracked(true),
/private/tmp/research/docs/architecture/state-management.md:816:        let repo = Repository::open(autonomic_dir)?;
/private/tmp/research/docs/architecture/state-management.md:839:        mutate: impl FnOnce(&mut AutoномicConfig) -> Result<(), StateError>,
/private/tmp/research/docs/architecture/state-management.md:872:        self.state_db.execute(
/private/tmp/research/docs/architecture/state-management.md:890:        self.state_db.execute(
/private/tmp/research/docs/architecture/session-management.md:54:    /// Optional system prompt override. Passed via `--system-prompt`.
/private/tmp/research/docs/architecture/session-management.md:392:pub struct SessionId(pub uuid::Uuid);
/private/tmp/research/docs/architecture/session-management.md:396:        Self(uuid::Uuid::now_v7())
/private/tmp/research/docs/architecture/scheduler.md:21:# Built-in jobs ship with Autonomic. Users can override any field.
/private/tmp/research/docs/architecture/scheduler.md:825:        self.state.state_db.execute(
/private/tmp/research/docs/architecture/scheduler.md:1319:The scheduler never writes to SQLite directly; it always goes through the `StateManager` API to maintain the single-writer invariant.
/private/tmp/research/docs/architecture/model-routing.md:11:2. **Empirical layer** (after data): Performance matrix built from measured outcomes overrides heuristics
/private/tmp/research/docs/architecture/model-routing.md:265:    // Check if empirical data overrides the heuristic
/private/tmp/research/PRD.md:216:Heuristic layer (day 1) + empirical layer (after 20 samples per model-task pair). External models (Codex, Gemini) recommended for verification, fallback to Opus self-review. Performance matrix built from measured outcomes, overrides heuristics when data is sufficient.
/private/tmp/research/PRD.md:375:- [ ] Two levels: L1 (block + ask) and L2 (self-verify prompt)
/private/tmp/research/PRD.md:643:| IDs | ulid | Sortable unique identifiers |
/private/tmp/research/docs/architecture/overview.md:239:| ID generation | ulid | latest | Sortable unique IDs |
exec
/bin/zsh -lc 'rg -n "validate_worktree|worktree path|claude_binary|cost_budget_usd|BudgetExceeded|max-budget-usd|can_afford|remaining_percent|classification.*check" /private/tmp/research/docs/architecture/session-management.md /private/tmp/research/docs/architecture/model-routing.md /private/tmp/research/docs/architecture/scheduler.md' in /private/tmp/research succeeded in 0ms:
/private/tmp/research/docs/architecture/scheduler.md:1114:fn can_afford_job(
/private/tmp/research/docs/architecture/model-routing.md:155:            model: if budget.remaining_percent() > 50.0 {
/private/tmp/research/docs/architecture/model-routing.md:396:    if budget.remaining_percent() < 60.0 {
/private/tmp/research/docs/architecture/session-management.md:70:    pub cost_budget_usd: f64,
/private/tmp/research/docs/architecture/session-management.md:125:        // 4. cost_budget_usd must be positive
/private/tmp/research/docs/architecture/session-management.md:126:        if self.cost_budget_usd <= 0.0 {
/private/tmp/research/docs/architecture/session-management.md:127:            return Err(SessionConfigError::InvalidBudget(self.cost_budget_usd));
/private/tmp/research/docs/architecture/session-management.md:561:    pub fn can_afford(&self, estimated_cost_usd: f64) -> bool {
/private/tmp/research/docs/architecture/session-management.md:641:fn validate_worktree(config: &SessionConfig) -> Result<(), SessionConfigError> {
/private/tmp/research/docs/architecture/session-management.md:654:        // working_dir must match worktree path
/private/tmp/research/docs/architecture/session-management.md:712:        cost_budget_usd: original.cost_budget_usd,
/private/tmp/research/docs/architecture/session-management.md:749:        cost_budget_usd: original.cost_budget_usd,
/private/tmp/research/docs/architecture/session-management.md:876:        cost_tracker.can_afford(estimated_cost)
/private/tmp/research/docs/architecture/session-management.md:914:    BudgetExceeded {
/private/tmp/research/docs/architecture/session-management.md:972:    #[error("Working dir {working_dir} does not match worktree path {worktree_path}")]
/private/tmp/research/docs/architecture/session-management.md:1142:    claude_binary: PathBuf,
/private/tmp/research/docs/architecture/session-management.md:1271:- `cost_tracker_can_afford`: Contention test with concurrent readers/writers
exec
/bin/zsh -lc "rg -n \"check\\)|check\" /private/tmp/research/docs/architecture/model-routing.md" in /private/tmp/research succeeded in 0ms:
62:    /// Health checks, status monitoring, simple queries.
101:        || lower.contains("audit") || lower.contains("check")
111:        || lower.contains("check") || lower.contains("monitor")
exec
/bin/zsh -lc "gemini --version 2>&1 | sed -n '1,40p'" in /private/tmp/researchexec
/bin/zsh -lc 'rg -n "metadata table|metadata" /private/tmp/research/docs/architecture/memory-system.md /private/tmp/research/docs/architecture/state-management.md /private/tmp/research/docs/architecture/evolution-engine.md' in /private/tmp/research succeeded in 0ms:
/private/tmp/research/docs/architecture/memory-system.md:23:4. **Structured over unstructured** — typed entries with categories, scopes, and metadata vs flat markdown
/private/tmp/research/docs/architecture/memory-system.md:311:2. Diff against last known content (stored in memory.sqlite metadata table)
/private/tmp/research/docs/architecture/state-management.md:275:    metadata        TEXT                       -- JSON blob for extensibility
/private/tmp/research/docs/architecture/state-management.md:356:    metadata: Option<serde_json::Value>,
exec
/bin/zsh -lc 'rg -n "experience_traces|variants|probe_tasks|proposals|learnings|capability_gaps|energy_history" /private/tmp/research/docs/architecture/state-management.md /private/tmp/research/docs/architecture/evolution-engine.md /private/tmp/research/PRD.md' in /private/tmp/research succeeded in 0ms:
/private/tmp/research/docs/architecture/evolution-engine.md:14:1. **Archive > Linear** (GEA, DGM): Maintaining diverse variants and evolving them as a group dramatically outperforms linear single-path improvement. GEA: SWE-bench 20% -> 71% in 30 iterations (archive) vs DGM: 20% -> 50% in 60 iterations (linear).
/private/tmp/research/docs/architecture/evolution-engine.md:42:**Formal representation** (from ARTEMIS): The configuration surface maps to `C = (P, T, M, Theta)` — Prompts (global_rules, project_rules), Tools (hooks), Memory (memory_config, context_assembly), Theta (numeric parameters in scheduler_config, model_routing thresholds, decay rates). Evolution uses semantic GA with LLM-ensemble mutation/crossover for NL components (P, T descriptions), and Bayesian optimization for numeric components (Theta). Hierarchical evaluation: cheap LLM scoring of proposals before expensive probe battery runs (10-36% improvement across four agent systems, ARTEMIS).
/private/tmp/research/docs/architecture/evolution-engine.md:80:Instead of one config with proposed sidecars, the system maintains an **append-only archive** of configuration variants:
/private/tmp/research/docs/architecture/evolution-engine.md:84:    /// All variants ever created. Never pruned. Append-only.
/private/tmp/research/docs/architecture/evolution-engine.md:85:    pub variants: Vec<ConfigVariant>,
/private/tmp/research/docs/architecture/evolution-engine.md:121:   initialized during bootstrapping. If no variants have probe_results,
/private/tmp/research/docs/architecture/evolution-engine.md:132:**Capability vector** (from GEA): A binary vector where each dimension represents a probe task. `1` = variant passes that probe, `0` = fails. KNN distance in this space measures how differently two variants behave — high distance = high novelty.
/private/tmp/research/docs/architecture/evolution-engine.md:203:**Uncertainty-guided selective adaptation** (from TT-SI): The analysis phase does not propose changes uniformly across all task categories. It computes failure rates per task type via Relative Softmax Scoring over probe results, and focuses evolution proposals on the highest-failure categories. TT-SI demonstrated +5.48% improvement with selective focus vs +1.04% with uniform adaptation — a 5x difference in evolution efficiency. In practice: if Rust async tasks fail 30% of the time but TypeScript tasks fail 5%, evolution proposals should overwhelmingly target the Rust async configuration.
/private/tmp/research/docs/architecture/evolution-engine.md:353:4. Update archive scores (performance + novelty for all variants)
/private/tmp/research/docs/architecture/evolution-engine.md:401:- Track approval latency and modification rate for queued proposals
/private/tmp/research/docs/architecture/evolution-engine.md:403:- If acceptance rate >85% over rolling 20 proposals: surface upgrade recommendation
/private/tmp/research/docs/architecture/evolution-engine.md:453:CREATE TABLE variants (
/private/tmp/research/docs/architecture/evolution-engine.md:455:    parent_id TEXT REFERENCES variants(id),
/private/tmp/research/docs/architecture/evolution-engine.md:469:-- old variants, the loader applies forward migrations (add missing fields with
/private/tmp/research/docs/architecture/evolution-engine.md:470:-- defaults). Old variants are NEVER rewritten — the migration happens at read time.
/private/tmp/research/docs/architecture/evolution-engine.md:475:    variant_id TEXT REFERENCES variants(id),
/private/tmp/research/docs/architecture/evolution-engine.md:476:    ancestor_id TEXT REFERENCES variants(id),
/private/tmp/research/docs/architecture/evolution-engine.md:482:CREATE TABLE probe_tasks (
/private/tmp/research/docs/architecture/evolution-engine.md:495:    variant_id TEXT REFERENCES variants(id),
/private/tmp/research/docs/architecture/evolution-engine.md:496:    probe_id TEXT REFERENCES probe_tasks(id),
exec
/bin/zsh -lc "rg -n \"COMPACT_MARKER|rm -f \\\"\\"'$COMPACT_MARKER'"\\\"|Only run on non-compact starts|session-start-context.sh|Traces stored in SQLite|cost, compaction count|model used|memory/index.json|memory/projects|Relevant Memory\" /private/tmp/research/PRD.md /private/tmp/research/docs/architecture/hook-system.md" in /private/tmp/research succeeded in 0ms:
/private/tmp/research/PRD.md:307:- [ ] Stop hook logs: duration, tool calls, errors, model used, cost, compaction count
/private/tmp/research/PRD.md:308:- [ ] Traces stored in SQLite (`state.sqlite`, `experience_traces` table)
/private/tmp/research/docs/architecture/hook-system.md:256:COMPACT_MARKER="${STATE_DIR}/.compact-pending-${SESSION_ID}"
/private/tmp/research/docs/architecture/hook-system.md:259:if [[ -f "$COMPACT_MARKER" ]]; then
/private/tmp/research/docs/architecture/hook-system.md:261:    rm -f "$COMPACT_MARKER"
/private/tmp/research/docs/architecture/hook-system.md:308:# --- Phase 3: Relevant Memory ---
/private/tmp/research/docs/architecture/hook-system.md:387:**File**: `~/.autonomic/hooks/session-start-context.sh`
/private/tmp/research/docs/architecture/hook-system.md:400:# session-start-context.sh — Injects assembled memory context on session start
/private/tmp/research/docs/architecture/hook-system.md:409:MEMORY_INDEX="${AUTONOMIC_DIR}/memory/index.json"
/private/tmp/research/docs/architecture/hook-system.md:411:# Only run on non-compact starts (compaction recovery handles its own memory)
/private/tmp/research/docs/architecture/hook-system.md:412:COMPACT_MARKER="${AUTONOMIC_DIR}/state/.compact-pending-${SESSION_ID}"
/private/tmp/research/docs/architecture/hook-system.md:413:if [[ -f "$COMPACT_MARKER" ]]; then
/private/tmp/research/docs/architecture/hook-system.md:419:PROJECT_CONTEXT="${AUTONOMIC_DIR}/memory/projects/${PROJECT_NAME}.md"
/private/tmp/research/docs/architecture/hook-system.md:1020:  ├─ 2. Write global hook scripts (compaction-recovery.sh, session-start-context.sh,
/private/tmp/research/docs/architecture/hook-system.md:1031:        ~/.autonomic/memory/projects/
/private/tmp/research/docs/architecture/hook-system.md:1274:session-start-context.sh        47     8ms    22ms     0       0        0
/private/tmp/research/docs/architecture/hook-system.md:1427:**With session-start-context.sh**: On session 2 startup, the hook injects recent project memories, including: "async lifetime issue in src/transport/stream.rs resolved by pinning the future with Box::pin(). The root cause was that the Stream trait bound required 'static but the borrow was scoped to the connection lifetime." The agent applies the fix directly.
exec
/bin/zsh -lc "rg -n \"daily health check|all projects compile \\+ test|compile \\+ test|weekly evolution analysis|health check of the Autonomic system|Weekly evolution analysis|model_tier = \\\"sonnet\\\"|model_tier = \\\"haiku\\\"\" /private/tmp/research/PRD.md /private/tmp/research/docs/architecture/scheduler.md /private/tmp/research/docs/architecture/overview.md" in /private/tmp/research succeeded in 0ms:
/private/tmp/research/PRD.md:415:- [ ] Daily health check: all projects compile + test (Haiku tier)
/private/tmp/research/PRD.md:418:- [ ] Weekly evolution analysis: metric patterns (Haiku)
/private/tmp/research/docs/architecture/scheduler.md:26:model_tier = "haiku"
/private/tmp/research/docs/architecture/scheduler.md:30:Perform a health check of the Autonomic system:
/private/tmp/research/docs/architecture/scheduler.md:44:model_tier = "sonnet"
/private/tmp/research/docs/architecture/scheduler.md:63:model_tier = "haiku"
/private/tmp/research/docs/architecture/scheduler.md:79:model_tier = "haiku"             # Minimal model usage — mostly internal ops
/private/tmp/research/docs/architecture/scheduler.md:88:model_tier = "haiku"
/private/tmp/research/docs/architecture/scheduler.md:97:model_tier = "haiku"
/private/tmp/research/docs/architecture/scheduler.md:119:model_tier = "sonnet"
/private/tmp/research/docs/architecture/scheduler.md:166:model_tier = "haiku"
exec
/bin/zsh -lc "rg -n \"enum Trigger|cron: String|struct JobDefinition|load_jobs\\(|parse_cron\\(|next_occurrence\\(|on_message|webhook|interval|once|poll\" /private/tmp/research/docs/architecture/scheduler.md" in /private/tmp/research succeeded in 0ms:
139:| `once` | One-time future execution | `2026-04-01T09:00:00Z` |
140:| `interval` | Every N duration | `every 30m`, `every 2h` |
141:| `poll` | Check condition, act if true | Poll project build status every 10m |
142:| `on_message` | React to incoming CLI/API request | User sends task via `autonomic run` |
143:| `webhook` | External HTTP trigger on daemon API | CI pipeline calls `/api/trigger/<job>` |
146:pub enum Trigger {
150:    Poll { check: PollCheck, interval: Duration },
230:struct JobDefinition {
232:    cron: String,
447:fn parse_cron(expression: &str) -> Result<Cron, SchedulerError> {
456:fn next_occurrence(cron: &Cron) -> Option<DateTime<Utc>> {
457:    cron.find_next_occurrence(&Utc::now(), false).ok()
468:    while let Ok(next) = cron.find_next_occurrence(&current, false) {
483:The scheduler maintains a unified next-fire queue across all trigger types. Cron and interval triggers compute their next fire time. Once triggers fire and self-remove. Poll triggers run their check function at their interval. OnMessage and webhook triggers are event-driven (not polled).
508:        let jobs = Self::load_jobs(&state, &config)?;
529:    fn load_jobs(
544:            let cron = parse_cron(&def.cron)?;
545:            let next_run = next_occurrence(&cron).ok_or_else(|| {
653:                // Apply adaptive scheduling: push next_run by one interval
654:                self.adapt_interval(idx);
exec
/bin/zsh -lc 'rg -n "<project>/.autonomic/hooks|~/.autonomic/hooks/command-guard.py|project-level|settings.json|merge" /private/tmp/research/docs/architecture/hook-system.md' in /private/tmp/research succeeded in 0ms:
29:        "command": "~/.autonomic/hooks/command-guard.py",
131:Hooks are registered in `.claude/settings.json` at three levels:
135:| User (global) | `~/.claude/settings.json` | All projects for this user |
136:| Project | `<project>/.claude/settings.json` | This project only (committed) |
139:Hooks from all levels are merged. Within the same event, hooks execute in registration order. A block from any hook prevents the tool call.
233:# Registered as SessionStart hook in ~/.claude/settings.json
547:**File**: `~/.autonomic/hooks/command-guard.py`
739:#### Registration (project-level)
750:            "command": "python3 ~/.autonomic/hooks/command-guard.py",
1012:Autonomic manages hook installation through its Rust orchestrator binary. The orchestrator owns the lifecycle of hook files and their registration in settings.json.
1022:  ├─ 3. Register global hooks in ~/.claude/settings.json
1023:  │     (merge with existing hooks, do not overwrite user hooks)
1024:  ├─ 4. Write project hook scripts to <project>/.autonomic/hooks/
1025:  ├─ 5. Register project hooks in <project>/.claude/settings.json
1026:  │     (merge with existing hooks)
1037:### Merge strategy for settings.json
1039:Autonomic never overwrites existing hooks. It merges by:
1041:1. Reading the existing `settings.json` and parsing the `hooks` object.
1047:Each Autonomic-managed hook command path contains `~/.autonomic/hooks/` or `<project>/.autonomic/hooks/`, making them identifiable during merge.
1065:  ├─ 1. Remove Autonomic hook entries from <project>/.claude/settings.json
exec
/bin/zsh -lc "rg -n \"start\\.elapsed|dispatch_codex|dispatch_gemini|ExternalResult|GeminiAndCodex|remaining_percent\" /private/tmp/research/docs/architecture/model-routing.md" in /private/tmp/research succeeded in 0ms:
155:            model: if budget.remaining_percent() > 50.0 {
183:            external_verification: Some(ExternalModel::GeminiAndCodex),
294:pub async fn dispatch_codex(prompt: &str, timeout: Duration) -> Result<ExternalResult> {
304:    Ok(ExternalResult {
308:        duration: start.elapsed(),
316:pub async fn dispatch_gemini(prompt: &str, timeout: Duration) -> Result<ExternalResult> {
328:    Ok(ExternalResult {
332:        duration: start.elapsed(),
396:    if budget.remaining_percent() < 60.0 {
exec
/bin/zsh -lc "rg -n \"CronExpr|PollCheck|remaining_percent\\(|get_budget_allocations|increment_budget_usage|load_config\\(|config.backup.remote|HookProposal|metadata table\" /private/tmp/research/docs/architecture/*.md" in /private/tmp/research succeeded in 0ms:
/private/tmp/research/docs/architecture/scheduler.md:147:    Cron(CronExpr),
/private/tmp/research/docs/architecture/scheduler.md:150:    Poll { check: PollCheck, interval: Duration },
/private/tmp/research/docs/architecture/scheduler.md:390:            let allocations = state.get_budget_allocations(*tier)?;
/private/tmp/research/docs/architecture/scheduler.md:430:        state.increment_budget_usage(tier, tokens)?;
/private/tmp/research/docs/architecture/scheduler.md:507:        let config = Self::load_config(&state)?;
/private/tmp/research/docs/architecture/scheduler.md:522:    fn load_config(state: &StateManager) -> Result<SchedulerConfig, SchedulerError> {
/private/tmp/research/docs/architecture/scheduler.md:757:        self.config = Self::load_config(&self.state)?;
/private/tmp/research/docs/architecture/scheduler.md:1011:            let config = state.load_config()?;
/private/tmp/research/docs/architecture/scheduler.md:1012:            let remote_url = config.backup.remote
/private/tmp/research/docs/architecture/model-routing.md:155:            model: if budget.remaining_percent() > 50.0 {
/private/tmp/research/docs/architecture/model-routing.md:396:    if budget.remaining_percent() < 60.0 {
/private/tmp/research/docs/architecture/memory-system.md:311:2. Diff against last known content (stored in memory.sqlite metadata table)
/private/tmp/research/docs/architecture/hook-system.md:1084:  │   Create a HookProposal with:
/private/tmp/research/docs/architecture/hook-system.md:1114:pub struct HookProposal {
exec
/bin/zsh -lc 'rg -n "After first helpful mark|neutral score of 1/2|new entries" /private/tmp/research/docs/architecture/memory-system.md' in /private/tmp/research succeeded in 0ms:
225:| SessionStart (resume) | Delta since last session (new entries only) | 2048 |
359:**Laplace smoothing** (Gemini review fix): The `+1/+2` ensures new entries (0 helpful, 0 misleading) get a neutral score of 1/2 = 0.5, not zero. Without this, new memories would have score 0 and never surface — the system would ignore everything it learns until manually marked helpful. After first helpful mark: 2/(2+0+2) = 0.5. After 10 helpful, 0 misleading: 11/(10+0+2) = 0.92.
383:3. Mark new entries with source_session for provenance
exec
/bin/zsh -lc 'rg -n "daemon.log|autonomic.log|logs/daemon|logs/autonomic" /private/tmp/research/PRD.md /private/tmp/research/docs/architecture/*.md' in /private/tmp/research succeeded in 0ms:
/private/tmp/research/PRD.md:253:- [ ] Structured tracing to `~/.autonomic/logs/daemon.log`
/private/tmp/research/PRD.md:623:- Per-session output capture (`~/.autonomic/logs/sessions/`)
/private/tmp/research/docs/architecture/state-management.md:28:    autonomic.log          # Current rotating log (not git-committed)
exec
/bin/zsh -lc 'rg -n "Project Registry|project add|project list|auto-discovers|hook inventory|registered project|projects/" /private/tmp/research/docs/architecture/*.md /private/tmp/research/PRD.md' in /private/tmp/research succeeded in 0ms:
/private/tmp/research/PRD.md:259:#### FR-002: Project Registry
/private/tmp/research/PRD.md:266:- [ ] `autonomic project add <path>` registers a project
/private/tmp/research/PRD.md:267:- [ ] `autonomic project list` shows all projects with status
/private/tmp/research/PRD.md:269:- [ ] Stores: path, language, last activity, last session cost, hook inventory
/private/tmp/research/docs/architecture/scheduler.md:123:Navigate to ~/projects/my-project and:
/private/tmp/research/docs/architecture/scheduler.md:128:context_files = ["projects/abc123/context.toml"]
/private/tmp/research/docs/architecture/memory-system.md:271:For each registered project, the orchestrator generates a project-specific MEMORY.md:
/private/tmp/research/docs/architecture/state-management.md:12:  projects/
/private/tmp/research/docs/architecture/state-management.md:66:projects/*/history.sqlite
/private/tmp/research/docs/architecture/hook-system.md:419:PROJECT_CONTEXT="${AUTONOMIC_DIR}/memory/projects/${PROJECT_NAME}.md"
/private/tmp/research/docs/architecture/hook-system.md:1031:        ~/.autonomic/memory/projects/
/private/tmp/research/docs/architecture/evolution-engine.md:334:  projects: all registered projects
exec
/bin/zsh -lc 'rg -n "launchd|plist|axum|/health|daemon.pid|health endpoint|localhost" /private/tmp/research/docs/architecture/*.md /private/tmp/research/PRD.md' in /private/tmp/research succeeded in 0ms:
/private/tmp/research/PRD.md:45:9. **Rust-Native** -- Sub-millisecond startup, <10MB RSS, crash-safe, launchd-supervised. No runtime dependencies.
/private/tmp/research/PRD.md:78:4. **Rust-native daemon** -- Sub-ms startup, ~5MB RSS, crash-safe, launchd-supervised.
/private/tmp/research/PRD.md:111:launchd (auto-start, KeepAlive, auto-restart)
/private/tmp/research/PRD.md:116:  +-- autonomic daemon (Rust, axum HTTP, ~5MB RSS)
/private/tmp/research/PRD.md:142:+-- autonomic-daemon/          # Main binary: axum, launchd
/private/tmp/research/PRD.md:246:Two-process architecture: daemon (main binary) + watchdog (lightweight monitor). Both managed by launchd. Watchdog prevents self-bricking from evolution failures.
/private/tmp/research/PRD.md:249:- [ ] Daemon binary starts via launchd Launch Agent plist
/private/tmp/research/PRD.md:250:- [ ] Watchdog binary starts via separate launchd plist, monitors daemon health
/private/tmp/research/PRD.md:254:- [ ] HTTP API on localhost (axum) for CLI communication
/private/tmp/research/PRD.md:255:- [ ] PID file at `~/.autonomic/daemon.pid`
/private/tmp/research/PRD.md:614:- launchd KeepAlive auto-restart
/private/tmp/research/PRD.md:634:| HTTP | axum | CLI-daemon communication |
/private/tmp/research/PRD.md:678:| DEC-011 | launchd over systemd/tmux | macOS-native: auto-start, auto-restart, throttle protection. | 2026-03-25 |
/private/tmp/research/docs/architecture/state-management.md:47:4. **Health monitoring**: Polls `http://localhost:<port>/health` every 30 seconds. 3 consecutive failures trigger a restart (without rollback — only evolution-related crashes trigger rollback).
/private/tmp/research/docs/architecture/state-management.md:135:    Signature::now("autonomic", "autonomic@localhost")
/private/tmp/research/docs/architecture/evolution-engine.md:612:1. Monitors daemon health via HTTP health endpoint
/private/tmp/research/docs/architecture/evolution-engine.md:615:4. If the daemon crashes without recent evolution: normal restart (launchd KeepAlive)
/private/tmp/research/docs/architecture/overview.md:13:launchd
/private/tmp/research/docs/architecture/overview.md:17:  +-- autonomic daemon (Rust, axum, ~5MB RSS)
/private/tmp/research/docs/architecture/overview.md:103:      main.rs, axum routes, launchd integration
exec
/bin/zsh -lc 'rg -n "stderr|RateLimited|Backoff|backoff|429|rate limit|NoStdout|NoStderr|child.stderr|ProcessCrashed" /private/tmp/research/docs/architecture/session-management.md' in /private/tmp/research succeeded in 0ms:
199:    cmd.stderr(Stdio::piped());
249:| `system` | (various) | System-level messages | Log, check for rate limit signals |
490:            .ok_or(SessionError::NoStdout)?;
538:    /// Rolling window of costs for rate limit awareness.
578:Claude Max rate limits are per-account and shared across all concurrent sessions. The session manager does not attempt to model the exact rate limit (which depends on model, plan tier, and server-side state). Instead, it uses these heuristics:
580:1. **Concurrency cap**: A configurable maximum number of concurrent `Running` sessions (default: 5). This prevents stampeding the rate limit.
581:2. **Backoff on 429**: If stderr output contains rate-limit indicators (HTTP 429 or "rate limit" text), the session is paused and retried with exponential backoff.
585:/// Configuration for rate limit management.
590:    /// Initial backoff duration after a rate limit hit.
591:    pub initial_backoff: Duration,
593:    /// Maximum backoff duration.
594:    pub max_backoff: Duration,
596:    /// Backoff multiplier per consecutive rate limit hit.
597:    pub backoff_multiplier: f64,
604:            initial_backoff: Duration::from_secs(10),
605:            max_backoff: Duration::from_secs(300),
606:            backoff_multiplier: 2.0,
789:                SessionError::RateLimited => ResumeStrategy::BackoffAndRetry,
790:                SessionError::ProcessCrashed { .. } => {
815:    /// Wait for backoff, then retry from scratch.
Plan update
Read all requested files, checked subsystem boundaries, and verified the installed Claude CLI where the docs depended on exact flags.
  ✓ Read all in-scope documents with line references
  ✓ Cross-check algorithms, types, schemas, and platform assumptions for consistency and feasibility
  → Produce structured findings with exact file/section references
codex
**Critical Issues**
- Scheduled jobs violate the architecture and likely fail as written. `autonomic-session` is supposed to be the only Claude spawner, but `autonomic-scheduler` shells out directly, uses unsupported flags, omits permission/worktree/env handling, and runs from `~/.autonomic` instead of the target project. That breaks project jobs, bypasses `CLAUDECODE` filtering, and ignores the session manager’s safety contract. [session-management.md:5](/private/tmp/research/docs/architecture/session-management.md#L5), [scheduler.md:879](/private/tmp/research/docs/architecture/scheduler.md#L879), [scheduler.md:903](/private/tmp/research/docs/architecture/scheduler.md#L903), [scheduler.md:905](/private/tmp/research/docs/architecture/scheduler.md#L905). I also checked the installed `claude` CLI on this machine: it exposes a positional prompt and `--max-budget-usd`, not `--prompt` or `--max-tokens`.
- The hook, memory, and evolution pipelines do not share a common storage contract. The memory spec centers on `memory.sqlite` and generated `.claude/MEMORY.md`, but hooks read `~/.autonomic/memory/*.jsonl` and `~/.autonomic/memory/projects/*.md`; the evolution spec expects `experience_traces` in SQLite, while the Stop hook writes ad hoc JSONL without the required fields. End-to-end recovery/evolution cannot work from these docs alone. [memory-system.md:31](/private/tmp/research/docs/architecture/memory-system.md#L31), [memory-system.md:298](/private/tmp/research/docs/architecture/memory-system.md#L298), [hook-system.md:312](/private/tmp/research/docs/architecture/hook-system.md#L312), [hook-system.md:419](/private/tmp/research/docs/architecture/hook-system.md#L419), [hook-system.md:516](/private/tmp/research/docs/architecture/hook-system.md#L516), [evolution-engine.md:516](/private/tmp/research/docs/architecture/evolution-engine.md#L516), [PRD.md:307](/private/tmp/research/PRD.md#L307).
- State durability and rollback are internally contradictory. The docs promise “every mutation” is git-committed, but SQLite is explicitly only committed at snapshot boundaries, and the shown write paths do not checkpoint/commit after normal inserts. On first run `StateManager::open()` skips repository initialization, and rollback force-removes untracked files, which can delete gitignored operational state like `secrets.toml`. [PRD.md:44](/private/tmp/research/PRD.md#L44), [state-management.md:96](/private/tmp/research/docs/architecture/state-management.md#L96), [state-management.md:816](/private/tmp/research/docs/architecture/state-management.md#L816), [state-management.md:516](/private/tmp/research/docs/architecture/state-management.md#L516), [state-management.md:70](/private/tmp/research/docs/architecture/state-management.md#L70).
- The scheduler advertises six trigger types, but the actual job model and loop only support cron. `JobDefinition` only has `cron: String`, `load_jobs()` only parses cron, and the run loop only advances cron occurrences. `once`, `interval`, `poll`, `on_message`, and `webhook` are specification-only. [scheduler.md:132](/private/tmp/research/docs/architecture/scheduler.md#L132), [scheduler.md:230](/private/tmp/research/docs/architecture/scheduler.md#L230), [scheduler.md:544](/private/tmp/research/docs/architecture/scheduler.md#L544), [scheduler.md:619](/private/tmp/research/docs/architecture/scheduler.md#L619).
- The session manager’s subprocess control is incomplete. `stderr` is piped but never consumed, yet rate-limit detection depends on stderr; the crash path records empty stderr; `validate_worktree()` exists but is never called from `validate()` or `run_session()`. This is enough to deadlock or silently drop the exact signals the retry logic depends on. [session-management.md:199](/private/tmp/research/docs/architecture/session-management.md#L199), [session-management.md:488](/private/tmp/research/docs/architecture/session-management.md#L488), [session-management.md:581](/private/tmp/research/docs/architecture/session-management.md#L581), [session-management.md:641](/private/tmp/research/docs/architecture/session-management.md#L641), [session-management.md:1068](/private/tmp/research/docs/architecture/session-management.md#L1068).

**Design Concerns**
- The rate-budget model is not coherent across docs. The PRD uses a 5-way `15/55/15/10/5` split including external and emergency reserve, the scheduler/state schema use 4 subsystems with per-tier percentages, and the session manager tracks a USD rolling window instead of tokens/requests. There is no single implementable budget contract. [PRD.md:222](/private/tmp/research/PRD.md#L222), [PRD.md:404](/private/tmp/research/PRD.md#L404), [state-management.md:312](/private/tmp/research/docs/architecture/state-management.md#L312), [scheduler.md:1157](/private/tmp/research/docs/architecture/scheduler.md#L1157), [session-management.md:527](/private/tmp/research/docs/architecture/session-management.md#L527).
- The evolution archive model does not reconcile single active deployment with per-project evolution. The archive has one `active_variant`, but group evolution creates project-specific descendants. That needs either per-project active variants or a higher-level composition model. [evolution-engine.md:83](/private/tmp/research/docs/architecture/evolution-engine.md#L83), [evolution-engine.md:331](/private/tmp/research/docs/architecture/evolution-engine.md#L331).
- The filesystem “frontier” is weaker than the docs claim on macOS when the daemon and watchdog run as the same user. `chmod 444` is a guardrail, not a hard capability boundary; the same process domain can revert permissions. [evolution-engine.md:58](/private/tmp/research/docs/architecture/evolution-engine.md#L58), [state-management.md:37](/private/tmp/research/docs/architecture/state-management.md#L37).
- “No runtime dependencies” is not true at the system level. The hook design assumes `bash`, `jq`, `python3`, and language-specific formatters/linters. For a solo macOS ARM setup that is still feasible, but it needs to be documented as a managed dependency surface. [PRD.md:45](/private/tmp/research/PRD.md#L45), [hook-system.md:238](/private/tmp/research/docs/architecture/hook-system.md#L238), [hook-system.md:561](/private/tmp/research/docs/architecture/hook-system.md#L561), [hook-system.md:803](/private/tmp/research/docs/architecture/hook-system.md#L803), [hook-system.md:877](/private/tmp/research/docs/architecture/hook-system.md#L877).
- Scope is too broad for one solo implementation pass. v0.1-v0.2 are realistic; v0.4-v0.6 adds archive evolution, timed rollback, multi-model verification, hook synthesis, and governance. The docs need a sharper cut between “first shippable orchestrator” and “research program.” [PRD.md:238](/private/tmp/research/PRD.md#L238), [PRD.md:420](/private/tmp/research/PRD.md#L420), [PRD.md:515](/private/tmp/research/PRD.md#L515), [PRD.md:557](/private/tmp/research/PRD.md#L557).

**Missing Elements**
- There is no implementation-grade architecture doc for the P0 Project Registry, only PRD requirements. [PRD.md:259](/private/tmp/research/PRD.md#L259).
- There is no concrete daemon/API/launchd spec despite FR-001 depending on localhost health checks, PID files, and dual LaunchAgents. [PRD.md:246](/private/tmp/research/PRD.md#L246), [PRD.md:254](/private/tmp/research/PRD.md#L254), [state-management.md:47](/private/tmp/research/docs/architecture/state-management.md#L47).
- The hook system depends on files with no owning producer spec: `current-task.json`, `pending-tasks.json`, `state/sessions/*.jsonl`, and `memory/index.json`. [hook-system.md:248](/private/tmp/research/docs/architecture/hook-system.md#L248), [hook-system.md:445](/private/tmp/research/docs/architecture/hook-system.md#L445), [hook-system.md:485](/private/tmp/research/docs/architecture/hook-system.md#L485), [hook-system.md:409](/private/tmp/research/docs/architecture/hook-system.md#L409).
- `memory.sqlite` needs a metadata table for MEMORY.md diffing, but the schema does not define one. [memory-system.md:311](/private/tmp/research/docs/architecture/memory-system.md#L311).
- The state-management migration section does not show how `state.sqlite` evolves to include `experience_traces`, `variants`, `probe_tasks`, `proposals`, `learnings`, and `energy_history`. Those tables exist only in the evolution doc. [state-management.md:705](/private/tmp/research/docs/architecture/state-management.md#L705), [evolution-engine.md:449](/private/tmp/research/docs/architecture/evolution-engine.md#L449).
- macOS launchd secret injection is underspecified. The docs currently rely on env substitution or future Keychain support, but LaunchAgents do not reliably inherit shell env; external model auth needs a concrete runtime plan. [PRD.md:246](/private/tmp/research/PRD.md#L246), [state-management.md:77](/private/tmp/research/docs/architecture/state-management.md#L77).

**Strengths**
- The evolution pipeline is unusually well-thought-out: diagnose separately from repair, gate on flips instead of averages, and deploy through timed verification windows. [evolution-engine.md:171](/private/tmp/research/docs/architecture/evolution-engine.md#L171), [evolution-engine.md:242](/private/tmp/research/docs/architecture/evolution-engine.md#L242), [evolution-engine.md:289](/private/tmp/research/docs/architecture/evolution-engine.md#L289).
- The memory design is pragmatic for a solo dev: SQLite + FTS5, decay-at-query-time, and MEMORY.md interoperability avoid unnecessary infrastructure. [memory-system.md:29](/private/tmp/research/docs/architecture/memory-system.md#L29), [memory-system.md:135](/private/tmp/research/docs/architecture/memory-system.md#L135), [memory-system.md:267](/private/tmp/research/docs/architecture/memory-system.md#L267).
- The session model is mostly sound: explicit config, typed outcomes, worktree isolation, and resumed-session context re-injection are the right primitives. [session-management.md:23](/private/tmp/research/docs/architecture/session-management.md#L23), [session-management.md:366](/private/tmp/research/docs/architecture/session-management.md#L366), [session-management.md:612](/private/tmp/research/docs/architecture/session-management.md#L612), [session-management.md:726](/private/tmp/research/docs/architecture/session-management.md#L726).
- The docs consistently constrain self-improvement to scaffold-shaped problems, which is the right feasibility boundary. [PRD.md:192](/private/tmp/research/PRD.md#L192), [evolution-engine.md:424](/private/tmp/research/docs/architecture/evolution-engine.md#L424).

**Specific Corrections**
- The compaction marker logic is wrong: `compaction-recovery.sh` deletes the marker before `session-start-context.sh` checks it, so the second hook cannot actually skip compact starts. [hook-system.md:259](/private/tmp/research/docs/architecture/hook-system.md#L259), [hook-system.md:411](/private/tmp/research/docs/architecture/hook-system.md#L411).
- `session-start-context.sh` queries `.level` and `.message` from trace JSON, but `session-metrics.sh` never emits those fields. [hook-system.md:430](/private/tmp/research/docs/architecture/hook-system.md#L430), [hook-system.md:526](/private/tmp/research/docs/architecture/hook-system.md#L526).
- The Laplace-smoothing example is mathematically wrong: after one helpful mark the formula should yield `2/3`, not `0.5`. [memory-system.md:359](/private/tmp/research/docs/architecture/memory-system.md#L359).
- `AutoномicConfig` contains a typo/non-ASCII identifier and will not match the rest of the type system. [state-management.md:839](/private/tmp/research/docs/architecture/state-management.md#L839).
- The scheduler writes directly to `state_db` even though the same document says it never writes to SQLite directly. [scheduler.md:825](/private/tmp/research/docs/architecture/scheduler.md#L825), [scheduler.md:1319](/private/tmp/research/docs/architecture/scheduler.md#L1319).
- `dispatch_codex()` and `dispatch_gemini()` use `start.elapsed()` without defining `start`. [model-routing.md:304](/private/tmp/research/docs/architecture/model-routing.md#L304), [model-routing.md:328](/private/tmp/research/docs/architecture/model-routing.md#L328).
- The session crash path moves `status` twice (`and_then` on the same `Option`), which will not compile as written. [session-management.md:1058](/private/tmp/research/docs/architecture/session-management.md#L1058).
- The Rust formatter hook hardcodes edition 2021 even though the project targets Rust 2024. [hook-system.md:806](/private/tmp/research/docs/architecture/hook-system.md#L806), [PRD.md:10](/private/tmp/research/PRD.md#L10).
- The PRD says L2 command-guard behavior is “self-verify prompt,” but the implementation always denies with exit 2 and has no override path. [PRD.md:375](/private/tmp/research/PRD.md#L375), [hook-system.md:718](/private/tmp/research/docs/architecture/hook-system.md#L718).
- The built-in job definitions drift from the PRD: daily health check does not compile/test all projects, and weekly evolution analysis is Sonnet in the scheduler doc but Haiku in the PRD. [PRD.md:415](/private/tmp/research/PRD.md#L415), [PRD.md:418](/private/tmp/research/PRD.md#L418), [scheduler.md:30](/private/tmp/research/docs/architecture/scheduler.md#L30), [scheduler.md:44](/private/tmp/research/docs/architecture/scheduler.md#L44).
- `memory.sqlite` is described once as storing “embeddings,” which conflicts with the FTS5-only design. [state-management.md:11](/private/tmp/research/docs/architecture/state-management.md#L11), [memory-system.md:31](/private/tmp/research/docs/architecture/memory-system.md#L31).

Overall verdict: the conceptual architecture is strong, but the docs are not yet implementation-ready. A solo macOS ARM build is realistic if the project is narrowed to daemon + session manager + FTS5 memory + hook installer first, and the cross-document contracts above are unified before coding the evolution/scheduler layers.
tokens used
301,204
**Critical Issues**
- Scheduled jobs violate the architecture and likely fail as written. `autonomic-session` is supposed to be the only Claude spawner, but `autonomic-scheduler` shells out directly, uses unsupported flags, omits permission/worktree/env handling, and runs from `~/.autonomic` instead of the target project. That breaks project jobs, bypasses `CLAUDECODE` filtering, and ignores the session manager’s safety contract. [session-management.md:5](/private/tmp/research/docs/architecture/session-management.md#L5), [scheduler.md:879](/private/tmp/research/docs/architecture/scheduler.md#L879), [scheduler.md:903](/private/tmp/research/docs/architecture/scheduler.md#L903), [scheduler.md:905](/private/tmp/research/docs/architecture/scheduler.md#L905). I also checked the installed `claude` CLI on this machine: it exposes a positional prompt and `--max-budget-usd`, not `--prompt` or `--max-tokens`.
- The hook, memory, and evolution pipelines do not share a common storage contract. The memory spec centers on `memory.sqlite` and generated `.claude/MEMORY.md`, but hooks read `~/.autonomic/memory/*.jsonl` and `~/.autonomic/memory/projects/*.md`; the evolution spec expects `experience_traces` in SQLite, while the Stop hook writes ad hoc JSONL without the required fields. End-to-end recovery/evolution cannot work from these docs alone. [memory-system.md:31](/private/tmp/research/docs/architecture/memory-system.md#L31), [memory-system.md:298](/private/tmp/research/docs/architecture/memory-system.md#L298), [hook-system.md:312](/private/tmp/research/docs/architecture/hook-system.md#L312), [hook-system.md:419](/private/tmp/research/docs/architecture/hook-system.md#L419), [hook-system.md:516](/private/tmp/research/docs/architecture/hook-system.md#L516), [evolution-engine.md:516](/private/tmp/research/docs/architecture/evolution-engine.md#L516), [PRD.md:307](/private/tmp/research/PRD.md#L307).
- State durability and rollback are internally contradictory. The docs promise “every mutation” is git-committed, but SQLite is explicitly only committed at snapshot boundaries, and the shown write paths do not checkpoint/commit after normal inserts. On first run `StateManager::open()` skips repository initialization, and rollback force-removes untracked files, which can delete gitignored operational state like `secrets.toml`. [PRD.md:44](/private/tmp/research/PRD.md#L44), [state-management.md:96](/private/tmp/research/docs/architecture/state-management.md#L96), [state-management.md:816](/private/tmp/research/docs/architecture/state-management.md#L816), [state-management.md:516](/private/tmp/research/docs/architecture/state-management.md#L516), [state-management.md:70](/private/tmp/research/docs/architecture/state-management.md#L70).
- The scheduler advertises six trigger types, but the actual job model and loop only support cron. `JobDefinition` only has `cron: String`, `load_jobs()` only parses cron, and the run loop only advances cron occurrences. `once`, `interval`, `poll`, `on_message`, and `webhook` are specification-only. [scheduler.md:132](/private/tmp/research/docs/architecture/scheduler.md#L132), [scheduler.md:230](/private/tmp/research/docs/architecture/scheduler.md#L230), [scheduler.md:544](/private/tmp/research/docs/architecture/scheduler.md#L544), [scheduler.md:619](/private/tmp/research/docs/architecture/scheduler.md#L619).
- The session manager’s subprocess control is incomplete. `stderr` is piped but never consumed, yet rate-limit detection depends on stderr; the crash path records empty stderr; `validate_worktree()` exists but is never called from `validate()` or `run_session()`. This is enough to deadlock or silently drop the exact signals the retry logic depends on. [session-management.md:199](/private/tmp/research/docs/architecture/session-management.md#L199), [session-management.md:488](/private/tmp/research/docs/architecture/session-management.md#L488), [session-management.md:581](/private/tmp/research/docs/architecture/session-management.md#L581), [session-management.md:641](/private/tmp/research/docs/architecture/session-management.md#L641), [session-management.md:1068](/private/tmp/research/docs/architecture/session-management.md#L1068).

**Design Concerns**
- The rate-budget model is not coherent across docs. The PRD uses a 5-way `15/55/15/10/5` split including external and emergency reserve, the scheduler/state schema use 4 subsystems with per-tier percentages, and the session manager tracks a USD rolling window instead of tokens/requests. There is no single implementable budget contract. [PRD.md:222](/private/tmp/research/PRD.md#L222), [PRD.md:404](/private/tmp/research/PRD.md#L404), [state-management.md:312](/private/tmp/research/docs/architecture/state-management.md#L312), [scheduler.md:1157](/private/tmp/research/docs/architecture/scheduler.md#L1157), [session-management.md:527](/private/tmp/research/docs/architecture/session-management.md#L527).
- The evolution archive model does not reconcile single active deployment with per-project evolution. The archive has one `active_variant`, but group evolution creates project-specific descendants. That needs either per-project active variants or a higher-level composition model. [evolution-engine.md:83](/private/tmp/research/docs/architecture/evolution-engine.md#L83), [evolution-engine.md:331](/private/tmp/research/docs/architecture/evolution-engine.md#L331).
- The filesystem “frontier” is weaker than the docs claim on macOS when the daemon and watchdog run as the same user. `chmod 444` is a guardrail, not a hard capability boundary; the same process domain can revert permissions. [evolution-engine.md:58](/private/tmp/research/docs/architecture/evolution-engine.md#L58), [state-management.md:37](/private/tmp/research/docs/architecture/state-management.md#L37).
- “No runtime dependencies” is not true at the system level. The hook design assumes `bash`, `jq`, `python3`, and language-specific formatters/linters. For a solo macOS ARM setup that is still feasible, but it needs to be documented as a managed dependency surface. [PRD.md:45](/private/tmp/research/PRD.md#L45), [hook-system.md:238](/private/tmp/research/docs/architecture/hook-system.md#L238), [hook-system.md:561](/private/tmp/research/docs/architecture/hook-system.md#L561), [hook-system.md:803](/private/tmp/research/docs/architecture/hook-system.md#L803), [hook-system.md:877](/private/tmp/research/docs/architecture/hook-system.md#L877).
- Scope is too broad for one solo implementation pass. v0.1-v0.2 are realistic; v0.4-v0.6 adds archive evolution, timed rollback, multi-model verification, hook synthesis, and governance. The docs need a sharper cut between “first shippable orchestrator” and “research program.” [PRD.md:238](/private/tmp/research/PRD.md#L238), [PRD.md:420](/private/tmp/research/PRD.md#L420), [PRD.md:515](/private/tmp/research/PRD.md#L515), [PRD.md:557](/private/tmp/research/PRD.md#L557).

**Missing Elements**
- There is no implementation-grade architecture doc for the P0 Project Registry, only PRD requirements. [PRD.md:259](/private/tmp/research/PRD.md#L259).
- There is no concrete daemon/API/launchd spec despite FR-001 depending on localhost health checks, PID files, and dual LaunchAgents. [PRD.md:246](/private/tmp/research/PRD.md#L246), [PRD.md:254](/private/tmp/research/PRD.md#L254), [state-management.md:47](/private/tmp/research/docs/architecture/state-management.md#L47).
- The hook system depends on files with no owning producer spec: `current-task.json`, `pending-tasks.json`, `state/sessions/*.jsonl`, and `memory/index.json`. [hook-system.md:248](/private/tmp/research/docs/architecture/hook-system.md#L248), [hook-system.md:445](/private/tmp/research/docs/architecture/hook-system.md#L445), [hook-system.md:485](/private/tmp/research/docs/architecture/hook-system.md#L485), [hook-system.md:409](/private/tmp/research/docs/architecture/hook-system.md#L409).
- `memory.sqlite` needs a metadata table for MEMORY.md diffing, but the schema does not define one. [memory-system.md:311](/private/tmp/research/docs/architecture/memory-system.md#L311).
- The state-management migration section does not show how `state.sqlite` evolves to include `experience_traces`, `variants`, `probe_tasks`, `proposals`, `learnings`, and `energy_history`. Those tables exist only in the evolution doc. [state-management.md:705](/private/tmp/research/docs/architecture/state-management.md#L705), [evolution-engine.md:449](/private/tmp/research/docs/architecture/evolution-engine.md#L449).
- macOS launchd secret injection is underspecified. The docs currently rely on env substitution or future Keychain support, but LaunchAgents do not reliably inherit shell env; external model auth needs a concrete runtime plan. [PRD.md:246](/private/tmp/research/PRD.md#L246), [state-management.md:77](/private/tmp/research/docs/architecture/state-management.md#L77).

**Strengths**
- The evolution pipeline is unusually well-thought-out: diagnose separately from repair, gate on flips instead of averages, and deploy through timed verification windows. [evolution-engine.md:171](/private/tmp/research/docs/architecture/evolution-engine.md#L171), [evolution-engine.md:242](/private/tmp/research/docs/architecture/evolution-engine.md#L242), [evolution-engine.md:289](/private/tmp/research/docs/architecture/evolution-engine.md#L289).
- The memory design is pragmatic for a solo dev: SQLite + FTS5, decay-at-query-time, and MEMORY.md interoperability avoid unnecessary infrastructure. [memory-system.md:29](/private/tmp/research/docs/architecture/memory-system.md#L29), [memory-system.md:135](/private/tmp/research/docs/architecture/memory-system.md#L135), [memory-system.md:267](/private/tmp/research/docs/architecture/memory-system.md#L267).
- The session model is mostly sound: explicit config, typed outcomes, worktree isolation, and resumed-session context re-injection are the right primitives. [session-management.md:23](/private/tmp/research/docs/architecture/session-management.md#L23), [session-management.md:366](/private/tmp/research/docs/architecture/session-management.md#L366), [session-management.md:612](/private/tmp/research/docs/architecture/session-management.md#L612), [session-management.md:726](/private/tmp/research/docs/architecture/session-management.md#L726).
- The docs consistently constrain self-improvement to scaffold-shaped problems, which is the right feasibility boundary. [PRD.md:192](/private/tmp/research/PRD.md#L192), [evolution-engine.md:424](/private/tmp/research/docs/architecture/evolution-engine.md#L424).

**Specific Corrections**
- The compaction marker logic is wrong: `compaction-recovery.sh` deletes the marker before `session-start-context.sh` checks it, so the second hook cannot actually skip compact starts. [hook-system.md:259](/private/tmp/research/docs/architecture/hook-system.md#L259), [hook-system.md:411](/private/tmp/research/docs/architecture/hook-system.md#L411).
- `session-start-context.sh` queries `.level` and `.message` from trace JSON, but `session-metrics.sh` never emits those fields. [hook-system.md:430](/private/tmp/research/docs/architecture/hook-system.md#L430), [hook-system.md:526](/private/tmp/research/docs/architecture/hook-system.md#L526).
- The Laplace-smoothing example is mathematically wrong: after one helpful mark the formula should yield `2/3`, not `0.5`. [memory-system.md:359](/private/tmp/research/docs/architecture/memory-system.md#L359).
- `AutoномicConfig` contains a typo/non-ASCII identifier and will not match the rest of the type system. [state-management.md:839](/private/tmp/research/docs/architecture/state-management.md#L839).
- The scheduler writes directly to `state_db` even though the same document says it never writes to SQLite directly. [scheduler.md:825](/private/tmp/research/docs/architecture/scheduler.md#L825), [scheduler.md:1319](/private/tmp/research/docs/architecture/scheduler.md#L1319).
- `dispatch_codex()` and `dispatch_gemini()` use `start.elapsed()` without defining `start`. [model-routing.md:304](/private/tmp/research/docs/architecture/model-routing.md#L304), [model-routing.md:328](/private/tmp/research/docs/architecture/model-routing.md#L328).
- The session crash path moves `status` twice (`and_then` on the same `Option`), which will not compile as written. [session-management.md:1058](/private/tmp/research/docs/architecture/session-management.md#L1058).
- The Rust formatter hook hardcodes edition 2021 even though the project targets Rust 2024. [hook-system.md:806](/private/tmp/research/docs/architecture/hook-system.md#L806), [PRD.md:10](/private/tmp/research/PRD.md#L10).
- The PRD says L2 command-guard behavior is “self-verify prompt,” but the implementation always denies with exit 2 and has no override path. [PRD.md:375](/private/tmp/research/PRD.md#L375), [hook-system.md:718](/private/tmp/research/docs/architecture/hook-system.md#L718).
- The built-in job definitions drift from the PRD: daily health check does not compile/test all projects, and weekly evolution analysis is Sonnet in the scheduler doc but Haiku in the PRD. [PRD.md:415](/private/tmp/research/PRD.md#L415), [PRD.md:418](/private/tmp/research/PRD.md#L418), [scheduler.md:30](/private/tmp/research/docs/architecture/scheduler.md#L30), [scheduler.md:44](/private/tmp/research/docs/architecture/scheduler.md#L44).
- `memory.sqlite` is described once as storing “embeddings,” which conflicts with the FTS5-only design. [state-management.md:11](/private/tmp/research/docs/architecture/state-management.md#L11), [memory-system.md:31](/private/tmp/research/docs/architecture/memory-system.md#L31).

Overall verdict: the conceptual architecture is strong, but the docs are not yet implementation-ready. A solo macOS ARM build is realistic if the project is narrowed to daemon + session manager + FTS5 memory + hook installer first, and the cross-document contracts above are unified before coding the evolution/scheduler layers.
