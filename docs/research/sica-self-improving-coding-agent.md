# SICA: Self-Improving Coding Agent - Implementation Reference

Paper: "A Self-Improving Coding Agent" (arXiv:2504.15228v2, May 2025)
Authors: Maxime Robeyns (U. Bristol), Martin Szummer (iGent AI), Laurence Aitchison (U. Bristol)
Code: https://github.com/MaximeRobeyns/self_improving_coding_agent
License: MIT

## Architecture

### High-Level Design

SICA is a single unified agent that serves as both the meta-agent (the improver) and the target-agent (the thing being improved). This is the key distinction from ADAS, which has a separate fixed meta-agent. In SICA, improvements to coding ability compound because the same agent that gets better at coding is the one doing the improving.

The system is implemented in standard Python (no DSL). The agent can edit its entire codebase, not just a single forward function.

### Core Loop (Algorithm 1)

```txt
Input: Benchmarks B = {b_1, ..., b_m}, iteration count n
Output: Improved agent A_n

1. Initialize agent A_0
2. for i = 0, 1, ..., n-1 do
3.   Evaluate A_i on benchmarks B, store results in p_i
4.   Run A_hat_i to generate A_{i+1}, using archive of A_{<=i} and p_{<=i}
     where hat_i = argmax_{j in {0,...,i}} p_j
5. return A_n
```

Key: at each iteration, the BEST-performing agent from the archive (not necessarily the latest) is selected as the meta-agent. This agent then examines the full archive of past agents and their results, identifies an improvement, and implements it on a copy of the best agent's code.

### Component Map

```txt
runner.py                         # Outer loop: benchmark + improve cycle
base_agent/
  agent.py                        # Entry point
  src/
    agents/
      base_agent.py               # Core agent loop (35KB)
      assistant_base_agent.py     # Assistant-mode agent (35KB)
      agent_calling.py            # Sub-agent invocation mechanism
      implementations/
        main_orchestrator.py      # Top-level orchestrator agent
        coder.py                  # Software developer sub-agent
        problem_solver.py         # Problem-solving sub-agent
        reasoner.py               # Reasoning sub-agent
        archive_explorer.py       # Archive analysis sub-agent
        review_committee_member.py # Code review sub-agent
    tools/
      base_tool.py                # Tool base class (14KB)
      base_agent_tools.py         # Open/close/overwrite files, shell exec
      file_tools.py               # File manipulation
      directory_tools.py          # Directory navigation
      execute_command.py          # Shell command execution
      calculator.py               # Simple calculator
      archive_tools.py            # Archive analysis tool (10KB)
      answer_submission.py        # Submit final answer to benchmark
      ripgrep_tool.py             # Code search (20KB)
      committee_design.py         # Review committee tool
      edit_tools/                 # File editing tools
      reasoning_structures/       # Reasoning structure tools
    oversight/
      overseer.py                 # Asynchronous LLM overseer (22KB)
      graph_visualisation.py      # Execution tree rendering
    benchmarks/
      base.py                     # BaseBenchmark, BenchmarkTracker
      swebench_verified.py        # SWE-Bench Verified
      livecodebench.py            # LiveCodeBench
      file_editing.py             # File editing benchmark
      symbol_location.py          # Symbol location benchmark
      humaneval.py                # HumanEval
      gsm8k.py, aime.py, gpqa.py # Math/reasoning benchmarks
      aiq_benchmark.py            # Synthetic agent benchmarks
    events/                       # Event bus system
    callgraph/                    # Agent call graph tracking
    llm/                          # LLM API abstraction
    config.py                     # Model selection, parameters
    utils/
      archive_analysis.py         # ArchiveAnalyzer (33KB)
```

## Utility Function

### Formula

```txt
U = w_score * p_score + w_cost * (1 - min(1, p_cost / $10)) + w_time * (1 - min(1, p_time / 300s))
```

Where:
- `p_score` in [0, 1]: normalized benchmark performance
- `p_cost`: dollar cost per problem
- `p_time`: wall-clock seconds per problem
- `w_score = 0.5`, `w_cost = 0.25`, `w_time = 0.25`
- Cost cap: $10 per problem (anything above saturates to 0 cost utility)
- Time cap: 300 seconds (hard timeout, agent cancelled after this)

### Timeout Penalty

```txt
U_final = U * (1 - tau)   if timed out
U_final = U               otherwise
```

Where `tau = 0.5`. This gives partial credit for work done before timeout.

### Implementation Note (from code)

The actual code in `archive_analysis.py` uses a simpler utility in `BenchmarkScore.utility()`:

```python
def utility(self, cost_limit=3.0, time_limit=300):
    if (self.avg_score > 0.0 and
        self.cost <= cost_limit and
        self.time <= time_limit and
        not self.timed_out):
        return self.avg_score
    else:
        return 0.0
```

This is a binary gate: if cost exceeds $3 or time exceeds 5 minutes or it timed out, utility is 0. Otherwise utility equals the raw score. The paper formula is the conceptual target; the code implements a stricter version.

### Agent Selection with Confidence Intervals

The `select_base_agent()` function in `runner.py` uses 95% confidence intervals:

1. Compute per-iteration statistics with `compute_statistics()`
2. Find the iteration with the highest mean score
3. Get its lower confidence bound: `ci_lower = mean - 1.96 * (std / sqrt(n))`
4. Scan from current iteration backward; first agent whose mean score >= best's lower bound is selected
5. This prevents regression to a lucky outlier

## Self-Editing Mechanism

### How the Agent Modifies Its Own Code

1. The runner copies the best agent's code directory to a new `agent_{i+1}/agent_code/` directory
2. The best-performing agent is launched inside a Docker container
3. The container mounts:
   - The full archive as read-only at `/home/agent/archive`
   - The new agent code directory as read-write at `/home/agent/workdir`
4. The agent is invoked in "improve" mode with `--workdir` pointing to its own editable code
5. The agent uses its standard tools (open files, overwrite files, execute shell commands) to modify the code at `/home/agent/workdir`
6. After the meta-improvement completes, the Docker image is rebuilt (in case requirements.txt changed)

### Critical Safety Design

- Agent always runs inside a Docker container (isolation from host)
- Archive is mounted read-only (agent cannot corrupt past results)
- Each benchmark problem gets its own container with a unique name
- Per-problem cost threshold: $3.00 USD
- Per-problem timeout: 10 minutes (600 seconds) with a 10-second grace period
- The runner has an additional `asyncio.timeout` safety net beyond the application-level timeout

### Tools Available to the Agent

Base tools (what the agent starts with):
- `open_file`: Add file contents to the LLM context
- `close_file`: Remove file from context
- `overwrite_file`: Write entire file contents
- `execute_command`: Run shell commands (with timeout, output capture)
- `calculator`: Simple arithmetic
- `submit_answer`: Submit benchmark answer
- `return_result`: Return value from sub-agent to caller
- `early_exit`: Exit sub-agent early
- `archive_analysis`: Extract summary statistics from the archive

The agent can (and does) create new tools for itself, such as:
- SmartEditor (AST-based precise code editing)
- Symbol locator (find definitions across codebase)
- Diff summarizer (compact representation of changes)
- Ripgrep integration (already present in base)

## Evaluation Loop

### Per-Iteration Cycle

```txt
for each iteration i:
  1. Run ALL registered benchmarks on agent_i
     - Each benchmark has N problems
     - Problems run in parallel (default 8 workers)
     - Each problem gets its own Docker container
     - Results stored in results.jsonl per benchmark
     - Traces stored in per-problem directories
  2. Generate performance statistics (perf.json per benchmark)
  3. Generate contextual summaries of each problem trace
     (using Gemini Flash 2 for cheap summarization)
  4. Select best agent from archive using confidence intervals
  5. Launch best agent in "improve" mode
     - Agent reads archive, analyzes failures
     - Agent identifies improvement opportunity
     - Agent modifies its own code
  6. Reset benchmark progress for next iteration
```

### Benchmark Configuration

Default subset size: 18 problems per benchmark (for speed). The benchmarks include:
- GSM8K (enabled by default in code)
- SWE-Bench Verified
- LiveCodeBench
- HumanEval
- File Editing (synthetic)
- Symbol Location (synthetic)
- AIME, GPQA, DROP, MATH
- AIQ benchmarks (linalg, CSV parsing, messaging app, distributed KV store)

### Trace Summarization

After each problem, a summary is generated using Gemini Flash 2:
- Input: problem statement, agent's answer, execution trace, score, parse errors
- Output: 1-2 paragraph critical analysis (logical approach? unnecessary steps? where reasoning failed? specific improvements?)
- These summaries go into the archive and are available to the meta-improvement agent

## Discovered Modifications (Concrete Examples)

The paper reports that across iterations, the agent discovered and implemented these categories of improvements:

### Tool Improvements
- **SmartEditor**: AST-based file editing tool that replaces the naive overwrite_file, enabling precise function/class-level edits without full file rewrites
- **Symbol Locator**: Tool to find symbol definitions across the codebase using AST parsing
- **Diff Summarizer**: Compact diff representations for the agent to review changes
- **Enhanced ripgrep integration**: Better code search with context

### Prompt Engineering
- The agent modified its own system prompts and agent definitions
- Improved problem decomposition strategies in the orchestrator prompt
- Better instructions for sub-agents on how to approach SWE-Bench style problems

### Orchestration Changes
- Modified file management strategies (what to keep in context, when to close files)
- Changed problem decomposition heuristics
- Improved delegation patterns between sub-agents

### Concrete Numbers
- SWE-Bench Verified: **17% to 53%** accuracy (on 18-problem random subset)
- File Editing benchmark: **82% to 94%**
- LiveCodeBench: additional gains (exact numbers not specified in available text)
- These gains came purely from scaffold changes, no weight updates

## Sub-Agent Architecture

### Design Philosophy

Sub-agents are invoked like tool calls: they accept input arguments and return a string value. This serves two purposes:
1. **Autonomous operation**: The main agent decomposes tasks and delegates without human guidance
2. **Context management**: Sub-agent calls naturally discard their context when they return, with only the return value summarizing the sub-task in the parent's context. This is critical for hour-long meta-improvement runs.

### Initial Sub-Agents (3)

1. **Coder** (`coder.py`, 14KB): Software developer agent with file editing tools, shell access, and coding-specific instructions
2. **Problem Solver** (`problem_solver.py`, 8KB): General problem-solving agent for benchmark tasks
3. **Reasoner** (`reasoner.py`, 12KB): Reasoning-focused agent, can take up to 5 minutes without feedback (overseer is told not to interrupt it)

### Additional Sub-Agents

4. **Main Orchestrator** (`main_orchestrator.py`, 9KB): Top-level entrypoint agent that receives the problem statement and delegates
5. **Archive Explorer** (`archive_explorer.py`, 12KB): Specialized agent for analyzing the archive during meta-improvement
6. **Review Committee Member** (`review_committee_member.py`, 5KB): Agent for code review tasks

### Context Window Structure

```txt
+---------------------------+
| System Prompt             |
|   - Agent definition      |
|   - Tool documentation    |
|   - Sub-agent docs        |
|   - System instructions   |
+---------------------------+
| Core Prompt (user msg)    |
|   - Problem statement     |
|   - Open file views       |
|   - Directory tree        |
+---------------------------+
| Assistant Messages        |
|   - CoT reasoning         |
|   - Tool calls + results  |
|   - Sub-agent calls       |
|   - Overseer notifications|
+---------------------------+
```

Content is append-only to preserve KV cache. File edits are appended as diffs, periodically reconsolidated into the stale file view.

### Event Stream Model

Each agent maintains an event stream published to a global event bus. The assistant message is reconstructed by filtering this event stream and concatenating values. Event types:
- `ASSISTANT_MESSAGE`: LLM-generated text
- `TOOL_CALL` / `TOOL_RESULT`: Tool invocations
- `AGENT_CALL` / `AGENT_RESULT`: Sub-agent invocations
- `FILE_EVENT`: File opens/edits
- `OVERSEER_NOTIFICATION`: Injected overseer messages (breaks the assistant turn, inserts a user message)
- `OVERSEER_UPDATE`: Overseer analysis records
- `APPLICATION_WARNING` / `APPLICATION_ERROR`: System-level events

## Asynchronous Overseer Design

### Purpose

The overseer is an LLM running in a concurrent thread that monitors the agent's execution. It ensures the agent:
- Remains on-task
- Does not loop
- Does not deviate from instructions
- Makes progress within reasonable time

### Implementation Details

- **Model**: Sonnet 3.7 by default (deliberately different from the main agent model)
- **Default check interval**: 60 seconds
- **Initial delay**: 15 seconds (let agent start working before checking)
- **Reasoning effort**: MEDIUM (to keep cost low)
- **Temperature**: 0.8

### Overseer Judgement Schema

The overseer produces a structured `OverseerJudgement` with these fields:

```txt
making_progress: bool          # Is the agent progressing?
is_looping: bool               # Is the agent stuck in a loop?
currently_running_agent: str   # ID of deepest active agent
needs_notification: bool       # Should an agent be notified?
agent_to_notify: str?          # Which agent to notify
notification_content: str?     # What to tell the agent
notes_for_next_iteration: str? # Persistent state across overseer checks
next_check_type: "time"|"events" # How to schedule next check
next_check_delay: float?       # Seconds until next time-based check (5-500)
target_agent_id: str?          # Agent to monitor for event-based checks
pending_events_count: int?     # Events to wait for before next check
force_cancel_agent: bool       # Force-cancel an agent?
force_cancel_agent_id: str?    # Which agent to cancel
```

### Intervention Mechanisms (escalation ladder)

1. **Notification**: A message is injected into the agent's context as a user message, breaking the assistant turn. The agent sees it as an overseer comment.
2. **Force Cancellation**: The overseer can cancel a running agent. It MUST first notify the parent agent explaining why the child was cancelled, providing handoff notes.
3. **Early Exit Suggestion**: If the task is untenable after multiple attempts, the overseer instructs the main agent to use `early_exit`.

### Dynamic Scheduling

The overseer adapts its check frequency:
- **Time-based**: Check again after N seconds (clamped 5-500s)
- **Event-based**: Wait for N events from a specific agent before checking again
- This prevents wasting tokens on frequent checks when the agent is working normally

### What the Overseer Sees

A comprehensive textual representation of the callgraph:
- Tree of agent and sub-agent execution
- Event stream per agent (LLM messages, tool calls/results, sub-agent calls/results)
- Timestamps for progress tracking
- Verbose content truncated to limit context size

### Key Overseer Rules (from prompt)

- Do not jump to negative conclusions early; wait for enough data
- Do not back-seat drive; the agent sees more than the overseer
- Reasoning agent can take up to 5 minutes without feedback (normal)
- If the main agent tries to call tools directly instead of delegating, intervene immediately
- If the agent writes pytest mocks and gets stuck, suggest end-to-end tests instead
- If the agent tries to test its own tools by invoking them directly (wrong: editing code != running code), intervene
- Make at least one notification attempt before force-cancelling

## Algorithms and State Machines

### Agent Loop State Machine

```txt
IDLE -> RUNNING -> (TOOL_CALL -> TOOL_RESULT)* -> COMPLETED
                -> (AGENT_CALL -> AGENT_RESULT)*
                -> CANCELLED (by overseer)
                -> TIMED_OUT
```

### Runner Main Loop

```txt
for i in range(start_iteration, max_iterations):
  1. run_benchmarks_with_job_queue(benchmarks, max_workers=8)
     - Create asyncio job queue
     - Each worker: pull job -> start Docker container -> run agent -> score -> summarize
  2. run_meta_agent_benchmark()
     a. select_base_agent() using confidence intervals
     b. Copy best agent code to new directory
     c. Start Docker container with archive (RO) + new code (RW)
     d. Run best agent in "improve" mode
     e. Rebuild Docker image if requirements changed
  3. Update metadata, reset benchmark progress
```

### Tool Calling Mechanism

Tools are called via XML in the assistant message. The LLM generates XML whose closing tag is a stop token. The XML is parsed to extract tool name and arguments, validated, executed, and the result is concatenated to the assistant message as a pre-fill for the next LLM call.

## Failure Modes and What Did Not Work

### Reasoning-Dominant Tasks

- **AIME and GPQA**: Performance gains were minimal or negative
- The base LLM (o3-mini) already approached the task ceiling
- Adding tool-based reasoning steps actually disrupted pretrained reasoning models
- Scaffold improvements cannot substitute for model capability on pure reasoning

### Path Dependency

- Early feature choices heavily influence subsequent features
- The self-improvement trajectory is path-dependent; different random seeds lead to different improvements
- Reducing variance across runs is an open problem listed in the repo's TODO

### Agent Confusion During Self-Improvement

- The agent sometimes confuses the code it is editing with the code it is currently running
- When trying to test its own tools by invoking them directly, this fails because the edited code is not the running code
- The overseer has special instructions to catch this pattern

### Loop and Stall Behaviors

- Agents can get stuck in loops (repeating the same action)
- Agents can stall without terminating
- The overseer's escalation ladder (notify -> cancel) handles this
- Pytest/mock traps: agents sometimes get stuck trying to make pytest fixtures work instead of writing simple end-to-end tests

### Meta-Improvement Reliability

- The meta-improvement task is challenging; the main agent may take ~1 hour per iteration
- Not all improvements are beneficial; the archive selection mechanism (best agent, not latest) serves as a rollback

## Benchmarks: Before and After

| Benchmark | Base Agent | After Self-Improvement | Notes |
| --- | --- | --- | --- |
| SWE-Bench Verified (18 subset) | 17% | 53% | +36 percentage points |
| File Editing | 82% | 94% | +12 percentage points |
| LiveCodeBench | improved | improved | Exact numbers not in available text |
| AIME | ~ceiling | no gain | Reasoning-dominant |
| GPQA | ~ceiling | no gain | Reasoning-dominant |

## Cost and Resource Usage

### Per-Problem Limits

- Cost threshold: $3.00 USD per problem (hard cut)
- Timeout: 600 seconds (10 minutes) per problem
- The utility function penalizes at $10 and 300s caps

### Meta-Improvement Cost

- Each meta-improvement iteration takes ~1 hour
- Token usage and cost are logged per run: `tokens|cached|cost|time` format
- The runner uses Gemini Flash 2 for cheap trace summarization

### Infrastructure

- Docker containers per problem (isolation)
- Parallel workers (default 8) for benchmark evaluation
- Multiple LLM providers supported: Anthropic (Claude), OpenAI (GPT-4o, o1, o3), Google (Gemini), Fireworks (DeepSeek/Llama), DeepSeek direct
- Overseer uses a different model (Sonnet 3.7) than the main agent to avoid correlation

### Benchmark Subset

- Default: 18 problems per benchmark (for practical cost)
- Full benchmarks can be enabled by changing `subset_size=None`
- Random seed: 1 (for reproducibility)

## Key Implementation Details for a Rust Port

### What to Replicate

1. **Archive system**: Directory structure with `agent_{i}/agent_code/` and `agent_{i}/benchmarks/{name}/results.jsonl` + `perf.json` + `traces/`
2. **Confidence-interval agent selection**: Not just "pick the best" but use statistical bounds to avoid regression to outliers
3. **Docker isolation**: Each agent run in a sandboxed container with RO archive + RW workspace
4. **Event bus + callgraph**: Central event system that all agents publish to; the callgraph tracks parent-child agent relationships
5. **Append-only context**: Preserve KV cache by only appending to the assistant message; periodic reconsolidation
6. **Overseer as concurrent task**: Separate async task with its own LLM, different model, checking periodically or on events
7. **Structured overseer output**: Parse overseer judgements into typed data; use XML schema with stop tokens
8. **Change log**: `agent_change_log.md` tracks what was changed and whether it was successful, carried forward across iterations

### What to Improve

- The paper notes high variance across runs (different seeds yield different improvement paths)
- The utility function in code is simpler (binary gate) than the paper formula; the paper formula is more nuanced
- The agent currently lacks efficient file editing tools in its base form (SmartEditor is discovered, not built-in)
- No tree-sitter or LSP integration in the base agent (these are expected to be bootstrapped)
- No mechanism for the agent to curate or build its own benchmarks (listed as TODO)
