# AgentDevel: Implementation-Grade Reference

Paper: "AgentDevel: Reframing Self-Evolving LLM Agents as Release Engineering"
Author: Di Zhang (Fudan University)
arXiv: 2601.04620v1, January 8, 2026

## Release Engineering Framework

AgentDevel reframes LLM agent improvement as **release engineering** rather than self-improvement. The agent is treated as a **shippable software artifact** with a single canonical version line (no population-based search, no competing variants).

### Pipeline Stages (3 stages, iterative)

The pipeline has three sequential stages per iteration, labeled A, B, C in the paper:

**Stage A: RUN & OBSERVE (Implementation-Blind Surface Signals)**

1. Run the current agent (blueprint `b_t`) on the entire development set (`D_train`).
2. Record structured **execution traces** (`tau_t`) for every example: actions taken, tools called, observations received, errors encountered, final output produced.
3. Run **programmatic scorers** where available (unit tests, schema validation, format checking) to get deterministic pass/fail signals.
4. Feed traces + rubric + optional programmatic scores to the **implementation-blind LLM critic** (see Core Design 1 below).
5. The critic produces **symptom-level categories** (not causal diagnoses, not repair proposals).

**Stage B: DIAGNOSE & SYNTHESIZE (Executable Engineering)**

1. Generate and execute **diagnostic scripts** (Python) that aggregate dominant failure patterns, typical triggering conditions, representative examples, and frequency of each issue.
2. Scripts are **regenerated each iteration** using the previous iteration's scripts as **soft references** (bootstrapped diagnosis, not rigid templates).
3. Based on the diagnosis, synthesize exactly **one Release Candidate (RC)** (`b_t^RC`) that may modify prompts, code, or tool wrappers in the blueprint.

**Stage C: FLIP-CENTERED GATING (The Regression Firewall)**

1. Evaluate the RC on the same `D_train`.
2. Compute per-example flip sets (P2F and F2P).
3. Apply the gate function `G` to accept or reject.
4. If accepted: `b_{t+1} = b_t^RC` (promoted to next official version).
5. If rejected: `b_{t+1} = b_t` (current version retained unchanged -- this IS the rollback mechanism).

### Iteration Termination (Stopping Criteria)

Iteration continues until:

- Marginal gains are exhausted (diminishing F2P returns).
- Further changes show clear **overfitting signals** (e.g., fixes becoming narrowly tailored to specific examples rather than addressing general patterns).
- Only then is the final version evaluated **once** on a held-out `TestSet`.

## Three Core Designs

### Core Design 1: Implementation-Blind LLM Critic

**What it receives:**

- The rubric (what counts as correct for each task)
- The execution traces (actions, tool calls, observations, errors, outputs)
- Optionally, programmatic scoring results (pass/fail from unit tests, etc.)

**What it does NOT receive:**

- The agent's blueprint (prompt, code, tooling internals)
- Any knowledge of how the agent is constructed

**What it produces:**

- Surface-level descriptions of what went wrong
- Grouping of failures into **symptom-like categories** such as:
  - "Missing a required step"
  - "Wrong order of actions"
  - "Invalid tool arguments"
  - "Incorrect output format"
  - "Premature termination"

**What it explicitly does NOT do:**

- Causal attribution (it does not explain WHY failures happen)
- Propose repairs (it does not suggest fixes)

**Rationale:** This separation prevents "informed bias" -- if the critic knew the agent's internals, it might anchor on implementation details rather than observable symptoms. The symptom categories are **not fixed**; they can evolve over time as new failure appearances emerge.

### Core Design 2: Script-Based Executable Diagnosis

After the critic produces symptom categories, AgentDevel generates **executable diagnostic scripts** (Python) that perform structured analysis:

**What the scripts do:**

1. Aggregate dominant failure appearances across all failing examples
2. Identify typical triggering conditions for each symptom category
3. Extract representative examples for each failure pattern
4. Count frequency/prevalence of each issue
5. Produce auditable engineering specifications

**Key properties:**

- Scripts are **regenerated each iteration** (not static templates)
- Previous iteration's scripts serve as **soft references** for the next iteration
- This creates a **bootstrapped diagnosis process** that evolves with the agent
- Output is deterministic and auditable (unlike pure LLM reasoning)
- The diagnosis drives RC synthesis: the single RC is designed to address the most dominant/frequent failure patterns identified

**Flow:** Traces + Symptom Categories --> Diagnostic Script Generation --> Script Execution --> Structured Failure Report --> RC Synthesis

### Core Design 3: Flip-Centered Gating

The gating mechanism treats per-example behavioral changes as first-class evidence, rather than relying on aggregate metrics.

## Flip-Centered Gating: Full Detail

### Formal Definitions

For each training example `x` in `D_train`:

- `p_t(x)` in `{0, 1}` -- pass indicator for current blueprint `b_t`
- `p_t^RC(x)` in `{0, 1}` -- pass indicator for release candidate `b_t^RC`

**Four possible per-example outcomes:**

| Current (`b_t`) | RC (`b_t^RC`) | Category | Meaning |
| --- | --- | --- | --- |
| Pass (1) | Pass (1) | P2P (Stable Pass) | No change, good |
| Pass (1) | Fail (0) | **P2F (REGRESSION)** | **CRITICAL RISK** -- broke something that worked |
| Fail (0) | Pass (1) | F2P (Fix) | Desired improvement |
| Fail (0) | Fail (0) | F2F (Persistent Fail) | Unresolved, neutral |

**Flip sets computed each iteration:**

```text
P2F_t = { x in D_train | p_t(x) = 1 AND p_t^RC(x) = 0 }
F2P_t = { x in D_train | p_t(x) = 0 AND p_t^RC(x) = 1 }
```

**Flip rates (for reporting/gating):**

```text
rho^P2F_t = |P2F_t| / (|{x : p_t(x) = 1}| + epsilon)
rho^F2P_t = |F2P_t| / (|{x : p_t(x) = 0}| + epsilon)
```

Where `epsilon = 1e-6` (denominator stabilization to avoid division by zero).

### Gate Function

```text
Accept_t = G(R_t, R_t^RC, P2F_t, F2P_t, I_t) in {0, 1}
```

Where:

- `R_t`, `R_t^RC` = detailed per-example run records for current and RC
- `P2F_t`, `F2P_t` = the flip sets
- `I_t` = the **intended change description** (what the RC was designed to fix)

### Acceptance Criteria (all must hold)

1. **Regression count below threshold:** `|P2F_t| <= tau_reg` (absolute count)
2. **Regression rate below threshold:** `rho^P2F_t < tau_reg_rate` (typically < 1% of all passing examples)
3. **Sufficient fixes delivered:** `|F2P_t| >= tau_fix` (e.g., at least 10 fixes)
4. **Fix-intent alignment:** The fraction of F2P flips that align with the stated intent `I_t` must exceed a threshold (e.g., > 95%), as judged by an LLM critic
5. **Improvements concentrated on targeted symptom categories** (not random scatter)

### Rejection Behavior (Implicit Rollback)

If the gate rejects:

- `b_{t+1} = b_t` (keep current version, discard RC entirely)
- The rejection is logged with full flip data for audit
- A new iteration begins with a fresh diagnosis cycle

There is no partial rollback or cherry-picking of changes. The RC is atomic: accept all or reject all.

## Regression-Aware Pipeline in Detail

### Complete Pseudocode (from paper)

```text
Given: b_0 (initial blueprint), D_train (development set)

for t = 0, 1, 2, ... do:
    // Stage A: Run & Observe
    for each x in D_train:
        tau_t(x) = execute(b_t, x)          // Run agent, collect trace
        p_t(x) = score(tau_t(x), rubric(x)) // Programmatic + critic scoring

    // Stage A continued: Implementation-blind critique
    symptoms_t = critic(
        rubrics,
        {tau_t(x) for all x where p_t(x) = 0},  // Only failed traces
        programmatic_scores
    )
    // critic produces symptom categories, NOT accessing b_t internals

    // Stage B: Diagnose & Synthesize
    diag_script_t = generate_diagnostic_script(
        symptoms_t,
        {tau_t(x)},
        diag_script_{t-1}  // Previous script as soft reference
    )
    diagnosis_t = execute(diag_script_t)
    // diagnosis_t contains: dominant failures, triggering conditions,
    //   representative examples, frequency counts

    b_t^RC = synthesize_rc(b_t, diagnosis_t)
    // Produces exactly ONE release candidate
    // May modify: prompts, code, tool wrappers

    // Stage C: Flip-Centered Gating
    for each x in D_train:
        tau_t^RC(x) = execute(b_t^RC, x)
        p_t^RC(x) = score(tau_t^RC(x), rubric(x))

    P2F_t = {x | p_t(x)=1 AND p_t^RC(x)=0}
    F2P_t = {x | p_t(x)=0 AND p_t^RC(x)=1}

    rho^P2F_t = |P2F_t| / (|{x: p_t(x)=1}| + epsilon)

    Accept_t = G(R_t, R_t^RC, P2F_t, F2P_t, I_t)

    if Accept_t == 1:
        b_{t+1} = b_t^RC    // Promote RC
    else:
        b_{t+1} = b_t       // Retain current version (rollback)

    // Stopping check
    if marginal_gains_exhausted(t) OR overfitting_detected(t):
        break

// Final evaluation (one-shot, held-out)
final_score = evaluate(b_final, D_test)
```

### What Counts as an "Agent Blueprint" (`b_t`)

The blueprint is the agent's full internal design:

- System prompt / instructions
- Agent code (tool-calling logic, parsing, control flow)
- Tool wrappers (how external tools are invoked)

An RC can modify any combination of these. The paper explicitly states the RC "may modify prompts, code, or tool wrappers in the blueprint."

## Change Tracking: How They Track Which Changes Cause Regressions

1. **Per-example tracking across versions:** Every example `x` has a pass/fail history `p_0(x), p_1(x), p_2(x), ...` across all iterations.
2. **Flip sets are the primary tracking mechanism:** At each iteration, P2F and F2P sets identify exactly which examples changed behavior.
3. **Intent alignment verification:** Each RC comes with a stated intent `I_t` (e.g., "fix missing-step failures in API-calling tasks"). The gate checks whether the actual F2P fixes correspond to this intent. If fixes appear in unrelated areas, that is suspicious.
4. **Full audit trail:** All of the following are versioned and logged:
   - Flip sets (P2F_t, F2P_t) for every iteration
   - Intent descriptions (I_t)
   - Accept/reject decisions
   - Diagnostic scripts and their outputs
   - Execution traces

## Rollback Mechanisms

AgentDevel uses a **simple but strict rollback model:**

- **Atomic accept/reject:** The RC is a single unified proposal. If rejected by the gate, it is discarded entirely. `b_{t+1} = b_t`.
- **No partial merges:** There is no mechanism to accept "some changes" from an RC while rejecting others.
- **No revert-to-earlier-version:** The rollback is always to the immediately preceding accepted version, never to an arbitrary historical version.
- **The version line is linear:** `b_0 -> b_1 -> b_2 -> ...` with possible gaps where rejected RCs were discarded.

This simplicity is intentional -- it mirrors how strict CI/CD pipelines work, where a failing build simply does not get deployed.

## Benchmark Results

### Benchmarks Used

The paper evaluates on **execution-heavy benchmarks** (not static Q&A):

1. **StableToolBench** -- Large-scale tool-use benchmark with API calls
2. **WebArena** -- Long-horizon web navigation tasks (812 tasks, self-hosted web environments)

### StableToolBench Results

From an 11-iteration run (Table 2 in paper):

| Iteration | Gate Decision | F2P (Fixes) | P2F (Regressions) | rho^P2F | Notes |
| --- | --- | --- | --- | --- | --- |
| 1 | Accepted | 38 | 4 | 0.006 (0.6%) | Good: many fixes, few regressions |
| 3 | **Rejected** | 42 | 28 | 0.040 (4.0%) | **Bad: too many regressions despite fixes** |

Key findings:

- Accepted releases maintained P2F regression rates **under 0.7%**
- Rejected iterations had P2F spikes up to **4%**
- The gate successfully blocked "bad releases" where aggregate score improved but many previously-passing examples broke

### WebArena Results

| Configuration | P2F Rate | Bad Releases |
| --- | --- | --- |
| **Full AgentDevel (with flip gating)** | **3.1%** | **0** |
| Without flip gating | **14.8%** | Multiple |

The absence of flip-centered gating caused P2F rate to increase from 3.1% to 14.8% and yielded multiple "bad releases" -- iterations that looked good on aggregate metrics but broke previously-working cases.

## What Failed and Why

### Failure Mode 1: Regressions from Overly Broad Changes

When an RC attempts to fix too many symptom categories at once, it frequently introduces regressions in unrelated areas. The paper observes this as the primary reason iterations get rejected. The gate catches these by detecting elevated P2F counts.

### Failure Mode 2: Symptom Interference

Fixing one symptom category can worsen another. For example, making the agent more thorough (to fix "missing step" symptoms) can cause it to take too many steps (triggering "premature timeout" or "action limit exceeded" failures). The flip-centered gate detects this as P2F regressions.

### Failure Mode 3: Overfitting to D_train

Over many iterations, the agent's blueprint becomes increasingly tailored to the specific examples in `D_train`. The paper uses this as a stopping signal -- when improvements become narrowly targeted at specific examples rather than addressing general patterns, iteration should stop.

### Failure Mode 4: LLM Critic Noise

The implementation-blind critic is an LLM, so its symptom categorizations carry inherent noise and potential bias. When programmatic graders exist, they are preferred. The paper acknowledges this as a limitation but argues it is acceptable because the critic never makes accept/reject decisions -- it only characterizes symptoms, and the gate makes the final call based on hard flip data.

### Failure Mode 5: Computational Overhead

Flip-centered analysis requires **rerunning all examples** in `D_train` for both the current agent and the RC. This doubles the compute per iteration compared to systems that only evaluate the RC. The paper acknowledges this as non-trivial but argues it is essential for regression detection.

### Failure Mode 6: Threshold Calibration

The thresholds `tau_reg`, `tau_fix`, and intent alignment cutoffs are **context- and deployment-specific**. There is no universal optimal setting. Too strict: nothing gets promoted. Too loose: regressions slip through. The paper provides example values but not a calibration algorithm.

## Key Design Decisions for Implementation

### What Makes This Different from Self-Refine / Reflexion

| Property | Self-Refine/Reflexion | AgentDevel |
| --- | --- | --- |
| Improvement location | Inside the agent | External pipeline |
| Version control | None (stateless or memory-based) | Single canonical version line |
| Regression tracking | None | Per-example flip tracking |
| Quality gate | None or aggregate score | Flip-centered gate with reject/rollback |
| Audit trail | None | Full trace + flip + intent logs |
| What gets modified | Agent's next response | Agent's blueprint (prompt/code/tools) |

### What Makes This Different from Population-Based Evolution (e.g., PromptBreeder)

| Property | Population-Based | AgentDevel |
| --- | --- | --- |
| Variants per iteration | Many concurrent | Exactly one RC |
| Selection criterion | Aggregate fitness | Per-example flip analysis |
| Regression awareness | None (mean can hide regressions) | P2F is primary rejection signal |
| Auditability | Low (which variant, why?) | High (intent, flip sets, scripts logged) |

### Implementation Checklist

1. **Define the blueprint format:** What constitutes the agent's modifiable artifact (prompt template, code files, tool configs).
2. **Build the trace collector:** Structured logging of every agent action, tool call, observation, error, and output per example.
3. **Implement programmatic scorers:** Deterministic pass/fail for as many examples as possible (unit tests, format validators, etc.).
4. **Build the implementation-blind critic:** LLM prompt that receives only rubric + traces + scores, outputs symptom categories. Must NOT see the blueprint.
5. **Build the diagnostic script generator:** LLM that generates Python scripts to analyze symptom patterns across all failing examples. Must use previous iteration's script as a soft reference.
6. **Build the RC synthesizer:** LLM that takes the current blueprint + diagnosis output and produces exactly one modified blueprint.
7. **Implement the flip gate:** Compare per-example pass/fail between `b_t` and `b_t^RC`, compute P2F/F2P sets, apply threshold logic.
8. **Implement intent alignment checking:** LLM-based check that F2P fixes correspond to the stated intent of the RC.
9. **Build the audit log:** Version every blueprint, flip set, intent, gate decision, diagnostic script, and trace.
10. **Implement stopping criteria:** Monitor marginal F2P gains and overfitting signals across iterations.

### Practical Thresholds (from paper examples)

| Parameter | Example Value | Notes |
| --- | --- | --- |
| `tau_reg` (max P2F count) | Context-dependent | Paper shows rejection at P2F=28 |
| `rho^P2F` (max P2F rate) | < 1% | Accepted releases stayed under 0.7% |
| `tau_fix` (min F2P count) | ~10+ | Paper shows accepted iterations with F2P=38 |
| Intent alignment | > 95% | Fraction of F2P aligned with stated intent |
| `epsilon` (denominator stabilizer) | 1e-6 | Prevents division by zero in rate computation |
