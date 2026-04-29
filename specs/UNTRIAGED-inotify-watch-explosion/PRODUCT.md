# Inotify Watch Explosion from Duplicate Recursive Repo Watchers

## Summary
Warp can consume an extremely large number of Linux inotify watches when large repositories are open. The likely cause is that multiple subsystems recursively watch the same repository trees independently instead of sharing one watcher.

## Problem
Observed locally:

- Warp used roughly `261673` inotify watches
- `~/vendrix` alone has roughly `37949` directories
- Other tools then failed with:

```text
ENOSPC: System limit for number of file watchers reached
```

Examples of affected tools:
- Pi subagent result watcher
- Vite / Tauri dev server

Increasing Linux inotify limits helps temporarily, but does not fix the underlying Warp behavior.

## Goals
1. Warp should not register duplicate OS-level recursive watches for the same repository tree.
2. Large repositories should not cause watch counts to multiply across subsystems.
3. Repository consumers should be able to share filesystem updates without each owning a separate recursive watcher.
4. Warp should be diagnosable when watch registration grows unexpectedly.

## Non-goals
- Solving all Linux inotify exhaustion on the whole machine.
- Changing unrelated watcher behavior for non-repository paths unless needed.
- Replacing the existing repository event architecture entirely.

## User Experience
### Before
Opening one or more large repositories in Warp can consume most or all inotify watches on the machine. Other dev tools may then fail to start filesystem watchers.

### After
Opening large repositories in Warp should use substantially fewer watches. Other tools should continue to function normally while Warp is open.

## Likely root cause
Multiple Warp subsystems appear to recursively watch the same repo roots:

- shared repository watcher
- local repo metadata watcher
- codebase indexing watcher

The low-level watcher layer appears to accept repeated registrations without deduplicating or reference counting them.

## Success Criteria
1. Repeated registrations for the same repo root no longer multiply OS-level watches.
2. Warp watch usage for a large repo is materially lower than before.
3. Repo metadata and codebase indexing can coexist without each adding another full recursive watch tree.
4. Debug logging or metrics can identify duplicate registration sources.

## Validation
Completed runtime validation on a large repo setup:
- before: about `261673` inotify watches
- after consolidation: about `45098` inotify watches
- reduction: about `82.7%`

Debug logging confirmed that recursive repo-tree registrations now primarily come from the shared `repo_metadata::directory_watcher`, not separate repo-tree watchers in repo metadata or codebase indexing.

Additional checks:
- repo metadata still indexed repositories successfully
- codebase indexing / outline-related repo activity still functioned
- unrelated tools should have substantially more inotify headroom while Warp is open
