//! Megu3D project IO: the `.megu3d` container, its manifest, and atomic saves.
//!
//! A project is a ZIP with a small, stable set of entries:
//!
//! ```text
//! project.megu3d
//! ├─ manifest.json   container layout, schema, app stamp, units
//! └─ scene.json      versioned envelope around the core scene
//! ```
//!
//! Saving never truncates the previous file: a temporary sibling is written and
//! fsynced, the old file becomes `*.bak`, and only then does the temporary file
//! take its place (`docs/assumptions.md`, `D-44`).

pub mod manifest;
pub mod migrate;
pub mod zip;

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use megu3d_core::Scene;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

pub use manifest::Manifest;
pub use zip::{ZipEntry, ZipError};

pub const MANIFEST_ENTRY: &str = "manifest.json";
pub const SCENE_ENTRY: &str = "scene.json";
pub const EXTENSION: &str = "megu3d";

#[derive(Debug, Error)]
pub enum IoError {
    #[error("container is not readable: {0}")]
    Zip(#[from] ZipError),
    #[error("json is not readable: {0}")]
    Json(String),
    #[error("container has no `{0}` entry")]
    MissingEntry(&'static str),
    #[error("file schema {file} is newer than this build ({app})")]
    SchemaTooNew { file: String, app: String },
    #[error("file schema {file} cannot be upgraded by this build")]
    SchemaUnsupported { file: String },
    #[error("`{path}` could not be accessed: {source}")]
    Fs { path: String, source: String },
}

impl IoError {
    /// Stable code for the UI error shape `{ code, message, details,
    /// recoverable }` (`docs/02-architecture.md`, §6).
    pub fn code(&self) -> &'static str {
        match self {
            IoError::Zip(_) => "IO_CONTAINER_INVALID",
            IoError::Json(_) => "IO_JSON_INVALID",
            IoError::MissingEntry(_) => "IO_ENTRY_MISSING",
            IoError::SchemaTooNew { .. } => "IO_SCHEMA_TOO_NEW",
            IoError::SchemaUnsupported { .. } => "IO_SCHEMA_UNSUPPORTED",
            IoError::Fs { .. } => "IO_FS_FAILED",
        }
    }

    /// A bad file leaves the session intact; a broken disk does not.
    pub fn recoverable(&self) -> bool {
        !matches!(self, IoError::Fs { .. })
    }

    fn fs(path: &Path, source: std::io::Error) -> IoError {
        IoError::Fs {
            path: path.display().to_string(),
            source: source.to_string(),
        }
    }

    fn json(error: serde_json::Error) -> IoError {
        IoError::Json(error.to_string())
    }
}

/// What `scene.json` holds. The envelope carries the version so migrations can
/// run before the core scene is typed.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneDocument {
    pub schema_version: String,
    pub units: String,
    pub scene: Scene,
}

/// Borrowed twin of [`SceneDocument`] so saving does not clone the scene.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SceneDocumentRef<'a> {
    schema_version: &'a str,
    units: &'a str,
    scene: &'a Scene,
}

#[derive(Debug)]
pub struct Project {
    pub manifest: Manifest,
    pub scene: Scene,
}

/// Packs a project into container bytes. Pretty-printed on purpose: a project
/// that diffs in git is worth more than a few saved kilobytes until `scene.bin`
/// lands with ADR-2 (`A-96`).
pub fn to_bytes(scene: &Scene, manifest: &Manifest) -> Result<Vec<u8>, IoError> {
    let document = SceneDocumentRef {
        schema_version: scene.schema_version.as_str(),
        units: manifest.units.as_str(),
        scene,
    };
    let scene_json = serde_json::to_vec_pretty(&document).map_err(IoError::json)?;
    let manifest_json = serde_json::to_vec_pretty(manifest).map_err(IoError::json)?;
    let entries = vec![
        ZipEntry::new(MANIFEST_ENTRY, manifest_json),
        ZipEntry::new(SCENE_ENTRY, scene_json),
    ];
    Ok(zip::write(&entries)?)
}

pub fn from_bytes(bytes: &[u8]) -> Result<Project, IoError> {
    let entries = zip::read(bytes)?;
    let manifest_entry =
        zip::entry(&entries, MANIFEST_ENTRY).ok_or(IoError::MissingEntry(MANIFEST_ENTRY))?;
    let scene_entry = zip::entry(&entries, SCENE_ENTRY).ok_or(IoError::MissingEntry(SCENE_ENTRY))?;
    let manifest: Manifest =
        serde_json::from_slice(&manifest_entry.data).map_err(IoError::json)?;
    let raw: Value = serde_json::from_slice(&scene_entry.data).map_err(IoError::json)?;
    let found = raw
        .get("schemaVersion")
        .and_then(Value::as_str)
        .unwrap_or(manifest.schema_version.as_str())
        .to_owned();
    let upgraded = migrate::migrate(raw, &found)?;
    let document: SceneDocument = serde_json::from_value(upgraded).map_err(IoError::json)?;
    let mut scene = document.scene;
    // The uuid index is derived state and is never stored.
    scene.rebuild_index();
    Ok(Project { manifest, scene })
}

/// Saves atomically and returns the manifest that reached the disk.
pub fn save(path: &Path, scene: &Scene) -> Result<Manifest, IoError> {
    let manifest = match load(path) {
        Ok(project) => project.manifest.touched(scene.schema_version.as_str()),
        Err(_) => Manifest::new(scene.schema_version.as_str()),
    };
    let bytes = to_bytes(scene, &manifest)?;
    write_atomically(path, &bytes)?;
    Ok(manifest)
}

pub fn load(path: &Path) -> Result<Project, IoError> {
    let bytes = fs::read(path).map_err(|error| IoError::fs(path, error))?;
    from_bytes(&bytes)
}

pub fn backup_path(path: &Path) -> PathBuf {
    sibling(path, ".bak")
}

pub fn temp_path(path: &Path) -> PathBuf {
    sibling(path, ".tmp")
}

fn sibling(path: &Path, suffix: &str) -> PathBuf {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| format!("project.{EXTENSION}"));
    path.with_file_name(format!("{name}{suffix}"))
}

fn write_atomically(path: &Path, bytes: &[u8]) -> Result<(), IoError> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    if !parent.as_os_str().is_empty() {
        fs::create_dir_all(parent).map_err(|error| IoError::fs(parent, error))?;
    }
    let temp = temp_path(path);
    {
        let mut file = fs::File::create(&temp).map_err(|error| IoError::fs(&temp, error))?;
        file.write_all(bytes)
            .map_err(|error| IoError::fs(&temp, error))?;
        file.sync_all().map_err(|error| IoError::fs(&temp, error))?;
    }
    if path.exists() {
        let backup = backup_path(path);
        // Windows refuses to rename onto an existing file, so clear it first.
        if backup.exists() {
            fs::remove_file(&backup).map_err(|error| IoError::fs(&backup, error))?;
        }
        fs::rename(path, &backup).map_err(|error| IoError::fs(path, error))?;
    }
    fs::rename(&temp, path).map_err(|error| IoError::fs(&temp, error))
}

#[cfg(test)]
mod tests {
    use super::*;
    use megu3d_core::SCHEMA_VERSION;

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("megu3d-io-{tag}-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).expect("scratch dir");
        dir
    }

    #[test]
    fn projects_round_trip_through_bytes() {
        let scene = Scene::startup();
        let bytes = to_bytes(&scene, &Manifest::new(SCHEMA_VERSION)).expect("write");
        let project = from_bytes(&bytes).expect("read");
        assert_eq!(project.scene.len(), scene.len());
        assert_eq!(project.scene.mesh_count(), scene.mesh_count());
        assert_eq!(project.scene.selection_uuids(), scene.selection_uuids());
        assert_eq!(project.scene.triangle_count(), scene.triangle_count());
        assert_eq!(project.manifest.schema_version, SCHEMA_VERSION);
        assert_eq!(project.manifest.container_version, manifest::CONTAINER_VERSION);
    }

    #[test]
    fn a_loaded_scene_can_resolve_uuids_again() {
        let scene = Scene::startup();
        let uuid = scene.selection_uuids().first().copied().expect("selection");
        let bytes = to_bytes(&scene, &Manifest::new(SCHEMA_VERSION)).expect("write");
        let project = from_bytes(&bytes).expect("read");
        assert!(
            project.scene.resolve(uuid).is_ok(),
            "the uuid index was not rebuilt after load"
        );
    }

    #[test]
    fn scene_json_stays_readable() {
        let bytes = to_bytes(&Scene::startup(), &Manifest::new(SCHEMA_VERSION)).expect("write");
        let entries = zip::read(&bytes).expect("read");
        let entry = zip::entry(&entries, SCENE_ENTRY).expect("scene entry");
        let text = String::from_utf8(entry.data.clone()).expect("utf-8");
        assert!(text.contains("\"schemaVersion\""));
        assert!(text.contains("\"units\""));
        assert!(text.contains('\n'), "scene.json should stay diffable");
    }

    #[test]
    fn saving_twice_keeps_a_backup_and_the_creation_stamp() {
        let dir = scratch("backup");
        let path = dir.join(format!("project.{EXTENSION}"));
        let scene = Scene::startup();
        let first = save(&path, &scene).expect("first save");
        let second = save(&path, &scene).expect("second save");
        assert_eq!(first.created, second.created);
        assert!(backup_path(&path).exists(), "the previous file must survive");
        assert!(!temp_path(&path).exists(), "the temp file must be gone");
        let project = load(&path).expect("load");
        assert_eq!(project.scene.len(), scene.len());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_newer_file_is_refused() {
        let scene = Scene::startup();
        let document = serde_json::to_value(SceneDocumentRef {
            schema_version: "9.0.0",
            units: "m",
            scene: &scene,
        })
        .expect("document");
        let entries = vec![
            ZipEntry::new(
                MANIFEST_ENTRY,
                serde_json::to_vec(&Manifest::new("9.0.0")).expect("manifest"),
            ),
            ZipEntry::new(SCENE_ENTRY, serde_json::to_vec(&document).expect("scene")),
        ];
        let bytes = zip::write(&entries).expect("zip");
        let error = from_bytes(&bytes).expect_err("must refuse");
        assert_eq!(error.code(), "IO_SCHEMA_TOO_NEW");
        assert!(error.recoverable());
    }

    #[test]
    fn a_missing_scene_entry_is_reported() {
        let bytes = zip::write(&[ZipEntry::new(
            MANIFEST_ENTRY,
            serde_json::to_vec(&Manifest::new(SCHEMA_VERSION)).expect("manifest"),
        )])
        .expect("zip");
        let error = from_bytes(&bytes).expect_err("must fail");
        assert_eq!(error.code(), "IO_ENTRY_MISSING");
    }

    #[test]
    fn a_damaged_container_is_reported() {
        let error = from_bytes(b"not a project at all").expect_err("must fail");
        assert_eq!(error.code(), "IO_CONTAINER_INVALID");
    }

    #[test]
    fn a_missing_file_is_a_filesystem_error() {
        let missing = scratch("missing").join(format!("nope.{EXTENSION}"));
        let error = load(&missing).expect_err("must fail");
        assert_eq!(error.code(), "IO_FS_FAILED");
        assert!(!error.recoverable());
    }
}
