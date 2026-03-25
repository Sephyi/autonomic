# Autonomic Hook System Architecture

## 1. Overview: Structure > Willpower

A 10-line hook is a guarantee. A 1000-line prompt is a wish.

Claude Code hooks provide structural enforcement of behaviors that natural language instructions cannot guarantee. No matter how explicit a CLAUDE.md instruction is, an LLM can drift, ignore, or "forget" it after compaction. A hook is code that runs outside the model's inference loop. It cannot be forgotten. It cannot be negotiated with. It either passes or it blocks.

### Concrete contrast

**Instruction-based approach (wish):**

```markdown
# CLAUDE.md
IMPORTANT: Never run `rm -rf` on any directory. Always run rustfmt after editing Rust files.
```

The agent follows this 95% of the time. After compaction, it drops to 60%. Under complex multi-step reasoning, it occasionally "decides" the formatting step is unnecessary. One `rm -rf /` is all it takes.

**Hook-based approach (guarantee):**

```json
{
  "hooks": {
    "PreToolUse": [{
      "matcher": "Bash",
      "hooks": [{
        "type": "command",
        "command": "~/.autonomic/hooks/command-guard.py",
        "timeout": 5
      }]
    }],
    "PostToolUse": [{
      "matcher": "Edit|Write",
      "hooks": [{
        "type": "command",
        "command": "~/.autonomic/hooks/format-on-edit.sh",
        "timeout": 30
      }]
    }]
  }
}
```

The agent cannot bypass these. `rm -rf /` is blocked with exit code 2 before execution. Every edit is formatted before the agent sees the result. The hooks run regardless of what the agent "thinks" it should do.

### The compaction problem

When Claude Code's context window fills, it compacts (summarizes) the conversation. After compaction, the agent retains a compressed summary but loses specific instructions, identity context, and task state. This is the single most dangerous moment in a long session. The agent is confused, disoriented, and prone to errors.

The 164th Lesson: inject content directly into the confused agent. Do not ask it to read files, recall context, or help itself. It cannot. Push the information in through stdout, which Claude Code automatically injects into the agent's context.

## 2. Claude Code Hook Lifecycle

### Event types and when they fire

| Event | Fires When | Stdin Contains | Can Block? |
| --- | --- | --- | --- |
| `SessionStart` | Session startup, resume from pause, resume after compaction | `session_id`, `cwd`, `hook_event_name`, trigger reason | No |
| `UserPromptSubmit` | User submits a prompt (before processing) | `session_id`, `cwd`, `hook_event_name`, `user_prompt` | No |
| `InstructionsLoaded` | CLAUDE.md / instructions files loaded | `session_id`, `cwd`, `hook_event_name`, loaded content | No |
| `SubagentStart` | A subagent (Task) is spawned | `session_id`, `cwd`, `hook_event_name`, subagent config | No |
| `PreToolUse` | Before a tool call executes | `session_id`, `cwd`, `hook_event_name`, `tool_name`, `tool_input` | Yes (exit 2) |
| `PostToolUse` | After a tool call completes | `session_id`, `cwd`, `hook_event_name`, `tool_name`, `tool_input`, `tool_output` | No |
| `PreCompact` | Before context compaction begins | `session_id`, `cwd`, `hook_event_name` | No |
| `Notification` | Agent emits a notification | `session_id`, `cwd`, `hook_event_name`, notification content | No |
| `SubagentStop` | A subagent completes | `session_id`, `cwd`, `hook_event_name`, subagent result | No |
| `Stop` | Session ends (agent stops or user exits) | `session_id`, `cwd`, `hook_event_name` | No |

### Stdin format (all events)

Every hook receives a JSON object on stdin:

```json
{
  "session_id": "abc123",
  "cwd": "/Users/dev/myproject",
  "hook_event_name": "PreToolUse",
  "tool_name": "Bash",
  "tool_input": {
    "command": "rm -rf /tmp/build"
  }
}
```

Fields vary by event. `session_id`, `cwd`, and `hook_event_name` are always present. `tool_name` and `tool_input` are present for `PreToolUse` and `PostToolUse`. `PostToolUse` additionally includes `tool_output`.

### Exit codes

| Exit Code | Meaning | Effect |
| --- | --- | --- |
| 0 | Success / allow | Tool call proceeds. Stdout content is injected into agent context. |
| 1 | Error | Hook failed. Logged but does not block. Stderr content logged. |
| 2 | Block | Tool call is prevented. Stdout/stderr reason shown to agent. |

### Output channels

- **stdout**: Content injected into the agent's conversation context. This is how hooks communicate with the agent. For `PreToolUse`, stdout can also return structured JSON to modify the tool input or issue a deny decision.
- **stderr**: Logged for debugging. For exit code 2, stderr content is shown to the agent as the block reason.

### Structured output (PreToolUse)

A `PreToolUse` hook can return structured JSON on stdout to modify behavior:

```json
{
  "hookSpecificOutput": {
    "hookEventName": "PreToolUse",
    "permissionDecision": "deny",
    "permissionDecisionReason": "Destructive command blocked by policy"
  }
}
```

Or to modify the tool input before execution:

```json
{
  "hookSpecificOutput": {
    "hookEventName": "PreToolUse",
    "permissionDecision": "allow",
    "updatedInput": {
      "command": "npm test 2>&1 | grep -A 5 -E '(FAIL|ERROR)' | head -100"
    }
  }
}
```

## 3. Hook Registration Format

Hooks are registered in `.claude/settings.json` at three levels:

| Level | File | Scope |
| --- | --- | --- |
| User (global) | `~/.claude/settings.json` | All projects for this user |
| Project | `<project>/.claude/settings.json` | This project only (committed) |
| Project local | `<project>/.claude/settings.local.json` | This project, this machine (gitignored) |

Hooks from all levels are merged. Within the same event, hooks execute in registration order. A block from any hook prevents the tool call.

### Schema

```json
{
  "hooks": {
    "<EventName>": [
      {
        "matcher": "<regex pattern matching tool_name or trigger>",
        "hooks": [
          {
            "type": "command",
            "command": "<shell command to execute>",
            "timeout": 30
          }
        ]
      }
    ]
  }
}
```

### Capability Format: Agent Skills Standard

Hook and skill definitions follow the **Agent Skills Standard** SKILL.md format for maximum portability across 20+ platforms (Claude Code, Codex, Cursor, VS Code, Gemini CLI, Microsoft Agent Framework). The format uses YAML frontmatter with 6 fields:

```yaml
---
name: command-guard
description: PreToolUse hook that blocks dangerous commands before execution.
license: MIT
compatibility:
  - claude-code
  - codex
  - cursor
metadata:
  author: autonomic
  version: '1.0'
allowed-tools:
  - Bash
---
```

Followed by markdown instructions and optional `scripts/` and `references/` directories. Progressive disclosure: at startup only name + description are loaded (~100 tokens per skill). Full content loaded on activation.

- **matcher**: A regex tested against `tool_name` for `PreToolUse`/`PostToolUse`, or against trigger context for lifecycle events. Use `".*"` or `"*"` for unconditional matching.
- **type**: `"command"` (run a script) or `"prompt"` (ask the agent to self-evaluate, weaker but zero-code).
- **command**: Shell command. Supports `~` expansion and environment variables.
- **timeout**: Seconds before the hook is killed. Default varies by event. Autonomic sets explicit timeouts on all hooks.

### Environment variables available to hooks

| Variable | Description |
| --- | --- |
| `CLAUDE_SESSION_ID` | Current session identifier |
| `CLAUDE_PROJECT_DIR` | Project root directory |
| `CLAUDE_PLUGIN_ROOT` | Plugin root (if hook is from a plugin) |

## 4. Managed Hooks

Autonomic manages six categories of hooks. Each is described below with its complete source, rationale, and registration.

### 4.1 Compaction Recovery Hook

**File**: `~/.autonomic/hooks/compaction-recovery.sh`
**Event**: `SessionStart`
**Matcher**: `".*"` (fires on startup, resume, and compact)
**Level**: User (global)
**Criticality**: Maximum. This is the single most important hook in the system.

#### Why it exists

After compaction, the agent loses:

- Its identity and operating principles
- Current task state and progress
- Memory context (what has been tried, what failed)
- Cognitive principles (verification requirements, error handling patterns)

Without this hook, post-compaction behavior degrades catastrophically. The agent makes previously-rejected decisions, repeats failed approaches, and loses coordination with the orchestrator.

#### Design principles (The 164th Lesson)

1. **Inject, do not reference.** Print the actual content to stdout. Do not print "read file X" because the confused agent may not follow through.
2. **Be concise but complete.** The injected content consumes context window space. Include only what the agent needs to function correctly.
3. **Phase the recovery.** Identity first, then task state, then cognitive principles. The agent processes sequentially.
4. **Detect the trigger.** Differentiate between startup (full context available), resume (partial), and compact (minimal). Scale injection accordingly.

#### Source

```bash
#!/usr/bin/env bash
# compaction-recovery.sh — Autonomic hook for identity recovery after compaction
# Registered as SessionStart hook in ~/.claude/settings.json
# Exit 0 always; stdout is injected into agent context.

set -euo pipefail

INPUT=$(cat)
SESSION_ID=$(echo "$INPUT" | jq -r '.session_id // "unknown"')
CWD=$(echo "$INPUT" | jq -r '.cwd // "."')
EVENT=$(echo "$INPUT" | jq -r '.hook_event_name // "SessionStart"')

AUTONOMIC_DIR="${HOME}/.autonomic"
STATE_DIR="${AUTONOMIC_DIR}/state"
MEMORY_DIR="${AUTONOMIC_DIR}/memory"
IDENTITY_FILE="${AUTONOMIC_DIR}/identity.md"
PRINCIPLES_FILE="${AUTONOMIC_DIR}/cognitive-principles.md"
TASK_STATE_FILE="${STATE_DIR}/current-task.json"
SESSION_HISTORY_FILE="${STATE_DIR}/sessions/${SESSION_ID}.jsonl"

# Determine trigger type from context
# After compaction, the session_id persists but context is compressed.
# We detect compaction by checking if a marker file exists that was written
# by the PreCompact hook.
TRIGGER="startup"
COMPACT_MARKER="${STATE_DIR}/.compact-pending-${SESSION_ID}"
RESUME_MARKER="${STATE_DIR}/.session-active-${SESSION_ID}"

if [[ -f "$COMPACT_MARKER" ]]; then
    TRIGGER="compact"
    rm -f "$COMPACT_MARKER"
elif [[ -f "$RESUME_MARKER" ]]; then
    TRIGGER="resume"
fi

# Mark session as active (for resume detection on next start)
mkdir -p "${STATE_DIR}/sessions"
touch "$RESUME_MARKER"

# --- Phase 1: Identity ---
# Always inject on compact, conditionally on resume/startup
if [[ "$TRIGGER" == "compact" ]] || [[ "$TRIGGER" == "startup" ]]; then
    if [[ -f "$IDENTITY_FILE" ]]; then
        echo "=== IDENTITY RECOVERY (post-${TRIGGER}) ==="
        echo ""
        cat "$IDENTITY_FILE"
        echo ""
    fi
fi

# --- Phase 2: Task State ---
# Critical after compaction. The agent must know what it was doing.
if [[ -f "$TASK_STATE_FILE" ]]; then
    TASK_NAME=$(jq -r '.task_name // "unknown"' "$TASK_STATE_FILE")
    TASK_STATUS=$(jq -r '.status // "unknown"' "$TASK_STATE_FILE")
    TASK_STEP=$(jq -r '.current_step // "unknown"' "$TASK_STATE_FILE")
    TASK_CONTEXT=$(jq -r '.context // ""' "$TASK_STATE_FILE")
    COMPLETED_STEPS=$(jq -r '.completed_steps // [] | join(", ")' "$TASK_STATE_FILE")
    REMAINING_STEPS=$(jq -r '.remaining_steps // [] | join(", ")' "$TASK_STATE_FILE")

    echo "=== CURRENT TASK STATE ==="
    echo "Task: ${TASK_NAME}"
    echo "Status: ${TASK_STATUS}"
    echo "Current step: ${TASK_STEP}"
    if [[ -n "$COMPLETED_STEPS" ]]; then
        echo "Completed: ${COMPLETED_STEPS}"
    fi
    if [[ -n "$REMAINING_STEPS" ]]; then
        echo "Remaining: ${REMAINING_STEPS}"
    fi
    if [[ -n "$TASK_CONTEXT" ]]; then
        echo ""
        echo "${TASK_CONTEXT}"
    fi
    echo ""
fi

# --- Phase 3: Relevant Memory ---
# Query the memory store for context relevant to the current task/project
if [[ "$TRIGGER" == "compact" ]] || [[ "$TRIGGER" == "startup" ]]; then
    PROJECT_NAME=$(basename "$CWD")
    MEMORY_FILE="${MEMORY_DIR}/${PROJECT_NAME}.jsonl"

    if [[ -f "$MEMORY_FILE" ]]; then
        # Extract the 5 most recent, highest-relevance memories
        MEMORIES=$(tail -20 "$MEMORY_FILE" | jq -r '.content' 2>/dev/null | tail -5)
        if [[ -n "$MEMORIES" ]]; then
            echo "=== RELEVANT MEMORY CONTEXT ==="
            echo "$MEMORIES"
            echo ""
        fi
    fi
fi

# --- Phase 4: Cognitive Principles ---
# Only inject after compaction (startup gets these from CLAUDE.md)
if [[ "$TRIGGER" == "compact" ]]; then
    if [[ -f "$PRINCIPLES_FILE" ]]; then
        echo "=== COGNITIVE PRINCIPLES (re-injection after compaction) ==="
        echo ""
        cat "$PRINCIPLES_FILE"
        echo ""
    fi
fi

# --- Phase 5: Session Continuity Marker ---
echo "=== SESSION ==="
echo "Session ID: ${SESSION_ID}"
echo "Trigger: ${TRIGGER}"
echo "Working directory: ${CWD}"
echo "Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)"

exit 0
```

#### Registration

```json
{
  "hooks": {
    "SessionStart": [
      {
        "matcher": ".*",
        "hooks": [
          {
            "type": "command",
            "command": "bash ~/.autonomic/hooks/compaction-recovery.sh",
            "timeout": 10
          }
        ]
      }
    ]
  }
}
```

#### Supporting hook: PreCompact marker

The compaction recovery hook needs to know that compaction occurred. This is achieved by a `PreCompact` hook that writes a marker file:

```bash
#!/usr/bin/env bash
# pre-compact-marker.sh — Writes a marker so SessionStart knows compaction happened
set -euo pipefail

INPUT=$(cat)
SESSION_ID=$(echo "$INPUT" | jq -r '.session_id // "unknown"')
STATE_DIR="${HOME}/.autonomic/state"
mkdir -p "$STATE_DIR"
touch "${STATE_DIR}/.compact-pending-${SESSION_ID}"

exit 0
```

### 4.2 Session Start Context Hook

**File**: `~/.autonomic/hooks/session-start-context.sh`
**Event**: `SessionStart`
**Matcher**: `".*"`
**Level**: User (global)

#### Why it exists

On fresh startup or resume (not compaction, which is handled by the recovery hook), assemble relevant memory context from the Autonomic memory store. This primes the agent with cross-session knowledge before it begins work.

#### Source

```bash
#!/usr/bin/env bash
# session-start-context.sh — Injects assembled memory context on session start
# Runs AFTER compaction-recovery.sh (registration order matters)
set -euo pipefail

INPUT=$(cat)
SESSION_ID=$(echo "$INPUT" | jq -r '.session_id // "unknown"')
CWD=$(echo "$INPUT" | jq -r '.cwd // "."')

AUTONOMIC_DIR="${HOME}/.autonomic"
MEMORY_INDEX="${AUTONOMIC_DIR}/memory/index.json"

# Only run on non-compact starts (compaction recovery handles its own memory)
COMPACT_MARKER="${AUTONOMIC_DIR}/state/.compact-pending-${SESSION_ID}"
if [[ -f "$COMPACT_MARKER" ]]; then
    exit 0
fi

# Check for project-specific context
PROJECT_NAME=$(basename "$CWD")
PROJECT_CONTEXT="${AUTONOMIC_DIR}/memory/projects/${PROJECT_NAME}.md"

if [[ -f "$PROJECT_CONTEXT" ]]; then
    echo "=== PROJECT CONTEXT: ${PROJECT_NAME} ==="
    cat "$PROJECT_CONTEXT"
    echo ""
fi

# Check for recent experience traces (last 3 sessions in this project)
TRACES_DIR="${AUTONOMIC_DIR}/traces"
if [[ -d "$TRACES_DIR" ]]; then
    RECENT_ERRORS=$(find "$TRACES_DIR" -name "*.jsonl" -newer "$TRACES_DIR" -mtime -7 \
        -exec grep -l "\"project\":\"${PROJECT_NAME}\"" {} \; 2>/dev/null \
        | head -3 \
        | xargs -I {} tail -5 {} 2>/dev/null \
        | jq -r 'select(.level == "error") | .message' 2>/dev/null \
        | head -5)

    if [[ -n "$RECENT_ERRORS" ]]; then
        echo "=== RECENT ERRORS (last 7 days) ==="
        echo "$RECENT_ERRORS"
        echo ""
    fi
fi

# Check for pending tasks
PENDING_TASKS="${AUTONOMIC_DIR}/state/pending-tasks.json"
if [[ -f "$PENDING_TASKS" ]]; then
    PROJECT_TASKS=$(jq -r --arg proj "$PROJECT_NAME" \
        '.[] | select(.project == $proj and .status == "pending") | "- \(.name): \(.description)"' \
        "$PENDING_TASKS" 2>/dev/null)

    if [[ -n "$PROJECT_TASKS" ]]; then
        echo "=== PENDING TASKS ==="
        echo "$PROJECT_TASKS"
        echo ""
    fi
fi

exit 0
```

### 4.3 Session Metrics Hook

**File**: `~/.autonomic/hooks/session-metrics.sh`
**Event**: `Stop`
**Matcher**: `".*"`
**Level**: User (global)

#### Why it exists

Collect structured metrics at session end for the experience trace system. Metrics feed the evolution engine, which identifies patterns in failures, costs, and tool usage to propose improvements.

#### Source

```bash
#!/usr/bin/env bash
# session-metrics.sh — Collect session metrics on Stop event
set -euo pipefail

INPUT=$(cat)
SESSION_ID=$(echo "$INPUT" | jq -r '.session_id // "unknown"')
CWD=$(echo "$INPUT" | jq -r '.cwd // "."')

AUTONOMIC_DIR="${HOME}/.autonomic"
TRACES_DIR="${AUTONOMIC_DIR}/traces"
SESSION_STATE="${AUTONOMIC_DIR}/state/sessions/${SESSION_ID}.jsonl"
METRICS_FILE="${TRACES_DIR}/$(date -u +%Y-%m-%d).jsonl"

mkdir -p "$TRACES_DIR"

# Calculate session duration from state file timestamps
START_TIME=""
if [[ -f "${AUTONOMIC_DIR}/state/.session-active-${SESSION_ID}" ]]; then
    START_TIME=$(stat -f %m "${AUTONOMIC_DIR}/state/.session-active-${SESSION_ID}" 2>/dev/null \
        || stat -c %Y "${AUTONOMIC_DIR}/state/.session-active-${SESSION_ID}" 2>/dev/null \
        || echo "")
fi

END_TIME=$(date +%s)
DURATION=0
if [[ -n "$START_TIME" ]]; then
    DURATION=$(( END_TIME - START_TIME ))
fi

# Count tool usage from session history if available
TOOL_COUNT=0
ERROR_COUNT=0
BLOCK_COUNT=0
if [[ -f "$SESSION_STATE" ]]; then
    TOOL_COUNT=$(wc -l < "$SESSION_STATE" | tr -d ' ')
    ERROR_COUNT=$(grep -c '"level":"error"' "$SESSION_STATE" 2>/dev/null || echo 0)
    BLOCK_COUNT=$(grep -c '"blocked":true' "$SESSION_STATE" 2>/dev/null || echo 0)
fi

PROJECT_NAME=$(basename "$CWD")

# Write metrics entry
jq -n \
    --arg sid "$SESSION_ID" \
    --arg proj "$PROJECT_NAME" \
    --arg cwd "$CWD" \
    --argjson duration "$DURATION" \
    --argjson tools "$TOOL_COUNT" \
    --argjson errors "$ERROR_COUNT" \
    --argjson blocks "$BLOCK_COUNT" \
    --arg ts "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
    '{
        type: "session_end",
        session_id: $sid,
        project: $proj,
        cwd: $cwd,
        duration_seconds: $duration,
        tool_calls: $tools,
        errors: $errors,
        blocked_actions: $blocks,
        timestamp: $ts
    }' >> "$METRICS_FILE"

# Clean up session markers
rm -f "${AUTONOMIC_DIR}/state/.session-active-${SESSION_ID}"
rm -f "${AUTONOMIC_DIR}/state/.compact-pending-${SESSION_ID}"

exit 0
```

### 4.4 Command Guard

**File**: `~/.autonomic/hooks/command-guard.py`
**Event**: `PreToolUse`
**Matcher**: `"Bash"`
**Level**: Project (installed by Autonomic into each managed project)

#### Why it exists

Prevents catastrophic and destructive commands. Two severity levels:

- **L1 (always block)**: Commands that are catastrophic and never legitimate in an agent context. `rm -rf /`, `mkfs`, `dd if=/dev/zero`, `:(){ :|:& };:`, `> /dev/sda`, `chmod -R 777 /`.
- **L2 (block with override)**: Commands that are dangerous but occasionally needed. `git push --force`, `git reset --hard`, `DROP TABLE`, `TRUNCATE`, `rm -rf` on non-root paths. The agent is told why the command was blocked and can rephrase.

#### Source

```python
#!/usr/bin/env python3
"""
command-guard.py — Autonomic PreToolUse hook for Bash command validation.
Blocks destructive commands at two severity levels.

Exit codes:
  0 — Command allowed
  1 — Hook error (logged, does not block)
  2 — Command blocked (tool call prevented)

Stdout JSON with deny reason is injected into agent context.
"""

import json
import re
import sys
from typing import NamedTuple


class Rule(NamedTuple):
    pattern: str
    message: str
    level: str  # "L1" (always block) or "L2" (block with explanation)


# L1: Catastrophic operations. Never allow under any circumstances.
L1_RULES: list[Rule] = [
    Rule(
        r"rm\s+-[^\s]*r[^\s]*f[^\s]*\s+/\s*$",
        "Blocked: rm -rf / is catastrophic and never permitted",
        "L1",
    ),
    Rule(
        r"rm\s+-[^\s]*f[^\s]*r[^\s]*\s+/\s*$",
        "Blocked: rm -fr / is catastrophic and never permitted",
        "L1",
    ),
    Rule(
        r"mkfs\.",
        "Blocked: filesystem formatting is never permitted",
        "L1",
    ),
    Rule(
        r"dd\s+if=/dev/(zero|urandom|random)\s+of=/dev/",
        "Blocked: raw device writes are never permitted",
        "L1",
    ),
    Rule(
        r":\(\)\s*\{\s*:\|:\s*&\s*\}\s*;",
        "Blocked: fork bomb detected",
        "L1",
    ),
    Rule(
        r">\s*/dev/sd[a-z]",
        "Blocked: raw device overwrites are never permitted",
        "L1",
    ),
    Rule(
        r"chmod\s+-R\s+777\s+/\s*$",
        "Blocked: recursive chmod 777 on root is never permitted",
        "L1",
    ),
    Rule(
        r"curl\s+.*\|\s*(sudo\s+)?bash",
        "Blocked: piping curl to bash is not permitted",
        "L1",
    ),
    Rule(
        r"wget\s+.*\|\s*(sudo\s+)?bash",
        "Blocked: piping wget to bash is not permitted",
        "L1",
    ),
]

# L2: Dangerous operations. Block with explanation; agent can rephrase.
L2_RULES: list[Rule] = [
    Rule(
        r"git\s+push\s+.*--force(?!-with-lease)",
        "Blocked: use --force-with-lease instead of --force to avoid overwriting others' work",
        "L2",
    ),
    Rule(
        r"git\s+reset\s+--hard",
        "Blocked: git reset --hard discards uncommitted changes. Commit or stash first, "
        "or use git reset --soft / git stash",
        "L2",
    ),
    Rule(
        r"git\s+clean\s+-[^\s]*f",
        "Blocked: git clean -f permanently deletes untracked files. Use git clean -n (dry run) first",
        "L2",
    ),
    Rule(
        r"\bDROP\s+(TABLE|DATABASE|SCHEMA)\b",
        "Blocked: DROP operations require explicit confirmation. Rephrase with a comment "
        "explaining why this is intentional",
        "L2",
    ),
    Rule(
        r"\bTRUNCATE\s+TABLE\b",
        "Blocked: TRUNCATE is destructive. Use DELETE with a WHERE clause, or confirm intent",
        "L2",
    ),
    Rule(
        r"rm\s+-[^\s]*r[^\s]*f",
        "Warning: rm -rf on this path. Verify the target is correct. "
        "Consider using trash-put or moving to /tmp instead",
        "L2",
    ),
    Rule(
        r"sudo\s+rm\s+-[^\s]*r",
        "Blocked: sudo rm -r is extremely dangerous. Remove sudo or use a more targeted deletion",
        "L2",
    ),
]


def check_command(command: str) -> tuple[str, str] | None:
    """Check command against all rules. Returns (message, level) or None."""
    # Normalize: collapse whitespace, strip
    cmd = " ".join(command.split())

    # L1 rules first (always block, no negotiation)
    for rule in L1_RULES:
        if re.search(rule.pattern, cmd, re.IGNORECASE):
            return (rule.message, rule.level)

    # L2 rules (block with explanation)
    for rule in L2_RULES:
        if re.search(rule.pattern, cmd, re.IGNORECASE):
            return (rule.message, rule.level)

    return None


def main() -> None:
    try:
        input_data = json.load(sys.stdin)
    except json.JSONDecodeError as e:
        print(f"command-guard: failed to parse stdin: {e}", file=sys.stderr)
        sys.exit(1)

    tool_name = input_data.get("tool_name", "")
    if tool_name != "Bash":
        sys.exit(0)

    command = input_data.get("tool_input", {}).get("command", "")
    if not command:
        sys.exit(0)

    result = check_command(command)
    if result is None:
        sys.exit(0)

    message, level = result

    # Emit structured deny output
    output = {
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "deny",
            "permissionDecisionReason": f"[{level}] {message}",
        }
    }
    json.dump(output, sys.stdout)
    print(file=sys.stdout)  # trailing newline

    # Also emit to stderr for logging
    print(f"command-guard [{level}]: {message}", file=sys.stderr)

    sys.exit(2)


if __name__ == "__main__":
    main()
```

#### Registration (project-level)

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "python3 ~/.autonomic/hooks/command-guard.py",
            "timeout": 5
          }
        ]
      }
    ]
  }
}
```

### 4.5 Format-on-Edit Hooks

**File**: `~/.autonomic/hooks/format-on-edit.sh`
**Event**: `PostToolUse`
**Matcher**: `"Edit|Write"`
**Level**: Project

#### Why it exists

Ensures every file edit is immediately formatted according to project standards. The agent sees the formatted result, not its raw output. This prevents formatting drift and teaches the agent the correct style through observation.

#### Source

```bash
#!/usr/bin/env bash
# format-on-edit.sh — Auto-format files after Edit/Write tool calls
set -euo pipefail

INPUT=$(cat)
TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name // ""')
CWD=$(echo "$INPUT" | jq -r '.cwd // "."')

# Extract the file path from tool_input
# Edit tool uses "file_path", Write tool uses "file_path"
FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // ""')

if [[ -z "$FILE_PATH" ]]; then
    exit 0
fi

# Resolve relative paths
if [[ "$FILE_PATH" != /* ]]; then
    FILE_PATH="${CWD}/${FILE_PATH}"
fi

# Skip if file does not exist (Write may have failed)
if [[ ! -f "$FILE_PATH" ]]; then
    exit 0
fi

# Determine formatter by file extension
EXT="${FILE_PATH##*.}"

case "$EXT" in
    rs)
        if command -v rustfmt &>/dev/null; then
            rustfmt --edition 2021 "$FILE_PATH" 2>/dev/null
            echo "Formatted ${FILE_PATH##*/} with rustfmt"
        fi
        ;;
    ts|tsx|js|jsx|json|jsonc)
        if command -v biome &>/dev/null; then
            biome format --write "$FILE_PATH" 2>/dev/null
            echo "Formatted ${FILE_PATH##*/} with biome"
        elif command -v npx &>/dev/null; then
            npx --yes @biomejs/biome format --write "$FILE_PATH" 2>/dev/null
            echo "Formatted ${FILE_PATH##*/} with biome (npx)"
        fi
        ;;
    py)
        if command -v ruff &>/dev/null; then
            ruff format "$FILE_PATH" 2>/dev/null
            echo "Formatted ${FILE_PATH##*/} with ruff"
        fi
        ;;
    go)
        if command -v gofmt &>/dev/null; then
            gofmt -w "$FILE_PATH" 2>/dev/null
            echo "Formatted ${FILE_PATH##*/} with gofmt"
        fi
        ;;
    *)
        # No formatter for this extension
        ;;
esac

exit 0
```

### 4.6 Lint-on-Edit Hooks

**File**: `~/.autonomic/hooks/lint-on-edit.sh`
**Event**: `PostToolUse`
**Matcher**: `"Edit|Write"`
**Level**: Project

#### Why it exists

Runs the appropriate linter immediately after formatting. Lint errors are injected into the agent's context on stdout, so the agent sees and can fix them in its next action. This creates a tight feedback loop: edit, format, lint, see errors, fix.

#### Source

```bash
#!/usr/bin/env bash
# lint-on-edit.sh — Auto-lint files after Edit/Write tool calls
# Runs AFTER format-on-edit.sh (registration order matters)
set -euo pipefail

INPUT=$(cat)
CWD=$(echo "$INPUT" | jq -r '.cwd // "."')
FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // ""')

if [[ -z "$FILE_PATH" ]]; then
    exit 0
fi

if [[ "$FILE_PATH" != /* ]]; then
    FILE_PATH="${CWD}/${FILE_PATH}"
fi

if [[ ! -f "$FILE_PATH" ]]; then
    exit 0
fi

EXT="${FILE_PATH##*.}"
LINT_OUTPUT=""

case "$EXT" in
    rs)
        if command -v cargo &>/dev/null; then
            # Run clippy on the specific file's crate
            # Find the nearest Cargo.toml
            DIR=$(dirname "$FILE_PATH")
            while [[ "$DIR" != "/" ]]; do
                if [[ -f "$DIR/Cargo.toml" ]]; then
                    LINT_OUTPUT=$(cd "$DIR" && cargo clippy --message-format=short 2>&1 \
                        | grep -E "^(warning|error)" | head -10)
                    break
                fi
                DIR=$(dirname "$DIR")
            done
        fi
        ;;
    ts|tsx|js|jsx)
        if command -v biome &>/dev/null; then
            LINT_OUTPUT=$(biome check "$FILE_PATH" 2>&1 | head -20)
        fi
        ;;
    py)
        if command -v ruff &>/dev/null; then
            LINT_OUTPUT=$(ruff check "$FILE_PATH" 2>&1 | head -20)
        fi
        ;;
    go)
        if command -v golangci-lint &>/dev/null; then
            LINT_OUTPUT=$(cd "$(dirname "$FILE_PATH")" && golangci-lint run "$(basename "$FILE_PATH")" 2>&1 | head -20)
        fi
        ;;
esac

if [[ -n "$LINT_OUTPUT" ]]; then
    echo "=== LINT RESULTS: ${FILE_PATH##*/} ==="
    echo "$LINT_OUTPUT"
fi

exit 0
```

## 5. Compaction Recovery in Detail

The compaction recovery hook is Autonomic's most critical component. This section describes its multi-phase operation and the supporting infrastructure.

### The compaction timeline

```txt
1. Agent is working, context window fills up
2. PreCompact hook fires → writes marker file (.compact-pending-<session_id>)
3. Claude Code compacts the conversation (summarizes to ~20% of context)
4. SessionStart fires (because the session "restarts" after compaction)
5. compaction-recovery.sh detects the marker file → TRIGGER="compact"
6. Hook outputs identity, task state, memory, and cognitive principles to stdout
7. Claude Code injects stdout into the agent's new (compacted) context
8. Agent resumes work with critical context restored
```

### What gets injected (compact trigger)

| Phase | Content | Size Budget |
| --- | --- | --- |
| 1. Identity | Operating principles, role definition, key constraints | ~500 tokens |
| 2. Task State | Current task name, step, completed/remaining steps, context notes | ~300 tokens |
| 3. Memory | Top 5 most relevant memories for this project from the memory store | ~400 tokens |
| 4. Cognitive Principles | Verification requirements, error handling, decision frameworks | ~300 tokens |
| 5. Session Marker | Session ID, trigger type, working directory, timestamp | ~50 tokens |

Total budget: approximately 1,550 tokens. This is 1-2% of a 128k context window, an acceptable overhead for dramatically improved post-compaction behavior.

### Identity file format

```markdown
# Identity

You are operating within the Autonomic orchestration system.

## Operating Principles
- Verify before committing: run tests, check types, lint before any commit
- Structure > Willpower: hooks enforce what instructions cannot
- Memory is external: check memory store before repeating investigation
- Admit uncertainty: say "I don't know" rather than guessing

## Constraints
- Never bypass hooks or suggest disabling them
- Always check task state before starting work (you may be resuming)
- Format and lint are automatic — do not manually format code
```

### Cognitive principles file format

```markdown
# Cognitive Principles (re-injected after compaction)

## Verification
- After any non-trivial change: run the relevant test suite
- Before commit: type check, lint, test
- If a test fails twice with the same approach: stop, reassess, check memory

## Error Handling
- Read the full error message before acting
- Check if this error has been seen before (memory store)
- Fix root causes, not symptoms

## Decision Making
- Prefer reversible actions over irreversible ones
- When uncertain between approaches: choose the simpler one
- Document non-obvious decisions in code comments
```

### Task state file format

```json
{
    "task_name": "Implement hook evolution engine",
    "status": "in_progress",
    "current_step": "Writing proposal evaluation logic",
    "completed_steps": [
        "Defined hook proposal schema",
        "Implemented pattern detector",
        "Created proposal storage"
    ],
    "remaining_steps": [
        "Write proposal evaluation logic",
        "Implement hook generation from proposals",
        "Add safety validation for generated hooks",
        "Integration tests"
    ],
    "context": "The evolution engine analyzes experience traces to propose new hooks. Each proposal must pass safety validation before installation. Currently working on the scoring function that ranks proposals by impact and confidence.",
    "updated_at": "2026-03-25T14:30:00Z"
}
```

## 6. Hook Installation and Update

Autonomic manages hook installation through its Rust orchestrator binary. The orchestrator owns the lifecycle of hook files and their registration in settings.json.

### Installation flow

```txt
autonomic init <project-dir>
  │
  ├─ 1. Create ~/.autonomic/hooks/ directory (if not exists)
  ├─ 2. Write global hook scripts (compaction-recovery.sh, session-start-context.sh,
  │     session-metrics.sh, pre-compact-marker.sh)
  ├─ 3. Register global hooks in ~/.claude/settings.json
  │     (merge with existing hooks, do not overwrite user hooks)
  ├─ 4. Write project hook scripts to <project>/.autonomic/hooks/
  ├─ 5. Register project hooks in <project>/.claude/settings.json
  │     (merge with existing hooks)
  └─ 6. Create supporting directories:
        ~/.autonomic/state/
        ~/.autonomic/state/sessions/
        ~/.autonomic/memory/
        ~/.autonomic/memory/projects/
        ~/.autonomic/traces/
        ~/.autonomic/identity.md (from template if not exists)
        ~/.autonomic/cognitive-principles.md (from template if not exists)
```

### Merge strategy for settings.json

Autonomic never overwrites existing hooks. It merges by:

1. Reading the existing `settings.json` and parsing the `hooks` object.
2. For each event type (e.g., `PreToolUse`), checking if an Autonomic hook with the same command path already exists.
3. If it exists: update the timeout and matcher if they have changed.
4. If it does not exist: append the new hook entry to the event's array.
5. Write the updated JSON back, preserving all non-hook settings.

Each Autonomic-managed hook command path contains `~/.autonomic/hooks/` or `<project>/.autonomic/hooks/`, making them identifiable during merge.

### Update flow

```txt
autonomic update
  │
  ├─ 1. Compare installed hook scripts against current templates
  ├─ 2. Diff and update changed scripts in-place
  ├─ 3. Update registrations if matchers or timeouts changed
  └─ 4. Report changes to stdout
```

### Uninstall flow

```txt
autonomic uninstall <project-dir>
  │
  ├─ 1. Remove Autonomic hook entries from <project>/.claude/settings.json
  ├─ 2. Remove <project>/.autonomic/ directory
  └─ 3. Optionally remove global hooks (--global flag)
```

## 7. Hook Evolution

The evolution engine is Autonomic's mechanism for self-improvement. It analyzes experience traces (collected by session-metrics.sh and other instrumentation) to identify recurring failure patterns, then proposes new hooks to prevent those failures structurally.

### Evolution pipeline

```txt
Experience Traces (JSONL)
  │
  ├─ Pattern Detection
  │   Scan traces for recurring errors, repeated tool blocks,
  │   timeout patterns, or high-cost sessions
  │
  ├─ Proposal Generation
  │   Create a HookProposal with:
  │   - Event type (PreToolUse, PostToolUse, etc.)
  │   - Matcher pattern
  │   - Hook script (generated)
  │   - Rationale (why this hook would help)
  │   - Confidence score (0.0 - 1.0)
  │   - Evidence (list of trace entries that triggered this)
  │
  ├─ Safety Validation
  │   - Generated script is statically analyzed (no network calls,
  │     no writes outside ~/.autonomic/, no privilege escalation)
  │   - Script is executed in a sandbox with mock stdin
  │   - Exit codes are verified (must be 0 or 2, never 1)
  │   - Timeout is tested
  │
  ├─ Human Review Gate
  │   - Proposals above confidence 0.8 are auto-staged
  │   - Proposals 0.5 - 0.8 are presented for review
  │   - Proposals below 0.5 are logged but not proposed
  │
  └─ Installation
      - Approved hooks are written to ~/.autonomic/hooks/evolved/
      - Registered in settings.json with an "evolved" comment
      - Tracked in ~/.autonomic/evolution/installed.json
```

### Proposal schema

```rust
/// A hook proposed by the evolution engine.
pub struct HookProposal {
    /// Unique identifier for this proposal.
    pub id: String,

    /// The lifecycle event this hook targets.
    pub event: HookEvent,

    /// Regex matcher for the hook.
    pub matcher: String,

    /// The generated hook script content.
    pub script: String,

    /// Script language (bash or python3).
    pub language: ScriptLanguage,

    /// Human-readable rationale.
    pub rationale: String,

    /// Confidence score from 0.0 to 1.0.
    pub confidence: f64,

    /// Experience trace entries that motivated this proposal.
    pub evidence: Vec<TraceReference>,

    /// Proposed timeout in seconds.
    pub timeout: u32,

    /// Whether this proposal has been approved.
    pub status: ProposalStatus,

    /// Timestamp of proposal creation.
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub enum HookEvent {
    PreToolUse,
    PostToolUse,
    SessionStart,
    Stop,
    PreCompact,
    Notification,
    UserPromptSubmit,
    InstructionsLoaded,
    SubagentStart,
    SubagentStop,
}

pub enum ScriptLanguage {
    Bash,
    Python3,
}

pub enum ProposalStatus {
    Pending,
    Staged,
    Approved,
    Installed,
    Rejected,
    Retired,
}
```

### Example: evolved hook from error patterns

Suppose the experience traces show that across 15 sessions, the agent attempted to run `cargo test` 8 times without first running `cargo check`, and 5 of those times `cargo test` failed with compilation errors (wasting 30+ seconds each time). The evolution engine proposes:

```bash
#!/usr/bin/env bash
# evolved-cargo-check-before-test.sh
# Proposed by evolution engine based on 5 compilation failures in cargo test
# Confidence: 0.85
# Evidence: traces/2026-03-18.jsonl:142, traces/2026-03-19.jsonl:87, ...
set -euo pipefail

INPUT=$(cat)
COMMAND=$(echo "$INPUT" | jq -r '.tool_input.command // ""')

# If the command is cargo test, first run cargo check
if echo "$COMMAND" | grep -qE '^cargo\s+test'; then
    CWD=$(echo "$INPUT" | jq -r '.cwd // "."')
    CHECK_OUTPUT=$(cd "$CWD" && cargo check 2>&1)
    CHECK_EXIT=$?

    if [[ $CHECK_EXIT -ne 0 ]]; then
        echo "Pre-check failed: cargo check reports compilation errors."
        echo "Fix compilation errors before running tests."
        echo ""
        echo "$CHECK_OUTPUT" | tail -20
        exit 2
    fi
fi

exit 0
```

## 8. Hook Metrics Collection

Every hook execution is instrumented by the Autonomic runtime. The orchestrator wraps each hook invocation and records:

### Metrics schema

```rust
pub struct HookExecution {
    /// Which hook was executed.
    pub hook_path: String,

    /// The lifecycle event that triggered it.
    pub event: HookEvent,

    /// The matcher that was evaluated.
    pub matcher: String,

    /// Wall-clock execution time in milliseconds.
    pub duration_ms: u64,

    /// Exit code returned by the hook.
    pub exit_code: i32,

    /// Whether the hook blocked a tool call (exit code 2).
    pub blocked: bool,

    /// Whether the hook timed out.
    pub timed_out: bool,

    /// Size of stdout output in bytes.
    pub stdout_bytes: usize,

    /// Size of stderr output in bytes.
    pub stderr_bytes: usize,

    /// Session ID.
    pub session_id: String,

    /// Timestamp.
    pub timestamp: chrono::DateTime<chrono::Utc>,
}
```

### Metrics aggregation

The orchestrator periodically (or on `autonomic stats`) aggregates hook metrics:

| Metric | Computation | Purpose |
| --- | --- | --- |
| P50/P95/P99 latency per hook | Percentiles of `duration_ms` | Identify slow hooks for optimization |
| Block rate per hook | `blocked_count / total_count` | Tune guard sensitivity |
| Error rate per hook | `exit_1_count / total_count` | Identify broken hooks |
| Timeout rate per hook | `timed_out_count / total_count` | Adjust timeout values |
| Context injection volume | Sum of `stdout_bytes` per session | Monitor context window budget |
| Blocks by command pattern | Group blocked commands by regex | Feed evolution engine |

### Dashboard output

```txt
$ autonomic stats hooks --last 7d

Hook                          Calls   P50ms  P95ms  Blocks  Errors  Timeouts
─────────────────────────────────────────────────────────────────────────────
compaction-recovery.sh          47    12ms    45ms     0       0        0
session-start-context.sh        47     8ms    22ms     0       0        0
session-metrics.sh              47     5ms    11ms     0       0        0
command-guard.py               312     3ms     8ms    14       0        0
format-on-edit.sh              891    85ms   340ms     0       2        1
lint-on-edit.sh                891   220ms   890ms     0       5        0
evolved-cargo-check.sh          38   450ms  1200ms     3       0        0

Top blocked commands (command-guard.py):
  git push --force           6 blocks
  rm -rf ./target            4 blocks
  sudo rm -rf                2 blocks
  DROP TABLE                 2 blocks
```

## 9. Template System

Autonomic generates project-specific hooks from templates. Templates are parameterized by project language, framework, and configuration.

### Template structure

```txt
~/.autonomic/templates/
├── hooks/
│   ├── command-guard.py.tmpl        # Language-agnostic
│   ├── format-on-edit.sh.tmpl       # Parameterized by language
│   ├── lint-on-edit.sh.tmpl         # Parameterized by language + linter config
│   └── project-conventions.sh.tmpl  # Project-specific conventions
└── settings/
    ├── global-hooks.json.tmpl       # User-level hook registration
    └── project-hooks.json.tmpl      # Project-level hook registration
```

### Template variables

```rust
pub struct TemplateContext {
    /// Primary language: "rust", "typescript", "python", "go"
    pub language: String,

    /// Formatter command: "rustfmt", "biome format", "ruff format", "gofmt"
    pub formatter: String,

    /// Formatter args (edition, config path, etc.)
    pub formatter_args: Vec<String>,

    /// Linter command: "cargo clippy", "biome check", "ruff check", "golangci-lint"
    pub linter: String,

    /// Linter args
    pub linter_args: Vec<String>,

    /// Project root path
    pub project_root: String,

    /// Additional blocked command patterns (project-specific)
    pub extra_blocked_patterns: Vec<BlockPattern>,

    /// File extensions to apply format/lint hooks to
    pub watched_extensions: Vec<String>,
}
```

### Template rendering

Templates use a minimal Handlebars-like syntax processed by the Rust orchestrator:

```bash
#!/usr/bin/env bash
# format-on-edit.sh — Generated by Autonomic for {{language}} project
set -euo pipefail

INPUT=$(cat)
FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // ""')

if [[ -z "$FILE_PATH" ]]; then
    exit 0
fi

EXT="${FILE_PATH##*.}"

case "$EXT" in
{{#each watched_extensions}}
    {{this}})
        {{../formatter}} {{#each ../formatter_args}}{{this}} {{/each}}"$FILE_PATH" 2>/dev/null
        echo "Formatted ${FILE_PATH##*/} with {{../formatter}}"
        ;;
{{/each}}
esac

exit 0
```

### Project detection

When `autonomic init` is run, it detects project characteristics:

| Signal | Detection Method | Sets |
| --- | --- | --- |
| `Cargo.toml` exists | File presence | `language=rust`, `formatter=rustfmt`, `linter=cargo clippy` |
| `package.json` + tsconfig | File presence | `language=typescript`, `formatter=biome`, `linter=biome check` |
| `pyproject.toml` or `setup.py` | File presence | `language=python`, `formatter=ruff format`, `linter=ruff check` |
| `go.mod` exists | File presence | `language=go`, `formatter=gofmt`, `linter=golangci-lint` |
| `biome.json` exists | File presence | Overrides biome config path in `formatter_args` |
| `.rustfmt.toml` exists | File presence | Overrides rustfmt config in `formatter_args` |

## 10. Failures That Hooks Prevent and Instructions Miss

### Case 1: Post-compaction identity loss

**Scenario**: 90-minute session implementing a complex feature. Context fills at minute 75. Compaction occurs. Agent resumes but has lost awareness of the task plan, completed steps, and verification requirements.

**Without hook**: Agent re-reads the codebase, makes changes that conflict with work done in the first 75 minutes, skips the verification pipeline it was previously following, and introduces a regression.

**With compaction-recovery.sh**: Within 50ms of the post-compaction SessionStart, the agent receives its identity, the exact task state (step 4 of 7, steps 1-3 completed), memory context about a tricky edge case discovered at minute 30, and the cognitive principles requiring verification. It resumes at step 4 without regression.

### Case 2: Destructive command during refactoring

**Scenario**: Agent is refactoring a directory structure. It decides to `rm -rf src/old-module/` to clean up. But `src/old-module/` contains files that have been moved but not yet committed.

**Without hook**: The files are deleted. The agent does not realize they were uncommitted. The refactoring is now incomplete and the uncommitted work is lost.

**With command-guard.py**: The `rm -rf` is blocked at L2. The agent receives: "Warning: rm -rf on this path. Verify the target is correct. Consider using trash-put or moving to /tmp instead." The agent checks git status, discovers the uncommitted files, commits first, then proceeds.

### Case 3: Formatting drift in multi-file edit

**Scenario**: Agent edits 12 TypeScript files in a single task. It applies its own formatting preferences (2-space indentation instead of tabs, single quotes instead of double quotes). The project uses Biome with specific settings.

**Without hook**: All 12 files now have inconsistent formatting. The pre-commit hook catches it, but the agent has already moved on and the diff is noisy. Review time doubles.

**With format-on-edit.sh**: Each of the 12 edits is immediately formatted by Biome. The agent sees the Biome-formatted result after each edit. By the third file, it has adapted its output to match Biome's preferences, reducing wasted formatting cycles. The commit is clean.

### Case 4: Force push to shared branch

**Scenario**: Agent encounters a merge conflict and decides the fastest resolution is `git push --force origin main`.

**Without hook**: The force push overwrites a colleague's work that was pushed 10 minutes ago. The colleague's changes are lost from the remote.

**With command-guard.py**: The command is blocked at L2 with: "use --force-with-lease instead of --force to avoid overwriting others' work." The agent uses `--force-with-lease`, which fails because the remote has diverged, prompting a proper merge.

### Case 5: Repeated test failure without reassessment

**Scenario**: Agent runs `cargo test`, gets a compilation error, makes a fix, runs `cargo test` again, gets the same error (different root cause), makes another fix, runs again. This cycle repeats 5 times, each compilation taking 30 seconds.

**Without hook**: 2.5 minutes wasted on compilation cycles. The agent never stops to check if the fundamental approach is wrong.

**With evolved-cargo-check-before-test.sh** (proposed by evolution engine): Before each `cargo test`, `cargo check` runs first (5 seconds). Compilation errors are caught immediately with clear diagnostics. The agent fixes all compilation errors before running the full test suite. Cycle time drops from 2.5 minutes to 40 seconds.

### Case 6: Lost memory across sessions

**Scenario**: In session 1, the agent spends 20 minutes debugging a subtle issue with async lifetime bounds in a specific module. In session 2 (next day), the agent encounters the same module and the same issue.

**Without hook**: The agent repeats the entire 20-minute investigation, arriving at the same conclusion.

**With session-start-context.sh**: On session 2 startup, the hook injects recent project memories, including: "async lifetime issue in src/transport/stream.rs resolved by pinning the future with Box::pin(). The root cause was that the Stream trait bound required 'static but the borrow was scoped to the connection lifetime." The agent applies the fix directly.
