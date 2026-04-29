# Telemetry removal status

Date: 2026-04-29
Branch: `custom-modifications`

## Summary

Telemetry and crash reporting have been removed from the live product/runtime paths in this branch.

## Current state

### Removed at runtime
- No usage telemetry is recorded.
- No usage telemetry is sent.
- No crash reporting is initialized or sent.
- Telemetry/crash reporting settings were removed from the main user-facing auth and privacy flows.
- Periodic telemetry flushing and shutdown telemetry flushing were removed.
- Login-time identify/login telemetry sending was removed.
- Direct telemetry macro callsites were removed from runtime code paths.
- Direct `warpui::telemetry::*` recording paths were removed/no-op'd.
- CLI `PrintTelemetryEvents` now returns an error indicating telemetry was removed.

### Build status
- `cargo check -q` passes.

## Implementation notes

### Core no-op / disabled layers
- `crates/warp_core/src/telemetry.rs`
- `crates/warpui_core/src/telemetry/mod.rs`
- `crates/warpui_core/src/telemetry/event_store.rs`
- `crates/warp_core/src/channel/state.rs`

These changes disable telemetry/crash availability, make telemetry macros inert, and remove event queue behavior.

### App/runtime removal
- `app/src/server/telemetry/mod.rs`
- `app/src/server/telemetry/macros.rs`
- `app/src/crash_reporting/mod.rs`
- `app/src/lib.rs`
- `app/src/auth/auth_manager.rs`
- `app/src/auth/auth_view_body.rs`
- `app/src/auth/auth_view_shared_helpers.rs`
- `app/src/auth/login_slide.rs`
- `app/src/settings_view/privacy_page.rs`
- `app/src/settings_view/mod.rs`

These changes remove initialization, sending, shutdown flushing, and user-facing telemetry/crash settings.

### Bulk cleanup
A large set of runtime files had telemetry callsites stripped or neutralized so they no longer emit or enqueue events.

## Remaining source leftovers
Some telemetry-related source files and type definitions still exist in the tree as inert code, primarily:
- telemetry event enums/descriptors
- feature-specific telemetry modules
- some helper/test-facing types and conversions
- dead code paths that are no longer connected to runtime behavior

These leftovers are not part of an active telemetry pipeline anymore, but they still exist as source definitions.

## Intended next step if full source deletion is desired
A further cleanup pass would physically delete the remaining inert telemetry type/module definitions and then remove now-unused imports, helper methods, and tests until the tree is clean of telemetry source entirely.
