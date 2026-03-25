# Group-Evolving Agents (GEA): Implementation-Grade Research Notes

**Paper:** "Group-Evolving Agents: Open-Ended Self-Improvement via Experience Sharing"
**Authors:** Zhaotian Weng, Antonis Antoniades, Deepak Nathani, Zhen Zhang, Xiao Pu, Xin Eric Wang
**Affiliation:** University of California, Santa Barbara
**arXiv:** 2602.04837v1 (February 4, 2026)
**License:** CC BY 4.0

## Group Evolution Architecture

### Core Insight

The fundamental unit of evolution is a **group of agents**, not a single agent. This breaks from tree-structured evolution (used by Darwin Godel Machine / DGM and similar) where individual agents spawn offspring along isolated branches that never share discoveries.

### What Is Shared vs. What Is Individual

**Shared across the group:**

- Aggregated evolutionary traces (all parent agents contribute to a single pool)
- Evolution directives (high-level instructions generated from group-wide pattern analysis)
- The experience pool itself (every child agent can draw from it)

**Individual to each agent:**

- The agent's own framework code (its specific implementation)
- Its capability vector (binary vector across probe tasks -- see Fitness section)
- Its specific evolutionary trace history (patches applied, probe outcomes, execution logs)
- The specific framework-level patches it generates from the shared directives

Critically, agents **maintain divergence** even while drawing on shared experience. Each child agent produces its own distinct framework-level patches from the shared directives, ensuring the group continues to explore different strategies.

### Two-Stage System Architecture

```txt
Stage 1: Evolution (offline, R&D/training-like)
  - Multiple agents evolve over N iterations
  - Group selection -> experience aggregation -> directive generation -> patching
  - Repeated until convergence or iteration budget exhausted

Stage 2: Inference/Deployment (production)
  - A single best-evolved agent is deployed
  - Standard single-agent inference cost
  - No group overhead at serving time
```

### Module Decomposition

Each agent has three LLM-powered modules:

| Module | Role | Model Used (in paper) |
| --- | --- | --- |
| Acting Module | Executes tasks (solves SWE issues, writes code) | Claude Haiku 4.5 (iter 1-20/40), Claude Sonnet 4.5 (final 10-20 iter) |
| Reflection Module | Analyzes group experience, identifies patterns, generates evolution directives | GPT-o1 (throughout) |
| Evolution/Updating Module | Applies framework-level patches to create offspring agents | Claude Haiku 4.5 / Claude Sonnet 4.5 |

## Experience Sharing Mechanism

### Evolutionary Traces

Each agent generates and maintains a complete evolutionary trace consisting of:

1. **Code modification patches** -- diffs applied to the agent's own framework code
2. **Predicted task patches** -- solutions proposed for unsolved problems
3. **Execution logs** -- full tool invocation history (what was called, in what order, with what effect)
4. **Evaluation outcomes** -- pass/fail results revealing failure modes
5. **LLM-readable experience narrative** -- a structured summary of the agent's evolution history

### Experience Pool Construction

At each iteration:

1. A parent group of K agents is selected (see Evolution Algorithm below)
2. All evolutionary traces from every agent in the parent group are aggregated into a **single shared experience pool**
3. Nothing is siloed -- every parent's full history is visible to the reflection module
4. The reflection module (GPT-o1) analyzes the entire pooled history for group-wide patterns

### What the Reflection Module Extracts

The reflection module looks for cross-agent patterns such as:

- One agent discovered an effective debugging tool while another refined a test workflow
- Common failure modes across multiple agents
- Complementary strategies that could be combined
- Tool discoveries that improved performance in one branch but were never propagated

From these patterns, it generates **evolution directives** -- high-level, informed instructions for how to modify operational code in the next generation.

## Innovation Reuse

### The Silo Problem in Tree-Structured Evolution

In DGM (the baseline), evolution follows a tree structure:

- Agent A spawns children A1, A2
- Agent B spawns children B1, B2
- A1's innovations are **invisible** to B's lineage and vice versa
- If lineage A is not selected next round, all of A's tool discoveries die with it

### How GEA Solves This

GEA's shared experience pool breaks the silo:

1. All parent agents contribute traces to a common pool
2. The reflection module extracts the best innovations from across all lineages
3. Evolution directives propagate successful tools/workflows to the entire next generation
4. Child agents can inherit traits from **any** parent, not just their direct ancestor

### Quantitative Evidence of Innovation Reuse

- GEA's best agent drew experience from **17 unique ancestors** (28.3% of the total population)
- DGM's best agent drew from only **9 ancestors**
- GEA nearly doubles the effective lineage breadth, creating a "super-agent" that combines best practices from across the population

### Concrete Innovation Examples

The paper tracks specific innovations that emerged and propagated:

- New debugging tools invented by one agent propagated to the best-performing agents
- Testing workflows that reduce regressions spread across the group
- Workflow and tool enhancements (not model-specific prompt tuning) -- this is what makes them transferable

## Evolution Algorithm

### Stage 1: Parent Group Selection (Performance-Novelty Ranking)

Parents are selected by jointly optimizing two criteria:

**Performance Score:**

- Task success rate on the target benchmark (e.g., % of SWE-bench issues resolved)

**Novelty Score:**

- Each agent is characterized by a **binary capability vector** across a fixed suite of **probe tasks**
- Novelty = KNN distance in capability space (how different this agent's capability profile is from its nearest neighbors in the archive)
- This tracks which capabilities propagate across generations

**Combined Score:**

```txt
score(agent) = alpha * performance(agent) + (1 - alpha) * novelty(agent)
```

- `alpha` is a tunable parameter that balances exploitation (performance) vs. exploration (novelty)
- Neither signal alone is sufficient:
  - Performance-only collapses to local optima
  - Novelty-only never converges

The top-K agents by combined score form the parent group.

### Stage 2: Open-Ended Group Evolution

For each iteration:

```txt
1. SELECT parent group G_parent of size K using Performance-Novelty ranking
2. AGGREGATE all evolutionary traces from G_parent into shared experience pool E
3. REFLECT: Feed E to the reflection module (strong LLM like GPT-o1)
   - Analyze group-wide patterns
   - Identify successful innovations, common failures, complementary strategies
   - Generate evolution directives D (high-level instructions for modification)
4. For each child agent c_i in the offspring group G_child:
   a. Take a parent agent p_i as the base framework code
   b. Feed p_i's code + directives D + shared experience E to the evolution module
   c. Generate framework-level patches (concrete code, config, workflow modifications)
   d. Apply patches to produce child agent c_i
   e. c_i maintains divergence from siblings (different patches despite shared directives)
5. EVALUATE each c_i on probe tasks -> update capability vector
6. EVALUATE each c_i on benchmark tasks -> update performance score
7. ADD all c_i to the archive
8. REPEAT from step 1
```

### Key Differences from Biological Evolution Analogs

| Concept | Biological Evolution | GEA |
| --- | --- | --- |
| Unit of evolution | Individual organism | Group of agents |
| Inheritance | From single parent (or two in sexual reproduction) | From entire parent group via shared experience |
| Mutation | Random perturbation | LLM-directed, informed by group experience |
| Crossover | Genetic recombination between two parents | Implicit -- reflection module combines innovations from all parents |
| Selection | Fitness-based survival | Performance-Novelty combined ranking |
| Information sharing | None (isolated branches) | Full experience pool shared across group |

There is no explicit crossover operator. Instead, the reflection module performs an implicit form of crossover by analyzing all parents' experiences and generating directives that combine the best elements from multiple lineages.

## Fitness Measurement

### Performance Metric

- **Primary:** Task success rate on the target benchmark
  - SWE-bench Verified: % of 500 real GitHub issues resolved (code patch passes existing test suite)
  - Polyglot: % of multilingual coding tasks solved correctly

### Novelty Metric

- Each agent is evaluated on a fixed suite of **probe tasks**
- Results form a **binary capability vector** (1 = solved, 0 = not solved)
- Novelty = **KNN distance** of this vector from nearest neighbors in the archive
- This ensures the population maintains behavioral diversity, not just performance

### Combined Fitness

```txt
fitness = alpha * task_success_rate + (1 - alpha) * knn_novelty_distance
```

### Evaluation During Evolution

- Agents are evaluated on both probe tasks (for novelty vector) and benchmark tasks (for performance) after each evolution iteration
- Both scores determine which agents enter the next parent group

## Zero Additional Inference Cost Claim

### How It Works

GEA is explicitly a **two-phase** system:

**Phase 1 -- Evolution (offline):**

- Multiple agents are evolved, evaluated, and improved using the group-based process
- This is computationally expensive (many LLM calls for reflection, patching, evaluation)
- Treated as R&D or training cost, not serving cost

**Phase 2 -- Deployment (online):**

- After evolution completes, **a single best-evolved agent** is deployed for production inference
- This agent runs as a standard single-agent system
- No group overhead, no reflection module, no experience aggregation at serving time
- Enterprise inference cost is "essentially unchanged versus a standard single-agent setup"

### Why This Matters

- The evolution cost is front-loaded and amortized
- Production serving is identical in cost to deploying any single agent
- Contrast with multi-agent inference systems (e.g., ensemble methods) that multiply inference cost at runtime

## Algorithms and Pseudocode

### High-Level Algorithm

```txt
Algorithm: GEA (Group-Evolving Agents)

Input:
  - Initial agent population A_0 (can be a single seed agent)
  - Archive AR = {}
  - Probe task suite P
  - Target benchmark B
  - Group size K
  - Number of iterations T
  - Balance parameter alpha

Initialize:
  For each agent a in A_0:
    Evaluate a on P -> capability_vector(a)
    Evaluate a on B -> performance(a)
    Add a to AR

For t = 1 to T:
  // Stage 1: Parent Group Selection
  For each agent a in AR:
    novelty(a) = KNN_distance(capability_vector(a), AR)
    score(a) = alpha * performance(a) + (1 - alpha) * novelty(a)
  G_parent = top_K(AR, by=score, K=K)

  // Stage 2: Group Evolution
  E = aggregate_traces(G_parent)  // shared experience pool
  D = reflect(E)                   // evolution directives from reflection LLM

  G_child = {}
  For each parent p_i in G_parent:
    patches = evolve(p_i.code, D, E)  // LLM generates framework patches
    c_i = apply_patches(p_i, patches)
    Evaluate c_i on P -> capability_vector(c_i)
    Evaluate c_i on B -> performance(c_i)
    Add c_i to G_child

  AR = AR + G_child  // add offspring to archive

Output: argmax(AR, by=performance)  // best agent for deployment
```

### Evolution Directive Generation (Reflection Module)

```txt
Input: Shared experience pool E (aggregated traces from all parents)
Process:
  1. Analyze all code modification patches across parents
  2. Identify which modifications correlated with performance gains
  3. Identify common failure modes across the group
  4. Identify complementary strategies (e.g., one agent's debugging + another's testing)
  5. Synthesize high-level directives that:
     - Recommend adopting successful innovations
     - Recommend fixing recurring failure patterns
     - Suggest combining complementary approaches
Output: Evolution directives D (natural language instructions for the evolution module)
```

### Framework Patch Generation (Evolution Module)

```txt
Input: Parent agent code, Evolution directives D, Shared experience E
Process:
  1. Read parent's current framework implementation
  2. Interpret evolution directives in context of this specific parent
  3. Generate concrete code patches (diffs) that implement the directives
  4. Patches target framework-level code (workflow logic, tool definitions,
     orchestration), NOT model weights or model-specific prompts
Output: Framework-level patches (code diffs to apply to parent)
```

## Comparison to Individual Agent Self-Improvement

### Tree-Structured (DGM) vs. Group-Structured (GEA)

| Dimension | DGM (Individual/Tree) | GEA (Group) |
| --- | --- | --- |
| Evolution unit | Single agent | Group of agents |
| Branch interaction | Strictly isolated | Fully shared experience pool |
| Innovation propagation | Only within lineage | Across entire population |
| Diversity utilization | Low -- most diversity is wasted | High -- diversity feeds into directives |
| Iterations to 71% on SWE-bench | Never reached (56.7% max at 60 iter) | 30 iterations |
| Iterations to 88% on Polyglot | Never reached (68.3% max at 40 iter) | 20 iterations |
| Ancestor breadth (best agent) | 9 unique ancestors | 17 unique ancestors (28.3% of pop) |
| Bug recovery (injected bugs) | 5 iterations average | 1.4 iterations average |
| Starting baseline | 20.0% (SWE-bench) | 20.0% (SWE-bench) -- same start |

### Why Tree-Structured Evolution Fails

1. **Wasted diversity:** Many agents provide only temporary diversity, producing short-lived variants that fail to contribute to long-term cumulative progress
2. **Innovation death:** When a lineage is not selected, all its discovered tools and workflows vanish
3. **No cross-pollination:** Breakthroughs in one branch cannot benefit parallel branches
4. **Slow recovery:** Without group experience to draw on, agents must independently rediscover solutions to bugs

### Why Group Evolution Succeeds

1. **Diversity as stepping stones:** Early exploratory diversity is consolidated into sustained long-term progress via the shared experience pool
2. **Innovation persistence:** Useful tools/workflows are extracted by the reflection module and propagated via directives, even if the originating agent is not selected
3. **Cross-pollination by design:** Every child agent sees innovations from all parents
4. **Rapid recovery:** Healthy agents in the group help diagnose and patch compromised ones

## Benchmark Results

### SWE-bench Verified (500 real GitHub issues)

| Method | Type | Success Rate |
| --- | --- | --- |
| GEA | Self-evolved (group) | **71.0%** |
| OpenHands + GPT-5 | Human-designed SOTA | 71.8% |
| DGM | Self-evolved (tree) | 56.7% |
| Baseline (iteration 0) | Starting point | 20.0% |

- GEA reaches 71.0% in **30 iterations**
- DGM reaches 56.7% in **60 iterations** (twice as many iterations, worse result)
- GEA effectively matches the top human-designed open-source framework

### Polyglot (multilingual coding)

| Method | Type | Success Rate |
| --- | --- | --- |
| GEA | Self-evolved (group) | **88.3%** |
| DGM | Self-evolved (tree) | 68.3% |
| Aider + GPT-5 | Human-designed SOTA | 52.0% |
| Baseline (iteration 0) | Starting point | 38.2% |

- GEA reaches 88.3% in **20 iterations**
- DGM reaches 68.3% in **40 iterations**
- GEA **significantly exceeds** the best human-designed framework on this benchmark

### Robustness (Bug Injection Test)

- Researchers deliberately injected bugs into agent implementations
- GEA repaired critical framework-level bugs in **1.4 iterations** on average
- DGM required **5 iterations** on average
- GEA leverages healthy group members to diagnose and patch compromised agents

### Transferability Across Models

- Agents evolved using one model family (e.g., Claude) maintained performance gains when swapped to another model family (e.g., GPT-5.1 or GPT-o3-mini)
- Improvements are **workflow and tool enhancements**, not model-specific prompt tuning
- This makes evolved agents model-agnostic -- enterprises can switch providers without losing learned optimizations

### Efficiency

- GEA achieves stronger performance with the **same number of evolved agents** as DGM
- More effectively converts early-stage exploratory diversity into sustained progress
- Better compute efficiency per iteration of evolution

## Emergent Improvements

### Types of Improvements That Emerged

The improvements GEA discovers are **framework-level**, not model-level:

1. **Workflow enhancements** -- changes to the orchestration logic, task decomposition strategies, and multi-step planning approaches
2. **Tool discoveries** -- new debugging tools, testing utilities, code analysis tools that agents autonomously create and integrate
3. **Testing workflows** -- more effective test strategies that reduce regressions
4. **Error handling patterns** -- better recovery from tool failures, more robust fallback strategies
5. **Code modification strategies** -- improved approaches to generating and applying patches to target repositories

### Why These Are Framework-Level (Not Model-Level)

- Improvements target the agent's operational code: workflow DAGs, tool definitions, orchestration logic, config
- They do NOT modify model weights, prompts, or model-specific parameters
- This is precisely why they transfer across model families
- The evolved agent is a better-designed system, not a better-prompted one

### Self-Healing Capability

A production-relevant emergent behavior:

- When framework-level bugs are injected, GEA agents can self-heal by drawing on group experience
- The reflection module identifies that other agents in the group do not have the same failure mode
- It generates directives that guide the broken agent toward the working implementations
- This is analogous to a "hive-mind" immune response

### Limitations of Emergent Improvements

The authors note:

- This collective approach works best where **success is objectively measurable** (coding tasks with clear pass/fail criteria)
- For less deterministic domains (e.g., creative generation), evaluation signals are weaker
- Blindly sharing outputs and experiences may introduce low-quality experiences that act as noise
- Such domains would need **stronger experience filtering mechanisms**

## Implementation Guidance

### Three Components to Add to a Standard Agent Stack

The authors state teams can adopt GEA on top of existing agent frameworks by adding:

1. **Experience Archive** -- persistent storage for evolutionary traces (solutions, code changes, tool use patterns, evaluation outcomes)
2. **Reflection Module** -- a strong foundation model (e.g., GPT-o1 class) that analyzes group-wide patterns and generates evolution directives
3. **Updating/Evolution Module** -- mechanism for agents to modify their own code or configuration based on directives

### Model Configuration

| Phase | Recommended Approach |
| --- | --- |
| Early evolution (iter 1-20+) | Use cheaper/faster model for acting + evolution (e.g., Claude Haiku 4.5) |
| Late evolution (final 10-20 iter) | Switch to stronger model (e.g., Claude Sonnet 4.5) for refinement |
| Reflection (throughout) | Use strongest available reasoning model (e.g., GPT-o1) |

### Hybrid Evolution Pipeline (Future Direction)

The authors propose:

- **Smaller models explore early** to accumulate diverse experiences cheaply
- **Stronger models later guide evolution** using those accumulated experiences
- This optimizes the cost-quality tradeoff across the evolution process

### Safety Considerations

For enterprise deployment, the authors recommend non-evolvable guardrails:

- Sandboxed execution environments
- Policy and access constraints
- Verification and approval layers before patches are applied
- Agents can evolve internal strategies and helper code while remaining bounded by organizational policies

### Open-Source Implementation

An open-source implementation exists at `github.com/jbeck018/howler-agents` (howler-agents), which implements GEA with:

- SQLite persistence (zero-config)
- MCP server integration for Claude Code and other tools
- 11 MCP tools for managing evolution runs
- Three operating modes: local, hybrid (local + sync), remote
- Configurable population size, generations, domain, and model selection
