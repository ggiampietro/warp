use std::{
    collections::{HashMap, HashSet},
    future::Future,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc::{self, channel},
    },
    thread,
    time::Duration,
};

pub mod home_watcher;
pub use home_watcher::{HomeDirectoryWatcher, HomeDirectoryWatcherEvent};

use anyhow::Result;
use futures::channel::oneshot;
use notify_debouncer_full::{
    new_debouncer_opt,
    notify::{
        self,
        event::{ModifyKind, RenameMode},
        EventKind, RecommendedWatcher, RecursiveMode, WatchFilter,
    },
    DebounceEventHandler, DebounceEventResult, DebouncedEvent, Debouncer, NoCache,
};
use warpui::{Entity, ModelContext};

static NEXT_WATCHER_INSTANCE_ID: AtomicUsize = AtomicUsize::new(1);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct RegistrationKey {
    path: PathBuf,
    recursive_mode: RecursiveMode,
}

#[derive(Debug, Default)]
struct RegistrationDiagnostics {
    total_register_requests: usize,
    total_unregister_requests: usize,
    duplicate_register_requests: usize,
    unmatched_unregister_requests: usize,
    path_refcounts: HashMap<PathBuf, usize>,
    registrations: HashMap<RegistrationKey, usize>,
}

#[derive(Debug)]
struct RegisterStats {
    normalized_path: PathBuf,
    refcount_for_registration: usize,
    duplicate_registration: bool,
}

#[derive(Debug)]
struct UnregisterStats {
    normalized_path: PathBuf,
    refcount_for_path_after_remove: usize,
    unmatched_unregister: bool,
}

impl RegistrationDiagnostics {
    fn normalize_path(path: &Path) -> PathBuf {
        std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
    }

    fn record_register(&mut self, path: &Path, recursive_mode: RecursiveMode) -> RegisterStats {
        self.total_register_requests += 1;

        let normalized_path = Self::normalize_path(path);
        let key = RegistrationKey {
            path: normalized_path.clone(),
            recursive_mode,
        };

        let path_refcount = self.path_refcounts.entry(normalized_path.clone()).or_default();
        *path_refcount += 1;

        let refcount_for_registration = self.registrations.entry(key).or_default();
        *refcount_for_registration += 1;
        let duplicate_registration = *refcount_for_registration > 1;
        if duplicate_registration {
            self.duplicate_register_requests += 1;
        }

        RegisterStats {
            normalized_path,
            refcount_for_registration: *refcount_for_registration,
            duplicate_registration,
        }
    }

    fn record_unregister(&mut self, path: &Path) -> UnregisterStats {
        self.total_unregister_requests += 1;

        let normalized_path = Self::normalize_path(path);
        let mut unmatched_unregister = false;
        let refcount_for_path_after_remove = match self.path_refcounts.get_mut(&normalized_path) {
            Some(refcount) => {
                *refcount -= 1;
                let remaining = *refcount;
                if remaining == 0 {
                    self.path_refcounts.remove(&normalized_path);
                    self.registrations.retain(|key, _| key.path != normalized_path);
                }
                remaining
            }
            None => {
                self.unmatched_unregister_requests += 1;
                unmatched_unregister = true;
                0
            }
        };

        UnregisterStats {
            normalized_path,
            refcount_for_path_after_remove,
            unmatched_unregister,
        }
    }
}

enum BackgroundFileWatcherCommand {
    AddPath {
        path: PathBuf,
        filter: WatchFilter,
        response: oneshot::Sender<Result<()>>,
        recursive_mode: RecursiveMode,
    },
    RemovePath {
        path: PathBuf,
        response: oneshot::Sender<Result<()>>,
    },
}

struct BackgroundFileWatcher {
    notifier: Debouncer<RecommendedWatcher, NoCache>,
    rx: mpsc::Receiver<BackgroundFileWatcherCommand>,
    instance_id: usize,
    debug_label: String,
    diagnostics: RegistrationDiagnostics,
}

impl BackgroundFileWatcher {
    fn new(
        debounce_duration: Duration,
        handler: WatcherEventHandler,
        rx: mpsc::Receiver<BackgroundFileWatcherCommand>,
        instance_id: usize,
        debug_label: String,
    ) -> Self {
        let debounced_watcher = new_debouncer_opt(
            debounce_duration,
            None,
            handler,
            NoCache,
            notify::Config::default(),
        )
        .expect("Should be able to create watcher");

        Self {
            notifier: debounced_watcher,
            rx,
            instance_id,
            debug_label,
            diagnostics: RegistrationDiagnostics::default(),
        }
    }

    /// Listen to streamed in commands to modify the internal notifier state.
    fn run(mut self) {
        while let Ok(res) = self.rx.recv() {
            match res {
                BackgroundFileWatcherCommand::AddPath {
                    path,
                    filter,
                    response,
                    recursive_mode,
                } => {
                    let stats = self.diagnostics.record_register(&path, recursive_mode);
                    log::debug!(
                        "[fs_watcher:{}#{}] register path={} recursive_mode={recursive_mode:?} duplicate_registration={} refcount_for_registration={} active_unique_paths={} active_unique_registrations={} total_register_requests={} duplicate_register_requests={}",
                        self.debug_label,
                        self.instance_id,
                        stats.normalized_path.display(),
                        stats.duplicate_registration,
                        stats.refcount_for_registration,
                        self.diagnostics.path_refcounts.len(),
                        self.diagnostics.registrations.len(),
                        self.diagnostics.total_register_requests,
                        self.diagnostics.duplicate_register_requests,
                    );

                    let _ = response.send(
                        self.notifier
                            .watch_filtered(path, recursive_mode, filter)
                            .inspect_err(|err| {
                                log::warn!(
                                    "[fs_watcher:{}#{}] Failed to watch path: {err:?}",
                                    self.debug_label,
                                    self.instance_id,
                                );
                            })
                            .map_err(anyhow::Error::new),
                    );
                }
                BackgroundFileWatcherCommand::RemovePath { path, response } => {
                    let stats = self.diagnostics.record_unregister(&path);
                    log::debug!(
                        "[fs_watcher:{}#{}] unregister path={} unmatched_unregister={} refcount_for_path_after_remove={} active_unique_paths={} active_unique_registrations={} total_unregister_requests={} unmatched_unregister_requests={}",
                        self.debug_label,
                        self.instance_id,
                        stats.normalized_path.display(),
                        stats.unmatched_unregister,
                        stats.refcount_for_path_after_remove,
                        self.diagnostics.path_refcounts.len(),
                        self.diagnostics.registrations.len(),
                        self.diagnostics.total_unregister_requests,
                        self.diagnostics.unmatched_unregister_requests,
                    );

                    let _ = response.send(
                        self.notifier
                            .unwatch(path)
                            .inspect_err(|err| {
                                log::warn!(
                                    "[fs_watcher:{}#{}] Failed to remove repo watcher: {err:?}",
                                    self.debug_label,
                                    self.instance_id,
                                );
                            })
                            .map_err(anyhow::Error::new),
                    );
                }
            }
        }
        log::debug!(
            "[fs_watcher:{}#{}] File watcher stream closed",
            self.debug_label,
            self.instance_id,
        )
    }
}

#[derive(Clone, Default, Debug)]
pub struct BulkFilesystemWatcherEvent {
    /// Paths that were created.
    pub added: HashSet<PathBuf>,

    /// Paths whose contents were modified.
    pub modified: HashSet<PathBuf>,

    /// List of paths that should be removed.
    pub deleted: HashSet<PathBuf>,

    /// Mapping from rename target to rename source.
    pub moved: HashMap<PathBuf, PathBuf>,
}

impl BulkFilesystemWatcherEvent {
    /// Iterator over paths that were added or modified.
    pub fn added_or_updated_iter(&self) -> impl Iterator<Item = &PathBuf> {
        self.added.iter().chain(self.modified.iter())
    }

    /// Returns an owned set of paths that were added or modified.
    ///
    /// Prefer `added_or_updated_iter` when you don't need ownership.
    pub fn added_or_updated_set(&self) -> HashSet<PathBuf> {
        self.added_or_updated_iter().cloned().collect()
    }

    fn is_empty(&self) -> bool {
        self.added.is_empty()
            && self.modified.is_empty()
            && self.deleted.is_empty()
            && self.moved.is_empty()
    }
}

/// Model for watching for all file / folder changes under target directories.
/// The updates are debounced with a configurable duration.
pub struct BulkFilesystemWatcher {
    tx: mpsc::Sender<BackgroundFileWatcherCommand>,
}

impl BulkFilesystemWatcher {
    fn next_instance_id() -> usize {
        NEXT_WATCHER_INSTANCE_ID.fetch_add(1, Ordering::Relaxed)
    }

    pub fn new(debounce_duration: Duration, ctx: &mut ModelContext<Self>) -> Self {
        Self::new_named("bulk_filesystem_watcher", debounce_duration, ctx)
    }

    pub fn new_named(
        debug_label: impl Into<String>,
        debounce_duration: Duration,
        ctx: &mut ModelContext<Self>,
    ) -> Self {
        let debug_label = debug_label.into();
        let instance_id = Self::next_instance_id();
        let (tx, rx) = async_channel::unbounded();
        let (bg_tx, bg_rx) = channel();

        // Note that we keep the file watcher in the background since registering and unregistering file path
        // involves fs calls.
        if let Err(e) = thread::Builder::new()
            .name(format!("Bulk Filesystem Watcher {debug_label}#{instance_id}"))
            .spawn(move || {
                let watcher = BackgroundFileWatcher::new(
                    debounce_duration,
                    WatcherEventHandler { tx },
                    bg_rx,
                    instance_id,
                    debug_label,
                );
                watcher.run();
            })
        {
            log::error!("Failed to spawn thread for background file watcher {e:?}");
        }
        ctx.spawn_stream_local(rx, Self::handle_watcher_event, |_, _| {});

        Self { tx: bg_tx }
    }

    pub fn new_for_test() -> Self {
        Self::new_for_test_named("bulk_filesystem_watcher")
    }

    pub fn new_for_test_named(_debug_label: impl Into<String>) -> Self {
        let (bg_tx, _) = channel();
        Self { tx: bg_tx }
    }

    /// Stop watching a path. The returned future resolves once the path is fully unregistered.
    /// Awaiting the future is *not* required for the path to be unregistered.
    pub fn unregister_path(&mut self, path: &Path) -> impl Future<Output = Result<()>> {
        let (tx, rx) = oneshot::channel();
        let send_result = self.tx.send(BackgroundFileWatcherCommand::RemovePath {
            path: path.to_path_buf(),
            response: tx,
        });

        if send_result.is_err() {
            log::warn!("Filesystem watcher thread has exited");
        }

        async move {
            send_result?;
            rx.await.map_err(anyhow::Error::new)??;
            Ok(())
        }
    }

    /// Add a new path to watch. The returned future resolves once the path is fully registered.
    /// Awaiting the future is *not* required for the path to be registered.
    pub fn register_path(
        &mut self,
        path: &Path,
        watch_filter: WatchFilter,
        recursive_mode: RecursiveMode,
    ) -> impl Future<Output = Result<()>> {
        let (tx, rx) = oneshot::channel();
        let send_result = self.tx.send(BackgroundFileWatcherCommand::AddPath {
            path: path.to_path_buf(),
            filter: watch_filter,
            response: tx,
            recursive_mode,
        });

        if send_result.is_err() {
            log::warn!("Filesystem watcher thread has exited");
        }

        async move {
            send_result?;
            rx.await.map_err(anyhow::Error::new)??;
            Ok(())
        }
    }

    fn handle_watcher_event(
        &mut self,
        event: BulkFilesystemWatcherEvent,
        ctx: &mut ModelContext<Self>,
    ) {
        ctx.emit(event);
    }
}

impl Entity for BulkFilesystemWatcher {
    type Event = BulkFilesystemWatcherEvent;
}

struct WatcherEventHandler {
    tx: async_channel::Sender<BulkFilesystemWatcherEvent>,
}

impl DebounceEventHandler for WatcherEventHandler {
    fn handle_event(&mut self, result: DebounceEventResult) {
        match result {
            Ok(debounce_events) => {
                if let Ok(config_event) =
                    deduplicate_and_merge_raw_notifier_events(&debounce_events)
                {
                    if let Err(e) = self.tx.try_send(config_event) {
                        log::warn!("Failed to send WatcherEvent: {e:?}");
                    }
                }
            }
            Err(e) => {
                log::warn!("Received error in repo watcher: {e:?}");
            }
        }
    }
}

/// Dedupe and standardize the raw notifier events into a BulkFilesystemWatcherEvent.
fn deduplicate_and_merge_raw_notifier_events(
    raw_fs_events: &[DebouncedEvent],
) -> Result<BulkFilesystemWatcherEvent> {
    let mut update = BulkFilesystemWatcherEvent::default();

    let mut created: HashSet<PathBuf> = HashSet::new();
    let mut modified: HashSet<PathBuf> = HashSet::new();

    let mut rename_from = None;
    for fs_event in raw_fs_events {
        match fs_event.event.kind {
            // Create and modify should always be preserved.
            EventKind::Create(_) => created.extend(fs_event.event.paths.clone()),
            // On Windows, ReadDirectoryChangesW emits ModifyKind::Any instead of
            // ModifyKind::Data for file content changes. Handle both variants.
            EventKind::Modify(ModifyKind::Data(_) | ModifyKind::Any) => {
                modified.extend(fs_event.event.paths.clone())
            }

            // If a path is created and then removed, we should not keep this path in the update event.
            // If a path is modified / moved and then removed, we should only keep the remove event.
            EventKind::Remove(_) => {
                for path in &fs_event.event.paths {
                    if created.remove(path) {
                        continue;
                    }

                    modified.remove(path);

                    // If a path is modified, remove the source instead of the target name.
                    update.deleted.insert(match update.moved.remove(path) {
                        Some(source) => source,
                        None => path.clone(),
                    });
                }
            }

            // Note that in MacOS, deleting could be either 1) a true removal 2) moving to trash. In the
            // second case, this event will come in as a EventKind::Modify on the name with RenameMode::Any.
            // Here we count this as a OutlineUpdate::remove.
            //
            // Another case of EventKind::Modify(ModifyKind::Name(RenameMode::Any)) is when a file was renamed
            // when we turn off file map caching. Since we cannot guarantee mapping from the old name to the new name
            // (this is ordering is unfortunately not strictly persisted in the event. e.g. rename A -> B and B -> A might
            // create an event stream of rename A, rename A, rename B, rename B), we will treating them as inserts and removes
            // for now based on the current state of the file system.
            EventKind::Modify(ModifyKind::Name(RenameMode::Any)) => {
                let is_rename = matches!(
                    fs_event.event.kind,
                    EventKind::Modify(ModifyKind::Name(RenameMode::Any))
                );
                for path in &fs_event.event.paths {
                    // Decides whether this is a rename to or rename from based on the current state of the file system.
                    // This is not ideal since when we receive the event, the state of the file system could have changed.
                    // E.g. rename A -> B, B -> C, if we receives the first event after B is already renamed to C, this will
                    // translate the events to delete (A, B), create (C).
                    // TODO(kevin)
                    let path_exists = is_rename && path.exists();

                    if path_exists {
                        created.insert(path.clone());
                        continue;
                    }

                    if created.remove(path) {
                        continue;
                    }

                    modified.remove(path);

                    // If a path is modified, remove the source instead of the target name.
                    update.deleted.insert(match update.moved.remove(path) {
                        Some(source) => source,
                        None => path.clone(),
                    });
                }
            }

            // If a path is renamed, we should check if it has been renamed in this update before and squash
            // any sequential renames.
            EventKind::Modify(ModifyKind::Name(rename_mode)) => 'rename: {
                let paths = &fs_event.event.paths;

                let (from, to) = match rename_mode {
                    RenameMode::From if !paths.is_empty() => {
                        rename_from = Some(paths.first().expect("Checked above").clone());
                        break 'rename;
                    }
                    RenameMode::To if !paths.is_empty() && rename_from.is_some() => (
                        rename_from.take().expect("Checked above"),
                        paths.first().expect("Checked above").clone(),
                    ),
                    RenameMode::Both if paths.len() > 1 => (
                        paths.first().expect("Checked above").clone(),
                        paths.get(1).expect("Checked above").clone(),
                    ),
                    _ => break 'rename,
                };

                match update.moved.remove(&from) {
                    Some(source) => update.moved.insert(to, source),
                    None => update.moved.insert(to, from),
                };
            }
            _ => (),
        }
    }

    // A path that is created and then modified within the debounce window should be considered "added".
    for path in &created {
        modified.remove(path);
    }

    update.added = created;
    update.modified = modified;

    if update.is_empty() {
        return Err(anyhow::anyhow!("No update event produced"));
    }

    Ok(update)
}

#[cfg(test)]
mod tests {
    use super::RegistrationDiagnostics;
    use notify_debouncer_full::notify::RecursiveMode;
    use std::path::Path;

    #[test]
    fn registration_diagnostics_tracks_duplicate_registrations() {
        let mut diagnostics = RegistrationDiagnostics::default();
        let path = Path::new("/tmp/warp-watcher-diagnostics");

        let first = diagnostics.record_register(path, RecursiveMode::Recursive);
        assert!(!first.duplicate_registration);
        assert_eq!(first.refcount_for_registration, 1);
        assert_eq!(diagnostics.duplicate_register_requests, 0);
        assert_eq!(diagnostics.path_refcounts.len(), 1);
        assert_eq!(diagnostics.registrations.len(), 1);

        let second = diagnostics.record_register(path, RecursiveMode::Recursive);
        assert!(second.duplicate_registration);
        assert_eq!(second.refcount_for_registration, 2);
        assert_eq!(diagnostics.duplicate_register_requests, 1);
        assert_eq!(diagnostics.path_refcounts.len(), 1);
        assert_eq!(diagnostics.registrations.len(), 1);
    }

    #[test]
    fn registration_diagnostics_tracks_unmatched_unregisters() {
        let mut diagnostics = RegistrationDiagnostics::default();
        let path = Path::new("/tmp/warp-watcher-diagnostics-missing");

        let stats = diagnostics.record_unregister(path);
        assert!(stats.unmatched_unregister);
        assert_eq!(stats.refcount_for_path_after_remove, 0);
        assert_eq!(diagnostics.unmatched_unregister_requests, 1);
    }

    #[test]
    fn registration_diagnostics_clears_path_once_refcount_reaches_zero() {
        let mut diagnostics = RegistrationDiagnostics::default();
        let path = Path::new("/tmp/warp-watcher-diagnostics-clear");

        diagnostics.record_register(path, RecursiveMode::Recursive);
        diagnostics.record_register(path, RecursiveMode::Recursive);

        let first_remove = diagnostics.record_unregister(path);
        assert!(!first_remove.unmatched_unregister);
        assert_eq!(first_remove.refcount_for_path_after_remove, 1);
        assert_eq!(diagnostics.path_refcounts.len(), 1);
        assert_eq!(diagnostics.registrations.len(), 1);

        let second_remove = diagnostics.record_unregister(path);
        assert!(!second_remove.unmatched_unregister);
        assert_eq!(second_remove.refcount_for_path_after_remove, 0);
        assert!(diagnostics.path_refcounts.is_empty());
        assert!(diagnostics.registrations.is_empty());
    }
}
