# recall-reflect PRD

## Goal

Define the official Reflect extension so Recall can turn local AI coding history
into a clean timeline of human intent, agent response, corrections, and
follow-up work.

Reflect is designed as the official `recall-reflect` extension. Recall core provides session data through the stable CLI JSON/JSONL protocol; the extension owns timeline reconstruction, observed pattern prompts, and future discussion/calibration workflow.

The primary workflow is:

1. A user or coding agent selects a repository root or repository identity.
2. Recall core finds relevant local sessions for that project.
3. `recall-reflect` reconstructs a conversation-first timeline across those sessions.
4. `recall-reflect` surfaces observed workflow patterns as discussion prompts.
5. The user decides whether any pattern should become a workflow, skill, agent,
   or instruction-file change.
6. If the user approves, the extension can prepare an explicit proposal or patch for a
   later apply step.

Reflect is not only a report. The complete product direction is an extension-led reflection and
calibration loop: inspect the timeline, discuss what the timeline means, propose
changes only after user confirmation, and later compare new sessions against
accepted changes.

## Problem

Recall already supports local session indexing, search, export, sharing, usage
reporting, and session-level actions. Those workflows are session-first: they
help users find, inspect, and reuse individual conversations.

AI coding work often does not stay inside one session. A project may move from
Claude Code to Codex, from Codex to OpenCode, or from a long exploration session
to several focused follow-up sessions. Users also copy agent output into new
prompts, restart with a cleaner context, retry failed approaches, or gradually
turn repeated friction into a personal workflow.

That makes reflection different from search. The useful unit is the project
timeline, not the individual session. A session-first summary can miss the
larger pattern: how work moved, where misunderstandings repeated, when the user
changed direction, and which behaviors might be worth preserving or changing.

## Users

- Power users who want to understand and improve their AI-assisted coding
  workflow from local history.
- Coding agents that need a clean project timeline before proposing workflow or
  skill changes.
- Maintainers who want Recall's reflection behavior to stay local-first,
  explicit, and reviewable.

## Definitions

- **Reflect**: the official `recall-reflect` extension, a project-level workflow
  that reconstructs a timeline, surfaces patterns for discussion, and can later
  produce user-approved change proposals.
- **Project timeline**: a chronological narrative built from multiple sessions
  that belong to the same repository or project scope.
- **Conversation-first timeline**: a timeline focused on user inputs, agent
  responses, decisions, corrections, and outcomes. Low-level tool calls and raw
  execution logs are supporting data, not the default narrative.
- **Observed pattern**: a repeated or notable behavior found in the timeline,
  such as a handoff between tools, repeated correction, retry loop, or recurring
  workflow step.
- **Discussion prompt**: a question that asks the user to confirm, reject, or
  refine an observed pattern before Recall treats it as a calibration target.
- **Calibration target**: a user-confirmed pattern that may justify changing a
  workflow, skill, agent instruction, project process, or instruction file.
- **Proposal**: an optional, reviewable suggestion generated from a calibration
  target. A proposal is not applied unless the user explicitly approves it.

## Scope

### In Scope

- Add an official reflect extension workflow scoped by repository root or
  repository identity.
- Reconstruct a timeline across relevant local sessions.
- Keep the default narrative conversation-first: user messages, assistant
  responses, titles, summaries, timestamps, and source/project metadata.
- Hide or summarize raw tool calls, file reads/writes, command output, and
  internal events by default.
- Surface observed workflow patterns as discussion prompts instead of final
  judgments.
- Support optional workflow, skill, agent, project-process, or instruction-file
  proposals after user confirmation.
- Keep proposal application explicit and reviewable.
- Provide machine-readable output for agents in addition to readable terminal
  output.

### Out of Scope

- Treating reflection as a personality profile or scorecard.
- Automatically editing skills, prompts, project files, or instruction files
  without user approval.
- Uploading reflection data or sharing it remotely by default.
- Replacing `recall search`, `recall session`, `recall export`, or the TUI.
- Making low-level tool/event logs the primary report format.

## Product Model

Reflect has five product layers. They describe the complete extension capability; an
implementation can deliver them incrementally.

### 1. Timeline Reconstruction

The user-facing input should stay simple. The common command should be Recall's
extension dispatch, based on the current repository or an explicit repository
root:

```bash
recall reflect --project /path/to/repo
recall reflect --repo owner/repo
```

Direct extension invocation is acceptable for testing and development:

```bash
recall-reflect --project /path/to/repo
recall-reflect --repo owner/repo
```

Internally, the extension consumes indexed session metadata, messages,
timestamps, source information, summaries, usage records, and event records
through Recall's stable CLI protocol. These inputs should not make the
user-facing command feel complex.

The output of this layer is a chronological project narrative. Sessions are
evidence sources, but the top-level structure is time.

Large project histories must not be handled by sending one full timeline to an
agent or model. Reflect should build a multi-resolution timeline: compact session
or time-window summaries first, then phase summaries, then a project-level
reflection. The user can drill back into the supporting sessions when needed, but
the default report should stay within a readable and reviewable size.

### 2. Conversation Reflection

The extension then looks for workflow patterns in the clean conversation timeline:

- work continuing across sessions or tools;
- agent output becoming the next prompt or direction;
- repeated corrections or redirections;
- repeated planning, debugging, review, or verification loops;
- manual multi-step workflows that recur across sessions;
- places where the user and agent appear to disagree about scope or timing.

These patterns should be phrased as observations and questions, not verdicts.
For example:

```text
This timeline shows several points where implementation expanded after the user
had narrowed the scope. Is that a real workflow problem, or are these unrelated
moments?
```

### 3. User-Guided Calibration

The extension should not convert every observation into a rule. The user confirms which
patterns matter.

The calibration step turns an observed pattern into a target only after user
input:

```text
Observed pattern: scope expanded during implementation in several timeline
moments.

What should Recall do with this?
1. Ignore it.
2. Keep it as a note for future reflection.
3. Draft a workflow change.
4. Draft an agent/skill/instruction change.
```

This keeps Reflect discussion-first. The extension helps surface patterns; the user
decides whether they are real, useful, or actionable.

### 4. Proposal Generation

After confirmation, the extension can generate reviewable proposals. Proposal types may
include:

- **Workflow proposal**: a new repeatable process or checkpoint.
- **Skill proposal**: a draft skill or a suggested change to an existing skill.
- **Agent behavior proposal**: a rule or behavior for an agent to follow in a
  specific situation.
- **Instruction-file proposal**: a suggested change for files such as
  `AGENTS.md` or `CLAUDE.md`.
- **Project-process proposal**: a project-level practice such as a checklist,
  handoff note, test gate, or review habit.

Each proposal should explain:

- what would change;
- why this pattern led to the proposal;
- which timeline moments are relevant;
- what the user needs to approve before anything is written.

### 5. Apply, Track, And Re-Reflect

The complete loop does not end at proposal generation. If a user accepts a
proposal, the extension should be able to track that calibration and revisit it later.

Future reflection can then answer:

- What workflow or instruction change was accepted?
- Which sessions happened after the change?
- Did the same pattern disappear, continue, or change shape?
- Did the accepted change create a new kind of friction?

This turns Reflect from a one-time summary into a long-term local feedback loop
for AI-assisted development.

## Command Design

### `recall reflect`

Reflect on the current or selected project timeline through Recall's extension dispatch.

```bash
recall reflect
recall reflect --project /path/to/repo
recall reflect --repo owner/repo
recall reflect --time 30d
recall reflect --format json
```

Options:

- `--project <path>`: project directory boundary, including child paths.
- `--repo <identity>`: repository identity such as `owner/repo` or a remote URL.
- `--time <today|7d|week|30d|month|all>`: time window, default `all` or a
  product-chosen recent default.
- `--source <source>`: optional source filter for focused inspection.
- `--format <text|json>`: default `text`.
- `--sync`: optionally run incremental sync before reflection.
- `--include-events`: include summarized low-level events as supporting context.

When no project or repo is provided, Recall should prefer the current repository
root when it can be resolved. If no repository can be resolved, the command
should ask the calling agent or user to choose a project scope rather than
reflecting over all history by accident.

### `recall reflect propose`

Draft a proposal from a selected or confirmed calibration target.

```bash
recall reflect propose --id <target-id>
recall reflect propose --id <target-id> --kind workflow
recall reflect propose --id <target-id> --kind instruction --target AGENTS.md
```

This command prepares a proposal. It does not apply it.

### `recall reflect apply`

Apply a proposal after explicit user approval.

```bash
recall reflect apply --proposal <proposal-id> --dry-run
recall reflect apply --proposal <proposal-id>
```

`--dry-run` should show the exact change. Applying should be a separate,
deliberate action.

## Output Design

Readable output should prefer short sections:

```text
Recall reflect for <project>:

1. Timeline
2. Observed workflow patterns
3. Discussion prompts
4. Calibration targets confirmed by the user
5. Optional proposals
6. Follow-up checks from previous calibrations
```

The main timeline should not read like a tool log. It should read like a concise
history of human intent and agent response.

For long histories, output should be layered rather than exhaustive:

1. a short project-level summary;
2. a small number of timeline phases;
3. representative moments under each phase;
4. optional drill-down commands or ids for deeper inspection.

JSON output should preserve enough structure for agents to continue the
conversation:

```json
{
  "scope": {
    "project": "/path/to/repo",
    "repo": "owner/repo",
    "time": "30d"
  },
  "timeline": [
    {
      "id": "moment-1",
      "timestamp": 1781234567890,
      "source": "codex",
      "session_id": "...",
      "kind": "conversation",
      "summary": "User narrowed scope; agent proposed a broader implementation."
    }
  ],
  "observed_patterns": [
    {
      "id": "pattern-1",
      "summary": "Scope expansion appeared after narrow requests.",
      "timeline_moments": ["moment-1", "moment-4"],
      "discussion_prompt": "Is this a workflow issue worth calibrating?"
    }
  ],
  "proposals": []
}
```

## Agent-Friendly Workflows

### Reflect Before Planning

```bash
recall reflect --project /path/to/repo --time 30d --format json
# Agent summarizes the timeline and asks which pattern, if any, the user wants to calibrate.
```

### Draft A Workflow Proposal

```bash
recall reflect --project /path/to/repo
# User confirms a pattern.
recall reflect propose --id <target-id> --kind workflow
```

### Preview An Instruction Change

```bash
recall reflect propose --id <target-id> --kind instruction --target AGENTS.md
recall reflect apply --proposal <proposal-id> --dry-run
# User reviews the exact diff before applying.
```

## Privacy And Safety

- Reflection data comes from local session history.
- Reports should summarize transcript content. They should not paste full
  transcripts unless explicitly requested.
- Low-level tool calls, file paths, command output, and internal events should
  be hidden or summarized by default.
- Reflect should not automatically modify source files, project configuration,
  skills, prompts, or instruction files.
- Proposal application must require explicit user approval.
- Shared or exported reflection output may contain private intent and project
  context; sharing belongs behind an explicit user action.

## Backward Compatibility

- Existing commands keep working: `recall search`, `recall session`,
  `recall export`, `recall import`, `recall usage`, `recall share init`, and
  the TUI remain unchanged.
- Existing session export schemas remain useful as internal data sources for
  reflection.
- Reflect can be added without changing existing session-level workflows.

## Relationship To Existing Skill Workflows

Some agent skill systems already encode behavior learned from repeated AI coding
friction. The extension should treat those systems as possible calibration targets,
not as required dependencies.

For example, a user might use reflection to decide that a recurring pattern
should become a new workflow, a skill change, or an instruction-file rule. The extension
should help prepare that proposal, but the user remains responsible for deciding
whether to adopt it.

## Open Questions

- Should `recall reflect` default to a recent time window or all indexed history?
- Should calibration targets be persisted in Recall's local database, written as
  files, or only emitted in reports at first?
- Should proposal application support only instruction files first, or also
  skills and project workflow documents?
- How should Reflect show optional low-level event context without turning the
  main report into a tool log?
- Should future TUI support show Reflect as a new dashboard, a usage tab, or a
  session-adjacent view?
