# Autonomic Research Sources

**Date**: 2026-03-25
**Research method**: 11 parallel research agents + direct codebase analysis

## Primary Sources (Direct Code Analysis)

### Instar

- **Repository**: https://github.com/JKHeadley/instar
- **Version analyzed**: v0.24.2
- **License**: MIT
- **Author**: JKHeadley (SageMindAI)
- **Impact**: Core architectural inspiration -- "Structure > Willpower" principle, compaction recovery hooks, sidecar mutation pattern, evolution engine (EvolutionManager, AutonomousEvolution, AutonomyProfileManager, TrustElevationTracker), graduated governance, LLM-supervised execution tiers, token-budgeted context assembly with decay, Playbook system, safety architecture (ExternalOperationGate, AdaptiveTrust), memory architecture (SemanticMemory, TopicMemory, WorkingMemoryAssembler)
- **Documentation site**: https://instar.sh
- **Files read**: README.md, CLAUDE.md, package.json, `.claude/settings.json`, all hook templates (`src/templates/hooks/*`), scaffold templates, all skills (`skills/*/SKILL.md`), `docs/LLM-SUPERVISED-EXECUTION.md`, `docs/CURRENT-STATUS.md`, `src/data/http-hook-templates.ts`, `src/core/EvolutionManager.ts`, `src/core/AutonomousEvolution.ts`, `src/core/AutonomyProfileManager.ts`, `src/core/AdaptiveTrust.ts`, `src/core/TrustElevationTracker.ts`, `src/core/AdaptationValidator.ts`, `src/core/ExternalOperationGate.ts`, `src/core/TrustRecovery.ts`, `src/core/RelationshipManager.ts`, `src/memory/*`, `src/knowledge/*`, `src/security/*`, `src/monitoring/*`, `playbook-scripts/*`, `docs/PROP-memory-architecture.md`, `docs/PLAYBOOK-*.md`, `docs/THREADLINE*.md`, `docs/UX-AND-AGENT-AGENCY-STANDARD.md`

### OpenClaw

- **Repository**: https://github.com/openclaw/openclaw
- **Stars**: 334K+ (March 2026)
- **License**: MIT
- **Author**: Peter Steinberger
- **Impact**: Gateway daemon pattern, hybrid memory search (vector + FTS + MMR + temporal decay), composite session keys, hot-reloadable config, auto-capture decisions pattern, skill-creator meta-skill, ExecApprovalManager with anti-replay, graduated sandbox tiers, channel abstraction layer, Plugin SDK architecture

### Gas Town

- **Repository**: https://github.com/steveyegge/gastown
- **Stars**: 12.8K+ (March 2026)
- **License**: MIT
- **Author**: Steve Yegge
- **Language**: Go (94.9%)
- **Impact**: Git-backed state (beads), validation ratchets pattern, ephemeral agents with persistent state, eventual consistency model, hierarchical agent delegation (Mayor/Polecat), self-development pattern (Gas Town agents work on Gas Town), "work propels itself" architecture

### Claude Agent SDK

- **Python**: https://github.com/anthropics/claude-agent-sdk-python (5.7K stars)
- **TypeScript**: https://github.com/anthropics/claude-agent-sdk-typescript (1K stars)
- **Packages**: `claude-agent-sdk` (pip), `@anthropic-ai/claude-agent-sdk` (npm)
- **Impact**: `query()` and `ClaudeSDKClient` programming models, understanding that SDK wraps Claude Code CLI subprocess, `CLAUDECODE=1` environment variable nesting issue, session history API, typed message streaming, cost tracking via `total_cost_usd`

### Google A2A Protocol

- **Repository**: https://github.com/a2aproject/A2A (22.8K stars)
- **Specification**: https://a2a-protocol.org
- **Version**: v1.0 (released 2026-03-12)
- **Governance**: Linux Foundation
- **SDK**: `@a2a-js/sdk` (npm, 1M weekly downloads)
- **Impact**: Agent Card discovery pattern, Task lifecycle model, A2A vs MCP complementary positioning, security model (OAuth 2.0, JWT, mTLS, signed Agent Cards), future integration path for multi-model agent coordination

## Research Papers (Full Content Read)

Papers marked with `[FULL]` were fetched as complete HTML from arxiv and deeply analyzed.
Papers marked with `[SUMMARY]` were processed via abstracts, blog coverage, and secondary sources.
Detailed implementation-grade analysis files are in `docs/research/`.

### SICA (Self-Improving Coding Agent) `[FULL]`

- **Title**: "Self-Improving Coding Agent"
- **Authors**: Robeyns, Szummer, Aitchison
- **Institution**: University of Bristol / iGent
- **Date**: April 2025
- **ArXiv abstract**: https://arxiv.org/abs/2504.15228
- **ArXiv full HTML**: https://arxiv.org/html/2504.15228v1
- **Source code**: https://github.com/MaximeRobeyns/self_improving_coding_agent (runner.py, overseer.py, archive_analysis.py read)
- **Detailed analysis**: `docs/research/sica-self-improving-coding-agent.md` (22KB, 489 lines)
- **Impact**: Proved single agent self-editing works (SWE-bench 17% -> 53%). No meta-agent needed. Utility function: `U = 0.5*score + 0.25*(1 - cost/$10) + 0.25*(1 - time/300s)`. Async overseer (Sonnet 3.7, 60s checks). Docker-containerized self-modification. Sub-agent architecture (coder, problem-solver, reasoner, orchestrator, archive explorer, review committee).

### Darwin Godel Machine `[FULL]`

- **Title**: "Darwin Godel Machine: Open-Ended Self-Improvement by Natural Selection of Self-Modifying Agents"
- **Authors**: Zhang, Hu, Lu, Lange, Clune
- **Institutions**: UBC, Sakana AI, Meta FAIR
- **Date**: May 2025 (ICLR 2026)
- **ArXiv abstract**: https://arxiv.org/abs/2505.22954
- **ArXiv full HTML**: https://arxiv.org/html/2505.22954v1
- **Detailed analysis**: `docs/research/dgm-darwin-godel-machine.md` (27KB, 493 lines) -- covers both DGM and DGM-Hyperagent
- **Impact**: Evolutionary archive of self-modified agents (SWE-bench 20% -> 50%). Append-only, never-pruned archive. Selection via softmax over fitness + inverse child count (exploitation + exploration). Discovered improvements: patch validation, granular file editing, multi-attempt generation, peer-review mechanisms.

### DGM-Hyperagent `[FULL]`

- **Title**: "DGM-Hyperagent: Metacognitive Self-Modification"
- **Institution**: Meta
- **Date**: March 2026
- **ArXiv abstract**: https://arxiv.org/abs/2603.19461
- **ArXiv full HTML**: https://arxiv.org/html/2603.19461v1
- **Detailed analysis**: `docs/research/dgm-darwin-godel-machine.md` (second half of file)
- **Impact**: Meta-level itself is editable Python code -- breaks "who improves the improver" infinite regress. Autonomously discovered: persistent memory, performance tracking, label bias detection, compute-aware strategy, failure analysis infrastructure. Transfer: meta agents from paper-review + robotics achieved improvement@50 = 0.63 on novel Olympiad math domain.

### Godel Agent `[FULL]`

- **Title**: "Godel Agent: A Self-Referential Agent Framework for Recursively Self-Improvement"
- **Authors**: Yin, Wang, Pan, Lin, Wan, Wang
- **Institutions**: Peking University, UCSB
- **Date**: October 2024 (ACL 2025)
- **ArXiv abstract**: https://arxiv.org/abs/2410.04444
- **ArXiv full HTML**: https://arxiv.org/html/2410.04444v1
- **Detailed analysis**: `docs/research/maria-os-godel-agent.md` (second half of file)
- **Impact**: Python runtime monkey-patching. Self-referential loop: `pi_{t+1}, I_{t+1} = I_t(pi_t, I_t, r_t, g)`. Six core actions. On Game of 24: switched from LLM to search-based approach after 6 failures, achieving 100%. Cost: $15 for 30 recursive improvements vs $300 for Meta Agent Search.

### AgentDevel `[FULL]`

- **Title**: "AgentDevel: Reframing Self-Evolving LLM Agents as Release Engineering"
- **Date**: January 2026
- **ArXiv abstract**: https://arxiv.org/abs/2601.04620
- **ArXiv full HTML**: https://arxiv.org/html/2601.04620v1
- **Detailed analysis**: `docs/research/agentdevel-release-engineering.md` (17KB, 362 lines)
- **Impact**: Agent improvement as release engineering. Three-stage iteration: Run & Observe, Diagnose & Synthesize, Flip-Centered Gating. Flip classification: P2P/P2F/F2P/F2F. Gate function with 5 acceptance criteria. Regression rate formula: `rho^P2F = |P2F| / (|passes| + epsilon)`. WebArena: 3.1% P2F with zero bad releases vs no gating: 14.8% P2F with multiple bad releases.

### Group-Evolving Agents (GEA) `[FULL]`

- **Title**: "Group-Evolving Agents"
- **Authors**: UC Santa Barbara
- **Date**: February 2026
- **ArXiv abstract**: https://arxiv.org/abs/2602.04837
- **ArXiv full HTML**: https://arxiv.org/html/2602.04837v1
- **Detailed analysis**: `docs/research/gea-group-evolving-agents.md` (21KB, 470 lines)
- **Impact**: Group is the evolutionary unit. Shared experience pool + reflection LLM generates directives. SWE-bench 20% -> 71.0% (vs 71.8% human SOTA). Performance-Novelty parent selection: `alpha * performance + (1-alpha) * novelty`. Best agent drew from 17 unique ancestors (28.3% of population). Improvements are framework-level (not model-specific), transfer across Claude/GPT families. Two-phase: offline evolution R&D, then deploy single agent at standard cost.

### MARIA OS SMAS `[FULL]`

- **Title**: "Self-Modifying Agent System"
- **Source**: https://os.maria-code.ai/blog/self-modifying-agent-system
- **Date**: March 2026
- **Detailed analysis**: `docs/research/maria-os-godel-agent.md` (first half of file, 25KB, 566 lines total)
- **Impact**: Formal modification operator: `M(t+1) = M(t) + delta_M(t)` constrained to subspace S. Lyapunov energy: `V(M) = SUM w_a * l_a(M)` must strictly decrease by epsilon per step. Bounded termination: `N_max = floor(V(M(0)) / epsilon)` (~50-200 in practice). 5-stage pipeline: Detect -> Propose -> Validate -> Apply (atomic) -> Verify (1h/24h/72h windows with auto-rollback). Four artifact types: ToolDefinition, CommandTemplate, WorkflowDefinition, DecisionRule.

### Agyn `[FULL]`

- **Title**: "Agyn: A Multi-Agent System for Team-Based Autonomous Software Engineering"
- **Date**: February 2026
- **ArXiv abstract**: https://arxiv.org/abs/2602.01465
- **ArXiv full HTML**: https://arxiv.org/html/2602.01465v1
- **Detailed analysis**: `docs/research/mas-orchestration-and-patterns.md`
- **Impact**: Manager + engineer + reviewer + researcher team structure (72.2% SWE-bench, +7.4% over single-agent). Dynamic coordination, not fixed pipelines. Structured artifacts, not chat. Role-specific model allocation.

### MAS Orchestration Survey `[FULL]`

- **Title**: "The Orchestration of Multi-Agent Systems: Architectures, Protocols, and Enterprise Adoption"
- **Date**: January 2026
- **ArXiv abstract**: https://arxiv.org/abs/2601.13671
- **ArXiv full HTML**: https://arxiv.org/html/2601.13671v1
- **Detailed analysis**: `docs/research/mas-orchestration-and-patterns.md`
- **Impact**: Five-subsystem orchestration layer (Planning, Policy, Execution/Control, State/Knowledge, Quality). Hierarchical orchestrator-worker recommended for 20+ agents. MCP + A2A as complementary protocols.

### LLM-based MAS Design Patterns `[FULL]`

- **Title**: "Designing LLM-based Multi-Agent Systems for Software Engineering Tasks"
- **Date**: November 2025
- **ArXiv abstract**: https://arxiv.org/abs/2511.08475
- **ArXiv full HTML**: https://arxiv.org/html/2511.08475v1
- **Detailed analysis**: `docs/research/mas-orchestration-and-patterns.md`
- **Impact**: 16 design patterns from 94 papers. Role-Based Cooperation (46.8%) and Self-Reflection (36.2%) dominate. CooperBench WARNING: agents achieve ~50% lower success collaborating vs solo. Orchestrator-mediated coordination essential.

### Self-Improving Agents at Test-Time (TT-SI) `[FULL]`

- **Title**: "Self-Improving Agents at Test-Time"
- **Date**: October 2025
- **ArXiv abstract**: https://arxiv.org/abs/2510.07841
- **ArXiv full HTML**: https://arxiv.org/html/2510.07841v1
- **Detailed analysis**: `docs/research/artemis-and-evolution-surveys.md`
- **Impact**: Three-stage: uncertainty detection (Relative Softmax Scoring), self-data augmentation, LoRA fine-tuning. Key finding: uncertainty-guided selective adaptation (+5.48%) vastly outperforms uniform (+1.04%). Focus evolution on struggling task categories.

### ARTEMIS `[FULL]`

- **Title**: "ARTEMIS: Evolving Excellence via Automated Optimization of LLM-based Agents"
- **Date**: December 2025
- **ArXiv abstract**: https://arxiv.org/abs/2512.09108
- **ArXiv full HTML**: https://arxiv.org/html/2512.09108v1
- **Detailed analysis**: `docs/research/artemis-and-evolution-surveys.md`
- **Impact**: Agent config formalized as `C = (P, T, M, Theta)` over mixed-type search space. Semantic GA with LLM-ensemble mutation/crossover for NL components, Bayesian for numeric. Hierarchical evaluation (cheap LLM scoring before expensive benchmarks). 10-36% improvements across four agent systems.

### Survey of Self-Evolving Agents `[FULL]`

- **Title**: "A Survey of Self-Evolving Agents"
- **ArXiv abstract**: https://arxiv.org/abs/2507.21046
- **ArXiv full HTML**: https://arxiv.org/html/2507.21046v4
- **Detailed analysis**: `docs/research/artemis-and-evolution-surveys.md`
- **Impact**: Comprehensive taxonomy: what/when/how/where to evolve. Inter-test-time evolution of context (prompts + memory) is lowest-risk, highest-impact starting point. Population-based methods validated across field.

## Community Sources and Articles

### Claude Code GitHub Issues (CLAUDE.md Compliance)

- **Issue #19635**: "Claude Code ignores CLAUDE.md rules repeatedly" (4 thumbs-up) -- https://github.com/anthropics/claude-code/issues/19635
- **Issue #32161**: "Systematically ignores CLAUDE.md knowledge retrieval rules" -- https://github.com/anthropics/claude-code/issues/32161
- **Issue #34197**: "Claude Code continually ignores CLAUDE.MD file" -- https://github.com/anthropics/claude-code/issues/34197
- **Issue #35309**: "Claude Code disregards the stored instructions" -- https://github.com/anthropics/claude-code/issues/35309
- **Issue #10683**: "CLAUDE.md user instructions are ignored" (4 thumbs-up) -- https://github.com/anthropics/claude-code/issues/10683
- **Issue #37626**: "Horizontal communication between subagents" -- https://github.com/anthropics/claude-code/issues/37626
- **Issue #34556**: "59 compactions across 26 days" memory persistence request -- https://github.com/anthropics/claude-code/issues/34556
- **Issue #34358**: "24+ hooks, 400-line CLAUDE.md, still ignores rules" -- https://github.com/anthropics/claude-code/issues/34358

### Blog Posts and Articles

- **DEV Community**: "I Wrote 200 Lines of Rules for Claude Code. It Ignored Them All."
- **Gary Dotzlaw / Christopher Montes**: "Your CLAUDE.md Is a Suggestion. Hooks Make It Law." -- hooks vs instructions analysis
- **Blake Crosley**: Claude Code as Infrastructure -- 95 hooks, "best hooks come from incidents not planning"
- **Marco Patzelt**: "Hooks as Guarantees" (February 2026) -- deterministic enforcement patterns
- **Morph**: 15-agent benchmark test -- "The same model scores 17 problems apart in different agents. The scaffolding matters more than the model."

### Industry Reports

- **Anthropic**: "2026 Agentic Coding Trends Report" -- https://resources.anthropic.com/hubfs/2026%20Agentic%20Coding%20Trends%20Report.pdf
- **Trilogy AI**: Gas Town architectural deep-dive
- **Epsilla**: "Features can be copied. Ecosystems cannot." -- OpenClaw vs Claude Code analysis

## Other Referenced Projects

### Kelos

- **Repository**: https://github.com/kelos-dev/kelos (86 stars)
- **License**: Apache-2.0
- **Language**: Go
- **Impact**: Kubernetes-native agent orchestration. Self-developing: 7 always-on TaskSpawners improve Kelos itself. Four primitives: Tasks, Workspaces, AgentConfigs, TaskSpawners.

### NTM (Neural Tmux Manager)

- **Repository**: https://github.com/Dicklesworthstone/ntm (191 stars)
- **Language**: Go
- **Impact**: tmux-based multi-agent command center with `--robot-*` JSON API, agent spawning, broadcast messaging, CASS session indexing.

### ClaudeClaw (moazbuilds) `[FULL]`

- **Repository**: https://github.com/moazbuilds/claudeclaw (592 stars)
- **Detailed analysis**: `docs/research/agent-skills-and-claude-wrappers.md`
- **Impact**: PID-file daemon, serial execution queue (sessions cannot be concurrently resumed), system prompt re-injection on every --resume (CRITICAL: --append-system-prompt does NOT persist), auto-compact on timeout, rate-limit detection via stdout regex with model fallback, keyword-based model router, SKILL.md discovery. **Key constraint discovered: system prompt must be re-injected on every resume call.**

### hcom

- **Repository**: https://github.com/aannoo/hcom (145 stars)
- **Language**: Python/Rust
- **Impact**: Inter-agent message bus with `@mention` messaging, transcript reading, file collision detection, cross-device relay.

### Agent Skills Standard `[FULL]`

- **Website**: https://agentskills.io
- **Detailed analysis**: `docs/research/agent-skills-and-claude-wrappers.md`
- **Impact**: YAML frontmatter schema (6 fields: name, description, license, compatibility, metadata, allowed-tools). Three-tier progressive disclosure (catalog/instructions/resources). Adopted by 20+ platforms including Microsoft Agent Framework. Created by Anthropic Oct 2025, open-standardized Dec 2025.

### Clawith `[FULL]`

- **Repository**: https://github.com/dataelement/Clawith (2.3K stars)
- **Detailed analysis**: `docs/research/agent-skills-and-claude-wrappers.md`
- **Impact**: "Aware" system: focus items with 6 trigger types (cron, once, interval, poll, on_message, webhook). Three-layer identity: soul.md (personality), state.json (runtime), memory/ directory. HEARTBEAT.md behavior template with 4 phases (review, explore, plaza, wrap-up). Plaza for organizational knowledge sharing. 30+ backend services.

### Awesome AI Agents 2026 `[FULL]`

- **Repository**: https://github.com/caramaschiHG/awesome-ai-agents-2026 (151 stars)
- **Detailed analysis**: `docs/research/mas-orchestration-and-patterns.md`
- **Impact**: 300+ agent tools across 20+ categories. No mature Rust multi-agent framework exists (swarms-rs 134 stars, Rig emerging). Validates building our own.
- **Impact**: Curated list of 300+ AI agent tools across 20+ categories.

## Commercial Solutions Referenced

| Product | Company | Relevance |
| --- | --- | --- |
| Devin 2.0 | Cognition | Fully autonomous cloud agent, 67% PR merge rate, $20/mo |
| Cursor | Anysphere | IDE-native AI copilot, 360K paying users, $29.3B valuation |
| Windsurf | Cognition (acquired) | Parallel code generation, $15/mo |
| Claude Code | Anthropic | Terminal-native, 80.9% SWE-bench, 17 hookable lifecycle events |
| Codex CLI | OpenAI | Terminal-native, 77.3% Terminal-Bench, 240+ tok/s |
| Google Antigravity | Google | 76.2% SWE-bench, free tier |

## Additional Sources (Full Content Read)

### Rust Self-Evolving Agent `[FULL]`

- **Source**: https://docs.bswen.com/blog/2026-03-05-self-evolving-ai-agent
- **Date**: March 2026
- **Detailed analysis**: `docs/research/rust-evolution-and-workspace-patterns.md`
- **Impact**: Rust agent grew from 200 to 1,500 lines via self-modification. Test-before-commit with git stash rollback. SQLite journal for failure deduplication. Budget manager for cost control. Ripgrep-based context management. GitHub Issues as task queue. Forbidden zones in system prompt. **Key pattern: two-process architecture (daemon + watchdog) prevents self-bricking.**

### Self-Modifying Workspace Patterns `[PARTIAL]`

- **Source**: https://docs.bswen.com/blog/2026-03-13-ai-agent-self-modifying-workspace
- **Date**: March 2026
- **Detailed analysis**: `docs/research/rust-evolution-and-workspace-patterns.md`
- **Note**: Content partially accessible; key patterns incorporated from preview.

### MARIA OS SEAA (Self-Extending Agent Architecture) `[FULL]`

- **Source**: https://os.maria-code.ai/blog/self-extending-agent-architecture
- **Date**: March 2026
- **Detailed analysis**: `docs/research/rust-evolution-and-workspace-patterns.md`
- **Impact**: Agents detect capability gaps, synthesize new tools, validate in sandbox, register into runtime. Agent state 4-tuple: X_t = (C, T, M, R). Self-extension operator: X_{t+1} = E_t ∘ G_t ∘ J_t(X_t). Capability Monotonicity Theorem: capabilities never shrink under validation gates. Tool synthesis 87.2% first-attempt success. Gap detection <200ms for 10K capabilities. Risk-gated: Risk(τ) = w1*Scope + w2*Reversibility + w3*DataSensitivity. Tool sharing protocol enables superlinear organizational capability growth.

### Multi-Agent AI Systems Study `[SUMMARY]`

- **Title**: "A Large-Scale Study on the Development and Issues of Multi-Agent AI Systems"
- **Date**: January 2026
- **ArXiv**: https://arxiv.org/abs/2601.07136
- **Note**: Covered via secondary analysis in MAS orchestration patterns research.
- **Impact**: 42K+ commits, 4.7K+ resolved issues across 8 MAS systems. Perfective commits 40.8% of changes.

### "AI Self-Improvement Only Works Where Outcomes Are Verifiable" `[SUMMARY]`

- **Impact**: Core principle validated by SICA, DGM, and GEA results. Scaffold improvements converge; model-capability improvements do not.
