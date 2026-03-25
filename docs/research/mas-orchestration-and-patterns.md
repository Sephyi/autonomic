# Multi-Agent System Orchestration and Design Patterns

Research synthesis for building a Rust-based orchestrator managing Claude Code instances across multiple projects.

Sources analyzed:

1. Adimulam et al., "The Orchestration of Multi-Agent Systems" (arXiv:2601.13671, Jan 2026)
2. Benkovich & Valkov, "Agyn: A Multi-Agent System for Team-Based Autonomous Software Engineering" (arXiv:2602.01465, Feb 2026)
3. Cai et al., "Designing LLM-based Multi-Agent Systems for SE Tasks: Quality Attributes, Design Patterns and Rationale" (arXiv:2511.08475, Nov 2025)
4. awesome-ai-agents-2026 (github.com/caramaschiHG/awesome-ai-agents-2026, March 2026)

## Source 1: MAS Orchestration Survey (arXiv:2601.13671)

### Core Thesis

Orchestrated multi-agent systems require a unified architectural framework integrating planning, policy enforcement, state management, and quality operations into a coherent orchestration layer. Two complementary protocols -- MCP (Model Context Protocol) and A2A (Agent-to-Agent) -- form the interoperable communication substrate.

### Architectural Composition

A well-designed MAS has three foundational layers:

1. **Specialized Agents** -- autonomous components executing role-specific tasks
2. **Orchestration Layer** -- the control plane coordinating agents into goal-directed collectives
3. **Communication Protocols** -- standardized information exchange (MCP + A2A)

### Agent Specialization Categories

Three categories of specialized agents, each relevant to our orchestrator:

| Category | Role | Our Mapping |
| --- | --- | --- |
| **Worker Agents** | Execute well-defined tasks (stateless or stateful). Operate in parallel, each specialized in a narrow sub-domain. | Claude Code instances doing implementation work |
| **Service Agents** | Shared operational utilities: quality assurance, compliance enforcement, diagnostics, automated recovery (healing agents), upgrade scheduling. | Linting/testing agents, health monitors, CI/CD integration |
| **Support Agents** | Meta-level oversight: monitoring system behavior, analyzing outcomes, managing data flows. Track decision latency, detect drift, visualize health. | Orchestrator's monitoring subsystem, analytics |

### Orchestration Layer -- Five Subsystems

This is the most implementation-relevant section. The orchestration layer decomposes into:

#### V-A: Planning and Policy Management

- **Planning Unit**: Goal-decomposition engine. Determines what tasks to do, in what order.
- **Policy Unit**: Embeds domain/governance constraints. Defines how tasks are performed.
- Together they produce: who performs which task, in what sequence, under what rules, with what oversight.

**Applicability**: Our orchestrator needs both. Planning = task decomposition across projects. Policy = quality gates, cost limits, model routing rules.

#### V-B: Execution and Control Management

- **Execution Unit**: Manages smooth operation of worker agents, collects telemetry from support agents.
- **Control Unit**: Manages concurrency and dependency across workflows. Handles parallel execution and synchronization at key checkpoints. Task prioritization and dynamic resource allocation.

**Applicability**: This maps to our task scheduler. Must handle concurrent Claude Code instances, checkpoint synchronization (e.g., "wait for all tests to pass before merging"), and dynamic resource allocation (rate limits, cost budgets).

#### V-C: State and Knowledge Management

- **State Unit**: Checkpoints, workflow progress, agent states, activity logs. Service agents restore checkpoints to preserve workflow integrity.
- **Knowledge Unit**: Contextual and domain-specific information. Connects to external data sources, exposes retrievable context.
- Separation of operational state from knowledge state preserves modularity and coherence.

**Applicability**: Critical for our system. State = which projects are in-flight, what stage each is at, git state. Knowledge = project-specific context (PRDs, architecture docs, test results).

#### V-D: Quality and Operations Management

- Validates aggregated outputs against defined schemas before integrating into shared state.
- Prevents invalid data from propagating through workflows.
- Monitors latency, throughput, success rate. Anomaly detection triggers preemptive interventions.
- Supports controlled deployment, testing, and sandboxing new components.

**Applicability**: Maps to our quality gates -- code review verification, test pass rates, lint checks. Also: monitoring Claude Code instance health, detecting stuck agents, cost anomaly detection.

### MCP and A2A: Complementary Protocols

#### MCP (Model Context Protocol)

Standardizes how agents access external tools and contextual data. "USB-C for AI."

Three primitives:

| Primitive | Analogy | Control Plane | Purpose |
| --- | --- | --- | --- |
| **Tools** | POST request | Model-controlled | Actions: search, send email, create record |
| **Resources** | GET request | Application-controlled | Read-only data: configs, schemas, docs |
| **Prompts** | Templates | User-controlled | Reusable interaction patterns |

2026 enhancements: multimodal support (images, video, audio), managed hosting by cloud providers, Linux Foundation governance.

**Applicability**: Our orchestrator exposes MCP servers for Claude Code instances to access project context, CI results, and cross-project knowledge. Each Claude Code instance is already an MCP client.

#### A2A (Agent-to-Agent Protocol)

Standardizes how agents talk to each other. Google-led, 50+ partners (Atlassian, Salesforce, SAP). Also under Linux Foundation.

Core concepts:

1. **Agent Cards**: JSON metadata describing capabilities, accepted inputs, auth requirements, endpoint URLs. Enable discovery -- a coordinator can query a registry to find the right specialist.
2. **JSON-RPC 2.0 over HTTP(S)**: Simple wire protocol for inter-agent communication.
3. **Task lifecycle**: Structured task states for tracking delegation and completion.

**Applicability**: Our orchestrator could publish Agent Cards for each Claude Code instance describing their project specialization. A2A gives us a standard for the orchestrator-to-agent communication protocol rather than inventing our own.

### Orchestration Architecture Patterns

From the survey and supplementary sources, five production patterns emerge:

| Pattern | Control | Scalability | Fault Tolerance | Best For |
| --- | --- | --- | --- | --- |
| **Orchestrator-Worker** | High | Medium (bottlenecked by orchestrator) | Low (SPOF) | Task decomposition, fan-out |
| **Swarm** | Low | High (no bottleneck) | High | Unknown solution paths, exploration |
| **Mesh** | Medium | Low (N-squared connections) | Medium | 3-8 agents iterating on shared artifact |
| **Hierarchical** | High | High (tree scales logarithmically) | Medium (branch isolation) | 20+ agents, multi-domain enterprise |
| **Pipeline** | High | Medium | Low (stage failure halts all) | Fixed sequential processing |

**Decision for our system**: Hierarchical with Orchestrator-Worker at leaf level. Our orchestrator is the top-level manager. Each project gets an orchestrator-worker team (lead Claude Code + specialist sub-agents). This scales to 20+ projects while maintaining control and observability.

### Enterprise Adoption Signals

- PwC Agent OS: switchboard for multi-agent coordination emphasizing composability
- Accenture Trusted Agent Huddle: governance for secure cross-organizational workflows
- 57% of orgs have agents in production (LangChain State of Agent Engineering)
- 72% of enterprise AI projects use multi-agent architectures (2025 data)

## Source 2: Agyn -- Team-Based Autonomous SE (arXiv:2602.01465)

### Core Thesis

Software engineering should be modeled as an organizational process with role separation, not as a monolithic or pipeline task. A team of manager + engineer + reviewer + researcher agents, each with isolated sandboxes, achieves 72.2% on SWE-bench 500 -- outperforming single-agent baselines by 7.4%.

**Key insight**: "Future progress may depend as much on organizational design and agent infrastructure as on model improvements."

### Team Structure

Four specialized roles:

| Role | Model | Responsibilities | Tools |
| --- | --- | --- | --- |
| **Manager** | GPT-5 (large, strong reasoning) | Coordinates workflow, decomposes tasks, assigns work, decides when done | Communication with all agents, methodology enforcement |
| **Researcher** | GPT-5 (large context) | Issue analysis, repository exploration, solution planning, task specification | Code search, file browsing, documentation access |
| **Engineer** | GPT-5-Codex (smaller, code-specialized) | Implementation, debugging, test execution | Shell access, code editor, isolated sandbox |
| **Reviewer** | GPT-5-Codex (code-specialized) | Inline code review, approval/rejection of PRs | PR review tools, code analysis |

**Critical design choice**: Role-specific model allocation. Reasoning-heavy roles (manager, researcher) use large general-purpose models. Implementation roles (engineer, reviewer) use smaller, cheaper, code-specialized models. This balances quality, cost, and execution efficiency.

### Communication Patterns

- Agents are configured independently with no shared global context or prompt.
- Coordination handled explicitly by the manager agent.
- The system does NOT follow a fixed pipeline -- interaction patterns emerge dynamically based on intermediate outcomes.
- The number of interaction steps, revisions, and review cycles is not predetermined.

**Structured communication over dialogue**: Agents exchange structured artifacts (task specs, PR diffs, review comments) rather than freeform natural language chat. This aligns with MetaGPT's finding that structured handoffs beat conversational back-and-forth (MetaGPT achieves 85.9% Pass@1 vs dialogue-based approaches).

### Execution Environment Design

Each agent operates in its own isolated sandbox:

- Separate filesystem, network, and secrets per agent
- Agents can independently modify code, run tests, explore alternatives
- Environments are first-class components aligned with agent roles
- Enables parallel exploration and controlled integration

**Key finding**: Starting agents from empty environments proved more effective than preconfigured setups. Agents use Nix to install dependencies as needed, avoiding implicit assumptions that conflict with project-specific requirements.

**Output management**: When command outputs exceed 50,000 tokens, they are automatically redirected to files rather than overwhelming the model context.

### GitHub-Native Workflow (Methodology)

The manager follows a defined methodology without prescribing exact steps:

1. **Research and understanding** -- Researcher analyzes issue, explores repository
2. **Task specification** -- Researcher produces structured spec of what to change and why
3. **Issue formulation** -- Manager formulates approach as GitHub issue
4. **Implementation via pull requests** -- Engineer creates PR with changes
5. **Iterative review cycles** -- Reviewer provides inline code review, Engineer iterates
6. **Approval** -- Reviewer explicitly approves when solution is acceptable

This mirrors real-world development: issues -> PRs -> code review -> merge.

### Code Review Mechanism

- Custom tooling enables agents to perform inline code reviews and manage PRs autonomously
- Reviewer provides specific, actionable feedback on code diffs
- Engineer iterates on feedback until Reviewer approves
- The review loop is the primary quality gate

### Applicability to Our Orchestrator

| Agyn Pattern | Our Implementation |
| --- | --- |
| Manager agent coordinates dynamically | Our Rust orchestrator serves as the manager |
| Role-specific model allocation | Route planning tasks to Opus, implementation to Sonnet/Haiku |
| Isolated sandboxes per agent | Each Claude Code instance in its own git worktree/container |
| Structured artifacts over chat | JSON task specs, structured status reports, typed messages |
| GitHub-native workflow | Our orchestrator drives the issue -> PR -> review -> merge cycle |
| Empty environment start | Each Claude Code instance starts clean, discovers project context |
| Iterative review until approval | Automated review agent or dialectic verification before merge |

### Performance Results

- 72.2% resolution on SWE-bench 500 (fully automated, no human intervention)
- 7.4% higher than mini-SWE-agent baseline with comparable models
- Competitive with OpenHands + GPT-5 (71.8%)
- System was NOT tuned for benchmark -- designed for production use

## Source 3: LLM-based MAS Design Patterns (arXiv:2511.08475)

### Core Thesis

Systematic study of 94 papers identifies 16 design patterns, 10 SE task categories, and quality attribute priorities for LLM-based multi-agent systems. Role-Based Cooperation is the dominant pattern; Functional Suitability (correctness) is the primary quality attribute.

### SE Tasks Addressed by LLM-based MASs

Ten categories identified. Code Generation is most common, followed by:

1. Code Generation (most common)
2. End-to-end Development
3. End-to-end Maintenance
4. Testing
5. Bug Repair / Fault Localization
6. Code Review
7. Requirements Engineering
8. Architecture Design
9. Code Translation
10. Release Management

### Quality Attributes Taxonomy

Based on ISO 25010, the QAs designers focus on:

| Quality Attribute | Focus Level | Description |
| --- | --- | --- |
| **Functional Suitability** | Highest | Output correctness -- does the generated code work? |
| **Reliability** | High | Error handling, fault tolerance, recovery |
| **Performance Efficiency** | Medium | Execution time, resource usage, throughput |
| **Maintainability** | Low (gap!) | Evolving agent prompts, roles, protocols as requirements change |
| **Security** | Low (gap!) | Protection against prompt injection, data leakage |
| **Scalability** | Low (gap!) | Coordination overhead, latency under load |

**Critical gaps identified**: MAS performance/scalability, maintainability, and security receive far less attention than correctness. These are precisely the concerns we must address in a production orchestrator.

### The 16 Design Patterns (Five Categories)

#### Cooperation Patterns (how agents divide and coordinate work)

| Pattern | Frequency | Description |
| --- | --- | --- |
| **Role-Based Cooperation** | 46.8% (most common) | Agents with distinct functional roles (coder, reviewer, tester) collaborate. Clear task allocation and coordination. |
| **Debate-Based Cooperation** | 4.3% | Multiple agents argue for different approaches, surface competing perspectives before convergence. |
| **Layered-Based Cooperation** | 4.3% | Agents organized in layers; each layer handles a different abstraction level. |
| **Voting-Based Cooperation** | 3.2% | Multiple agents independently produce outputs; majority or synthesized answer accepted. |
| **Hierarchical Coordination** | 3.2% | Orchestrator agents direct worker agents; workers report structured results. Simple tasks to L1 agents, complex to L2+. |

#### Reflection Patterns (how agents validate and improve outputs)

| Pattern | Frequency | Description |
| --- | --- | --- |
| **Self-Reflection** | 36.2% (second most common) | Agent iteratively evaluates and improves its own output. Automated critique-revise loop. |
| **Cross-Reflection** | 12.8% | A separate agent critiques another agent's output (peer review). |
| **Human-Reflection** | 5.3% | Keeps user in the loop with repeated feedback on designs and implementations. |

#### Planning Patterns (how agents decompose and sequence work)

| Pattern | Frequency | Description |
| --- | --- | --- |
| **Single-Path Plan Generator** | 5.3% | Generates one linear execution plan decomposed into sub-tasks. |
| **Multi-Path Plan Generator** | 3.2% | Generates multiple alternative plans; selects best or explores in parallel. |

#### Infrastructure Patterns (how agents access tools and context)

| Pattern | Frequency | Description |
| --- | --- | --- |
| **Tool-Agent Registry** | 14.9% | Registry of available tools/agents. Dynamic discovery and invocation. |
| **Retrieval-Augmented Generation (RAG)** | 10.6% | Agents query vector stores for grounding facts before generation. |
| **Agent Adapter** | 5.3% | Wraps heterogeneous agents/tools behind a uniform interface. |
| **Prompt/Response Optimiser** | 4.3% | Optimizes prompts or post-processes responses for quality. |
| **Agent Evaluator** | 3.2% | Dedicated agent that evaluates other agents' outputs against criteria. |
| **Incremental Model Querying** | 3.2% | Queries model in smaller increments to stay within context limits. |

#### Memory Patterns (from supplementary taxonomy)

| Pattern | Description |
| --- | --- |
| **Shared Memory** | Agents read/write to a common knowledge store |
| **Individual Memory** | Each agent maintains private state; sharing is explicit |
| **External Memory** | Agents offload long-term state to databases/files outside context window |

#### Communication Patterns (from supplementary taxonomy)

| Pattern | Description |
| --- | --- |
| **Structured Message Passing** | Agents exchange typed, schema-validated payloads |
| **Shared Workspace** | Agents communicate through shared artifacts (files, tickets, code) |

### Design Pattern Combinations for Code Generation

The most effective combination for code generation (our primary use case):

- **Role-Based Cooperation** (coder/reviewer/tester split) + **Self-Reflection** (iterative improvement) + **Cross-Reflection** (peer review)
- Primary rationale: Improving the Quality of Generated Code

### Patterns Most Applicable to Our Orchestrator

| Pattern | How We Use It |
| --- | --- |
| Role-Based Cooperation | Each Claude Code instance has a defined role (implementer, reviewer, researcher) |
| Hierarchical Coordination | Rust orchestrator -> project leads -> task workers |
| Self-Reflection | Claude Code instances self-verify with tests before reporting completion |
| Cross-Reflection | Separate review agent validates implementation agent's output |
| Tool-Agent Registry | Dynamic registry of available Claude Code instances and their capabilities |
| Agent Adapter | Uniform interface wrapping Claude Code CLI, different model backends |
| External Memory | Persistent state in SQLite/files, cross-session memory via Mem0 |
| Shared Workspace | Git repositories as shared artifacts; file-lock-based claiming |
| Structured Message Passing | JSON-typed messages between orchestrator and agents |

### Key Implications from the Study

1. **Correctness over speed**: Designers prioritize output quality above all else. Our orchestrator should gate on correctness (tests pass, review approved) not speed.
2. **Role separation works**: The coder/reviewer/tester pattern is empirically dominant for good reason -- it catches errors that self-reflection alone misses.
3. **Maintainability gap**: Most MAS designs do not address how to evolve agent configurations over time. Our orchestrator should support versioned agent configs (Agyn uses HCL/Terraform-style config-as-code).
4. **Security gap**: Prompt injection, data leakage, and agent manipulation are under-studied. Our orchestrator needs sandboxing and policy enforcement.

## Source 4: Awesome AI Agents 2026

### Agent Framework Landscape

#### Multi-Agent Orchestration Frameworks

All major frameworks are Python-based:

| Framework | Language | Key Feature |
| --- | --- | --- |
| AutoGen (Microsoft) | Python | Multi-agent conversations |
| CrewAI | Python | Role-based crew members with goals and tools |
| MetaGPT | Python | PM/architect/engineer roles, SOP-based |
| OpenAI Agents SDK | Python | Multi-step agents with handoffs |
| Google ADK | Python | Native Gemini, multi-agent orchestration |
| Strands Agents (AWS) | Python | Model-driven tool use |
| DeerFlow (ByteDance) | Python | Planning, tools, memory, execution |
| AgentScope (Alibaba) | Python | Multi-agent framework |

**Gap**: No mature Rust-based multi-agent orchestration framework exists in this list. This validates the need for our Rust orchestrator.

#### Rust-Based Agent Frameworks Discovered

| Framework | Description | Stars | Status |
| --- | --- | --- | --- |
| **swarms-rs** | Enterprise-grade multi-agent orchestration in Rust. Apache 2.0. Async/Tokio-based. Supports OpenAI, DeepSeek, Anthropic. | 134 | Active (last push Dec 2025) |
| **Rig** | Rust framework for agentic AI. Implements prompt chaining, routing, parallelization, orchestrator-worker, evaluator-optimizer patterns. | - | Active |

**swarms-rs** (github.com/The-Swarm-Corporation/swarms-rs):

- First enterprise-grade multi-agent framework in Rust
- Zero-cost abstractions, fearless concurrency via Tokio
- Supports multiple LLM providers (OpenAI, DeepSeek, Anthropic)
- Agent struct with tools, memory, and autonomous execution
- 97.7% Rust, Apache 2.0 license
- 8 contributors, relatively early stage (134 stars)

**Rig** (dev.to article on implementing agentic patterns):

- Implements all six Anthropic design patterns in Rust: prompt chaining, routing, parallelization, orchestrator-worker, evaluator-optimizer, autonomous agent
- Uses Tokio for async, pipeline abstractions for chaining
- Clean API for creating agents with preambles and tools

#### Rust Parallel Agent Implementation Pattern

From a detailed blog post on Rust vs Claude Code agent teams:

A `team.rs` library (641 lines) implements the complete Claude Code agent-teams model in pure Rust:

- `TaskQueue` -- shared task list with claiming
- `Mailbox` -- inter-worker messaging
- `PlanGate` -- coordination checkpoint
- `ShutdownToken` -- graceful termination

Decision rule: "Do your agents need to talk to each other? If no, `tokio::spawn` + `Arc`. If yes, build team.rs or use TeamCreate."

Static fan-out (compile-time task assignment) vs dynamic coordination (runtime task claiming with file locks) represents the fundamental architectural choice.

### Protocols and Standards

| Protocol | Purpose |
| --- | --- |
| **MCP** (Anthropic) | Agent-to-tool standard. "USB-C for AI." Industry standard. |
| **A2A** (Google) | Agent-to-agent communication. |
| **OpenAI Function Calling** | Native tool-use via JSON schema. |
| **Anthropic Tool Use** | Claude native tool-use. Structured JSON. |
| **OpenAPI** | Foundation for agent tool definitions. |

### Relevant Coding Agent Platforms

| Platform | Relevance |
| --- | --- |
| **Claude Code** | Our primary managed agent. 80.9% SWE-bench. Agent Teams feature. |
| **OpenAI Codex CLI** | Alternative agent backend. Multi-agent via Agents SDK. |
| **Aider** | OSS pair programmer. Git-aware. Any LLM. Potential lightweight agent. |
| **OpenHands** | OSS autonomous SE. Sandboxed runtime. Could inform sandbox design. |
| **Agyn** | Open-source multi-agent SE platform. HCL config. Isolated sandboxes. |

### Observability and Evaluation

| Tool | Purpose |
| --- | --- |
| **Langfuse** | OSS LLM observability: traces, evals, prompts |
| **Arize Phoenix** | OSS AI observability: traces, evals, embeddings |
| **Helicone** | OSS LLM observability: one-line integration |
| **SWE-bench** | Industry standard for coding agent evaluation (top: 80.9% Opus) |

### Market Context (2026)

- Market size: $10.91B in 2026, projected $52.63B by 2030
- 57% of orgs have agents in production
- 85% of devs use AI coding tools regularly
- Top barrier: Quality (32%), Latency (20%)
- Coding market: $4B. Cursor + Copilot + Claude Code = 70%+ share

## Cross-Source Synthesis: Architecture for Our Rust Orchestrator

### Recommended Architecture: Hierarchical Orchestrator-Worker

Based on all four sources, the recommended architecture:

```text
                    +-----------------------+
                    |   Rust Orchestrator    |
                    |   (Planning + Policy   |
                    |    + State + Quality)  |
                    +-----------+-----------+
                                |
              +-----------------+-----------------+
              |                 |                 |
     +--------v------+ +-------v-------+ +------v--------+
     | Project Lead A | | Project Lead B | | Project Lead C |
     | (Claude Code)  | | (Claude Code)  | | (Claude Code)  |
     +--------+------+ +-------+-------+ +------+--------+
              |                 |                 |
        +-----+-----+    +-----+-----+     +----+----+
        |     |     |    |     |     |     |    |    |
      Impl  Rev  Res  Impl  Rev  Res   Impl Rev  Res
```

### Layer Mapping

| Orchestration Subsystem | Implementation |
| --- | --- |
| **Planning Unit** | Task decomposition engine. Reads PRDs/issues, creates DAG of subtasks, assigns to project teams. |
| **Policy Unit** | Quality gates (tests must pass), cost limits (per-project budgets), model routing (Opus for planning, Sonnet for implementation). |
| **Execution Unit** | Tokio-based async runtime. Spawns Claude Code instances, manages concurrency, collects telemetry. |
| **Control Unit** | Dependency management, checkpoint synchronization, rate limit handling, retry logic. |
| **State Unit** | SQLite for workflow state, git worktree state, agent status. Checkpoint/restore for fault tolerance. |
| **Knowledge Unit** | Project context (PRDs, architecture docs), cross-project knowledge (Mem0), RAG for codebase search. |
| **Quality Unit** | Test result validation, code review verification, output schema validation, anomaly detection. |

### Communication Design

| Channel | Protocol | Content |
| --- | --- | --- |
| Orchestrator -> Agent | MCP (tool invocations) or CLI (Claude Code commands) | Task assignments, context injection |
| Agent -> Orchestrator | Structured JSON over stdout/file | Status updates, completion reports, error reports |
| Agent -> Agent | Shared workspace (git) + structured message files | Code changes, review comments, specifications |
| Orchestrator -> Human | Notifications (approval requests, error escalation) | Status dashboards, intervention requests |

### Quality Assurance Pipeline

Drawing from all sources:

1. **Self-Reflection**: Each Claude Code instance runs tests and self-verifies before reporting completion
2. **Cross-Reflection**: Separate reviewer agent examines implementation agent's output
3. **Automated Gates**: Linting (Biome/clippy), type checking, test suite execution
4. **Dialectic Verification**: For critical changes, parallel independent reviews from multiple models
5. **Human Checkpoint**: Configurable approval gates for high-risk changes

### Key Design Decisions Informed by Research

1. **Role-specific model allocation** (Agyn): Use expensive models for planning/research, cheap models for implementation/review. This is the highest-impact cost optimization.

2. **Isolated sandboxes** (Agyn, Claude Code Agent Teams): Each agent in its own git worktree or container. Zero shared state. File-lock-based task claiming prevents race conditions.

3. **Structured communication over dialogue** (MetaGPT, Agyn): Exchange typed JSON specs, not freeform chat. 85.9% vs lower pass rates for dialogue-based approaches.

4. **Dynamic coordination, not fixed pipelines** (Agyn): The manager decides next steps based on intermediate outcomes. Number of review cycles is not predetermined.

5. **Empty environment start** (Agyn): Agents discover project context rather than receiving preconfigured environments. Avoids implicit assumptions.

6. **Config-as-code** (Agyn/Terraform-style): Define agents, tools, and team structure in version-controlled configuration. Reproducible, reviewable, rollback-able.

7. **Hierarchical for scale** (Orchestration Survey): Tree structure scales logarithmically. Domain-specific agent clusters with a top-level strategic coordinator is the only viable option for 20+ agents.

8. **Address the gaps** (Design Patterns study): Maintainability, security, and scalability are under-addressed in existing MAS designs. Our orchestrator must treat these as first-class concerns.

### CooperBench Warning (2026)

The CooperBench benchmark found agents achieve ~50% lower success rates when collaborating versus solo. Failure modes:

- **Communication breakdown** (26%): Vague messages, failure to respond
- **Commitment failures** (32%): Agents promise actions they never execute
- **Expectation failures** (42%): Agents fail to update mental models of what partners are doing

Mitigation strategies for our system:

- Structured message passing with typed schemas (eliminates vague messages)
- Orchestrator tracks task state externally (detects commitment failures)
- Shared task board with three states: pending, in-progress, completed (maintains accurate expectations)
- Prefer orchestrator-mediated coordination over peer-to-peer (reduces coordination complexity)

### Agentic Drift Risk

When parallel agents work on related code without coordination, they gradually diverge. Example: three agents each implement dynamic model discovery with different class names, interfaces, and assumptions.

Mitigation:

- File reservation leases (exclusive locks before editing)
- Architectural constraints defined in task specs (which files to touch, which interfaces to use)
- Integration checkpoints where orchestrator validates cross-agent consistency
- Prefer coarse-grained task decomposition (entire features) over fine-grained (individual functions) to reduce overlap

### Rust-Specific Implementation Recommendations

1. **Tokio async runtime** for concurrent agent management
2. **swarms-rs** and **Rig** as reference implementations for Rust agent patterns
3. **SQLite** (via rusqlite) for persistent state management
4. **Serde** for typed JSON message serialization between orchestrator and agents
5. **Git2-rs** for programmatic git worktree management
6. **Tower** for middleware (rate limiting, retry, timeout) on agent communication
7. **Tracing** crate for distributed observability across agent interactions
8. **Config-as-code** via TOML/HCL parsed with serde for agent team definitions
