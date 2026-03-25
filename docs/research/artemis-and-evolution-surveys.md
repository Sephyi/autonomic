# Research Analysis: Evolutionary Optimization of LLM Agent Configurations

Three papers analyzed for implementation-relevant patterns applicable to Autonomic, a Rust daemon that evolves its own Claude Code configuration.

## Paper 1: ARTEMIS -- Evolving Excellence via Automated Optimization of LLM-based Agents

**Source**: arXiv:2512.09108v1 (Dec 2025), TurinTech AI et al.

### Core Architecture

ARTEMIS is a no-code evolutionary optimization platform that jointly optimizes LLM agent configurations through semantically-aware genetic operators. It treats agents as **black boxes** -- no architectural modifications required. Users provide only a benchmark script and natural language goals.

### Problem Formulation (Section 3)

Agent optimization is formalized as evolutionary search over mixed-type configuration spaces.

**Definition -- Agent Configuration**: `C = (P, T, M, Theta)` where:

- `P = {p1, ..., pn}`: Natural language prompts (system, user, assistant templates)
- `T = {t1, ..., tm}`: Tool descriptions, error messages, usage instructions
- `M = {m1, ..., mk}`: Model assignments and routing decisions (discrete choices)
- `Theta = {theta1, ..., theta_l}`: Continuous parameters (temperature, thresholds, timeouts)

**Configuration space**: `S = P x T x M x R^l` -- a product space combining infinite-dimensional NL spaces, discrete model selections, and continuous parameters.

**Optimization objective**:

```txt
C* = argmax_{C in S} f(A; C, B)
```

Where fitness function `f: A x S x B -> R` is derived by instantiating agent A with configuration C and executing benchmark B. Users provide NL optimization goals (e.g., "maximize accuracy while maintaining reasonable latency") and the platform uses LLM-based semantic understanding to evaluate fitness.

### Why Evolutionary Algorithms Fit

1. **Mixed-type variables**: NL + discrete + continuous requires specialized operators
2. **Non-differentiable objective**: No gradient information available
3. **Multimodal landscape**: Multiple local optima (different prompting strategies)
4. **Expensive evaluation**: Population-based search exploits information efficiently
5. **Semantic constraints**: Not all configs are valid; LLM-powered operators maintain validity

### The ARTEMIS Platform (Section 4)

Three-stage workflow:

1. **Project Setup**: Upload codebase, define NL optimization objectives, specify evaluation benchmark, configure search parameters (including which LLMs to invoke)
2. **Component Discovery**: Automatic codebase analysis to identify optimizable components. Supports global criteria ("find all prompts") and NL queries ("find components related to error handling") using semantic search
3. **Optimization Strategies**:
   - **Local Optimization**: Evolves individual components independently using genetic algorithms (GA). Semantic mutations + crossovers maintaining contextual validity. Best for components without strong interdependencies
   - **Global Optimization**: Uses Bayesian optimization for optimal combinations when components interact. Explores combinatorial space of component versions for synergistic configs. Essential when prompt-tool interactions affect performance

### Genetic Operators (Semantic GA)

The optimization engine uses **semantic genetic algorithms** where LLM ensembles perform intelligent mutations:

- **LLM-ensemble mutations**: Multiple LLMs generate candidate mutations of text components, preserving meaning while exploring variations
- **LLM-ensemble crossovers**: Combine successful patterns from two parent configurations using LLM understanding of semantics
- **Hierarchical evaluation**: Cheap filters (LLM scoring) applied before expensive validation (benchmark execution)
- Creates a **dynamically evolving search tree** where branches represent mutation/crossover lineages

**Key design**: The platform exposes candidate mutations, fitness scores, and lineage information to users, supporting optional human-in-the-loop inspection during optimization.

### Benchmark Results

| Agent | Benchmark | Metric | Improvement |
| --- | --- | --- | --- |
| ALE Agent | AtCoder Heuristic Contest | Acceptance rate | +13.6% |
| Mini-SWE Agent | SWE-Perf (140 instances) | Performance score | +10.1% (statistically significant) |
| CrewAI Agent | Math Odyssey | Token cost reduction | -36.9% (statistically significant) |
| MathTales-Teacher | GSM8K (Qwen2.5-7B) | Accuracy | +22% |

Project-level SWE-Perf breakdowns:

- psf/requests: 36.1% -> 43.3% speedup (+20% relative)
- scikit-learn: 3.5% -> 4.5% speedup (+29% relative)
- astropy: 2.9% -> 4.7% speedup (+62% relative)

### Comparative Analysis (Table 1)

| Framework | Scope | Generality | Arch-agnostic | Semantic | Scalable |
| --- | --- | --- | --- | --- | --- |
| APE | Prompts | High | Yes | Limited | High |
| PromptBreeder | Prompts | High | Yes | Medium | Medium |
| ADAS | Workflow | Medium | No | No | Medium |
| AFlow | Workflow | Medium | No | No | High |
| GEPA | Prompts | High | Yes | Medium | Medium |
| ShinkaEvolve | Code | Medium | No | Yes | Low |
| **ARTEMIS** | **Full agent** | **High** | **Yes** | **High** | **Medium** |

### Key Insights for Autonomic

1. **Black-box approach works**: Treating agents as input/output black boxes with benchmark-driven fitness is practical and effective
2. **Joint optimization matters**: Optimizing prompts in isolation misses critical interdependencies with tool configs and parameters
3. **Semantic GA operators**: Using LLM ensembles for mutation/crossover of NL components is far more effective than random perturbation
4. **Hierarchical evaluation saves cost**: Use cheap LLM-based scoring as a filter before expensive benchmark runs
5. **Component discovery is automatable**: Semantic search over codebases can identify optimizable components without manual specification
6. **Under-optimized systems benefit most**: Well-tuned agents show limited room; the biggest gains come from systems that haven't been manually optimized
7. **Bayesian optimization for global combination**: When components interact, BO over the combinatorial space of locally-evolved variants finds synergistic configs

## Paper 2: A Survey of Self-Evolving Agents -- What, When, How, and Where to Evolve

**Source**: arXiv:2507.21046v4 (Jul 2025, published TMLR Jan 2026), Princeton/Tsinghua/UIUC et al.
**GitHub**: https://github.com/CharlesQ9/Self-Evolving-Agents

### Core Framework

First systematic survey organizing self-evolving agents around three dimensions: **what**, **when**, and **how** to evolve.

### Formal Definitions

**Environment** as POMDP: `E = (G, S, A, T, R, Omega, O, gamma)` where:

- `G`: set of potential goals (task objectives)
- `S`: set of states (internal environment state)
- `A`: set of actions (textual reasoning + retrieval + tool calls)
- `T`: state transition probability `T(s'|s,a)`
- `R: S x A x G -> R`: feedback/reward function (scalar score or textual feedback)
- `Omega`: observations accessible to agent
- `O`: observation probability function
- `gamma`: discount factor

**Agent system**: `Pi = (Gamma, {psi_i}, {C_i}, {W_i})` where:

- `Gamma`: architecture (control flow / collaborative structures), represented as sequence of nodes `(N1, N2, ...)` organized by graph or code structures
- Each node `N_i` has:
  - `psi_i`: underlying LLM/MLLM
  - `C_i`: context information (prompt `P_i` and memory `M_i`)
  - `W_i`: set of available tools/APIs
- Agent policy: `pi_{theta_i}(.|o)` where `theta_i = (psi_i, C_i)`
- Action space: union of natural language space and tool space `W_i`

### Taxonomy: WHAT to Evolve

#### 1. Model Evolution

- **Policy**: SCoRe, PAG, TextGrad, AutoRule, SRLM -- evolving the model's decision-making weights
- **Lesson**: Reflexion, AdaPlanner, SICA, SelfRefine, Learn-by-interact, RAGEN, DYSTIL -- evolving extracted lessons/rules from experience

#### 2. Context Evolution

- **Memory**: SAGE, Mem0, MemInsight, REMEMBER, Expel, Agent Workflow Memory, ICE -- evolving what the agent remembers
- **Prompt**: APE, ORPO, ProTeGi, PromptAgent, REVOLVE, PromptBreeder, DSPy, Trace, TextGrad, SPO, LLM-AutoDiff, EvoAgent -- evolving instruction prompts

#### 3. Tool Evolution

- **Creation**: Voyager, Alita, ATLASS, CREATOR, SkillWeaver, CRAFT -- creating new tools
- **Mastery**: LearnAct, DRAFT, ToolLLM, Toolformer, Gorilla -- learning to use tools better
- **Selection**: ToolGen, AgentSquare, Darwin Godel Machine, COLT, TOOLRET, ToolRerank, PTR, SSO -- choosing the right tool

#### 4. Architecture Evolution

- **Single-Agent**: AgentSquare, Darwin Godel Machine, Godel Agent, AlphaEvolve, TextGrad, EvoFlow, MASS
- **Multi-Agent**: AFlow, ADAS, AutoFlow, GPTSwarm, ScoreFlow, FlowReasoner, ReMA, GIGPO

### Taxonomy: WHEN to Evolve

#### Intra-test-time Self-evolution (during a single task execution)

- **ICL-based**: Reflexion, SELF, AdaPlanner, TrustAgent -- adapt via in-context learning within the current episode
- **SFT-based**: Self-Adaptive LM, TTT-NN, SIFT -- fine-tune parameters during test time
- **RL-based**: LADDER, Ttrl -- reinforcement learning during inference

#### Inter-test-time Self-evolution (between tasks/episodes)

- **SFT-based**: SELF, STaR, Quiet-STaR, SiriuS -- supervised fine-tuning between episodes
- **RL-based**: RAGEN, Learning-Like-Humans, WebRL, DigiRL -- RL-based improvement between episodes

### Taxonomy: HOW to Evolve

#### Reward-based Self-Evolution

- **Textual Feedback**: Reflexion, AdaPlanner, AgentS2, SELF, Self-Refine, SCoRe, PAG, TextGrad
- **Internal Rewards**: CISC, Self-Ensemble, SRSI, Self-Certainty, Self-Rewarding Language Models
- **External Rewards**: Self-Train LM, SICA, RAGEN, SPIRAL, LADDER, AutoRule, and many others
- **Implicit Rewards**: Reward Is Enough, Endogenous reward

#### Imitation and Demonstration Learning

- **Self-Generated**: STaR, V-STaR, AdaSTaR, STIC, GENIXER
- **Others**: SiriuS, SOFT, RISE, IoE

#### Population-based and Evolutionary Methods

- **Single Agent**: DGM, GENOME, SPIN, SPC, STL
- **Multi-Agent**: EvoMAC, Puppeteer, MDTeamGPT, MedAgentSim

### Evaluation Dimensions

- **Adaptivity**: Can the agent adapt to new tasks/environments?
- **Generalization**: Does improvement transfer across domains?
- **Efficiency**: Cost of evolution vs. improvement gained
- **Safety**: Does evolution preserve safety constraints?

Evaluation paradigms: static benchmarks, short-horizon (single task), long-horizon (multi-episode streams).

### Open Problems and Future Directions

1. **Safety during evolution**: Preventing evolved agents from developing unsafe behaviors
2. **Personalization**: Adapting to individual user preferences while maintaining generality
3. **Multi-agent co-evolution**: Coordinating evolution across agent populations
4. **Scalability**: Efficient evolution as agent complexity grows
5. **Co-evolution of evaluation and agents**: Benchmarks must evolve alongside agents
6. **Catastrophic forgetting**: Retaining previously learned capabilities during evolution

### Key Insights for Autonomic

1. **Evolve context (prompts + memory) first**: Lowest risk, highest immediate impact, no model weight changes needed. This maps directly to evolving CLAUDE.md, rules files, and MCP configs
2. **Inter-test-time evolution is the sweet spot**: Collect performance signals across sessions, evolve between sessions. This is exactly Autonomic's operating model
3. **Textual feedback is the most practical reward signal**: For a config-evolving daemon, parsing execution logs and task outcomes as textual feedback is the most accessible approach
4. **Population-based methods work for config search**: Maintaining a population of config variants and evolving them via selection/crossover/mutation maps well to the problem
5. **Tool evolution (selection + mastery)** is a distinct axis: Autonomic should track which MCP tools get used, which fail, and evolve tool routing/descriptions
6. **Architecture evolution is possible**: Even the agentic workflow (single vs. multi-agent, sequential vs. parallel) can be evolved, not just the prompts

## Paper 3: Self-Improving LLM Agents at Test-Time (TT-SI)

**Source**: arXiv:2510.07841v1 (Oct 2025), UIUC (Acikgoz, Qian, Ji, Hakkani-Tur, Tur)

### Core Architecture

A three-stage test-time self-improvement algorithm:

1. **Self-Awareness**: Uncertainty Estimator (H) identifies samples the model struggles with
2. **Self-Data Augmentation**: Data Synthesis Function (G) generates similar examples from uncertain samples
3. **Self-Improvement**: Test-Time Fine-tuning (T) applies lightweight updates using generated instances

Two variants:

- **TT-SI**: Same model generates + learns from its own uncertain cases
- **TT-D**: Stronger teacher model generates examples for uncertain cases (distillation)

### Algorithm (Pseudocode from Algorithm 1)

```txt
Input: D_test, model M, generation prompt P, temp dataset size K, initial params theta_0

For each x_i in D_test:
  Step 1: Uncertainty Estimator (H)
    Compute NLL for each candidate action:
      l_n = -log P_M(a_n | x_i)  for all a_n
    Apply Relative Softmax Scoring (RSS):
      p_n = exp(l_n - max_j l_j) / sum_k exp(l_k - max_j l_j)
    Compute uncertainty:
      u(x_i) = p^(1) - p^(2)    # highest minus second-highest RSS scores

  Step 2: Data Synthesis Function (G)
    If u(x_i) < tau:             # below uncertainty threshold = uncertain
      Generate K synthetic samples:
        D_i = L_gen(x_i, K)     # LLM generates similar examples

  Step 3: Test-Time Fine-tuning (T)
      Learn temporary params via LoRA:
        theta_i* = argmin_{theta_0} sum_{(x',y') in D_i} loss(M(x'; theta_0), y')
      Infer with adapted params:
        y_hat_i = M(x_i; theta_i*)
      Reset params:
        theta_i* -> theta_0      # restore original

    Else:
      Infer directly:
        y_hat_i = M(x_i; theta_0)
```

### Key Formulas

**Uncertainty Estimator (H)**: Uses Relative Softmax Scoring (RSS) over negative log-likelihoods of candidate actions. The confidence gap `u(x_i) = p^(1) - p^(2)` measures how much more confident the model is about its top choice vs. second choice. Low gap = high uncertainty.

**Self-improvement objective (from Section 2.3)**:

```txt
theta_i ~= argmax_theta r_self(y | x_i, theta),  y ~ M_theta(. | x_i)
```

Where `r_self` is an implicit intrinsic reward induced by the model's own objective. Self-improvement works through a **sharpening mechanism**: the model iteratively refines its output distribution to favor high-confidence predictions, surfacing **hidden knowledge** in latent representations.

**Empirical risk minimization at test time**:

```txt
L_train(theta) = (1/N) * sum_{i=1}^{N} loss(F_theta(x_i), y_i)
```

### Fundamental Problems with Inductive Fine-Tuning (motivating TT-SI)

1. **Distributional shift**: P_test != P_train
2. **Computation cost**: N >> 10^4 samples required
3. **Redundancy**: N_eff << N (most samples are redundant)
4. **Catastrophic forgetting**: Fine-tuning on new tasks degrades old skills

### Benchmark Results

| Benchmark | Baseline | TT-SI | Delta |
| --- | --- | --- | --- |
| ToolAlpaca | - | - | +5.84% |
| NexusRaven | - | - | +6.05% |
| SealTool | 66.37% | 72.43% | +5.76% |
| API-Bank | - | - | +4.26% |
| **Average** | - | - | **+5.48%** |

**Data efficiency**: TT-SI uses **68x fewer training samples** than standard SFT while surpassing its accuracy on SealTool (72.43% vs 70.20%).

### Ablation Results

- **Uncertainty filtering is critical**: Adapting to ALL test inputs (without filtering) yields only +1.04% gain with 104 additional LoRA weight updates vs. 190 uncertain samples identified by H
- **Cheating experiment**: TTT trained on actual test set achieves 78.89% on SealTool; TT-SI achieves 72.43% -- remarkably close, suggesting similar (not exact) samples are sufficient
- **ICL fallback**: When training is infeasible, TT-SI with ICL (training-free) outperforms standard ICL baselines
- **Scaling**: TT-SI consistently outperforms SFT across all data scales, with improvements becoming more pronounced as more uncertain examples are incorporated

### Key Insights for Autonomic

1. **Uncertainty-guided adaptation is the key principle**: Don't try to improve everything uniformly. Detect which configs/tasks the system struggles with and focus evolution there
2. **Self-awareness before self-improvement**: The uncertainty estimator (measuring confidence gap between top-1 and top-2 choices) is more important than the improvement mechanism itself
3. **Minimal data suffices**: Even a single synthesized training instance per uncertain case produces significant gains. Autonomic doesn't need massive datasets -- targeted generation from failure cases works
4. **Temporary adaptations are safe**: TT-SI resets parameters after each instance. Autonomic could similarly maintain "temporary evolved configs" for specific task types, reverting to baseline when done
5. **The sharpening mechanism**: Self-improvement doesn't create knowledge ex nihilo -- it surfaces hidden knowledge already in the model. For Autonomic, this means good configs may already exist in the search space; evolution just needs to find and amplify them
6. **68x data efficiency**: Targeted, uncertainty-aware improvement is vastly more efficient than brute-force training on all data

## Cross-Paper Synthesis: Implementation Patterns for Autonomic

### Architecture Recommendation

Combining insights from all three papers, Autonomic should implement:

```txt
                     +-------------------+
                     |   Config Space    |
                     |  (ARTEMIS-style)  |
                     |  P x T x M x R^l |
                     +--------+----------+
                              |
                     +--------v----------+
                     | Uncertainty       |
                     | Detection         |
                     | (TT-SI style)     |
                     | Which tasks fail? |
                     +--------+----------+
                              |
              +---------------+---------------+
              |                               |
     +--------v----------+          +--------v----------+
     | Local Evolution    |          | Global Evolution   |
     | (Semantic GA)      |          | (Bayesian Opt)     |
     | Per-component      |          | Cross-component    |
     | mutations          |          | combinations       |
     +--------+-----------+          +--------+-----------+
              |                               |
              +---------------+---------------+
                              |
                     +--------v----------+
                     | Hierarchical      |
                     | Evaluation        |
                     | LLM score -> bench|
                     +--------+----------+
                              |
                     +--------v----------+
                     | Selection &       |
                     | Population Mgmt   |
                     | (Survey taxonomy) |
                     +-------------------+
```

### Concrete Implementation Steps

1. **Config Representation** (from ARTEMIS):
   - Parse CLAUDE.md, rules/*.md, settings.json into a structured config `C = (P, T, M, Theta)`
   - P = system prompts, rules, instructions
   - T = MCP tool descriptions, routing tables
   - M = model routing decisions (which model for which task)
   - Theta = temperature, timeout, retry parameters

2. **Fitness Function** (from ARTEMIS + TT-SI):
   - Primary: task success rate from execution logs
   - Secondary: token cost, latency, error rate
   - Use NL goal specification: "maximize task completion while minimizing token cost"
   - Implement hierarchical eval: quick LLM scoring as filter, then actual benchmark

3. **Uncertainty Detection** (from TT-SI):
   - Track which task categories the agent struggles with
   - Measure confidence gaps in model responses (if accessible)
   - Alternatively: track failure rates per task type as uncertainty proxy
   - Focus evolution effort on high-uncertainty task categories

4. **Evolution Loop** (from ARTEMIS + Survey):
   - **When**: Inter-session (between Claude Code sessions) -- the survey's "inter-test-time" paradigm
   - **What**: Prompts and memory first (lowest risk), then tool configs, then model routing
   - **How**: Semantic GA with LLM-ensemble mutations for NL components; standard GA for discrete/continuous params
   - Maintain population of config variants (size 5-10)
   - Tournament selection with elitism

5. **Genetic Operators** (from ARTEMIS):
   - **Mutation**: Use an LLM to generate semantically meaningful variations of prompt/rule text
   - **Crossover**: Use an LLM to intelligently combine sections from two high-fitness configs
   - **Both preserve validity**: Unlike random perturbation, LLM operators maintain syntactic and semantic correctness

6. **Safety** (from Survey):
   - Always maintain a known-good baseline config
   - Temporary configs for exploration (TT-SI's reset pattern)
   - Rollback on regression detection
   - Human-in-the-loop approval for configs that change significantly

### What Worked vs. What Didn't (Across Papers)

**What worked**:

- Semantic (LLM-driven) mutation/crossover >> random perturbation for NL components
- Joint optimization of interdependent components >> isolated component optimization
- Uncertainty-guided selective adaptation >> uniform adaptation across all cases
- Hierarchical evaluation (cheap filter + expensive benchmark) for cost efficiency
- Black-box approach requiring no agent architectural changes
- Population-based evolutionary search for multimodal config landscapes
- Minimal targeted data (even 1 sample) for test-time adaptation

**What didn't work / limitations**:

- Well-tuned agents showed limited room for further optimization (ARTEMIS)
- Adapting ALL samples without uncertainty filtering was wasteful (+1.04% vs +5.48%) (TT-SI)
- Standard SFT with large datasets was less effective than targeted TT-SI with 68x fewer samples
- Prompt-only optimization misses tool/param interdependencies (Survey finding)
- Catastrophic forgetting remains unsolved for continuous evolution (Survey)
- Evaluation co-evolution: static benchmarks become stale as agents improve (Survey)
- Multi-agent co-evolution coordination is an open problem (Survey)

### Key Numbers to Remember

| Metric | Value | Source |
| --- | --- | --- |
| ARTEMIS improvement range | 10-36% | Paper 1 |
| TT-SI average accuracy gain | +5.48% | Paper 3 |
| TT-SI data efficiency vs SFT | 68x fewer samples | Paper 3 |
| Uncertainty filtering efficiency | +5.48% vs +1.04% (all samples) | Paper 3 |
| Multi-agent failure modes | 14 types across 3 categories | MAST taxonomy via Paper 1 |
| Prompting techniques in literature | 41+ distinct techniques | Survey via Paper 1 |
| AlphaCodium flow engineering gain | 19% -> 44% on CodeContests | Paper 1 related work |
| ReAct improvement on ALFWorld | +34% absolute | Paper 1 related work |
| Tree of Thoughts on Game of 24 | 4% -> 74% | Paper 1 related work |
