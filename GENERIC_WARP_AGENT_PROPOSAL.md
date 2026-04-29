# Generic Warp Agent Proposal

## Goal

Evolve Warp Agent from a cloud-tied agent implementation into a more **generic agent platform** that can:
- support multiple model providers
- support local and hosted inference backends
- reduce hard dependency on Warp cloud services for core agent functionality
- eventually incorporate selected **pi** runtime capabilities

In short:

> **Make Warp Agent generic first, then integrate pi functionality through that architecture.**

---

## Motivation

Today, Warp Agent appears to be strongly coupled to Warp-native backend concepts and cloud-oriented flows, including:
- cloud task/run lifecycle
- run/task APIs
- cloud-mode/ambient-agent assumptions
- request-usage and billing assumptions
- backend-specific run/event/artifact semantics

This makes several desirable outcomes harder than they should be:
- local-only agent usage
- self-hosted providers
- arbitrary provider support
- swapping or experimenting with different runtimes
- integrating a backend like **pi** cleanly

A more generic architecture would improve:
- extensibility
- local-first support
- backend experimentation
- long-term maintainability

---

## Core proposal

Refactor Warp Agent into layered concerns:

### 1. UI layer
Warp Agent frontend/UI remains the user-facing experience.

### 2. Runtime/backend abstraction
Warp Agent should depend on a generic agent runtime interface, not directly on one backend implementation.

### 3. Provider/model abstraction
Model access should be provider-agnostic and support:
- Warp-hosted providers
- local OpenAI-compatible providers
- direct provider integrations
- future custom providers

### 4. Tool/execution abstraction
Tool usage and execution permissions should be structured independently of any one backend.

### 5. Optional cloud services
Cloud features should become optional add-ons rather than hard requirements for the agent core.

---

## Why this is better than “just adapt pi to Warp”

One possible route is to leave Warp Agent architecture mostly unchanged and force **pi** to imitate Warp’s backend contract.

That can work as a prototype, but it has drawbacks:
- Warp stays structurally backend-coupled
- pi must adapt to Warp’s current assumptions
- long-term maintenance is worse
- local-first support remains a workaround instead of a core architecture feature

The better long-term path is:
- make Warp Agent generic
- then plug in pi-inspired/runtime-backed behavior cleanly

This turns pi integration from a compatibility hack into a proper backend option.

---

## Two layers of genericity

# A. Generic model/provider support

Warp Agent should not assume one inference/provider source.

It should support a provider abstraction that can handle:
- Warp cloud models
- local OpenAI-compatible endpoints
- direct OpenAI/Anthropic/Gemini-style providers
- future custom/self-hosted providers

This is necessary for local/self-hosted inference support.

However, model-provider abstraction alone is **not enough** to absorb pi.

Why:
- pi is more than a model provider
- pi includes runtime/orchestration behavior, tool execution, file edits, and workflow logic

So provider abstraction is important, but not sufficient.

---

# B. Generic runtime/backend support

Warp Agent should also support a generic backend/runtime abstraction.

This is the layer that would allow:
- Warp-native backend
- local runtime backend
- pi-backed runtime
- other future runtimes

This abstraction should cover:
- session lifecycle
- turns/prompts
- progress and streaming events
- approvals and user-input requests
- artifacts/results
- resume/reconnect semantics

This is the real foundation for meaningful pi integration.

---

## How pi should fit

pi is best treated as a **runtime/backend**, not merely as a model provider.

That means the desired architecture is:

- Warp UI
- generic agent runtime/backend interface
- runtime implementations:
  - Warp-native runtime
  - pi runtime
- provider/model abstraction under those runtimes where applicable

In other words:
- **pi should plug in at the runtime/backend layer**
- not just at the model-provider layer

---

## What pi functionality is worth migrating

Not all pi behavior needs to be imported into Warp Agent immediately.

Best candidates for migration:

### 1. Tool execution and filesystem actions
- shell command execution
- file reads/writes/edits
- code modification flows

### 2. Structured run lifecycle concepts
- explicit step progression
- structured execution states
- clearer background/foreground task handling

### 3. Local-first behavior
- core flows that do not require login
- operation without cloud dependency
- local model/runtime support as a first-class mode

### 4. Better runtime boundaries
- explicit runtime contracts
- clearer event streams
- less coupling between UI and backend-specific details

### 5. Potential later capabilities
- delegation/subagents
- richer orchestration behaviors
- more advanced coding workflows

These should only be migrated where they improve Warp’s product and architecture, not just for parity.

---

## Target architecture

# Layer 1 — Warp Agent UI

Responsible for:
- rendering conversations
- rendering progress/status
- displaying artifacts
- handling approvals/user interactions
- presenting session history and restore flows

This layer should not depend directly on a specific backend protocol.

---

# Layer 2 — Agent runtime service

Introduce a generic runtime service that Warp UI uses for all agent interactions.

Conceptually responsible for:
- start session
- send turn
- interrupt turn
- stop session
- recover session
- stream canonical runtime events

This service routes requests to a chosen backend implementation.

---

# Layer 3 — Agent backend interface

Introduce a backend/runtime adapter contract.

Example conceptual interface:

```rust
trait AgentBackend {
    fn backend_kind(&self) -> AgentBackendKind;

    async fn start_session(&self, input: StartSessionInput) -> Result<SessionHandle>;
    async fn send_turn(&self, input: SendTurnInput) -> Result<TurnHandle>;
    async fn interrupt_turn(&self, session_id: SessionId, turn_id: Option<TurnId>) -> Result<()>;
    async fn respond_to_request(&self, session_id: SessionId, request_id: RequestId, decision: Decision) -> Result<()>;
    async fn stop_session(&self, session_id: SessionId) -> Result<()>;
    async fn list_sessions(&self) -> Result<Vec<SessionHandle>>;
    async fn read_thread(&self, session_id: SessionId) -> Result<ThreadSnapshot>;

    fn stream_events(&self) -> Stream<RuntimeEvent>;
}
```

Backends should conform to this interface rather than pushing their native details directly into the UI.

---

# Layer 4 — Backend registry

Introduce a registry/service that resolves available backends.

Potential backend kinds:
- `warp_native`
- `pi`
- `local_runtime`
- future others

Responsibilities:
- resolve backend implementation
- expose backend capabilities
- support configuration/selection of backend

---

# Layer 5 — Canonical runtime event model

The UI should consume a normalized event stream rather than backend-native events.

Suggested canonical events:
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

This event layer is the key decoupling mechanism.

---

# Layer 6 — Provider/model abstraction

Separately from runtime backends, add a provider/model abstraction for inference.

Possible provider kinds:
- Warp cloud provider
- OpenAI-compatible local provider
- direct OpenAI provider
- Anthropic provider
- Gemini provider
- other custom providers

This lets Warp Agent use multiple model backends cleanly regardless of runtime.

---

# Layer 7 — Optional cloud services layer

Cloud-specific functionality should become optional rather than structurally required.

Examples:
- sync/restore across devices
- hosted run/task persistence
- billing and request-usage tracking
- cloud conversation storage
- team/cloud object integration

The agent core should still work without this layer for local/self-hosted modes.

---

## Alternative approaches

# Option 1 — Keep current Warp Agent and add pi adapter only

## Summary
Leave Warp Agent architecture mostly unchanged and build a compatibility adapter for pi.

## Pros
- faster to prototype
- lower upfront refactor cost
- good for proving feasibility

## Cons
- keeps Warp backend coupling intact
- local-first remains awkward
- long-term architecture remains less extensible
- harder to support arbitrary providers cleanly

## Best use case
A short-term prototype or proof-of-concept.

---

# Option 2 — Make Warp Agent generic at provider level only

## Summary
Refactor model access/provider selection, but keep runtime/backend mostly Warp-native.

## Pros
- improves local/self-hosted inference support
- useful for immediate provider flexibility
- less work than full backend abstraction

## Cons
- not enough to absorb pi properly
- runtime/orchestration still backend-coupled
- only partial solution

## Best use case
If the main goal is local/self-hosted model support, not runtime pluggability.

---

# Option 3 — Make Warp Agent generic at runtime + provider levels (recommended)

## Summary
Refactor both the runtime/backend boundary and the provider/model boundary.

## Pros
- best long-term architecture
- supports local and hosted backends
- supports pi cleanly
- supports arbitrary providers more naturally
- makes cloud optional rather than foundational

## Cons
- larger refactor
- slower to first complete result
- needs careful migration strategy

## Best use case
If the goal is a real next-generation Warp Agent platform.

---

## Recommended migration strategy

A full rewrite is not necessary up front.

### Phase 1 — Provider abstraction
First make model/provider access generic.

Support:
- Warp-hosted provider
- local OpenAI-compatible provider
- future provider integrations

This already improves local/self-hosted use cases.

---

### Phase 2 — Canonical runtime event model
Define and adopt a normalized runtime event schema.

This is the foundation for backend pluggability.

---

### Phase 3 — Backend/runtime abstraction
Introduce the generic backend/runtime interface and registry.

Wrap the existing Warp-native agent behavior behind that interface.

---

### Phase 4 — Native Warp backend adapter
Implement a `WarpNativeAgentBackend` so the current system keeps working through the new abstraction.

---

### Phase 5 — pi backend adapter
Implement a `PiAgentBackend` that uses the same interface and emits canonical runtime events.

Start with a narrow MVP:
- session start
- send turn
- stream progress
- final result
- simple artifacts

---

### Phase 6 — Migrate selected pi capabilities
Once the backend abstraction exists, migrate specific pi features where they provide clear value:
- local-first operation
- tool execution patterns
- clearer run lifecycle
- better structured coding workflows

---

## What success looks like

Warp Agent should eventually be able to:
- run using Warp-native backend
- run using a local backend
- run using a pi-backed backend
- use multiple model/provider types
- operate without login for local/self-hosted modes
- keep cloud services optional for users who want them

That would turn Warp Agent from a single backend/product flow into a more generic platform.

---

## Risks

### 1. Scope expansion
Trying to genericize everything at once could stall delivery.

Mitigation:
- stage the migration
- first solve provider abstraction, then runtime abstraction

### 2. Hidden UI/backend coupling
Warp UI may rely on many backend-specific assumptions.

Mitigation:
- introduce canonical runtime event projection gradually
- keep a native backend adapter during transition

### 3. pi/runtime mismatch
pi may not map perfectly onto Warp’s current assumptions.

Mitigation:
- use a canonical event model rather than forcing exact parity
- start with a narrow MVP

### 4. Product ambiguity
Need clear goals about whether Warp Agent is:
- a cloud product
- a local-first product
- or a hybrid platform

Mitigation:
- define supported modes explicitly

---

## Final recommendation

The best long-term path is:

> **Make Warp Agent more generic first, at both the provider/model layer and the runtime/backend layer, then integrate selected pi functionality through that architecture.**

This is better than only building a pi compatibility shim because it:
- improves Warp itself
- enables local/self-hosted backends naturally
- supports more providers cleanly
- makes pi integration an architectural feature rather than a workaround
