# Tech Spec: Inotify Watch Explosion from Duplicate Recursive Repo Watchers

## Problem
Warp appears to register overlapping recursive filesystem watchers for the same repository roots. On Linux, this can explode inotify watch usage for large repositories.

Observed locally:

- Warp process used about `261673` inotify watches
- `~/vendrix` had about `37949` directories
- ratio strongly suggests multiple overlapping watch trees

## Relevant Code
- `crates/watcher/src/lib.rs` — low-level `BulkFilesystemWatcher` and background watcher thread
- `crates/repo_metadata/src/watcher.rs` — `DirectoryWatcher`, shared repository watcher
- `crates/repo_metadata/src/local_model.rs` — local repo metadata model creates its own bulk watcher and recursively registers repo paths
- `crates/ai/src/index/full_source_code_embedding/manager.rs` — codebase index manager creates another bulk watcher and recursively registers repo paths
- `app/src/lib.rs` — application wiring that initializes shared models including repo metadata and codebase indexing

## Current State
### Low-level watcher layer
`BulkFilesystemWatcher::register_path()` forwards every add request to a background thread.
That thread calls `watch_filtered(path, recursive_mode, filter)` directly.

There does not appear to be a dedup or ref-count registry at this level.

### Shared repository watcher
`DirectoryWatcher` appears to be the intended shared watcher for repository updates.
It starts recursive watching for repository roots when the first subscriber appears.

### Additional recursive watchers
At least two other subsystems also create their own `BulkFilesystemWatcher` instances and recursively watch repo paths:

1. `LocalRepoMetadataModel`
2. `CodebaseIndexManager`

This likely duplicates recursive registration over the same large repository roots.

## Proposed Changes

### 1. Add registration deduplication/ref-counting in `BulkFilesystemWatcher`
File:
- `crates/watcher/src/lib.rs`

Introduce a registration map in the background watcher layer.

Proposed behavior:
- canonicalize or otherwise normalize path keys when possible
- key registrations by path plus enough additional metadata to preserve correctness
- on duplicate add, increment ref-count instead of calling `watch_filtered(...)` again
- on remove, decrement ref-count and only call `unwatch(...)` at zero

This provides immediate protection against repeated registration of the same path.

### 2. Add instrumentation
Files:
- `crates/watcher/src/lib.rs`

Add debug logs and counters around:
- register requests
- duplicate register requests
- unregister requests
- unmatched unregister requests
- current unique watched path count

This makes it possible to confirm which subsystems are causing repeated registration.

### 3. Consolidate repo-tree watching onto `DirectoryWatcher`
Files:
- `crates/repo_metadata/src/local_model.rs`
- `crates/ai/src/index/full_source_code_embedding/manager.rs`
- possibly nearby subscribers using repo updates

Target direction:
- `DirectoryWatcher` remains the single owner of recursive repo-tree watching
- repo metadata, indexing, and similar consumers subscribe to shared repository updates instead of each creating another recursive watcher over the same roots

This is the architectural cleanup after low-level protection is in place.

## Implementation Plan

### Phase 1: confirm duplication with instrumentation
1. Add temporary logging in `BulkFilesystemWatcher::register_path()` and unregister flow.
2. Record normalized path, recursive mode, and whether the registration is new or duplicate.
3. Reproduce with a large repository such as `~/vendrix`.
4. Confirm the same repo root is registered from multiple subsystems.

### Phase 2: implement low-level dedup/ref-counting
1. Extend the background watcher with a registration map.
2. On add:
   - if registration is new, call `watch_filtered(...)`
   - otherwise increment ref-count only
3. On remove:
   - decrement ref-count
   - call `unwatch(...)` only when ref-count reaches zero
4. Add tests for nested add/remove behavior.

### Phase 3: reduce architectural duplication
1. Audit all `BulkFilesystemWatcher::new(...)` call sites used for repo trees.
2. Remove or reduce independent recursive repo watches in:
   - `LocalRepoMetadataModel`
   - `CodebaseIndexManager`
3. Convert those consumers to reuse `DirectoryWatcher` repository updates where practical.

### Phase 4: regression coverage
1. Add tests ensuring duplicate registration of the same repo path does not multiply watcher state.
2. Add integration coverage for repo metadata + codebase indexing active together.
3. Verify watcher-driven features still function correctly after consolidation.

## Risks and Mitigations

### Filter semantics
Different callers may register the same path with different filters.

Mitigation:
- either include filter identity in the dedup key
- or consolidate repo-tree watching so one broad watcher owns the OS-level watch and consumers filter events in memory

### Path aliasing / symlinks
Different path spellings may bypass dedup.

Mitigation:
- normalize or canonicalize where safe
- add logging for non-normalized duplicates during rollout

### Unwatch correctness
Ref-count bugs can leak watches or remove them too early.

Mitigation:
- add explicit add/remove sequencing tests
- log unmatched unregisters

## Testing and Validation
- Compare inotify watch usage before and after on a large repo
- Verify repo metadata updates still fire
- Verify codebase indexing still updates
- Verify other watcher users are unaffected
- Verify unrelated local dev tools can still start watchers while Warp is open

## Expected Outcome
After deduplication and watcher consolidation:

- Warp should consume far fewer inotify watches on large repos
- watch usage should no longer scale multiplicatively across subsystems
- unrelated tools should stop failing due to watch exhaustion when Warp is running
