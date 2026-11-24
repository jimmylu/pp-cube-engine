//! Model versions kept in shared storage, where the semantic layer publishes
//! them and every replica reads them.
//!
//! The layout isolates business lines and asset types:
//!
//! ```text
//! <business line>[/<sub line>[/<sub line>]]/models/<version>/manifest.json
//!                                                           /cubes/<cube>.yml
//!                                                           /views/<view>.yml
//! ```
//!
//! `models` is the one asset type this service reads; what lies under any other
//! name at that place is left alone, for other kinds of semantic assets. A
//! version is the model directory Cube recommends, one cube or view per file.
//! Its `manifest.json` is written last and marks it complete. Versions are
//! immutable: a loaded one is not read again, and removing its manifest retires
//! it. The version a caller names is the business line and the version joined
//! by `/`, such as `property/motor/v12`.
//!
//! A store is a flat list of keys and a way to read one, which is what a
//! directory tree and an S3-protocol bucket (`crate::s3`) have in common.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use pp_cube_model::{assemble, BundleError, ModelFile};

use crate::CubeService;

/// The asset type this service reads.
pub const MODELS: &str = "models";
/// The file that marks a version complete.
pub const MANIFEST: &str = "manifest.json";
/// A business line has at most this many levels.
pub const MAX_LEVELS: usize = 3;
/// A model file larger than this is refused.
pub const MAX_FILE_BYTES: u64 = 4 * 1024 * 1024;
/// A store with more keys than this is refused: the listing is held in memory.
pub const MAX_KEYS: usize = 100_000;

/// Keys with `/` between their parts, relative to the root of the store.
pub trait ModelStore: Send + Sync {
    /// Every key of the store.
    fn list(&self) -> std::io::Result<Vec<String>>;
    /// The text of one key.
    fn read(&self, key: &str) -> std::io::Result<String>;
}

/// A directory tree, such as a NAS mount. Entries whose name starts with a dot
/// are not part of the store: file systems keep their own there.
#[derive(Debug, Clone)]
pub struct DirectoryStore {
    root: PathBuf,
}

impl DirectoryStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn walk(
        &self,
        directory: &Path,
        prefix: &str,
        depth: usize,
        keys: &mut Vec<String>,
    ) -> std::io::Result<()> {
        // Three levels of business line, the asset type, the version, a folder, a file.
        if depth > MAX_LEVELS + 4 {
            return Ok(());
        }
        for entry in std::fs::read_dir(directory)? {
            let entry = entry?;
            let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            if name.starts_with('.') {
                continue;
            }
            let key = if prefix.is_empty() {
                name
            } else {
                format!("{prefix}/{name}")
            };
            // Follows links, so a version may be a link to where it is kept.
            let metadata = std::fs::metadata(entry.path())?;
            if metadata.is_dir() {
                self.walk(&entry.path(), &key, depth + 1, keys)?;
            } else if metadata.is_file() {
                if keys.len() >= MAX_KEYS {
                    return Err(std::io::Error::other("the store holds too many files"));
                }
                keys.push(key);
            }
        }
        Ok(())
    }
}

impl ModelStore for DirectoryStore {
    fn list(&self) -> std::io::Result<Vec<String>> {
        let mut keys = Vec::new();
        self.walk(&self.root, "", 1, &mut keys)?;
        keys.sort();
        Ok(keys)
    }

    fn read(&self, key: &str) -> std::io::Result<String> {
        // Keys come from `list`; one that climbs out of the root is not one of them.
        if key
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
        {
            return Err(std::io::Error::other("not a key of the store"));
        }
        let path = key
            .split('/')
            .fold(self.root.clone(), |path, part| path.join(part));
        if std::fs::metadata(&path)?.len() > MAX_FILE_BYTES {
            return Err(std::io::Error::other("the file is too large"));
        }
        std::fs::read_to_string(path)
    }
}

/// Why a version in the store is not offered. A fixed set, safe to log and count.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rejection {
    /// The path is not a business line, `models` and a version.
    Layout,
    /// `manifest.json` is not the manifest of schema 1.
    Manifest,
    /// A file could not be read.
    Read,
    /// The files are not a model directory.
    Files,
    /// The model is not inside the contract.
    Model,
    /// The content is not what the manifest's digest says.
    Digest,
    /// A cube is on a data source this service has no dialect for.
    DataSource,
    /// The service holds as many versions as it may.
    Full,
}

impl Rejection {
    pub fn name(self) -> &'static str {
        match self {
            Self::Layout => "layout",
            Self::Manifest => "manifest",
            Self::Read => "read",
            Self::Files => "files",
            Self::Model => "model",
            Self::Digest => "digest",
            Self::DataSource => "data-source",
            Self::Full => "full",
        }
    }
}

/// A rejected version with what an operator needs to find the fault: a file or
/// a rule, never model content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rejected {
    pub version: String,
    pub reason: Rejection,
    pub detail: String,
}

/// The versions the keys of a store hold: for each complete one, the keys of its
/// files. Keys under another asset type are not this service's.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Layout {
    pub versions: BTreeMap<String, Vec<String>>,
    pub rejected: Vec<Rejected>,
}

fn line_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    bytes
        .next()
        .is_some_and(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        && bytes.all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-')
        && name.len() <= 64
}

fn version_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    bytes.next().is_some_and(|b| b.is_ascii_alphanumeric())
        && bytes.all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
        && name.len() <= 64
}

/// Reads the layout out of `keys`. Pure.
pub fn layout(keys: &[String]) -> Layout {
    let mut found: BTreeMap<String, (bool, Vec<String>)> = BTreeMap::new();
    let mut faults: BTreeMap<String, String> = BTreeMap::new();
    for key in keys {
        let parts: Vec<&str> = key.split('/').collect();
        // The asset type follows one to three levels of business line.
        let Some(at) = parts.iter().position(|part| *part == MODELS) else {
            continue;
        };
        let place = parts[..=at].join("/");
        if at == 0 || at > MAX_LEVELS || !parts[..at].iter().all(|part| line_name(part)) {
            faults.entry(place).or_insert_with(|| {
                format!("`{MODELS}` belongs under one to {MAX_LEVELS} levels of business line")
            });
            continue;
        }
        let Some(version) = parts.get(at + 1).filter(|_| parts.len() > at + 2) else {
            faults.entry(key.clone()).or_insert_with(|| {
                "a file directly under `models`; versions are directories".into()
            });
            continue;
        };
        let id = format!("{}/{version}", parts[..at].join("/"));
        if !version_name(version) {
            faults.entry(id).or_insert_with(|| {
                "a version is named with letters, digits, `.`, `_` and `-`".into()
            });
            continue;
        }
        let entry = found.entry(id).or_default();
        if parts[at + 2..] == [MANIFEST] {
            entry.0 = true;
        } else {
            entry.1.push(key.clone());
        }
    }
    let mut layout = Layout::default();
    for (version, detail) in faults {
        layout.rejected.push(Rejected {
            version,
            reason: Rejection::Layout,
            detail,
        });
    }
    // A version without its manifest is still being written, or was retired.
    for (id, (complete, files)) in found {
        if complete {
            layout.versions.insert(id, files);
        }
    }
    layout
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema: u32,
    /// The digest ValidateModel returns for this content; checked when given.
    digest: Option<String>,
}

/// What one pass over the store changed and found.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Scan {
    pub loaded: Vec<String>,
    pub retired: Vec<String>,
    /// Every version that cannot be offered now.
    pub rejected: Vec<Rejected>,
    /// Those of `rejected` that the pass before rejected for the same reason:
    /// counted, and not logged again.
    pub unchanged: Vec<String>,
}

/// Keeps the versions of a service equal to the complete versions of a store.
pub struct StoreSync {
    store: Box<dyn ModelStore>,
    /// Tells the stores of one process apart in its metrics.
    name: String,
    /// The versions this store put into the service; only those are retired by it.
    owned: Mutex<BTreeSet<String>>,
    /// The rejections of the last pass, to report a change once.
    rejected: Mutex<BTreeMap<String, Rejection>>,
    passes: AtomicU64,
    failures: AtomicU64,
    last_failed: AtomicU64,
}

impl StoreSync {
    pub fn new(store: impl ModelStore + 'static) -> Self {
        Self {
            store: Box::new(store),
            name: "0".to_owned(),
            owned: Mutex::default(),
            rejected: Mutex::default(),
            passes: AtomicU64::new(0),
            failures: AtomicU64::new(0),
            last_failed: AtomicU64::new(0),
        }
    }

    /// Names the store in its metrics; a process numbers its stores.
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// One pass: offer every complete version not yet offered, retire the ones this
    /// store offered that are no longer complete. Fails, changing nothing, when the
    /// store cannot be listed; a version that cannot be offered is reported and does
    /// not stop the others.
    pub fn scan(&self, service: &CubeService) -> std::io::Result<Scan> {
        let keys = match self.store.list() {
            Ok(keys) => keys,
            Err(error) => {
                self.failures.fetch_add(1, Ordering::Relaxed);
                self.last_failed.store(1, Ordering::Relaxed);
                return Err(error);
            }
        };
        let Layout { versions, rejected } = layout(&keys);
        let mut scan = Scan {
            rejected,
            ..Scan::default()
        };
        let available: BTreeSet<String> = service.versions().into_iter().map(|(v, _)| v).collect();
        let mut owned = self.owned.lock().unwrap_or_else(|e| e.into_inner());
        // A version retired through the service while its manifest stands comes back.
        owned.retain(|version| available.contains(version));
        for (version, files) in &versions {
            if owned.contains(version) {
                continue;
            }
            match self.offer(service, version, files) {
                Ok(()) => {
                    owned.insert(version.clone());
                    scan.loaded.push(version.clone());
                }
                Err((reason, detail)) => scan.rejected.push(Rejected {
                    version: version.clone(),
                    reason,
                    detail,
                }),
            }
        }
        let gone: Vec<String> = owned
            .iter()
            .filter(|v| !versions.contains_key(*v))
            .cloned()
            .collect();
        for version in gone {
            let _ = service.retire(&version);
            owned.remove(&version);
            scan.retired.push(version);
        }
        let mut before = self.rejected.lock().unwrap_or_else(|e| e.into_inner());
        let unchanged = scan
            .rejected
            .iter()
            .filter(|r| before.get(&r.version) == Some(&r.reason));
        scan.unchanged = unchanged.map(|r| r.version.clone()).collect();
        *before = scan
            .rejected
            .iter()
            .map(|r| (r.version.clone(), r.reason))
            .collect();
        drop(before);
        self.passes.fetch_add(1, Ordering::Relaxed);
        self.last_failed.store(0, Ordering::Relaxed);
        Ok(scan)
    }

    fn offer(
        &self,
        service: &CubeService,
        version: &str,
        files: &[String],
    ) -> Result<(), (Rejection, String)> {
        let line = version.rsplit_once('/').map_or("", |(line, _)| line);
        let name = version.rsplit('/').next().unwrap_or(version);
        let root = format!("{line}/{MODELS}/{name}/");
        let read = |key: &str| {
            self.store
                .read(key)
                .map_err(|_| (Rejection::Read, format!("`{key}` could not be read")))
        };
        let manifest: Manifest = serde_json::from_str(&read(&format!("{root}{MANIFEST}"))?)
            .map_err(|_| {
                (
                    Rejection::Manifest,
                    format!("`{MANIFEST}` is not a manifest"),
                )
            })?;
        if manifest.schema != 1 {
            return Err((
                Rejection::Manifest,
                format!("`{MANIFEST}` has an unknown schema"),
            ));
        }
        let mut texts = Vec::with_capacity(files.len());
        for key in files {
            texts.push((key.strip_prefix(&root).unwrap_or(key), read(key)?));
        }
        let parts: Vec<ModelFile<'_>> = texts
            .iter()
            .map(|(path, text)| ModelFile { path, text })
            .collect();
        let model = assemble(&parts).map_err(|error| (Rejection::Files, error.to_string()))?;
        match service.publish(version, model, manifest.digest.as_deref()) {
            Ok(_) => Ok(()),
            Err(BundleError::Model(issues)) => {
                let first = issues
                    .first()
                    .map(|i| format!("{} ({})", i.path, i.rule_id));
                Err((
                    Rejection::Model,
                    format!(
                        "{} issues; the first at {}",
                        issues.len(),
                        first.unwrap_or_default()
                    ),
                ))
            }
            Err(BundleError::DigestMismatch) => Err((
                Rejection::Digest,
                "the content is not what the manifest's digest says".into(),
            )),
            Err(BundleError::UnmappedDataSource { data_source }) => Err((
                Rejection::DataSource,
                format!("no dialect serves the data source `{data_source}`"),
            )),
            Err(BundleError::CatalogFull) => Err((
                Rejection::Full,
                "the service holds as many versions as it may".into(),
            )),
            // The version is offered already with other content, by a file or a caller.
            Err(_) => Err((
                Rejection::Digest,
                "the version is already offered with other content".into(),
            )),
        }
    }

    /// This store's part of the Prometheus exposition.
    pub fn render(&self) -> String {
        render(&[self])
    }
}

/// The stores' part of the Prometheus exposition, each labelled with its name.
pub fn render(stores: &[&StoreSync]) -> String {
    let mut text = String::new();
    let _ = writeln!(text, "# TYPE ppcube_model_store_scans_total counter");
    for store in stores {
        for (outcome, count) in [("ok", &store.passes), ("error", &store.failures)] {
            let _ = writeln!(
                text,
                "ppcube_model_store_scans_total{{store=\"{}\",outcome=\"{outcome}\"}} {}",
                store.name,
                count.load(Ordering::Relaxed)
            );
        }
    }
    let _ = writeln!(text, "# TYPE ppcube_model_store_last_scan_failed gauge");
    for store in stores {
        let _ = writeln!(
            text,
            "ppcube_model_store_last_scan_failed{{store=\"{}\"}} {}",
            store.name,
            store.last_failed.load(Ordering::Relaxed)
        );
    }
    let _ = writeln!(text, "# TYPE ppcube_model_store_rejected_versions gauge");
    for store in stores {
        let mut reasons: BTreeMap<&str, usize> = BTreeMap::new();
        for reason in store
            .rejected
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .values()
        {
            *reasons.entry(reason.name()).or_default() += 1;
        }
        for (reason, count) in reasons {
            let _ = writeln!(
                text,
                "ppcube_model_store_rejected_versions{{store=\"{}\",reason=\"{reason}\"}} {count}",
                store.name
            );
        }
    }
    text
}

/// One JSON line per change of a pass, for the host's log: versions and fixed
/// reasons only. A version that stays rejected for the same reason is logged once.
pub fn log_lines(scan: &Scan) -> Vec<String> {
    let line = |version: &str, outcome: &str, reason: Option<&str>| {
        let mut event = serde_json::json!({"event": "model_store", "model_version": version, "outcome": outcome});
        if let Some(reason) = reason {
            event["reason"] = reason.into();
        }
        event.to_string()
    };
    let mut lines = Vec::new();
    lines.extend(scan.loaded.iter().map(|v| line(v, "loaded", None)));
    lines.extend(scan.retired.iter().map(|v| line(v, "retired", None)));
    let news = scan
        .rejected
        .iter()
        .filter(|r| !scan.unchanged.contains(&r.version));
    lines.extend(news.map(|r| line(&r.version, "rejected", Some(r.reason.name()))));
    lines
}
