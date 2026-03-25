# Self-Modifying Agent Systems: MARIA OS SMAS and Godel Agent

Comparative implementation-grade analysis of two self-modifying agent architectures.

Sources:

- MARIA OS SMAS: https://os.maria-code.ai/blog/self-modifying-agent-system (Published 2026-03-08)
- Godel Agent: arXiv:2410.04444, ACL 2025 Long Paper (Yin et al., Peking University / UCSB / U. Arizona)
- Godel Agent Repo: https://github.com/Arvid-pku/Godel_Agent

## Part 1: MARIA OS SMAS (Self-Modifying Agent System)

### 1.1 Architecture Overview

SMAS enables agents to detect performance degradation, propose targeted modifications to their own operational code, validate modifications against safety constraints, apply them atomically, and verify post-modification behavior. It is implemented within MARIA OS with responsibility gates governing every pipeline stage.

Key distinction from tool-creation paradigms: instead of creating tool B to replace tool A, the agent modifies tool A in-place -- preserving identity, workflow position, and integration points while changing implementation.

### 1.2 The Four Modifiable Artifact Types

```typescript
type ModifiableArtifact =
  | ToolDefinition       // Executable functions with typed I/O
  | CommandTemplate      // Parameterized command strings for external systems
  | WorkflowDefinition   // DAGs of steps with conditional branching
  | DecisionRule         // Predicate functions that gate agent behavior

interface ModificationScope {
  artifact: ModifiableArtifact
  modifiableFields: string[]      // What CAN be changed
  frozenFields: string[]          // What CANNOT be changed (safety invariants)
  requiredApproval: GateLevel     // auto | agent-review | human-approval
  rollbackWindow: number          // Seconds before modification becomes permanent
}
```

Per-artifact modification semantics:

| Artifact | Modifiable | Frozen |
| --- | --- | --- |
| Tools | Implementation, parameters, retry logic, timeouts, error handling | Type signature (I/O types), tool identity, security permissions |
| Commands | Endpoint URLs, auth methods, parameter mappings, serialization | Semantic intent, target system identity, audit classification |
| Workflows | Step ordering, branch predicates, parallelism, timeouts | Entry point, terminal output type, responsibility assignments |
| Decision Rules | Thresholds, feature sets, logic structure | Decision domain, escalation targets, compliance classifications |

### 1.3 The Formal Modification Operator

Agent operational state at time t: `M(t) in M` (space of all valid agent configurations). State comprises tool registry, command templates, workflow definitions, and decision rules.

**State transition:**

```
M(t+1) = M(t) + delta_M(t)
```

**Constraint on delta_M:** restricted to modification subspace S:

```
delta_M(t) in S where S = { delta in M : pi_frozen(delta) = 0 AND phi_safety(M(t) + delta) = true }
```

- `pi_frozen`: projection onto frozen dimensions (must be zero -- no change allowed)
- `phi_safety`: safety predicate that must hold after modification

**Derivation of the modification operator (optimization formulation):**

```
delta_M(t) = argmin_{delta in S} [ L(M(t) + delta) + lambda * ||delta||^2 ]
```

- `L`: loss function measuring operational suboptimality
- `lambda * ||delta||^2`: regularization penalizing large modifications (prefer minimal changes)

This ensures the agent makes the smallest modification necessary to address the trigger.

### 1.4 Lyapunov Stability Constraints

**Lyapunov function** `V: M -> R_>=0` measures distance from optimal operation:

```
V(M) = SUM_{a in artifacts(M)} w_a * l_a(M)
```

- `l_a(M)`: loss (suboptimality) of artifact a under configuration M
- `w_a`: importance weight

**Strict Lyapunov decrease condition:**

```
V(M(t+1)) < V(M(t))    for all t where delta_M(t) != 0
```

Since V is bounded below by 0 (perfect operation) and strictly decreasing, convergence is guaranteed by the monotone convergence theorem.

**Practical relaxation:** require `V(M(t+1)) < V(M(t)) - epsilon` where epsilon > 0 is a minimum improvement threshold, preventing trivially small modifications.

**Enforcement:** at Stage 3 (Validate) of the pipeline. Every proposal must demonstrate -- through sandbox testing -- that the modified artifact's loss is strictly lower than the current artifact's loss. If validation cannot confirm strict decrease, the proposal is rejected.

### 1.5 The Energy Function and Bounded Termination

The Lyapunov function V serves as the energy function. Since every modification decreases V by at least epsilon, and V is bounded below by 0:

```
N_max = floor(V(M(0)) / epsilon)
```

This is the maximum number of modifications the system will ever perform. After N_max modifications, no further modification can satisfy the Lyapunov decrease condition, and the system halts (reaches a stable fixed point).

**Practical example:** For a typical deployment with V(M(0)) ~ 100 and epsilon = 0.1, maximum modifications = 1,000. In practice, systems stabilize after 50-200 modifications during initial adaptation, then enter steady state where modifications occur only in response to external changes.

Three structural constraints make the halting problem decidable for SMAS:

1. **Finite modification space:** S is finite-dimensional (artifacts have finite size, parameters have bounded ranges, modifiable fields are fixed). M is compact.
2. **Strict Lyapunov decrease:** bounded number of modifications as shown above.
3. **Cooldown periods:** each artifact has a minimum time between modifications (`stable(a, w)` predicate), preventing rapid oscillation.

### 1.6 The Modification Frontier

The boundary between what agents CAN and CANNOT modify. Defined architecturally, not by agent choice -- agents cannot expand their own modification frontier.

**Critical Meta category (frozen):**

1. The modification frontier definition itself
2. The safety constraint predicates `phi_safety`
3. The audit logging system
4. The gate level assignments determining which modifications require human approval
5. The Lyapunov function V and its decrease threshold epsilon

The modification frontier itself is frozen. An agent cannot modify the rules that govern self-modification. This is the fundamental safety invariant. If an agent could modify its own modification constraints, no safety guarantee would hold. The frontier is set by human architects and can only be changed through the standard MARIA OS responsibility gate process with human approval.

### 1.7 Modification Triggers

Three trigger categories, formalized as:

```
T_trigger(a) = {
  1  if Delta_perf(a) > theta_degrade           (performance degradation)
  1  if exists r in R_new : not satisfies(a, r)  (new requirements)
  1  if Delta_opt(a) > theta_opt AND stable(a,w)  (efficiency optimization)
  0  otherwise
}
```

**Performance Degradation:** latency increases > 2x, error rate > 5%, success rate below historical p95.

**New Requirements:** new data format, regulatory constraint, deprecated endpoint.

**Efficiency Optimization:** only when expected improvement exceeds minimum threshold (default: 10% latency reduction or 5% resource reduction) to prevent modification churn. Requires `stable(a, w)` -- artifact must have been stable for at least w time units.

### 1.8 The 5-Stage Modification Pipeline

No stage can be skipped. Each stage produces an immutable record.

**Stage 1 - Detect:** Monitoring subsystem identifies trigger, produces `ModificationRequest` (target artifact, trigger type, evidence, priority score). Fully automated.

**Stage 2 - Propose:** Modification engine generates `ModificationProposal` objects containing:

```typescript
interface ModificationProposal {
  id: string
  targetArtifact: ArtifactReference
  trigger: ModificationTrigger
  diff: ArtifactDiff           // Before/after with line-level granularity
  impactAnalysis: {
    directDependents: ArtifactReference[]
    transitiveDependents: ArtifactReference[]
    breakingChanges: BreakingChange[]
    riskScore: number          // 0.0 - 1.0
  }
  validation: {
    typeCheck: ValidationResult
    sandboxExecution: ValidationResult
    regressionTests: ValidationResult
    safetyConstraints: ValidationResult
    resourceBounds: ValidationResult
  }
  expectedImprovement: MetricDelta
  requiredApproval: GateLevel
}
```

Uses template-based patching (known patterns like URL changes) and LLM-based code generation (novel modifications).

**Stage 3 - Validate:** Battery of checks:

- Type checking (static type analysis)
- Sandbox execution (test suite from historical inputs/outputs)
- Regression testing (all dependent artifacts)
- Safety constraint checking (frozen fields, modification frontier)
- Resource bounds checking (memory, CPU, network)
- Lyapunov decrease verification

**Stage 4 - Apply:** Atomic application. Current artifact version is snapshotted, modification applied, new version registered with monotonically increasing version number. Transactional -- if any step fails, entire modification rolls back. Distributed transaction protocol for multi-artifact workflow modifications (all-or-nothing).

**Stage 5 - Verify:** Post-deployment verification window:

- Tools: 1 hour
- Workflows: 24 hours
- Decision rules: 72 hours

System monitors modified artifact behavior against expected improvement metrics. Automatic rollback if expectations not met or unexpected side effects detected.

### 1.9 Atomic Application with Rollback

Application is transactional with these guarantees:

1. Current artifact version is snapshotted (immutable historical record)
2. Modification is applied
3. New version registered with monotonically increasing version number
4. If any step fails, entire modification rolls back

Rollback is always available -- any artifact can be reverted to any previous version by creating a new version whose content matches the target historical version. Rollback is itself a modification (goes through the pipeline with abbreviated validation) and produces its own audit record. There is no way to modify an artifact without leaving a trace.

### 1.10 Immutable Audit Trails

```typescript
interface ArtifactVersion {
  versionId: string               // Monotonically increasing
  artifactId: string
  content: string                 // Full artifact source
  contentHash: string             // SHA-256 of content
  previousVersionId: string | null
  modification: {
    triggerId: string             // Link to ModificationRequest
    proposalId: string            // Link to ModificationProposal
    diff: ArtifactDiff            // What changed
    justification: string         // Why it changed (natural language)
    causalChain: string[]         // Evidence chain from trigger to proposal
  }
  metadata: {
    createdAt: string
    createdBy: AgentCoordinate    // MARIA coordinate of modifying agent
    approvedBy: AgentCoordinate | "auto"
    verificationStatus: "pending" | "verified" | "rolled-back"
    performanceBaseline: MetricSnapshot
    performanceActual: MetricSnapshot | null
  }
}
```

Version history is an append-only log -- versions are never deleted or mutated. Creates a complete, tamper-evident record. Stored in the `decision_transitions` table with a `self_modification` transition type.

**Full modification evidence bundle:**

```typescript
interface ModificationEvidence {
  modificationId: string
  artifactId: string
  fromVersion: string
  toVersion: string
  timestamp: string
  agentCoordinate: string       // e.g., G1.U1.P9.Z3.A1

  trigger: {
    type: "degradation" | "new-requirement" | "optimization"
    evidence: MetricTrace[] | RequirementSpec[] | OptimizationAnalysis
    detectedAt: string
  }

  diff: {
    before: string              // Full artifact source (pre-modification)
    after: string               // Full artifact source (post-modification)
    hunks: DiffHunk[]           // Line-level diff hunks
    summary: string             // Natural language summary
  }

  validation: {
    typeCheck: { passed: boolean; details: string }
    sandboxResults: TestResult[]
    regressionResults: TestResult[]
    safetyCheck: { passed: boolean; constraintsEvaluated: string[] }
    lyapunovDecrease: { before: number; after: number; delta: number }
  }

  verification: {
    windowStart: string
    windowEnd: string
    status: "pending" | "verified" | "rolled-back"
    productionMetrics: MetricSnapshot
    anomalies: AnomalyReport[]
  }

  approval: {
    gateLevel: "auto" | "agent-review" | "human-approval"
    approvedBy: string
    approvedAt: string
    justification: string
  }
}
```

### 1.11 MARIA OS Responsibility Gates on Self-Modification

| Modification Type | Example | Gate Level |
| --- | --- | --- |
| Parameter adjustment | Timeout 30s to 60s | Auto |
| Implementation change | Algorithm optimization | Agent-review |
| Interface change | Adding a tool parameter | Agent-review + impact analysis |
| Workflow restructuring | Reordering pipeline steps | Human-approval |
| Decision rule logic | Changing approval threshold | Human-approval |

## Part 2: Godel Agent

### 2.1 Architecture Overview

Godel Agent is a self-referential agent framework inspired by Schmidhuber's Godel machine (2003). Unlike the original Godel machine which required formal proofs of improvement, Godel Agent uses empirical feedback from the environment and asks an LLM to improve its code based on that feedback.

**Three agent paradigms (increasing degrees of freedom):**

1. **Hand-Designed Agent:** fixed policy pi, no adaptation. Lowest freedom.
2. **Meta-Learning Optimized Agent:** fixed meta-learning algorithm I updates policy based on feedback. `pi_{t+1} = I(pi_t, r_t)` where `r_t = U(E, pi_t)`. I is fixed.
3. **Self-Referential Agent (Godel Agent):** updates BOTH policy pi AND meta-learning algorithm I recursively. Highest freedom.

**Modular structure:** LLM serves as decision-making module. Sensor reads own code. Executor writes own code and interacts with environment. The sensor and executor can read and write ALL of the agent's own code -- including the code responsible for analysis and modification.

### 2.2 Self-Referential Monkey-Patching Mechanism

**Core technique:** Monkey patching -- dynamically modifying classes or modules during Python execution at runtime.

**Implementation approach:**

- The agent is an instance of a Python `Agent` class
- It has access to the Python environment's runtime memory (local/global variables, functions, modules)
- It can read its own source code via `action_read_logic` (inspects `__source__` attribute attached to functions/classes)
- It can modify its own code via `action_adjust_logic` which calls the monkey patching function
- Modified code is stored in a custom `__source__` attribute attached to both functions and classes

**Key implementation choice:** The main function (`evolve`) is implemented as a RECURSIVE function, not a loop-iterative one. This allows the agent to update the logic of the running main function -- after modification, the next recursive call uses the updated code.

**Two types of monkey patching targets:**

- Module-level attributes (e.g., `solver`): simple name reference
- Class attributes (e.g., `Agent.evolve`): dot notation reference

### 2.3 The Recursive Self-Improvement Loop

**Formal definition:**

```
pi_{t+1}, I_{t+1} = I_t(pi_t, I_t, r_t, g)
r_t = U(E, pi_t)
```

Where g is the high-level goal (e.g., "solve the given mathematical problem with highest accuracy").

**Algorithm 1 (from paper):**

```
Input: Initial agent policy pi_0, initial decision function f_0,
       goal g, environment state E, utility function U,
       self code reading function SELF_INSPECT

1. s <- SELF_INSPECT()           # Get all agent code, including this algorithm
2. r <- U(E, pi_0)               # Compute initial performance
3. pi, s <- SELF_IMPROVE(pi, s, r, g)  # Perform recursive self-improvement
4. return pi, s

function SELF_IMPROVE(E, pi, s, r, g):
    a_1, ..., a_n <- f_0(pi, s, r, g)   # Obtain action sequence from LLM
    for a_i in a_1, ..., a_n:
        pi, s, r <- EXECUTE(E, pi, s, r, a_i)
    return pi, s

function EXECUTE(E, pi, s, r, a):
    switch a.name:
        case self_state:
            s <- SELF_INSPECT()
        case interact:
            r <- U(E, pi)
        case self_update:
            pi, s <- a.code          # Apply new code
        case continue_improve:
            pi, s <- SELF_IMPROVE(E, pi, s, r, g)  # Recursive invocation
    return pi, s, r
```

### 2.4 How High-Level Objectives Guide Modification

The agent receives a high-level goal g (e.g., "achieve highest accuracy on MATH benchmark") via prompting. The goal is specified in `goal_prompt.md`. The LLM-based decision function f_0 determines actions based on:

- Current policy pi_t (the `solver` function code)
- Current agent code s (all readable code)
- Performance feedback r_t (utility score from environment)
- The goal g

The agent is NOT told HOW to improve -- only WHAT to optimize for. The LLM decides strategy autonomously.

### 2.5 The Six Core Actions

```
action_display_analysis    # Planning/reasoning display
action_environment_aware   # Inventory available resources, modules, files
action_read_logic          # Read current code of any module/function/class
action_adjust_logic        # Modify code via monkey patching
action_run_code            # Execute Python or bash code
action_evaluate_on_task    # Test performance on benchmark
```

**Self-improvement loop per evolve() call:**

1. Reset optimization history
2. `action_environment_aware()` -- inventory available resources
3. `action_read_logic("agent_module", "solver")` -- read current solver
4. LLM decides sequence of actions based on goal + history
5. Actions executed, results appended to `optimize_history`
6. Check evolution counter; if < 100, trigger next `evolve()` iteration

### 2.6 What Specific Modifications the Agent Made

**Game of 24 (case study from paper):**

- After 6 unsuccessful LLM-based optimization attempts, the agent switched from an LLM-based method to a search-based method
- Rewrote the solver code entirely, achieving 100% accuracy
- In other runs: added verification steps, testing with additional data, added error tracing library, improved logging, removed unnecessary code

**Across benchmarks, observed self-modifications:**

- Prompt restructuring and chain-of-thought refinement
- Adding self-verification/self-correction loops
- Switching algorithmic approaches entirely (LLM-based to search-based)
- Adding exploration mechanisms
- Code cleanup and optimization

**Self-referential modifications (modifying the optimizer itself):**

- The agent can modify `Agent.evolve` (the optimization loop itself)
- It can modify any action function
- It can expand the action set A with new actions

### 2.7 Safety Constraints on Self-Modification

Godel Agent has minimal built-in safety constraints compared to MARIA OS SMAS:

**Restricted variant (default):**

- Cannot change the model (locked to gpt-3.5-turbo for evaluation)
- No internet access
- gpt-4o used for self-improvement decisions; gpt-3.5-turbo for evaluating optimized policy

**Code-level safety (from implementation):**

- Blocks modifications to essential LLM calling methods
- Prevents `time.sleep` in the solver
- Prohibits `logging` imports
- Error handling: error trace feedback when LLM introduces bugs
- Wrapped solver execution (`wrap.py`): try/except around solver calls to catch crashes from bad modifications

**No formal safety guarantees.** Unlike MARIA OS SMAS:

- No Lyapunov stability analysis
- No modification frontier
- No frozen fields
- No formal convergence proof
- No mandatory validation pipeline
- No immutable audit trail
- Agent CAN modify its own optimization code (fully self-referential)
- Relies on empirical feedback rather than formal proofs

**Unrestricted variant:** has access to everything, including ability to switch to more powerful models.

### 2.8 Benchmark Results

**Benchmarks:** DROP, MGSM, MMLU, GPQA

**Baselines compared against:**

- Hand-Designed: CoT, CoT-SC, Self-Refine, LLM Debate, Step-back-Abs, Quality-Diversity, Role Assignment
- Meta-Learning: Meta Agent Search

**Results (restricted Godel Agent):**

- Outperformed all baselines on all benchmarks
- Significant improvements on DROP and MGSM
- Slight improvements on GPQA
- Unrestricted variant performed even better (often by switching to more powerful models)

**Known benchmark scores from repo results:**

- MMLU: 0.7087
- DROP: 80.892
- GPQA: 0.3494
- MGSM: 0.6425

**Cost:** Complete evolutionary process across 4 benchmarks with 30 recursive self-improvements: $15 (vs $300 for competing Meta Agent Search).

**Initial policy sensitivity:** Stronger initial policy led to better convergence. CoT-initialized agent did not outperform ToT after all improvements, suggesting limited capacity for radical innovation from weak starting points.

## Part 3: Comparative Analysis

### 3.1 Architecture Comparison

| Dimension | MARIA OS SMAS | Godel Agent |
| --- | --- | --- |
| Modification target | Tools, commands, workflows, decision rules | Python functions, classes, modules (any code) |
| Modification mechanism | 5-stage pipeline with gates | Direct monkey patching via LLM |
| Safety model | Formal (Lyapunov + frontier + frozen fields) | Minimal (error handling + restricted mode) |
| Convergence guarantee | Proven (bounded termination via energy function) | None formal (empirical observation) |
| Audit trail | Immutable, append-only, hash-chained | Optimization history list (conversation trace) |
| Rollback | Any version, transactional | Agent relies on history to recover |
| Self-referentiality | Partial (cannot modify meta/safety layer) | Full (can modify own optimization code) |
| Modification scope | Bounded by frozen fields | Bounded only by restricted/unrestricted mode |
| Trigger mechanism | Automated performance monitoring | LLM-driven analysis of feedback |
| Validation | Type check + sandbox + regression + safety + resources | Error handling wrapper around execution |
| Production readiness | Enterprise-grade (governance, compliance) | Research prototype |

### 3.2 Modification Mechanism Comparison

**MARIA OS SMAS:** Optimization-based. Finds argmin of loss + regularization within constrained subspace. Minimal change principle via `lambda * ||delta||^2`.

**Godel Agent:** LLM-based. The LLM receives the current code, performance feedback, and goal, then generates new code. No formal optimization -- relies entirely on LLM judgment. The agent can make arbitrarily large changes.

### 3.3 Safety Model Comparison

**MARIA OS SMAS safety stack:**

1. Modification frontier (architectural boundary, immutable)
2. Frozen fields per artifact type
3. Safety predicate phi_safety
4. Lyapunov decrease requirement (validated in sandbox)
5. Responsibility gates (auto / agent-review / human-approval)
6. Verification windows (1h / 24h / 72h)
7. Automatic rollback on failure
8. Immutable audit trail

**Godel Agent safety stack:**

1. Restricted mode (model lock, no internet)
2. Blocked modifications to essential LLM methods
3. Error trace feedback on crashes
4. Wrapped solver execution (try/except)

### 3.4 Key Implementation Differences

**State representation:**

- SMAS: `M(t)` as formal state in configuration space M, comprising typed artifact registries
- Godel: Python runtime memory -- all reachable objects, functions, classes, modules

**Modification application:**

- SMAS: Transactional with distributed protocol for multi-artifact changes
- Godel: In-place monkey patching; next recursive call picks up changes

**Feedback loop:**

- SMAS: Continuous metric monitoring with automated trigger thresholds
- Godel: Explicit `action_evaluate_on_task` calls returning utility scores

### 3.5 What to Use When

**MARIA OS SMAS:** When you need production-grade self-modification with governance, compliance, auditability, and formal convergence guarantees. Enterprise agent systems where modifications must be traceable and reversible.

**Godel Agent:** When you want maximum flexibility for research/experimentation. When the agent should be able to discover radically different approaches (e.g., switching from LLM-based to search-based solving). When formal safety guarantees are less important than exploration capability.

### 3.6 Synthesis: Building a Production Self-Modifying Agent

A production system could combine both approaches:

1. Use Godel Agent's fully self-referential LLM-driven modification generation as the proposal engine
2. Wrap it in MARIA OS SMAS's validation pipeline (type check, sandbox, regression, safety constraints)
3. Apply the modification frontier to prevent modifications to safety-critical code
4. Enforce Lyapunov decrease via benchmark validation before applying changes
5. Maintain immutable audit trails for all modifications
6. Use responsibility gates for high-risk modifications

This would give the creative exploration capability of Godel Agent with the safety guarantees of SMAS.
