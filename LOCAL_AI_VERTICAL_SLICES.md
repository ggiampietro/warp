# Local AI Support Plan (Vertical Slices First)

## Goal

Add local AI support for basic Warp features without requiring Warp servers, login, or the built-in Warp Agent.

Initial focus:

1. Natural language → shell command generation
2. Later: local AI-powered autosuggestions

## Development approach

This work will be implemented as **vertical slices, not horizontal layers**.

That means each slice must be:

- user-visible
- end-to-end testable
- minimally integrated through settings, runtime plumbing, and UI
- shippable or at least manually verifiable on its own

We explicitly want to avoid building isolated infrastructure first and only wiring it up much later.

## Guiding principle

For every slice, prefer:

- a thin but complete user flow
- one working path through the system
- real manual validation of the feature

Over:

- broad provider abstractions up front
- large model-management refactors before feature delivery
- partially wired backend-only groundwork

## Slice plan

### Slice 1: Local AI configuration + login-free enablement

Outcome:

- user can enable local AI in settings
- user can configure a local OpenAI-compatible endpoint
- AI is allowed when local AI is configured, even if logged out

Scope:

- add settings for:
  - local AI enabled
  - base URL
  - optional API key
  - model name
- update AI gating so local AI can be active without Warp auth
- keep cloud-only features disabled unless separately supported

Validation:

- launch Warp while logged out
- configure local endpoint
- verify AI-related local feature gates become available

### Slice 2: End-to-end natural language → command via local model

Outcome:

- command search can translate natural language into shell commands using the local endpoint

Scope:

- reuse existing AI command search UI
- route command-generation requests to local AI when local mode is enabled
- parse local model output into existing generated-command structures
- keep the cloud path intact when local mode is disabled

Validation:

- while logged out, open AI command search
- enter a prompt like "find large files modified today"
- receive shell command suggestions from the local model
- accept one into the terminal input

### Slice 3: Local model fallback / minimal model representation

Outcome:

- local mode works cleanly with a configured model, without depending on cloud-provided model catalogs

Scope:

- add a minimal local-model representation sufficient for Slice 2
- avoid a full model-management redesign initially
- optionally synthesize one local model entry from settings

Validation:

- command generation still works with no cloud model metadata present
- model-dependent code paths use the configured local model cleanly

### Slice 4: Local AI autosuggestions

Outcome:

- Warp can show AI-powered autosuggestions from the local model

Scope:

- reuse existing autosuggestion hooks where possible
- only enable the local path when local AI is configured
- keep latency bounded and fail closed on timeout/errors

Validation:

- while logged out, with local AI enabled, type into the terminal input
- verify AI-powered suggestion text can appear
- verify normal non-AI completions remain functional

## Non-goals for the first pass

- making Warp Agent fully local
- implementing all local providers natively
- redesigning execution profiles
- replacing cloud-only agent/orchestration features
- broad provider abstraction before the first end-to-end feature works

## Provider strategy

Start with a single provider type:

- **OpenAI-compatible local endpoint**

Examples:

- LM Studio
- LocalAI
- vLLM
- Ollama through a compatible endpoint

This is the fastest path to a working vertical slice.

## Implementation rule

Before starting a new horizontal subsystem improvement, confirm that the current slice already supports:

1. configuration
2. runtime execution
3. visible UI behavior
4. manual validation

If not, finish the slice first.

## Initial execution order

1. Slice 1 — local config + login-free enablement
2. Slice 2 — local NL → command end to end
3. Slice 3 — minimal local model handling cleanup
4. Slice 4 — local autosuggestions

## Success criterion for implementation start

Implementation should begin with Slice 1 and immediately continue into Slice 2 so that the first meaningful end-to-end behavior is available as early as possible.

## Current status

- [x] Slice 1 — local config + login-free AI enablement
- [x] Slice 2 — end-to-end local natural language → command generation
- [x] Slice 3 — local basic assist routing beyond command search
  - local next-command / intelligent autosuggestion backend routing
  - local prompt suggestions backend routing
  - offline/local gating updates so localhost AI still works without internet
  - more reliable `#` trigger handling
- [ ] Slice 4 — broader local model handling cleanup and additional local-only UX polish
