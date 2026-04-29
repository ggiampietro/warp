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
- `crates/repo_metadata/src/local_model.rs` — local repo metadata model
- `crates/ai/src/index/full_source_code_embedding/manager.rs` — codebase index manager
- `app/src/lib.rs` — application wiring that initializes shared models including repo metadata and codebase indexing

## Current State
### Low-level watcher layer
`BulkFilesystemWatcher::register_path()` still forwards every add request to a background thread and calls `watch_filtered(path, recursive_mode, filter)` directly.

There is still no OS-level dedup or ref-count registry at this layer.

What is now implemented:
- debug instrumentation around register/unregister flow
- watcher instance labels so logs identify the owning subsystem
- counters for duplicate register attempts and unmatched unregisters

### Shared repository watcher
`DirectoryWatcher` is now the primary owner of recursive repo-tree watching.
It starts recursive watching for repository roots when the first subscriber appears.

### Repo metadata and codebase indexing
The two primary duplicate repo-tree watchers have now been removed:

1. `LocalRepoMetadataModel` no longer creates a recursive repo watcher for git repositories; it subscribes to shared `Repository` updates from `DirectoryWatcher`
2. `CodebaseIndexManager` no longer creates a recursive repo watcher for repo trees; it subscribes to shared `Repository` updates and converts them into `ChangedFiles`

`LocalRepoMetadataModel` still uses its private watcher for standalone lazily-loaded non-repository paths, which is expected and not part of the repo-tree explosion.

## Implemented Changes
### 1. Instrumentation in `BulkFilesystemWatcher`
File:
- `crates/watcher/src/lib.rs`

Implemented:
- register/unregister debug logs
- duplicate registration diagnostics
- unmatched unregister diagnostics
- watcher instance naming (`repo_metadata::directory_watcher`, `repo_metadata::local_model`, `ai::codebase_index_manager`, etc.)

### 2. Consolidate repo-tree watching onto `DirectoryWatcher`
Files:
- `crates/repo_metadata/src/local_model.rs`
- `crates/ai/src/index/full_source_code_embedding/manager.rs`

Implemented:
- `LocalRepoMetadataModel` now subscribes to `Repository` updates instead of owning a recursive repo watcher
- `CodebaseIndexManager` now subscribes to `Repository` updates instead of owning a recursive repo watcher
- both consumers keep their existing higher-level update logic, only the filesystem event source changed

## Deferred Change
### Low-level dedup/ref-counting in `BulkFilesystemWatcher`
File:
- `crates/watcher/src/lib.rs`

Status:
- deferred for now

Reason:
- filter semantics make naive low-level dedup risky
- after the architectural consolidation, the main repo-tree explosion should already be addressed with less behavioral risk

If needed later, the design remains:
- canonicalize or otherwise normalize path keys when possible
- key registrations by path plus enough metadata to preserve correctness
- on duplicate add, increment ref-count instead of calling `watch_filtered(...)`
- on remove, decrement ref-count and only call `unwatch(...)` at zero

## Implementation Plan Status
### Phase 1: confirm duplication with instrumentation
Status: implemented in code, runtime repro still recommended.

### Phase 2: implement low-level dedup/ref-counting
Status: deferred.

### Phase 3: reduce architectural duplication
Status: implemented for the main repo-tree duplicates.

Completed:
- `LocalRepoMetadataModel` ✅
- `CodebaseIndexManager` ✅

### Phase 4: regression coverage
Status: partial.

Completed:
- diagnostics-level watcher tests ✅
- repo metadata subscription/update regression test ✅
- codebase index changed-files conversion regression test ✅

Still useful:
- end-to-end runtime validation on a large repo
- before/after inotify count measurement

## Risks and Mitigations
### Filter semantics
Different callers may register the same path with different filters.

Mitigation:
- prefer shared broad repo-tree watching with downstream in-memory filtering
- keep low-level dedup deferred unless filter identity is handled correctly

### Path aliasing / symlinks
Different path spellings may bypass dedup or lookup.

Mitigation:
- canonicalize where safe
- keep debug logging to detect aliasing

### Unwatch correctness
Subscription cleanup bugs can leak watchers or remove them too early.

Mitigation:
- explicit unsubscribe on repo/index removal
- regression tests around subscription-driven updates

## Testing and Validation
- Compare inotify watch usage before and after on a large repo
- Run with debug logs and verify repo-tree registrations now primarily come from `repo_metadata::directory_watcher`
- Verify repo metadata updates still fire
- Verify codebase indexing still updates
- Verify other watcher users are unaffected
- Verify unrelated local dev tools can still start watchers while Warp is open

Suggested runtime command:

```bash
RUST_LOG=watcher=debug,repo_metadata=debug,ai=debug ./script/run
```

## Expected Outcome
After the implemented consolidation:

- Warp should consume far fewer inotify watches on large repos
- watch usage should no longer scale multiplicatively across repo metadata and codebase indexing
- unrelated tools should be less likely to fail due to watch exhaustion while Warp is running
