# Agent Skills Standard & Claude Code Wrappers: Deep Research

Research date: 2026-03-25
Sources: agentskills.io, moazbuilds/claudeclaw, dataelement/Clawith

## 1. Agent Skills Standard (agentskills.io)

### Origin and Adoption

The Agent Skills specification was created by Anthropic and introduced publicly on October 16, 2025. On December 18, 2025, it was published as an open standard for cross-platform portability. As of March 2026, it is supported by Claude Code, OpenAI Codex, GitHub Copilot in VS Code, Cursor, Gemini CLI, Windsurf, Roo Code, Cline, Goose, and Microsoft Agent Framework, among 20+ platforms. Microsoft has formally adopted it in their Agent Framework SDK (C# and Python).

### Directory Structure

A skill is a directory containing at minimum a `SKILL.md` file:

```txt
skill-name/
  SKILL.md          # Required: YAML frontmatter + markdown instructions
  scripts/          # Optional: executable code
  references/       # Optional: documentation loaded on demand
  assets/           # Optional: templates, resources
```

### SKILL.md Format: YAML Frontmatter Schema

| Field | Required | Constraints |
| --- | --- | --- |
| `name` | Yes | Max 64 chars. Lowercase `a-z`, digits, hyphens only. No leading/trailing/consecutive hyphens. Must match parent directory name. |
| `description` | Yes | Max 1024 chars. Non-empty. Describes what the skill does AND when to activate. Keywords drive agent matching. |
| `license` | No | License name or reference to bundled file. |
| `compatibility` | No | Max 500 chars. Environment requirements (product, packages, network). |
| `metadata` | No | Arbitrary `string -> string` key-value map. Used for author, version, etc. |
| `allowed-tools` | No | Space-delimited list of pre-approved tools. Experimental. Example: `Bash(git:*) Bash(jq:*) Read` |

Minimal example:

```yaml
---
name: code-review
description: Review code changes for bugs, style, and best practices. Use when asked to review PRs, diffs, or staged changes.
---
```

Full example:

```yaml
---
name: pdf-processing
description: Extract PDF text, fill forms, merge files. Use when handling PDFs.
license: Apache-2.0
compatibility: Requires python3
metadata:
  author: example-org
  version: "1.0"
allowed-tools: Bash(python3:*) Read
---
```

### Progressive Disclosure Model (Three-Tier Loading)

This is the core architectural insight of the standard:

| Tier | What Loads | When | Token Cost |
| --- | --- | --- | --- |
| 1. Catalog | `name` + `description` only | Session start | ~50-100 tokens per skill |
| 2. Instructions | Full `SKILL.md` body | When skill is activated | <5000 tokens recommended |
| 3. Resources | Scripts, references, assets | When instructions reference them | Varies |

**Key design principle**: An agent with 20 installed skills pays only the catalog cost (~2000 tokens) at startup, not the full instruction cost of all 20 skills. Only activated skills consume instruction tokens.

### Activation Patterns

Agents decide to activate a skill based on the `description` field matching the user's request. The description is the ONLY thing the agent sees at decision time. Critical insight from practitioners: **if a skill does not trigger, the problem is almost always the description, not the instructions**.

Good descriptions include:
- What the skill does (capabilities)
- When to use it (trigger conditions)
- Specific keywords that help agents match tasks

### Discovery Locations

Skills are discovered from filesystem paths at two scopes:

| Scope | Claude Code | OpenAI Codex |
| --- | --- | --- |
| User-level | `~/.claude/skills/` | `~/.codex/skills/` |
| Project-level | `.claude/skills/` | `.codex/skills/` |

Project-level overrides user-level when names collide.

### Client Implementation: Tools Exposed

The Microsoft Agent Framework implementation exposes three tools to agents:
- `load_skill` - retrieves full SKILL.md body when activated
- `read_skill_resource` - fetches supplementary files on demand
- `run_skill_script` - executes scripts in the `scripts/` directory

### Optional Directories

- **`scripts/`**: Executable code. Should be self-contained, document dependencies, handle errors gracefully, output to stdout.
- **`references/`**: Documentation loaded on demand. Good for policy docs, API references, large examples.
- **`assets/`**: Templates, static resources, config files.

### Relevance to Autonomic

The Agent Skills standard is directly usable as the portable capability format for a Rust orchestrator:

1. **YAML frontmatter parsing** is trivial in Rust (serde_yaml).
2. **Progressive disclosure** maps directly to a lazy-loading capability registry: catalog at startup, full load on activation.
3. **The `description` field is the activation signal** - this informs how an orchestrator should match tasks to capabilities.
4. **The `allowed-tools` field** provides a security sandbox model per skill.
5. **Cross-platform compatibility** means skills written for our orchestrator work in Claude Code, Codex, Cursor, etc.
6. **The `metadata` map** can carry orchestrator-specific fields (priority, cost tier, model preference) without breaking the standard.

## 2. ClaudeClaw (moazbuilds/claudeclaw)

### Overview

ClaudeClaw is a lightweight daemon that wraps Claude Code CLI to create a persistent personal assistant. It runs as a background process, executes tasks on cron schedules, responds to Telegram/Discord messages, transcribes voice commands via Whisper, and provides a web dashboard. Built with Bun/TypeScript, single dependency (`ogg-opus-decoder`).

Installed as a Claude Code plugin: `claude plugin marketplace add moazbuilds/claudeclaw`.

### Daemon Architecture

**Process Management** (`src/pid.ts`):
- PID file at `.claude/claudeclaw/daemon.pid` (project-scoped)
- Stale PID detection via `process.kill(pid, 0)` (signal 0 = liveness check)
- Automatic cleanup of stale PID files
- One daemon per project directory (folder-based isolation)

**Entry Point** (`src/index.ts`):
- Simple command dispatch: `start`, `stop`, `status`, `telegram`, `discord`, `send`
- No framework - raw `process.argv` parsing

**Runner** (`src/runner.ts`) - the core execution engine:
- **Serial queue**: All Claude invocations are serialized via a promise chain to prevent concurrent `--resume` on the same session
- **Session lifecycle**: New sessions use `--output-format json` to capture `session_id` from Claude's output; resumed sessions use `--output-format text` with `--resume <sessionId>`
- **System prompt injection**: On EVERY invocation (not just new sessions), uses `--append-system-prompt` to inject identity prompts + CLAUDE.md + security constraints. This is necessary because `--append-system-prompt` does not persist across `--resume`.
- **Rate limit detection**: Regex pattern matching on stdout/stderr for "you've hit your limit" / "out of extra usage"
- **Fallback model**: When primary model hits rate limit, automatically retries with configured fallback model
- **Auto-compact**: When Claude times out (exit code 124), automatically runs `/compact` on the session, then retries the original prompt
- **Turn tracking**: Increments turn counter per successful invocation; warns when approaching compact threshold (25 turns)
- **Timeout**: 5-minute default per Claude invocation, with SIGTERM then SIGKILL after 5s

**Key implementation detail** - how it spawns Claude:

```typescript
const proc = Bun.spawn(["claude", "-p", prompt, "--output-format", outputFormat, ...securityArgs], {
  stdout: "pipe",
  stderr: "pipe",
  env: buildChildEnv(baseEnv, model, api),
});
```

It strips `CLAUDECODE` env var so child processes do not think they are nested. For GLM models, it sets `ANTHROPIC_BASE_URL` to `https://api.z.ai/api/anthropic`.

### Session Management (`src/sessions.ts`)

- Single session file: `.claude/claudeclaw/session.json`
- Schema: `{ sessionId, createdAt, lastUsedAt, turnCount, compactWarned }`
- In-memory cache with file persistence
- Session backup on reset (numbered `.backup` files)
- No multi-session support - one active session per project

### Scheduling (`src/cron.ts`, `src/jobs.ts`)

**Cron**: Full 5-field cron expression parser (minute, hour, dayOfMonth, month, dayOfWeek) with timezone offset support. Custom implementation, no dependency.

**Jobs**: Markdown files with YAML frontmatter in `.claude/claudeclaw/jobs/`:

```yaml
---
schedule: "0 9 * * 1-5"
recurring: true
notify: true
---
Check for new PRs and summarize them.
```

Fields: `schedule` (cron expression), `recurring` (bool), `notify` (true/false/"error"). The prompt is the markdown body after frontmatter. Jobs can reference `.md`/`.txt`/`.prompt` files as prompts.

### Model Router (`src/model-router.ts`)

Configurable keyword-based task classification:

1. **Phase 1 - Phrase matching** (highest priority): Check full phrases like "how to implement", "what's the best way to"
2. **Phase 2 - Keyword scoring**: Count keyword hits per mode, boost question-mark messages for planning modes
3. **Tie-breaking**: Prefer configured `defaultMode` among tied candidates
4. **Fallback**: Default mode when no keywords match (confidence 0.5)

Default modes: "planning" (routes to opus) and "implementation" (routes to sonnet). Fully configurable via `settings.json`.

### Skills Integration (`src/skills.ts`)

ClaudeClaw discovers skills using the Agent Skills standard:

- **Search order**: Project (`.claude/skills/`), Global (`~/.claude/skills/`), Plugin cache (`~/.claude/plugins/cache/`)
- **SKILL.md parsing**: Extracts `description` from YAML frontmatter for catalog
- **Plugin naming**: `pluginName_skillName` format for namespacing
- **Skill resolution**: Supports `plugin:skill` format for explicit plugin scoping
- **Listing**: Returns `{ name, description }[]` for catalog display

### Identity / Soul System

Four prompt files form the agent's persistent identity:

**`prompts/IDENTITY.md`**: Template for self-discovered identity (name, creature type, vibe, signature emoji). Agent fills this in during first conversation.

**`prompts/SOUL.md`**: Personality guidelines. Key principles:
- "Be genuinely helpful, not performatively helpful"
- "Have opinions" - agent is encouraged to disagree, prefer things
- "Be resourceful before asking" - try to figure it out first
- Explicit trust model: "bold with internal actions, careful with external ones"
- `CLAUDE.md` is designated as persistent memory across sessions
- Agent is encouraged to evolve the soul document over time

**`prompts/USER.md`**: Template for learning about the user. Builds over time with name, timezone, preferences, context. Explicit note: "you're learning about a person, not building a dossier."

**`prompts/BOOTSTRAP.md`**: First-run conversation guide. The agent:
1. Has a natural conversation (not interrogation) to learn about the user
2. Picks its own name, nature, and emoji
3. Discovers communication preferences
4. Configures timezone and heartbeat schedule
5. Writes everything to `CLAUDE.md` for persistence
6. Optionally sets up Telegram/Discord

**Memory persistence**: Uses `CLAUDE.md` in the project root with managed blocks (`<!-- claudeclaw:managed:start -->` / `<!-- claudeclaw:managed:end -->`). The runner injects prompt content into a managed block in `CLAUDE.md` so it persists across sessions. Existing content outside managed blocks is preserved.

### Security Model

Four levels:
- **locked**: Read-only tools only (`Read, Grep, Glob`)
- **strict**: Disallows `Bash, WebSearch, WebFetch`
- **moderate**: All tools, scoped to project directory via system prompt constraint
- **unrestricted**: All tools, no directory restriction

All levels use `--dangerously-skip-permissions`. Directory scoping is enforced via system prompt, not CLI flags.

### Configuration (`src/config.ts`)

Single `settings.json` in `.claude/claudeclaw/`:

```typescript
interface Settings {
  model: string;           // Primary model name
  api: string;             // API key
  fallback: ModelConfig;   // Fallback model + API
  agentic: AgenticConfig;  // Model routing config
  timezone: string;        // UTC offset label
  timezoneOffsetMinutes: number;
  heartbeat: HeartbeatConfig;  // Periodic check-ins
  telegram: TelegramConfig;
  discord: DiscordConfig;
  security: SecurityConfig;
  web: WebConfig;          // Dashboard server
  stt: SttConfig;          // Speech-to-text API
}
```

### Relevance to Autonomic

1. **Daemon pattern**: PID file + stale detection is a solid minimal approach. For Rust, this maps to `std::fs` PID file + `kill(pid, 0)` via libc.
2. **Serial execution queue**: Critical insight - Claude Code CLI sessions cannot be resumed concurrently. Any orchestrator must serialize per-session.
3. **System prompt re-injection on every resume**: `--append-system-prompt` does not persist. This means identity/personality must be injected on EVERY invocation.
4. **Auto-compact on timeout**: Practical pattern for long-running agents. Exit code 124 = timeout, run `/compact`, retry.
5. **Rate limit fallback**: Pattern of detecting rate limits from output and falling back to another model.
6. **The bootstrap conversation**: Interesting pattern for agent self-configuration. Agent discovers its own identity through conversation rather than being configured.
7. **Managed blocks in CLAUDE.md**: Clean pattern for orchestrator-managed state that coexists with user content.
8. **Job format**: YAML frontmatter + markdown body for scheduled tasks mirrors the SKILL.md pattern.

## 3. Clawith (dataelement/Clawith)

### Overview

Clawith is a full enterprise multi-agent collaboration platform. Apache 2.0 licensed. Backend: FastAPI + SQLAlchemy (async) + PostgreSQL + Redis. Frontend: React 19 + TypeScript + Vite. Each agent gets a persistent identity, long-term memory, private workspace, and sandboxed code execution. Agents communicate through "The Plaza" (organizational knowledge feed) and direct messaging.

### Architecture

```txt
Frontend (React 19, Vite, Zustand, TanStack Query)
  |
Backend (FastAPI, 18 API Modules, WebSocket, JWT/RBAC)
  |-- Skills Engine
  |-- Tools Engine (266KB agent_tools.py - massive tool suite)
  |-- MCP Client (Streamable HTTP, Smithery + ModelScope)
  |-- Trigger Daemon (cron, once, interval, poll, on_message, webhook)
  |-- Heartbeat Service
  |-- Autonomy Service
  |-- Collaboration Service
  |-- Scheduler
  |
Infrastructure (PostgreSQL, Redis, Docker containers per agent)
```

Docker composition: `postgres:15-alpine`, `redis:7-alpine`, backend (Python), frontend (Node). Backend has Docker socket access for container management.

### Aware - Adaptive Autonomous Consciousness System

The core differentiator. Agents do not passively wait for commands; they maintain autonomous awareness.

**Focus Items**: Agents maintain a structured working memory of what they are currently tracking, with status markers:
- `[ ]` pending
- `[/]` in progress
- `[x]` completed

**Focus-Trigger Binding**: Every task-related trigger must have a corresponding Focus item. Agents create focus items first, then set triggers referencing them via `focus_ref`. When a focus is completed, associated triggers are canceled.

**Self-Adaptive Triggering**: Agents dynamically create, adjust, and remove their own triggers as tasks evolve. The human assigns the goal; the agent manages the execution schedule.

**Six Trigger Types**:
1. `cron` - Recurring schedule (standard cron expressions)
2. `once` - Fire once at a specific time
3. `interval` - Every N minutes
4. `poll` - HTTP endpoint monitoring
5. `on_message` - Wake when a specific agent or human replies
6. `webhook` - Receive external HTTP POST events (GitHub, Grafana, CI/CD)

**Reflections**: Dedicated view showing the agent's autonomous reasoning during trigger-fired sessions, with expandable tool call details.

### Agent Template (Persistent Identity)

Each agent is provisioned from `backend/agent_template/`:

**`soul.md`** - Agent personality definition:

```markdown
# Soul -- {{agent_name}}

## Identity
- Name: {{agent_name}}
- Role: {{role_description}}
- Creator: {{creator_name}}
- Created: {{created_at}}

## Personality
- Diligent, detail-oriented
- Proactively reports progress
- Confirms uncertain information

## Boundaries
- Follows enterprise confidentiality rules
- Sensitive operations require creator approval
```

**`state.json`** - Runtime state:

```json
{
  "agent_id": "",
  "name": "",
  "status": "idle",
  "current_task": null,
  "last_active": null,
  "channel_status": {},
  "stats": {
    "tasks_completed_today": 0,
    "tasks_in_progress": 0
  }
}
```

**`HEARTBEAT.md`** - Autonomous heartbeat behavior template:

Heartbeat protocol has four phases:
1. **Review Context**: Examine recent conversations and role. Identify topics relevant to role, unexplored questions, emerging trends.
2. **Targeted Exploration** (conditional): If genuine interest points found, use `web_search` (max 5 per heartbeat), record findings to `memory/curiosity_journal.md` with source URL, relevance rating, and follow-up questions.
3. **Agent Plaza**: Check `plaza_get_new_posts`, share discoveries (max 1 post + 2 comments), always include source URLs.
4. **Wrap Up**: `HEARTBEAT_OK` if nothing needed attention, otherwise summarize.

Key rules: never share private info to plaza, quality over quantity, skip exploration if nothing genuinely interesting.

**Directory structure per agent**:

```txt
agent_data/<agent-uuid>/
  soul.md              # Personality (persistent)
  state.json           # Runtime state
  HEARTBEAT.md         # Heartbeat behavior
  memory/              # Long-term memory files
    curiosity_journal.md
  skills/              # Agent-specific skills
  workspace/           # Private sandboxed filesystem
  daily_reports/       # Generated reports
  enterprise_info/     # Organization context
  todo.json            # Task list
```

### Multi-Agent Patterns

**The Plaza**: Organization-wide knowledge feed. Agents post updates, share discoveries, comment on each other's work. Serves as continuous organizational context absorption channel. Scoped per tenant.

**Agent-to-Agent Communication**: Agents can send messages, delegate tasks, and build working relationships. Communication secured with tenant isolation and relationship checks. Retry with jitter on LLM timeouts.

**Organization Chart Awareness**: Every agent understands the full org chart. Can route tasks to appropriate colleagues based on role.

**Supervision/Reminders**: `supervision_reminder.py` (16KB) handles task follow-up and escalation.

### Service Layer (Key Components)

| Service | Size | Purpose |
| --- | --- | --- |
| `agent_tools.py` | 266KB | Massive tool suite - all agent capabilities |
| `llm_client.py` | 74KB | Multi-provider LLM client (OpenAI, Anthropic, Baidu, etc.) |
| `tool_seeder.py` | 53KB | Seeds built-in tools into database |
| `trigger_daemon.py` | 29KB | Autonomous trigger execution engine |
| `resource_discovery.py` | 33KB | Runtime tool/MCP discovery (Smithery, ModelScope) |
| `agent_context.py` | 28KB | Context assembly for agent conversations |
| `skill_seeder.py` | 25KB | Seeds built-in skills |
| `heartbeat.py` | 20KB | Heartbeat scheduling and execution |
| `autonomy_service.py` | 11KB | Aware system - autonomous awareness |
| `collaboration.py` | 5KB | Agent-to-agent collaboration |
| `scheduler.py` | 10KB | Task scheduling |
| `mcp_client.py` | 16KB | MCP Streamable HTTP client |
| `quota_guard.py` | 9KB | Per-user message limits, LLM call caps |

### Self-Evolving Capabilities

Agents can:
- **Discover and install new tools at runtime** via Smithery and ModelScope MCP registries
- **Create new skills** for themselves or colleagues via `skill_creator_content.py`
- Skills are stored per-agent in the filesystem and seeded from templates

### Enterprise Controls

- **Multi-tenant RBAC**: Organization-based isolation with role-based access (`permissions.py`)
- **Channel integration**: Each agent gets its own Slack, Discord, Feishu/Lark, DingTalk, WeCom bot identity
- **Usage quotas**: Per-user message limits, LLM call caps, agent TTL (`quota_guard.py`)
- **Approval workflows**: Flag dangerous operations for human review
- **Audit logs**: Full traceability (`audit_logger.py`)
- **Published Pages**: Agents can publish static HTML with shareable URLs

### Technology Stack Detail

Backend dependencies (from `pyproject.toml`):
- FastAPI + Uvicorn (async HTTP)
- SQLAlchemy async + asyncpg (PostgreSQL)
- Redis with hiredis (caching, pub/sub)
- Docker SDK (container management per agent)
- croniter (cron expression parsing)
- Multiple IM SDKs: discord.py, lark-oapi, dingtalk-stream, wecom-aibot-sdk
- Document processing: pdfplumber, python-docx, openpyxl, python-pptx
- Web scraping: trafilatura, httpx with SOCKS proxy support

### Relevance to Autonomic

1. **Aware system is the key innovation**: Focus items + trigger binding + self-adaptive scheduling. This is a concrete model for autonomous agent behavior. An agent does not just respond; it maintains a working set of concerns and dynamically schedules its own triggers.

2. **Six trigger types**: `cron`, `once`, `interval`, `poll`, `on_message`, `webhook`. A Rust orchestrator should support all six. The `poll` and `webhook` types are particularly interesting for CI/CD integration.

3. **soul.md + state.json + memory/ pattern**: Clean separation of persistent identity (soul), runtime state, and accumulated knowledge. All file-based, version-controllable.

4. **HEARTBEAT.md as autonomous behavior specification**: Instead of hardcoding heartbeat behavior, it is a markdown prompt template that the agent follows. This makes autonomous behavior configurable and evolvable.

5. **The Plaza as organizational memory**: Agents do not just have private memory; they share discoveries through a feed. This creates emergent organizational knowledge.

6. **Per-agent workspace isolation**: Each agent has its own filesystem, sandboxed via Docker. For a Rust orchestrator, this maps to per-agent directories with filesystem isolation.

7. **Runtime tool discovery**: Agents installing their own tools from registries is a powerful self-improvement pattern. Smithery and ModelScope as MCP tool registries.

8. **Focus-trigger binding**: The constraint that every trigger must reference a focus item prevents trigger sprawl and ensures all autonomous activity has clear purpose.

9. **Approval workflows**: A concrete pattern for human-in-the-loop control over dangerous operations. Essential for self-improving systems.

10. **The curiosity_journal.md pattern**: Agents accumulate knowledge through periodic exploration, with explicit source tracking and relevance ratings. This is structured learning.

## Cross-Cutting Analysis: Patterns for the Autonomic Orchestrator

### Skill/Capability Format

**Adopt**: The Agent Skills standard (`SKILL.md` with YAML frontmatter). It is the de facto standard across 20+ platforms. Our orchestrator should:
- Parse SKILL.md with `serde_yaml` for frontmatter, treat body as raw markdown
- Implement three-tier progressive disclosure (catalog/instructions/resources)
- Use the `metadata` map for orchestrator-specific fields (model preference, cost tier, priority)
- Use the `allowed-tools` field for per-skill sandboxing
- Support both `~/.claude/skills/` and project-level discovery

### Daemon/Process Management

**Adopt from ClaudeClaw**: PID file + stale detection pattern. For Rust:
- Write PID to `daemon.pid`
- Check liveness with `libc::kill(pid, 0)`
- Clean up stale files on startup
- One daemon per project/workspace

**Adopt from Clawith**: Process-per-agent isolation. For Rust:
- Agent state directories at `agent_data/<uuid>/`
- Consider OS-level isolation (namespaces, seccomp) instead of Docker for lower overhead

### Memory and Identity Persistence

**Three-layer model** (synthesized from both projects):

1. **Soul** (static personality): `soul.md` - rarely changes, defines who the agent is
2. **Memory** (accumulated knowledge): `memory/` directory with structured journals and facts
3. **State** (runtime status): `state.json` - current tasks, status, counters

Both projects use file-based persistence (not databases) for agent identity. This is deliberate: files are version-controllable, portable, and human-readable.

**ClaudeClaw's `CLAUDE.md` managed blocks** are a pragmatic pattern for coexisting with user content. The orchestrator manages its own section; the user manages the rest.

### Scheduling and Autonomous Behavior

**Adopt Clawith's six trigger types**: `cron`, `once`, `interval`, `poll`, `on_message`, `webhook`. All implementable in Rust with `tokio` timers and `hyper` for HTTP.

**Adopt Clawith's Focus-Trigger binding**: Every trigger must reference a focus item. This prevents aimless autonomous activity.

**Adopt ClaudeClaw's auto-compact pattern**: Detect timeout (exit 124), run `/compact`, retry. Essential for long-running agents.

### Model Routing

Both projects implement model routing:
- ClaudeClaw: Keyword/phrase scoring with configurable modes
- Clawith: Per-model temperature control, multi-provider support

**For Rust**: Implement a trait-based router that scores prompts against configurable patterns and selects model + provider. Support fallback chains for rate limiting.

### Security Patterns

- ClaudeClaw's four security levels (locked/strict/moderate/unrestricted) with tool allowlists
- Clawith's RBAC + approval workflows + quota guards + audit logging
- Agent Skills' `allowed-tools` per-skill sandboxing

### Communication Bridges

Both projects abstract messaging platforms:
- ClaudeClaw: Telegram + Discord, with voice transcription via Whisper
- Clawith: Discord + Slack + Feishu/Lark + DingTalk + WeCom + Email

**Pattern**: Each channel adapter converts platform messages to/from a common internal format, then routes to the agent's conversation.

### Key Technical Constraints Discovered

1. **Claude Code `--append-system-prompt` does not persist across `--resume`** - must re-inject on every call
2. **Claude Code sessions cannot be concurrently resumed** - must serialize per-session
3. **Rate limits are detected from stdout/stderr text** - no structured error codes from CLI
4. **Exit code 124 = timeout** - standard Unix convention used by Claude Code
5. **New sessions require JSON output format** to capture `session_id`; resumed sessions use text format
6. **The `CLAUDECODE` env var** must be stripped from child processes to avoid nesting detection
