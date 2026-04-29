# Warp + pi Adapter Architecture Proposal

## Goal

Define a concrete architecture for using **Warp’s agent-oriented UI** with **pi as the backend runtime**, inspired by the provider-adapter pattern used in `t3code`.

This document translates the general lessons from `t3code` into a Warp-specific proposal.

---

## Problem statement

Warp’s current agent UI appears to be tightly coupled to Warp-native backend concepts such as:
- task creation
- run status polling
- agent run event streams
- artifacts
- conversation restore/history
- ambient/cloud-mode specific task APIs

That makes it hard to simply “swap in pi” as the backend.

At the same time, `t3code` shows a clean approach where:
- the UI is backend-agnostic
- provider-specific logic lives behind adapters
- runtime events are normalized into a canonical event model
- orchestration/domain state is projected from those runtime events

The proposal here is to move Warp in that direction, either incrementally or via an adapter layer.

---

## Design objective

We want:

> **Warp UI should not depend directly on one backend implementation.**

Instead, Warp should talk to a generic backend contract, while concrete backends implement that contract.

For this project, the two important backends would be:
- existing Warp-native backend
- pi backend

---

## Reference pattern from t3code

`t3code` uses these layers:

### 1. Provider adapter contract
Each backend/provider implements a common interface.

### 2. Adapter registry
A registry resolves which adapter handles a given provider/backend.

### 3. Provider service
A generic orchestration layer routes requests and maintains shared lifecycle rules.

### 4. Canonical runtime event stream
Adapters emit normalized runtime events rather than raw backend-specific data.

### 5. Runtime-to-domain projection
A projection layer converts runtime events into the domain/UI model.

This is the key pattern to copy.

---

## Proposed Warp architecture

# Layer 1 — Warp frontend/UI

This includes:
- Warp Agent / Ambient Agent pane UI
- input UI
- progress rendering
- status banners
- details panels
- artifact rendering
- run history / restore surfaces

This layer should stop depending directly on Warp-native backend types where possible.

Instead, it should consume:
- canonical run/session state
- canonical event stream
- canonical artifact model

---

# Layer 2 — Agent backend service

Introduce a generic service, conceptually similar to `t3code`’s `ProviderService`.

Suggested name:
- `AgentBackendService`

Responsibilities:
- create or recover backend sessions/runs
- route commands to the selected backend
- manage lifecycle state
- expose a unified runtime event stream
- maintain thread/run → backend binding

This service should not implement backend-specific protocols itself.

---

# Layer 3 — Backend adapter contract

Introduce a trait/interface analogous to `ProviderAdapter` in `t3code`.

Suggested conceptual interface:

```rust
trait AgentBackendAdapter {
    fn backend_kind(&self) -> AgentBackendKind;

    async fn start_session(&self, input: StartSessionInput) -> Result<BackendSession>;
    async fn send_turn(&self, input: SendTurnInput) -> Result<TurnStartResult>;
    async fn interrupt_turn(&self, session_id: SessionId, turn_id: Option<TurnId>) -> Result<()>;
    async fn respond_to_request(&self, session_id: SessionId, request_id: RequestId, decision: BackendDecision) -> Result<()>;
    async fn stop_session(&self, session_id: SessionId) -> Result<()>;
    async fn list_sessions(&self) -> Result<Vec<BackendSession>>;
    async fn read_thread(&self, session_id: SessionId) -> Result<BackendThreadSnapshot>;
    async fn rollback_thread(&self, session_id: SessionId, turns: usize) -> Result<BackendThreadSnapshot>;

    fn stream_events(&self) -> Stream<BackendRuntimeEvent>;
}
```

The exact surface should be minimized for Warp’s needs, but the key idea is:
- Warp code routes through this abstraction
- each backend is implemented independently

---

# Layer 4 — Adapter registry

Introduce a registry to resolve adapters by backend kind.

Suggested backend kinds:
- `warp_cloud`
- `pi`

Possible future kinds:
- `local_harness`
- `third_party`

Suggested service:
- `AgentBackendRegistry`

Responsibilities:
- return adapter by backend kind
- list available backends
- possibly expose backend capabilities

---

# Layer 5 — Session/run directory

Analogous to `ProviderSessionDirectory` in `t3code`.

Warp needs a persistent mapping from UI thread/run identity to backend session identity.

Suggested structure:
- UI run/thread id
- backend kind
- backend session id / resume cursor
- status
- runtime metadata
- model selection
- cwd / environment metadata

Suggested service:
- `AgentSessionDirectory`

This is important for:
- restoring sessions
- routing follow-up turns
- reconnecting after restart
- switching between backend implementations predictably

---

# Layer 6 — Canonical runtime event model

Introduce a backend-agnostic event schema.

This is one of the strongest lessons from `t3code`.

Instead of making Warp UI understand:
- Warp cloud event shape
- pi event shape
- local harness event shape

all backends should emit a normalized event stream.

Example canonical events:
- `session.started`
- `session.state.changed`
- `thread.started`
- `thread.state.changed`
- `turn.started`
- `turn.completed`
- `message.delta`
- `message.completed`
- `tool.started`
- `tool.completed`
- `approval.requested`
- `approval.resolved`
- `user_input.requested`
- `user_input.resolved`
- `artifact.created`
- `runtime.warning`
- `runtime.error`

This event model does not need to perfectly mirror any one backend.
It only needs to be stable and expressive enough for Warp UI.

---

# Layer 7 — Runtime-to-Warp projection layer

Warp UI probably should not consume raw runtime events directly.

Instead, add a projection layer that converts canonical runtime events into Warp-friendly state/models.

Conceptually similar to `t3code`’s runtime ingestion / projection layer.

Responsibilities:
- maintain UI state from canonical events
- derive progress/status banners
- derive message transcript state
- derive artifact panel state
- derive task/run list state

Suggested name:
- `AgentRuntimeProjection`

This layer can maintain compatibility with existing Warp UI models while freeing the backend implementation.

---

## Concrete adapter types

# Adapter A — Warp native backend adapter

This adapter wraps the current Warp backend behavior.

Responsibilities:
- call existing Warp-native APIs
- translate Warp-native agent/task events into canonical runtime events
- preserve current functionality

This adapter lets the system continue working during migration.

---

# Adapter B — pi backend adapter

This adapter turns pi into a canonical Warp backend.

Responsibilities:
- start and manage pi sessions
- send prompts/turns to pi
- map pi progress to canonical events
- map pi tool calls and code edits to canonical events
- expose simple artifacts such as:
  - edited files
  - plans
  - summaries

This is the adapter that makes Warp UI usable with pi.

---

## Suggested MVP for pi adapter

To reduce scope, the first pi adapter should support only the minimum meaningful subset.

### MVP features
- start a pi-backed session
- send a prompt/turn
- stream text/progress
- complete successfully or fail
- optionally emit file artifacts

### MVP non-goals
- complete parity with Warp cloud agent backend
- full restore/resume fidelity
- advanced artifact types
- scheduled runs
- full conversation metadata parity
- all ambient/cloud-mode edge cases

---

## Mapping pi concepts to canonical backend concepts

### Session mapping
pi concept:
- session / run

canonical concept:
- backend session

### Turn mapping
pi concept:
- prompt submission / turn execution

canonical concept:
- thread turn

### Progress mapping
pi concept:
- model response chunks
- tool execution events
- file edits
- command runs

canonical concept:
- `message.delta`
- `tool.started`
- `tool.completed`
- `artifact.created`
- `turn.state.changed`

### Error mapping
pi concept:
- transport errors
- tool failures
- blocked actions
- cancellation

canonical concept:
- `runtime.error`
- `turn.completed` with failure state
- `approval.requested`
- `session.state.changed`

---

## Artifact mapping proposal

The first useful artifact mapping for pi should be minimal.

### Proposed mappings
- pi plan text → `plan` artifact
- pi edited files → `file` artifacts
- pi final summary → `summary` or transcript item

Optional later:
- screenshot artifacts
- structured patch/diff artifacts
- PR artifacts

---

## Interactive/approval flows

If Warp’s UI expects approvals or user input prompts, the canonical model should support them.

Needed canonical interactions:
- approval requested
- approval granted/denied
- user input requested
- user input answered

pi adapter can emit those only if/when pi supports them in a structured way.

If pi does not currently expose them cleanly, this can be deferred in MVP.

---

## Migration strategy

# Strategy A — Big refactor first

Refactor Warp internals to use the new abstraction everywhere before adding pi.

### Pros
- clean architecture
- easiest long-term maintenance

### Cons
- high up-front cost
- slower to first working prototype

---

# Strategy B — Compatibility facade first (recommended)

Keep most current Warp UI intact.
Introduce a compatibility facade near the backend boundary.

Steps:
1. define canonical adapter contract
2. wrap current Warp backend in a native adapter
3. implement pi adapter
4. route selected UI entrypoints through the new service
5. migrate more UI/state gradually

### Pros
- lower migration risk
- earlier working prototype
- easier to compare native vs pi behavior

### Cons
- temporary duplication likely
- some transitional complexity

---

## Alternative implementation path

Instead of refactoring Warp internals immediately, one could build a **local HTTP/WebSocket adapter service** outside Warp that emulates enough of Warp’s backend contract and uses pi underneath.

This is architecturally different but related.

### External adapter service
Pros:
- less invasive to Warp initially
- faster proof of concept

Cons:
- duplicates backend contract outside the app
- may become awkward if deep UI integration is needed

This is still a viable first step if the goal is proving feasibility quickly.

---

## Recommended path

## Short-term recommendation

Use **Strategy B**:

> Introduce a canonical backend adapter layer inside or adjacent to Warp, then implement a pi adapter against it.

Why:
- closest to the successful `t3code` pattern
- pragmatic path to a prototype
- preserves a route toward long-term backend pluggability

## If very fast validation is needed

Start with an **external compatibility adapter service** first, then later fold the abstractions back into Warp if the prototype is successful.

---

## Proposed phase plan

### Phase 1 — Backend abstraction design
- define canonical session/turn/event model
- define `AgentBackendAdapter`
- define `AgentBackendRegistry`
- define `AgentSessionDirectory`

### Phase 2 — Native adapter
- wrap current Warp backend behavior in `WarpNativeAgentAdapter`
- prove that existing UI still works through the abstraction

### Phase 3 — pi adapter MVP
- implement `PiAgentAdapter`
- support session start, send turn, stream progress, completion/failure
- emit minimal artifacts

### Phase 4 — Projection layer hardening
- normalize runtime events into Warp UI models
- improve error/status mapping
- support simple restore/reconnect

### Phase 5 — Broader parity work
- richer artifacts
- approvals/user input
- better restore/history
- optional advanced ambient-agent features

---

## Open questions

1. Should the first prototype be **inside Warp** or as an **external adapter service**?
2. How much existing Warp Agent UI must be preserved in MVP?
3. Does pi already expose enough structured runtime events, or does it need a new canonical event export layer?
4. Which artifact types are essential for the first prototype?
5. Should backend selection be per-app, per-session, or per-thread?

---

## Final recommendation

The clearest path, using the `t3code` lessons, is:

> Build a **canonical backend adapter architecture** for Warp Agent, with a `WarpNativeAgentAdapter` and a `PiAgentAdapter`, plus a canonical runtime event stream and projection layer.

This is the architecture that most directly enables:
- Warp UI reuse
- pi as backend runtime
- future backend pluggability
- manageable long-term complexity
