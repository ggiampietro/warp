# Full Local AI Plan

## Goal

Make Warp usable with **local AI only**, without requiring:

- Warp login
- anonymous-user creation
- Warp/cloud-hosted model endpoints
- cloud-only AI metadata dependencies
- cloud-backed agent surfaces for basic AI workflows

This plan is broader than the current vertical-slice work. It describes the full path to a coherent local-AI mode.

## Product target

A user with a local OpenAI-compatible model server running should be able to:

1. start Warp while fully logged out
2. enable and configure local AI
3. use natural language → command generation
4. use local AI autosuggestions / next-command style assistance
5. optionally use local chat-style assistance for lightweight prompt workflows
6. avoid hitting cloud auth flows and cloud AI endpoints for those local features

## Non-goals

Not in scope for the first complete local-AI mode:

- fully local Warp Agent orchestration parity with cloud agent mode
- local replacement for every Warp cloud feature
- local reimplementation of cloud conversations / cloud storage / Warp Drive sync
- local replacement for remote/cloud-only execution environments

## Design principles

1. **Vertical slices first**
   - every increment should be manually testable end to end
2. **Local mode must fail closed**
   - if cloud is unavailable, basic local AI still works
3. **Cloud and local paths should coexist**
   - local-only users should not need cloud
   - cloud users should keep current behavior
4. **Do not route local AI through cloud abstractions unless unavoidable**
5. **Prefer one provider first: OpenAI-compatible local endpoint**

## Current state

Already implemented:

- local AI settings
- login-free AI enablement when local AI is configured
- local natural language → command generation in AI command search

Not yet complete:

- some AI surfaces still call cloud endpoints
- first-run / onboarding still has cloud-oriented assumptions in places
- normal prompt AI assistance is not fully local
- local model management is still minimal
- command-search trigger reliability still needs hardening

## Full roadmap

---

## Phase 1 — Solidify local command generation

### Objective
Make `# prompt` → command generation reliable and independent of cloud.

### Work
- fix the input trigger path so `# ...` consistently opens AI command search
- preserve a robust explicit `NaturalLanguage` filter path
- improve close behavior so users can tell the search actually activated
- add visible diagnostics or a user-facing state if needed during development
- ensure no cloud request usage refresh is attempted in local mode

### Validation
- logged out user types `# list all files in this directory`
- AI command search opens reliably
- command suggestions appear from local model
- selecting a result inserts/uses the command without cloud access

---

## Phase 2 — Remove cloud dependency from passive AI assistance

### Objective
Stop basic prompt assistance from requiring Warp servers.

### Work
Audit and localize these cloud-backed surfaces where reasonable:

- prompt suggestions
n- next-command / intelligent autosuggestions
- natural-language autosuggestions
- lightweight explain/rewrite flows tied to input assistance

For each feature:
- identify the current server API call
- introduce a local inference path
- gate request-usage refresh / billing messaging when local mode is active
- ensure logged-out local usage does not try to fetch access tokens

### Validation
- no `Attempted to retrieve access token when user is logged out` errors for local-only AI assistance flows
- ghost text and suggestion banners come from local model where supported

---

## Phase 3 — Introduce a real local-AI runtime abstraction

### Objective
Move from ad hoc local command-generation plumbing to a reusable local AI backend.

### Work
Add a small internal abstraction for local inference requests, for example:

- `LocalAIClient`
- request builders for:
  - command generation
  - short completion / autosuggestion
  - dialogue / lightweight chat

Responsibilities:
- load local AI config from settings
- normalize base URL / auth
- send OpenAI-compatible requests
- parse provider responses robustly
- tolerate fenced JSON / structured and semi-structured outputs
- centralize timeouts and error mapping

### Validation
- multiple local AI features share the same client path
- cloud and local routing are explicit and testable

---

## Phase 4 — Proper local model management

### Objective
Remove cloud dependence for model availability and selection.

### Work
Current local mode uses a minimal configured model string. Expand that into:

- `/v1/models` discovery for local endpoints
- a local model cache
- a local model entry in model selection UI
- fallback behavior when discovery fails but a manual model is configured

Potential steps:
1. keep manual model name as source of truth
2. optionally fetch `/v1/models`
3. merge local model list into model preferences for local-capable features

### Validation
- user can configure model manually and use it immediately
- if the endpoint supports `/v1/models`, the UI can show discovered local models
- no cloud model metadata is required for local-only features

---

## Phase 5 — First-run and onboarding support for local AI

### Objective
Make local AI a real first-run path, not a workaround.

### Work
- allow onboarding completion without login when the user chooses AI but not Warp Drive
- add explicit local-AI-friendly messaging in onboarding/login flow
- optionally add a local AI setup entrypoint in onboarding or welcome flow
- avoid showing cloud-signup nudges for local-only successful paths

### Validation
- new user can install Warp, configure local AI, and start using AI-assisted command generation without ever signing in

---

## Phase 6 — Remove residual cloud/auth assumptions from local mode

### Objective
Local mode should not keep tripping cloud-only codepaths for basic features.

### Work
Audit and gate:

- AI request usage / billing checks
- free-tier / upgrade banners
- anonymous-user creation prompts
- cloud-only telemetry tied specifically to cloud AI usage
- cloud model-choice refreshes on auth completion
- prompt suggestion backends that still require server tokens

Desired rule:
- if a feature is explicitly running in local AI mode, it should not require auth tokens or cloud request metadata unless that exact feature is cloud-only

### Validation
- logged-out local-AI sessions do not spam cloud auth errors in logs for supported local features

---

## Phase 7 — Optional local chat / lightweight assistant mode

### Objective
Support a small subset of non-agent conversational AI locally.

### Work
If desired after the above slices:
- reuse local AI runtime for dialogue-style requests
- keep scope intentionally narrow:
  - ask a question
  - explain command
  - rewrite command
  - summarize short terminal output
- do not attempt full cloud-agent parity here

### Validation
- user can perform lightweight local chat-style tasks without login or cloud endpoints

---

## Phase 8 — Optional local provider expansion

### Objective
Support more than one local provider shape.

### Initial supported provider
- OpenAI-compatible endpoint

### Possible future providers
- Ollama-native API
- llama.cpp-native completion endpoint
- custom provider adapters

### Recommendation
Do not add additional providers until the OpenAI-compatible path is solid end to end.

---

## Architecture changes required

### 1. Routing layer
Every AI feature should choose one of:
- cloud routing
- local routing
- unsupported-in-local-mode

This routing must be explicit and not inferred indirectly from auth state.

### 2. Capability matrix
Introduce a lightweight internal capability model, for example:
- local command generation
- local autosuggestions
- local dialogue
- local image support (optional)

The UI should only expose local features the configured provider can reasonably support.

### 3. Settings model
Expand settings to support:
- provider type
- endpoint URL
- model
- optional API key
- future provider-specific options

### 4. Error model
Add user-facing local errors such as:
- local endpoint unreachable
- configured model not found
- invalid response format
- request timed out

---

## Suggested implementation order

1. make `# prompt` flow reliable
2. remove cloud dependency from prompt/next-command assistance
3. add reusable `LocalAIClient`
4. add local model discovery / proper model selection
5. improve onboarding / first-run local path
6. audit and eliminate residual cloud-only assumptions
7. optionally add lightweight local chat

## Risks

### Risk: hidden cloud fallback remains
A feature may appear local but still hit cloud paths through shared AI plumbing.

Mitigation:
- add explicit routing per feature
- test while logged out and offline where possible

### Risk: local providers return inconsistent formats
Some OpenAI-compatible servers return extra fields like reasoning content or fenced JSON.

Mitigation:
- centralize parser normalization
- keep prompts strict
- tolerate common variants

### Risk: UI still communicates cloud assumptions
Users may be asked to log in or upgrade even when local AI is correctly configured.

Mitigation:
- audit first-run, banners, and settings copy for local mode

### Risk: autosuggestions are latency-sensitive
Local models may be too slow for pleasant ghost-text UX.

Mitigation:
- keep timeouts short
- degrade gracefully
- prefer command generation first, autosuggestions second

## Definition of done for “full local AI support”

Warp should meet all of the following:

1. A logged-out user can configure local AI and use basic AI functionality.
2. Natural language → command generation works with no cloud dependency.
3. At least one passive AI assistance flow works locally (autosuggestion or next-command class feature).
4. Local mode does not require anonymous-user creation for supported local features.
5. Supported local features do not emit cloud-auth failures during ordinary use.
6. First-run flow does not force login just because the user wants AI.
7. Model selection/configuration does not depend on cloud model metadata.

## Manual QA checklist

- Start Warp with no logged-in user
- Verify local AI settings can be configured
- Verify `# list files` returns local command suggestions
- Verify onboarding can complete without login when Warp Drive is off
- Verify supported passive AI feature works locally
- Verify no cloud auth/token errors are emitted for supported local flows
- Verify app still behaves normally when local AI is disabled

## Immediate next actions

1. harden the `#` trigger path until it is fully reliable
2. audit prompt suggestions / next-command paths still attempting cloud auth
3. factor local request handling into a reusable local AI client
