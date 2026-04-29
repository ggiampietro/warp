# Local AI Status

## Summary

Local AI support has been implemented across the main basic-assist flows needed to use Warp without Warp login/server dependency for command generation and some passive AI assistance.

The current implementation targets a **local OpenAI-compatible endpoint** and is being tested against local models such as Gemma 4 and Qwen served on localhost.

## Implemented

### 1. Local AI settings
Implemented in:
- `app/src/settings/ai.rs`
- `app/src/settings_view/ai_page.rs`
- `app/src/settings/ai_tests.rs`

Added settings for:
- `agents.local_ai.enabled`
- `agents.local_ai.openai_compatible.base_url`
- `agents.local_ai.openai_compatible.model`
- `agents.local_ai.openai_compatible.api_key`

Added helpers:
- `is_local_ai_configured()`
- `is_local_ai_enabled()`

### 2. Logged-out AI enablement
`AISettings::is_any_ai_enabled(...)` now allows AI to be enabled while logged out if Local AI is configured.

This is covered by tests.

### 3. Local command generation
Implemented in:
- `app/src/server/server_api/ai.rs`
- `app/src/ai_assistant/mod.rs`
- `app/src/server/server_api/ai_test.rs`

Added local routing for:
- natural language → shell command generation

Behavior:
- when Local AI is configured, command generation uses the local OpenAI-compatible `/chat/completions` endpoint
- response parsing tolerates fenced JSON responses
- generated commands are mapped into existing AI-generated workflow structures

### 4. Command search / `# ...` flow improvements
Implemented in:
- `app/src/terminal/input.rs`
- `app/src/search/command_search/view.rs`
- `app/src/search/command_search/warp_ai.rs`

Changes:
- explicit Natural Language filter when opening command search from `#`
- Enter fallback to AI command search when the input begins with `#`
- local mode removes the legacy static Warp-AI placeholder items that rewrote input into:
  - `What is the command to: ...`
- command-search datasource registration is hardened for local mode
- loading/processing state now shows a local-AI-specific message instead of briefly showing `No results found`

Current UI text:
- `Generating local command suggestions…`

### 5. Local backend routing for passive AI support
Implemented in:
- `app/src/server/server_api.rs`
- `app/src/terminal/input.rs`
- `app/src/ai/blocklist/passive_suggestions/legacy.rs`

Local routing added for:
- next-command / intelligent autosuggestion backend
- prompt suggestions backend

Also updated gating so these flows can run in local mode even if Warp considers the client offline, as long as Local AI is enabled.

### 6. First-run/login flow improvement
Implemented in:
- `app/src/root_view.rs`
- `app/src/root_view_tests.rs`

Behavior:
- onboarding no longer forces login just because AI was selected
- login is still required for Warp Drive-backed flows

### 7. Settings-page local-mode billing cleanup
Implemented in:
- `app/src/settings_view/ai_page.rs`

Behavior:
- AI settings page skips cloud request usage refresh when Local AI is enabled
- Usage section now explains that Local AI requests go directly to the configured local endpoint
- Warp credits / cloud billing messaging is suppressed there for local mode

## Current behavior

### Working well
- Local AI can be configured in settings
- Local AI can be used while logged out
- `# ...` command search path now works much better
- Local generated commands appear in command search
- legacy cloud prompt rewrite path has been removed from local mode
- loading state now better communicates that local generation is in progress
- local prompt-suggestion and next-command backend paths no longer require access-token fetches

### Known-good local config examples
Example config:

```toml
[agents.local_ai]
enabled = true

[agents.local_ai.openai_compatible]
base_url = "http://127.0.0.1:8080/v1"
model = "Gemma-4"
api_key = ""
```

Actual model string must match the local provider’s `/v1/models` response.

## Tests run successfully

### Targeted local AI tests
```bash
cargo test -p warp local_ai -- --nocapture
```

### Local parsing tests
```bash
cargo test -p warp parse_local_ -- --nocapture
```

### Command search tests
```bash
cargo test -p warp command_search -- --nocapture
```

All of the above were passing in the current environment.

## Important implementation notes

### Linux run path
On this machine, `./script/run` launches:
- `warp-oss`

because `warp-channel-config` is not installed.

That means the relevant config path is:
- `~/.config/warp-oss/settings.toml`

not the local channel path.

### Why a previous failure happened
A previous broken state came from the old sync command-search items (`OpenWarpAI` / `TranslateUsingWarpAI`) being used instead of actual local generated command results. That caused input to be rewritten to:

```text
What is the command to: ...
```

This has been removed in local mode.

## Remaining work / next steps

### 1. Manual verification pass
Do a manual end-to-end verification of:
- `# list all files in this directory`
- `# find rust files containing todo`
- `# show the 20 biggest directories here`

Check:
- command search opens
- shows `Generating local command suggestions…`
- returns commands instead of cloud prompt rewrites

### 2. Audit remaining cloud-only AI surfaces
There may still be some cloud-only UI or background flows outside the already-rerouted features.

Good candidates to audit next:
- buy-credits / quota banners in terminal AI surfaces
- other AI settings subpanels with cloud-only assumptions
- any remaining auth-token fetch attempts during local-only workflows

### 3. Improve local provider robustness
Potential enhancements:
- model discovery UI from `/v1/models`
- better local endpoint error messages
- optional support for more provider variants beyond OpenAI-compatible APIs

### 4. Add more direct tests for local command-search behavior
Current tests cover parsing and search subsystems, but not a full UI integration test that asserts the `# ...` flow returns generated commands in local mode.

Recommended next test work:
- add integration or view-level test for local AI command search registration and async result rendering

### 5. Local autosuggestion UX polish
Backends are routed, but UX may still need refinement for:
- zero-state next-command timing
- prompt suggestion visibility / prioritization
- making local generation state more obvious in the UI

## Recommended next engineering step

If continuing immediately, the highest-value next step is:

**audit terminal AI banners and remaining cloud-assumption UI in local mode**, then add one integration-style test for the local `# ...` command-search flow.
