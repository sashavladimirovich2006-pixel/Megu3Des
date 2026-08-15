//! Which file the session is bound to, and whether it has unsaved work.
//!
//! The layer is deliberately thin: `megu3d-io` owns the bytes, the history owns
//! the undo stack, and a [`Document`] only remembers the path, a revision
//! counter and the last save. Rotating autosave slices live here too (`D-43`).

use std::fs;
use std::path::{Path, PathBuf};

use megu3d_core::dto::DocumentStateDto;
use megu3d_core::scene::Scene;
use megu3d_io::IoError;

use crate::CmdError;

/// Name used until the document is written somewhere.
pub const UNTITLED: &str = "Untitled";

/// How many autosave slices survive a rotation (`D-43`).
pub const AUTOSAVE_KEEP: usize = 5;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Document {
    path: Option<PathBuf>,
    revision: u32,
    saved_revision: u32,
    saved_at: Option<String>,
    autosave_path: Option<PathBuf>,
}

impl Document {
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub fn revision(&self) -> u32 {
        self.revision
    }

    /// True when the scene moved past the revision that reached the disk.
    pub fn dirty(&self) -> bool {
        self.revision != self.saved_revision
    }

    /// One mutating dispatch. Saturating on purpose: a session left running for
    /// weeks stops counting instead of wrapping back to a clean-looking zero.
    pub fn touch(&mut self) {
        self.revision = self.revision.saturating_add(1);
    }

    /// Binds the document to `path` and marks the current revision as saved.
    pub fn bind(&mut self, path: PathBuf, saved_at: String) {
        self.path = Some(path);
        self.saved_revision = self.revision;
        self.saved_at = Some(saved_at);
    }

    /// Back to a never-saved document (File > New).
    pub fn reset(&mut self) {
        *self = Document::default();
    }

    pub fn note_autosave(&mut self, path: PathBuf) {
        self.autosave_path = Some(path);
    }

    /// File name without extension, or `Untitled` while the document is unbound.
    pub fn stem(&self) -> String {
        self.path
            .as_ref()
            .and_then(|path| path.file_stem())
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_else(|| UNTITLED.to_owned())
    }

    pub fn file_name(&self) -> Option<String> {
        self.path
            .as_ref()
            .and_then(|path| path.file_name())
            .map(|name| name.to_string_lossy().into_owned())
    }

    pub fn state(&self) -> DocumentStateDto {
        DocumentStateDto {
            path: self.path.as_ref().map(|path| path.display().to_string()),
            file_name: self.file_name(),
            dirty: self.dirty(),
            revision: self.revision,
            saved_at: self.saved_at.clone(),
            autosave_path: self
                .autosave_path
                .as_ref()
                .map(|path| path.display().to_string()),
        }
    }
}

/// Writes the scene and rebinds the document. A failed save leaves both the
/// previous binding and the dirty flag untouched.
pub fn save(
    document: &mut Document,
    scene: &Scene,
    path: &Path,
) -> Result<DocumentStateDto, CmdError> {
    let manifest = megu3d_io::save(path, scene)?;
    document.bind(path.to_path_buf(), manifest.modified);
    Ok(document.state())
}

/// Reads a project. The caller swaps its scene in only once this has succeeded.
pub fn load(path: &Path) -> Result<megu3d_io::Project, CmdError> {
    Ok(megu3d_io::load(path)?)
}

/// Writes a rotating autosave slice into `dir` and prunes the oldest ones.
/// Autosaving never clears the dirty flag: the real file is still behind.
pub fn autosave(
    document: &mut Document,
    scene: &Scene,
    dir: &Path,
) -> Result<DocumentStateDto, CmdError> {
    let stem = document.stem();
    let path = dir.join(format!(
        "{stem}-{revision:06}.{extension}",
        revision = document.revision(),
        extension = megu3d_io::EXTENSION
    ));
    megu3d_io::save(&path, scene)?;
    prune(dir, &stem, AUTOSAVE_KEEP)?;
    document.note_autosave(path);
    Ok(document.state())
}

/// Keeps the `keep` newest slices of `stem`. Slice names carry a zero-padded
/// revision, so lexicographic order is chronological order.
fn prune(dir: &Path, stem: &str, keep: usize) -> Result<(), CmdError> {
    let prefix = format!("{stem}-");
    let suffix = format!(".{}", megu3d_io::EXTENSION);
    let mut slices: Vec<PathBuf> = Vec::new();
    for entry in fs::read_dir(dir).map_err(|error| fs_error(dir, error))? {
        let entry = entry.map_err(|error| fs_error(dir, error))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with(&prefix) && name.ends_with(&suffix) {
            slices.push(entry.path());
        }
    }
    if slices.len() <= keep {
        return Ok(());
    }
    slices.sort();
    let doomed = slices.len() - keep;
    for path in slices.into_iter().take(doomed) {
        fs::remove_file(&path).map_err(|error| fs_error(&path, error))?;
    }
    Ok(())
}

fn fs_error(path: &Path, error: std::io::Error) -> CmdError {
    CmdError::from(IoError::Fs {
        path: path.display().to_string(),
        source: error.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dispatch::Session;
    use megu3d_core::dto::ErrorDto;

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("megu3d-doc-{tag}-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).expect("scratch dir");
        dir
    }

    #[test]
    fn a_fresh_document_is_clean_and_unbound() {
        let document = Document::default();
        assert!(!document.dirty());
        assert_eq!(document.path(), None);
        assert_eq!(document.stem(), UNTITLED);
        assert_eq!(document.state().file_name, None);
    }

    #[test]
    fn mutations_make_the_document_dirty() {
        let mut document = Document::default();
        document.touch();
        assert!(document.dirty());
        assert_eq!(document.revision(), 1);
    }

    #[test]
    fn saving_binds_the_path_and_clears_the_flag() {
        let dir = scratch("save");
        let path = dir.join("scene.megu3d");
        let mut document = Document::default();
        document.touch();
        let state = save(&mut document, &Scene::startup(), &path).expect("save");
        assert!(!state.dirty);
        assert_eq!(state.file_name.as_deref(), Some("scene.megu3d"));
        assert!(state.saved_at.is_some());
        assert_eq!(document.stem(), "scene");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_failed_save_keeps_the_document_dirty() {
        let dir = scratch("failed");
        let blocker = dir.join("blocker");
        fs::write(&blocker, b"not a directory").expect("blocker");
        let mut document = Document::default();
        document.touch();
        let error = save(&mut document, &Scene::startup(), &blocker.join("scene.megu3d"))
            .expect_err("must fail");
        assert_eq!(ErrorDto::from(error).code, "IO_FS_FAILED");
        assert!(document.dirty());
        assert_eq!(document.path(), None);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn autosave_keeps_only_the_newest_slices() {
        let dir = scratch("autosave");
        let scene = Scene::startup();
        let mut document = Document::default();
        for _ in 0..AUTOSAVE_KEEP + 3 {
            document.touch();
            autosave(&mut document, &scene, &dir).expect("autosave");
        }
        let slices = fs::read_dir(&dir)
            .expect("read dir")
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .ends_with(&format!(".{}", megu3d_io::EXTENSION))
            })
            .count();
        assert_eq!(slices, AUTOSAVE_KEEP);
        assert!(document.dirty(), "autosave must not pretend the file is saved");
        assert!(document.state().autosave_path.is_some());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_saved_session_reopens_with_the_same_scene() {
        let dir = scratch("session");
        let path = dir.join("project.megu3d");
        let mut session = Session::new();
        let saved = session.save(&path).expect("save");
        assert!(!saved.dirty);
        let mut other = Session::empty();
        let opened = other.open(&path).expect("open");
        assert!(!opened.dirty);
        assert_eq!(other.scene().len(), session.scene().len());
        assert_eq!(other.scene().selection_uuids(), session.scene().selection_uuids());
        assert!(!other.history().can_undo(), "opening resets the history");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn saving_a_never_saved_session_is_reported() {
        let error = Session::new().save_current().expect_err("must fail");
        let dto = ErrorDto::from(error);
        assert_eq!(dto.code, "DOC_NO_PATH");
        assert!(dto.recoverable);
    }

    #[test]
    fn a_new_document_forgets_the_previous_file() {
        let dir = scratch("new");
        let path = dir.join("project.megu3d");
        let mut session = Session::new();
        session.save(&path).expect("save");
        let state = session.new_document();
        assert_eq!(state.path, None);
        assert!(!state.dirty);
        assert_eq!(state.revision, 0);
        fs::remove_dir_all(&dir).ok();
    }
}
