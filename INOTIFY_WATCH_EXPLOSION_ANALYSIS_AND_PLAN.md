# Inotify watch explosion: analysis and implementation plan

## Summary
Warp appears to consume an unexpectedly large number of Linux inotify watches when large repositories are open.

Observed locally:

- Warp process used roughly `261673` inotify watches
- `~/vendrix` alone has roughly `37949` directories
- The ratio strongly suggests the same large tree is being watched multiple times by different Warp subsystems

This is not just a system configuration issue. Higher `fs.inotify.max_user_watches` values help temporarily, but the Warp codebase likely has an architectural duplication problem in filesystem watching.

## Symptom
Large repositories can cause Warp to exhaust the machine's inotify watch budget, which then breaks unrelated tools such as Pi, Vite, or Tauri with errors like:

```text
ENOSPC: System limit for number of file watchers reached
```

## Likely root cause
Warp has multiple independent recursive watchers over repository trees, and the low-level watcher layer does not appear to deduplicate or ref-count registrations.

### Core watcher layer
File:
- `crates/watcher/src/lib.rs`

`BulkFilesystemWatcher::register_path()` forwards every registration request to the background watcher thread.
The background watcher then calls:

- `watch_filtered(path, recursive_mode, filter)`

There does not appear to be a registry of already-watched paths or ref-counting for repeated registrations.

Consequence:
- if multiple subsystems register the same repo root recursively, duplicate watches are created

## Code paths contributing to duplicate repo watches

### 1. Shared repository watcher
File:
- `crates/repo_metadata/src/watcher.rs`

`DirectoryWatcher::start_watching_directory()` registers repository roots recursively.
This looks like the intended shared watcher for repository-level updates.

### 2. Repo metadata creates its own recursive watcher
File:
- `crates/repo_metadata/src/local_model.rs`

`LocalRepoMetadataModel::new()` creates its own `BulkFilesystemWatcher`.
Then `index_directory()` registers repository paths recursively.

This appears to duplicate coverage already available through `DirectoryWatcher`.

### 3. Codebase index manager creates another recursive watcher
File:
- `crates/ai/src/index/full_source_code_embedding/manager.rs`

`CodebaseIndexManager::new()` creates another `BulkFilesystemWatcher`.
Then `watch_path()` registers repository roots recursively.

This likely duplicates repository watching yet again.

## Why this diagnosis fits the observed numbers
With ~38k directories in `~/vendrix`, one recursive watcher could already be expensive.
But ~261k active watches strongly suggests multiple overlapping recursive registrations over the same or similar paths.

Approximate ratio:

- `261673 / 37949 ~= 6.9`

That is consistent with multiple overlapping watch trees, not a single reasonable registration.

## Probable architectural issue
The codebase currently seems to allow these patterns simultaneously:

- one subsystem owns a repo watcher
- another subsystem independently creates its own bulk watcher
- another subsystem again creates its own bulk watcher
- all may recursively watch the same repository roots

Without path deduplication or shared ownership, watch counts scale badly on large repos.

## Recommended direction
Prefer one shared recursive watcher for repository trees, with downstream consumers subscribing to higher-level events instead of registering their own recursive filesystem watches.

The best candidate for that shared owner appears to be:
- `DirectoryWatcher`

## Implementation plan

### Phase 1: add instrumentation and confirm duplication
Goal: prove exactly which paths are being registered repeatedly and from where.

1. Add debug logging around `BulkFilesystemWatcher::register_path()` and `unregister_path()` in:
   - `crates/watcher/src/lib.rs`
2. Log at least:
   - requested path
   - recursive mode
   - call site tag if available
   - whether this is a first registration or duplicate
3. Add temporary counters for:
   - total register requests
   - unique registered paths
   - duplicate registration attempts
4. Reproduce with a large repo such as `~/vendrix`
5. Confirm which subsystems repeatedly register the same root

### Phase 2: add dedup/ref-counting to the low-level watcher
Goal: stop duplicate watch registrations from exploding watch counts.

Implement inside:
- `crates/watcher/src/lib.rs`

Proposed design:

1. Track registrations in a map keyed by something like:
   - canonicalized path
   - recursive mode
   - possibly a normalized filter key if needed
2. On `AddPath`:
   - if key is new, call `watch_filtered(...)` and set refcount to 1
   - if key already exists, increment refcount and do not re-register with the OS
3. On `RemovePath`:
   - decrement refcount
   - only call `unwatch(...)` when refcount reaches zero
4. Log mismatched unregister calls for diagnostics

Notes:
- If filter identity cannot be compared safely, start by deduplicating the common cases where the same subsystem registers the same path repeatedly with equivalent semantics.
- A stricter API may be needed if different filters for the same path must coexist.

### Phase 3: reduce architectural duplication
Goal: converge on one shared repo watcher.

Investigate and likely refactor:

1. `crates/repo_metadata/src/local_model.rs`
   - avoid independent recursive repo registrations if `DirectoryWatcher` already covers them
2. `crates/ai/src/index/full_source_code_embedding/manager.rs`
   - prefer subscribing to repository-level updates instead of owning another recursive watcher for the same repo roots
3. Audit any other recursive repo registrations using `BulkFilesystemWatcher::new()` and `register_path(..., RecursiveMode::Recursive)`

Target outcome:
- one shared recursive watcher per repo tree
- multiple consumers subscribe to shared events
- no repeated OS-level watch registration for the same root

### Phase 4: add regression tests
1. Add tests for low-level ref-count behavior in:
   - `crates/watcher/src/lib.rs`
2. Add tests ensuring repeated registration of the same path does not multiply watches
3. Add higher-level integration tests covering:
   - repo metadata + codebase indexing active together
   - watch count staying stable when multiple consumers observe the same repo

## Risks and edge cases

### Filter semantics
Different watchers may register the same path with different filters.
If dedup is keyed only by path, behavior could become incorrect.

Mitigation:
- either include filter identity in the dedup key
- or unify repo-tree watchers around a shared broad watcher and let downstream consumers filter events in-memory

### Canonicalization
Path aliases and symlinks can bypass simple dedup.

Mitigation:
- canonicalize when possible before keying registrations
- document fallback behavior for non-canonicalizable paths

### Unregister correctness
Ref-counting must not leak watchers or prematurely unwatch paths.

Mitigation:
- test nested register/unregister sequences carefully

## Suggested first practical fix
If only one fix is done first, it should be:

1. implement ref-counted dedup in `crates/watcher/src/lib.rs`
2. add logging to identify duplicate callers

That would provide immediate protection even before the broader architectural cleanup is completed.

## Candidate files to modify
- `crates/watcher/src/lib.rs`
- `crates/repo_metadata/src/watcher.rs`
- `crates/repo_metadata/src/local_model.rs`
- `crates/ai/src/index/full_source_code_embedding/manager.rs`

## Expected outcome
After deduplication and watcher consolidation:

- large repositories should no longer multiply inotify usage across subsystems
- Warp should consume far fewer inotify watches
- unrelated tools should stop failing due to watch exhaustion when Warp is open
