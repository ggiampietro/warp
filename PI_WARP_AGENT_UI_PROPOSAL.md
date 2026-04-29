# Proposal: Use Warp Agent UI with pi as the backend

## Goal

Explore whether Warp’s existing Agent / Ambient Agent UI can be reused as the frontend while **pi** remains the real backend runtime.

Desired outcome:
- keep Warp’s agent-oriented UI and in-terminal UX
- use pi for actual agent execution, tool calls, file edits, and orchestration
- avoid depending on Warp cloud agent servers for the core agent backend

In short:

> **Warp UI as the frontend, pi as the execution backend.**

---

## Why this is attractive

Warp already provides useful UI pieces for agent-style workflows:
- agent/ambient-agent panes
- streaming/progress-oriented UI
- terminal-integrated interaction model
- result rendering inside the Warp workspace
- MCP server management UI
- agent-related settings surfaces

pi already provides useful backend/runtime pieces:
- agent orchestration
- coding actions and tool usage
- filesystem access
- shell command execution
- multi-step reasoning and code-edit workflows
- subagent / delegation concepts

So the idea is appealing because it combines:
- **Warp’s UX**
- with **pi’s backend/runtime**

---

## Current reality in Warp

From the codebase, Warp does **not** currently appear to expose a clean, generic “pluggable agent backend” interface for the full Warp Agent UI.

What Warp *does* support today:

### 1. MCP servers
Warp supports MCP servers as extensions of the Warp Agent’s capabilities.

This is a **tool/plugin model**, not a full agent-backend swap.

Meaning:
- MCP can add tools and data sources
- MCP does **not** replace the agent runtime itself

### 2. Local / third-party harness support in some flows
Warp has code for local harness launches and some third-party harness integration concepts.

This suggests Warp can interoperate with some external agent-like processes in specific workflows.

However, this is still not the same as:
- a fully generic “bring your own agent backend” architecture for the entire Warp Agent UI

### 3. Strong coupling to Warp’s agent backend contract
The current agent UI appears tightly coupled to Warp-native task/run APIs and models, including concepts like:
- `spawn_agent(...)`
- `create_agent_task(...)`
- `list_ambient_agent_tasks(...)`
- `get_ambient_agent_task(...)`
- `agent/runs/...`
- event sequences
- artifacts
- conversation restore/history
- cloud-mode state and task persistence

This means the current Warp Agent UI expects a fairly specific backend contract.

---

## Proposal

The most realistic proposal is:

> Build a compatibility layer that lets Warp Agent UI talk to pi as if pi were a Warp-style agent backend.

This is **not** just a model-provider swap.
It is closer to a backend adapter or translation layer.

---

## Main implementation approaches

# Option A — Compatibility adapter service (recommended)

## Idea
Run a local adapter service between Warp and pi.

Flow:
- Warp Agent UI talks to a Warp-like local backend API
- adapter translates Warp-style requests into pi runs
- adapter translates pi progress/results back into Warp-style runs/events/artifacts

## Architecture

### Layer 1: Warp frontend
- existing agent UI
- ambient agent pane
- task/run views
- progress/details views

### Layer 2: local compatibility adapter
Responsibilities:
- accept Warp-style agent run/task requests
- start/manage pi sessions
- map pi state to Warp run/task state
- stream progress/events
- persist minimal run metadata for restore/history
- expose artifacts/files in Warp-compatible shape

### Layer 3: pi backend
- actual execution runtime
- tools
- shell commands
- file edits
- subagents / orchestration

## Pros
- best chance of preserving Warp UI with minimal frontend changes
- keeps pi as the true backend
- architecture is explicit and maintainable
- can start with a minimal subset and grow over time

## Cons
- requires implementing a non-trivial adapter
- Warp backend contract is fairly rich
- restore/history/artifact parity adds complexity

## Best use case
Choose this if the real goal is:
- **keep Warp’s agent UI mostly intact**
- **replace the backend with pi**

---

# Option B — Add a first-class pluggable agent-backend abstraction inside Warp

## Idea
Refactor Warp so the frontend depends on a generic `AgentBackend` trait/interface instead of directly depending on Warp-native backend semantics.

Then implement:
- `WarpCloudAgentBackend`
- `PiAgentBackend`

## Pros
- cleanest long-term architecture
- best developer ergonomics after initial investment
- enables future backends beyond pi

## Cons
- largest frontend/backend refactor
- requires touching many Warp internals
- more invasive than building an adapter
- likely slower to first working result

## Best use case
Choose this if the goal is not just pi, but a broader Warp architecture change such as:
- true backend pluggability
- multiple agent backends
- long-term upstreamable design

---

# Option C — Use MCP as the integration mechanism

## Idea
Try to expose pi through MCP and let Warp Agent call into it as a tool provider.

## What this gives you
- Warp can invoke pi-backed tools
- pi can expose capabilities/data/tooling to the Warp Agent

## What it does *not* give you
- pi does not become the main agent runtime
- Warp Agent still remains the orchestrator/agent brain
- Warp UI still fundamentally belongs to Warp’s own agent backend model

## Pros
- lowest integration friction if tool augmentation is enough
- uses supported Warp extension path
- more aligned with existing Warp extensibility story

## Cons
- does **not** satisfy “pi is the backend agent”
- pi becomes a tool provider, not the real agent runtime

## Best use case
Choose this if your goal is actually:
- “let Warp Agent use pi capabilities”
not:
- “replace Warp Agent backend with pi”

---

# Option D — Reuse only selected Warp UI surfaces, not full Warp Agent UI

## Idea
Instead of integrating with the full Warp Agent system, reuse smaller UI surfaces:
- command search
- AI input / prompts
- terminal overlays
- panes/modals
- lightweight local assistant surfaces

Then route those directly to pi.

## Pros
- much lower complexity than full agent-backend replacement
- avoids task/run/artifact/restore complexity
- can deliver useful results faster

## Cons
- you do not get the full current Warp Agent UI
- user experience will be more partial / custom

## Best use case
Choose this if the real near-term goal is:
- “get pi nicely integrated into Warp UX quickly”
without needing complete ambient-agent parity.

---

## Recommendation

## Recommended path: Option A

The best balance of feasibility and outcome is:

> **Build a local compatibility adapter that makes pi look like a Warp-style agent backend.**

Why this is the best option:
- preserves Warp Agent UI as much as possible
- avoids a deep frontend refactor up front
- keeps pi as the real runtime
- can be built incrementally
- does not depend on Warp already having true backend pluggability

## Secondary recommendation
If Option A proves too heavy, the fallback is:

> **Option D: integrate pi into selected Warp AI surfaces rather than the full current Warp Agent UI.**

That yields value much faster and avoids the deepest backend coupling.

---

## Proposed MVP scope for Option A

Build only the minimum adapter needed for a convincing demo/prototype.

### MVP should support
- start a pi-backed run from Warp Agent UI
- stream text/progress updates
- show terminal-oriented intermediate progress
- finish with success/failure state
- optionally expose edited files as simple artifacts

### MVP should defer
- full restore/history parity
- advanced artifact types
- scheduled agents
- cloud conversation syncing
- all ambient-agent edge cases
- perfect telemetry parity

---

## State mapping needed

A compatibility adapter would need to map pi backend state to Warp-style states.

Example mapping:

### Warp-style states
- queued
- pending
- in progress
- succeeded
- failed
- blocked
- cancelled

### pi-side equivalents
- run accepted / waiting
- tool call running
- planning
- applying edits
- final success
- model/tool/backend error
- permission or policy blocked
- user cancel

This mapping does not need to be perfect at first, but it must be consistent enough for the UI.

---

## Event mapping needed

Warp UI expects progress-style events.
An adapter should translate pi activity into a stream like:
- run created
- planning started
- message emitted
- tool call started
- tool call completed
- file changed
- command executed
- run completed
- run failed

Even if pi’s raw events differ, a normalized stream is possible.

---

## Artifact mapping needed

Potential initial artifact mapping:
- plan text → plan artifact
- changed files → file artifacts
- final summary → run output / transcript artifact

Optional later:
- screenshots
- PR artifacts
- structured execution logs

---

## Open questions

Before implementation, answer these:

1. **How much of Warp Agent UI must be preserved?**
   - full parity?
   - just enough for a useful agent pane?

2. **Does pi expose enough structured streaming state already?**
   - or would the adapter need to infer a lot?

3. **Should adapter persistence be temporary or durable?**
   - memory only for prototype?
   - sqlite/json for resume/history?

4. **How much artifact support is required in MVP?**
   - just text + files?
   - or full Warp artifact model?

5. **Do we want cloud-mode UI language renamed or reused as-is?**
   - today Warp uses cloud/ambient-agent concepts in parts of the UI

---

## Suggested phased plan

### Phase 1 — Feasibility prototype
- confirm one Warp entrypoint can launch a pi-backed run
- create adapter endpoint or local shim
- show streaming status in a simplified agent UI flow

### Phase 2 — Minimal usable backend adapter
- implement create/run/status/event APIs
- support success/failure lifecycle
- support basic text artifacts and changed files

### Phase 3 — UI integration hardening
- support restore/reopen
- smooth state transitions
- better status/error rendering
- better artifact display

### Phase 4 — Advanced parity work
- richer events
- conversation metadata/history
- resume/replay
- broader ambient-agent features

---

## Decision guide

Choose **Option A** if:
- you want Warp Agent UI with pi backend
- you accept building an adapter layer

Choose **Option B** if:
- you want a long-term architectural cleanup inside Warp
- you are willing to refactor deeply

Choose **Option C** if:
- tool/plugin augmentation is enough
- pi does not need to be the main backend

Choose **Option D** if:
- you want fast practical integration
- full agent UI parity is not required

---

## Final recommendation

If the product goal is:

> “Warp Agent UI should be the interface, but pi should be the real backend”

then the clearest proposal is:

> **Build a local Warp-agent compatibility adapter for pi (Option A).**

That is the most realistic route to preserving Warp’s current UI while making pi the actual execution engine.
