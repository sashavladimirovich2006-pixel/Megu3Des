//! Undoable commands: the only writers of scene state.
//!
//! Every mutation is a [`Command`] that can invert itself, grouped into a
//! [`Transaction`] and pushed onto [`History`] (`D-40`..`D-42`). Commands
//! address nodes by [`Uuid`], never by slotmap key, so an undo that recreates a
//! deleted node keeps the identity later steps refer to.

pub mod dispatch;

use megu3d_core::dto::{
    ErrorDto, HistoryStateDto, PrimitiveKindDto, ScenePatch, SelectionModeDto,
};
use megu3d_core::scene::{
    CameraData, LightData, LightKind, NodeData, Scene, SceneError, StoredNode, Transform,
};
use thiserror::Error;
use uuid::Uuid;

/// Undo depth (`D-42`). Older steps are dropped from the bottom.
pub const UNDO_LIMIT: usize = 64;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CmdError {
    #[error("node not found")]
    NodeNotFound,
    #[error("{0}")]
    Invalid(String),
    #[error(transparent)]
    Scene(#[from] SceneError),
    #[error("nothing to undo")]
    NothingToUndo,
    #[error("nothing to redo")]
    NothingToRedo,
    #[error("no preview is active")]
    NoPreview,
    #[error("not a valid uuid: {0}")]
    BadUuid(String),
    #[error("the core lock was poisoned")]
    LockPoisoned,
}

impl From<CmdError> for ErrorDto {
    fn from(error: CmdError) -> Self {
        let message = error.to_string();
        match error {
            CmdError::NodeNotFound | CmdError::Scene(SceneError::NodeNotFound) => {
                ErrorDto::new("SCENE_NODE_NOT_FOUND", message)
            }
            CmdError::Scene(SceneError::MeshNotFound) => {
                ErrorDto::new("SCENE_MESH_NOT_FOUND", message)
            }
            CmdError::Scene(SceneError::Cycle) => ErrorDto::new("SCENE_HIERARCHY_CYCLE", message),
            CmdError::Invalid(_) | CmdError::Scene(SceneError::Invalid(_)) => {
                ErrorDto::new("SCENE_INVALID_ARGUMENT", message)
            }
            CmdError::NothingToUndo => ErrorDto::new("HISTORY_NOTHING_TO_UNDO", message),
            CmdError::NothingToRedo => ErrorDto::new("HISTORY_NOTHING_TO_REDO", message),
            CmdError::NoPreview => ErrorDto::new("HISTORY_NO_PREVIEW", message),
            CmdError::BadUuid(_) => ErrorDto::new("SCENE_INVALID_UUID", message),
            CmdError::LockPoisoned => ErrorDto::fatal("CORE_LOCK_POISONED", message),
        }
    }
}

/// A single reversible mutation.
///
/// `apply` takes `&mut self` so a command can record what it needs to undo
/// itself (the previous name, the detached subtree, the old parent). `invert`
/// is therefore only meaningful after `apply` has run.
/// `Send` is required because the session lives in Tauri managed state.
pub trait Command: std::fmt::Debug + Send {
    fn label(&self) -> &'static str;
    fn apply(&mut self, scene: &mut Scene) -> Result<ScenePatch, CmdError>;
    fn invert(&self) -> Box<dyn Command>;
}

/// One user-visible undo step: commands applied in order, inverted in reverse.
#[derive(Debug)]
pub struct Transaction {
    pub label: String,
    pub commands: Vec<Box<dyn Command>>,
}

impl Transaction {
    pub fn new(label: impl Into<String>, commands: Vec<Box<dyn Command>>) -> Self {
        Self {
            label: label.into(),
            commands,
        }
    }

    pub fn single(label: impl Into<String>, command: Box<dyn Command>) -> Self {
        Self::new(label, vec![command])
    }

    pub fn apply(&mut self, scene: &mut Scene) -> Result<ScenePatch, CmdError> {
        let mut patch = ScenePatch::default();
        for command in &mut self.commands {
            patch.merge(command.apply(scene)?);
        }
        Ok(patch)
    }

    pub fn inverted(&self) -> Transaction {
        Transaction {
            label: self.label.clone(),
            commands: self
                .commands
                .iter()
                .rev()
                .map(|command| command.invert())
                .collect(),
        }
    }
}

/// Undo/redo stacks plus the live drag preview.
///
/// The stacks hold *inverse* transactions: undoing means applying the top of
/// `undo` and pushing its inverse onto `redo`.
#[derive(Debug, Default)]
pub struct History {
    undo: Vec<Transaction>,
    redo: Vec<Transaction>,
    /// Restores the state from before the current drag started.
    preview: Option<Transaction>,
}

impl History {
    pub fn commit(
        &mut self,
        scene: &mut Scene,
        mut transaction: Transaction,
    ) -> Result<ScenePatch, CmdError> {
        let mut patch = ScenePatch::default();
        if self.preview.is_some() {
            // A committed command while dragging: drop the uncommitted preview
            // so history never contains half a gesture.
            patch.merge(self.cancel_preview(scene)?);
        }
        patch.merge(transaction.apply(scene)?);
        self.redo.clear();
        self.undo.push(transaction.inverted());
        if self.undo.len() > UNDO_LIMIT {
            self.undo.remove(0);
        }
        Ok(patch)
    }

    /// Applies a gesture without touching the stacks. Preview transforms are
    /// absolute, so streaming many previews in a row is safe: the stored
    /// inverse always restores the pre-drag state.
    pub fn preview(
        &mut self,
        scene: &mut Scene,
        mut transaction: Transaction,
    ) -> Result<ScenePatch, CmdError> {
        let patch = transaction.apply(scene)?;
        if self.preview.is_none() {
            self.preview = Some(transaction.inverted());
        }
        Ok(patch)
    }

    /// Turns the current preview into a real undo step. The scene is already in
    /// its final state, so nothing is re-applied.
    pub fn commit_preview(&mut self) -> Result<ScenePatch, CmdError> {
        let inverse = self.preview.take().ok_or(CmdError::NoPreview)?;
        self.redo.clear();
        self.undo.push(inverse);
        if self.undo.len() > UNDO_LIMIT {
            self.undo.remove(0);
        }
        Ok(ScenePatch::default())
    }

    pub fn cancel_preview(&mut self, scene: &mut Scene) -> Result<ScenePatch, CmdError> {
        let mut inverse = self.preview.take().ok_or(CmdError::NoPreview)?;
        inverse.apply(scene)
    }

    pub fn undo(&mut self, scene: &mut Scene) -> Result<ScenePatch, CmdError> {
        let mut patch = ScenePatch::default();
        if self.preview.is_some() {
            patch.merge(self.cancel_preview(scene)?);
        }
        let mut transaction = self.undo.pop().ok_or(CmdError::NothingToUndo)?;
        patch.merge(transaction.apply(scene)?);
        self.redo.push(transaction.inverted());
        Ok(patch)
    }

    pub fn redo(&mut self, scene: &mut Scene) -> Result<ScenePatch, CmdError> {
        let mut patch = ScenePatch::default();
        if self.preview.is_some() {
            patch.merge(self.cancel_preview(scene)?);
        }
        let mut transaction = self.redo.pop().ok_or(CmdError::NothingToRedo)?;
        patch.merge(transaction.apply(scene)?);
        self.undo.push(transaction.inverted());
        Ok(patch)
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn depth(&self) -> usize {
        self.undo.len()
    }

    pub fn preview_active(&self) -> bool {
        self.preview.is_some()
    }

    pub fn last_label(&self) -> Option<String> {
        self.undo.last().map(|entry| entry.label.clone())
    }

    pub fn state(&self) -> HistoryStateDto {
        HistoryStateDto {
            can_undo: self.can_undo(),
            can_redo: self.can_redo(),
            undo_depth: self.undo.len() as u32,
            undo_limit: UNDO_LIMIT as u32,
            last_label: self.last_label(),
            preview_active: self.preview.is_some(),
        }
    }
}

/// Adds a primitive, empty, camera or light and selects it.
#[derive(Debug)]
pub struct AddNode {
    uuid: Uuid,
    primitive: PrimitiveKindDto,
    name: String,
    parent: Option<Uuid>,
    transform: Option<Transform>,
    mesh: Option<megu3d_core::scene::MeshId>,
    selection_before: Vec<Uuid>,
}

impl AddNode {
    pub fn new(primitive: PrimitiveKindDto, name: impl Into<String>, parent: Option<Uuid>) -> Self {
        Self {
            uuid: Uuid::new_v4(),
            primitive,
            name: name.into(),
            parent,
            transform: None,
            mesh: None,
            selection_before: Vec::new(),
        }
    }

    pub fn with_transform(mut self, transform: Transform) -> Self {
        self.transform = Some(transform);
        self
    }

    pub fn uuid(&self) -> Uuid {
        self.uuid
    }
}

impl Command for AddNode {
    fn label(&self) -> &'static str {
        "Add node"
    }

    fn apply(&mut self, scene: &mut Scene) -> Result<ScenePatch, CmdError> {
        self.selection_before = scene.selection_uuids();
        let parent = match self.parent {
            Some(uuid) => Some(scene.resolve(uuid)?),
            None => None,
        };
        let data = match self.primitive.mesh() {
            Some(primitive) => {
                // Redo reuses the mesh inserted by the first apply, so the
                // scene does not accumulate a copy per undo cycle.
                let mesh = match self.mesh {
                    Some(mesh) => mesh,
                    None => {
                        let mesh = scene.insert_mesh(primitive.label(), primitive.build());
                        self.mesh = Some(mesh);
                        mesh
                    }
                };
                NodeData::Mesh(mesh)
            }
            None => match self.primitive {
                PrimitiveKindDto::Camera => NodeData::Camera(CameraData::default()),
                PrimitiveKindDto::Light => NodeData::Light(LightData {
                    kind: LightKind::Point,
                    color: [1.0, 1.0, 1.0],
                    intensity: 100.0,
                }),
                _ => NodeData::Empty,
            },
        };
        let id = scene.add_node_with_uuid(self.uuid, self.name.clone(), parent, data)?;
        if let Some(transform) = self.transform {
            if let Some(node) = scene.node_mut(id) {
                node.transform = transform;
            }
        }
        scene.select(&[self.uuid], SelectionModeDto::Replace);
        Ok(ScenePatch::structural([self.uuid]))
    }

    fn invert(&self) -> Box<dyn Command> {
        Box::new(DeleteNodes::with_selection(
            vec![self.uuid],
            self.selection_before.clone(),
        ))
    }
}

/// Deletes whole subtrees and remembers them for undo.
#[derive(Debug)]
pub struct DeleteNodes {
    nodes: Vec<Uuid>,
    restore_selection: Option<Vec<Uuid>>,
    groups: Vec<Vec<StoredNode>>,
    selection_before: Vec<Uuid>,
}

impl DeleteNodes {
    pub fn new(nodes: Vec<Uuid>) -> Self {
        Self {
            nodes,
            restore_selection: None,
            groups: Vec::new(),
            selection_before: Vec::new(),
        }
    }

    /// Used when undoing an add or duplicate: after removing the nodes the
    /// selection goes back to what it was before the gesture.
    pub fn with_selection(nodes: Vec<Uuid>, selection: Vec<Uuid>) -> Self {
        Self {
            nodes,
            restore_selection: Some(selection),
            groups: Vec::new(),
            selection_before: Vec::new(),
        }
    }
}

impl Command for DeleteNodes {
    fn label(&self) -> &'static str {
        "Delete"
    }

    fn apply(&mut self, scene: &mut Scene) -> Result<ScenePatch, CmdError> {
        self.selection_before = scene.selection_uuids();
        let mut groups = Vec::new();
        let mut changed = Vec::new();
        for uuid in &self.nodes {
            let Ok(id) = scene.resolve(*uuid) else {
                continue;
            };
            groups.push(scene.detach_subtree(id));
            changed.push(*uuid);
        }
        if changed.is_empty() {
            return Err(CmdError::NodeNotFound);
        }
        self.groups = groups;
        if let Some(selection) = &self.restore_selection {
            scene.select(selection, SelectionModeDto::Replace);
        }
        Ok(ScenePatch::structural(changed))
    }

    fn invert(&self) -> Box<dyn Command> {
        Box::new(RestoreNodes::new(
            self.groups.clone(),
            self.selection_before.clone(),
        ))
    }
}

/// Puts detached subtrees back with their original uuids, order and hierarchy.
#[derive(Debug)]
pub struct RestoreNodes {
    groups: Vec<Vec<StoredNode>>,
    selection: Vec<Uuid>,
}

impl RestoreNodes {
    pub fn new(groups: Vec<Vec<StoredNode>>, selection: Vec<Uuid>) -> Self {
        Self { groups, selection }
    }
}

impl Command for RestoreNodes {
    fn label(&self) -> &'static str {
        "Restore"
    }

    fn apply(&mut self, scene: &mut Scene) -> Result<ScenePatch, CmdError> {
        let mut changed = Vec::new();
        for group in &self.groups {
            changed.extend(scene.restore_subtree(group)?);
        }
        scene.select(&self.selection, SelectionModeDto::Replace);
        Ok(ScenePatch::structural(changed))
    }

    fn invert(&self) -> Box<dyn Command> {
        let roots = self
            .groups
            .iter()
            .filter_map(|group| group.first())
            .map(|entry| entry.uuid)
            .collect();
        Box::new(DeleteNodes::new(roots))
    }
}

/// Copies subtrees under the same parent with fresh uuids.
///
/// Mesh data is shared with the source in M3: independent geometry per copy
/// arrives with Edit Mode in M5 (`docs/assumptions.md`, `D-83`).
#[derive(Debug)]
pub struct DuplicateNodes {
    sources: Vec<Uuid>,
    groups: Vec<Vec<StoredNode>>,
    selection_before: Vec<Uuid>,
}

impl DuplicateNodes {
    pub fn new(sources: Vec<Uuid>) -> Self {
        Self {
            sources,
            groups: Vec::new(),
            selection_before: Vec::new(),
        }
    }
}

impl Command for DuplicateNodes {
    fn label(&self) -> &'static str {
        "Duplicate"
    }

    fn apply(&mut self, scene: &mut Scene) -> Result<ScenePatch, CmdError> {
        self.selection_before = scene.selection_uuids();
        if self.groups.is_empty() {
            let mut taken: Vec<String> = Vec::new();
            let mut groups = Vec::new();
            for uuid in &self.sources {
                let id = scene.resolve(*uuid)?;
                let source = scene.export_subtree(id);
                groups.push(remap_subtree(scene, &source, &mut taken));
            }
            self.groups = groups;
        }
        let mut changed = Vec::new();
        for group in &self.groups {
            changed.extend(scene.restore_subtree(group)?);
        }
        scene.select(&changed, SelectionModeDto::Replace);
        Ok(ScenePatch::structural(changed))
    }

    fn invert(&self) -> Box<dyn Command> {
        let roots = self
            .groups
            .iter()
            .filter_map(|group| group.first())
            .map(|entry| entry.uuid)
            .collect();
        Box::new(DeleteNodes::with_selection(
            roots,
            self.selection_before.clone(),
        ))
    }
}

#[derive(Debug)]
pub struct RenameNode {
    node: Uuid,
    name: String,
    previous: Option<String>,
}

impl RenameNode {
    pub fn new(node: Uuid, name: impl Into<String>) -> Self {
        Self {
            node,
            name: name.into(),
            previous: None,
        }
    }
}

impl Command for RenameNode {
    fn label(&self) -> &'static str {
        "Rename"
    }

    fn apply(&mut self, scene: &mut Scene) -> Result<ScenePatch, CmdError> {
        let id = scene.resolve(self.node)?;
        let node = scene.node_mut(id).ok_or(CmdError::NodeNotFound)?;
        self.previous = Some(node.name.clone());
        node.name = self.name.clone();
        Ok(ScenePatch::structural([self.node]))
    }

    fn invert(&self) -> Box<dyn Command> {
        Box::new(RenameNode::new(
            self.node,
            self.previous.clone().unwrap_or_default(),
        ))
    }
}

#[derive(Debug)]
pub struct SetTransform {
    node: Uuid,
    transform: Transform,
    previous: Option<Transform>,
}

impl SetTransform {
    pub fn new(node: Uuid, transform: Transform) -> Self {
        Self {
            node,
            transform,
            previous: None,
        }
    }
}

impl Command for SetTransform {
    fn label(&self) -> &'static str {
        "Transform"
    }

    fn apply(&mut self, scene: &mut Scene) -> Result<ScenePatch, CmdError> {
        let id = scene.resolve(self.node)?;
        let node = scene.node_mut(id).ok_or(CmdError::NodeNotFound)?;
        self.previous = Some(node.transform);
        node.transform = self.transform;
        Ok(ScenePatch::nodes([self.node]))
    }

    fn invert(&self) -> Box<dyn Command> {
        Box::new(SetTransform::new(
            self.node,
            self.previous.unwrap_or_default(),
        ))
    }
}

#[derive(Debug)]
pub struct SetVisible {
    node: Uuid,
    visible: bool,
    previous: Option<bool>,
}

impl SetVisible {
    pub fn new(node: Uuid, visible: bool) -> Self {
        Self {
            node,
            visible,
            previous: None,
        }
    }
}

impl Command for SetVisible {
    fn label(&self) -> &'static str {
        "Visibility"
    }

    fn apply(&mut self, scene: &mut Scene) -> Result<ScenePatch, CmdError> {
        let id = scene.resolve(self.node)?;
        let node = scene.node_mut(id).ok_or(CmdError::NodeNotFound)?;
        self.previous = Some(node.visible);
        node.visible = self.visible;
        Ok(ScenePatch::nodes([self.node]))
    }

    fn invert(&self) -> Box<dyn Command> {
        Box::new(SetVisible::new(
            self.node,
            self.previous.unwrap_or(!self.visible),
        ))
    }
}

#[derive(Debug)]
pub struct Reparent {
    node: Uuid,
    parent: Option<Uuid>,
    index: Option<usize>,
    previous: Option<(Option<Uuid>, usize)>,
}

impl Reparent {
    pub fn new(node: Uuid, parent: Option<Uuid>, index: Option<usize>) -> Self {
        Self {
            node,
            parent,
            index,
            previous: None,
        }
    }
}

impl Command for Reparent {
    fn label(&self) -> &'static str {
        "Reparent"
    }

    fn apply(&mut self, scene: &mut Scene) -> Result<ScenePatch, CmdError> {
        let id = scene.resolve(self.node)?;
        let parent = match self.parent {
            Some(uuid) => Some(scene.resolve(uuid)?),
            None => None,
        };
        self.previous = Some(scene.reparent(id, parent, self.index)?);
        Ok(ScenePatch::structural([self.node]))
    }

    fn invert(&self) -> Box<dyn Command> {
        let (parent, index) = self.previous.unwrap_or((None, 0));
        Box::new(Reparent::new(self.node, parent, Some(index)))
    }
}

/// `"Cube.001"` -> `"Cube"`, so duplicating a copy does not stack suffixes.
pub fn name_stem(name: &str) -> &str {
    match name.rsplit_once('.') {
        Some((stem, suffix))
            if suffix.len() == 3 && suffix.chars().all(|value| value.is_ascii_digit()) =>
        {
            stem
        }
        _ => name,
    }
}

/// First free `Name`, `Name.001`, `Name.002`, ... considering both the scene and
/// names reserved earlier in the same command.
pub fn unique_name(scene: &Scene, base: &str, taken: &[String]) -> String {
    let stem = name_stem(base);
    let free = |candidate: &str| !scene.has_name(candidate) && !taken.iter().any(|name| name == candidate);
    if free(stem) {
        return stem.to_owned();
    }
    for index in 1..1000u32 {
        let candidate = format!("{stem}.{index:03}");
        if free(&candidate) {
            return candidate;
        }
    }
    format!("{stem}.{}", Uuid::new_v4().simple())
}

/// Rebuilds a detached subtree with fresh uuids and names, keeping the internal
/// hierarchy and placing the copy right after its source.
fn remap_subtree(scene: &Scene, source: &[StoredNode], taken: &mut Vec<String>) -> Vec<StoredNode> {
    let mut mapping: std::collections::HashMap<Uuid, Uuid> = std::collections::HashMap::new();
    for entry in source {
        mapping.insert(entry.uuid, Uuid::new_v4());
    }
    source
        .iter()
        .enumerate()
        .map(|(position, entry)| {
            let name = unique_name(scene, &entry.name, taken);
            taken.push(name.clone());
            let parent = match entry.parent {
                Some(parent) => mapping.get(&parent).copied().or(Some(parent)),
                None => None,
            };
            StoredNode {
                uuid: mapping.get(&entry.uuid).copied().unwrap_or(entry.uuid),
                parent,
                // The copy of the root lands next to its source; children keep
                // their own order inside the copy.
                index: if position == 0 {
                    entry.index + 1
                } else {
                    entry.index
                },
                name,
                transform: entry.transform,
                visible: entry.visible,
                locked: entry.locked,
                data: entry.data.clone(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use megu3d_core::Primitive;

    fn scene_with_cube() -> (Scene, Uuid) {
        let mut scene = Scene::new();
        let mesh = scene.insert_mesh("Cube", Primitive::Cube.build());
        let id = scene.add_node("Cube", None, NodeData::Mesh(mesh));
        let uuid = scene.uuid_of(id).unwrap();
        (scene, uuid)
    }

    #[test]
    fn history_is_capped_at_the_undo_limit() {
        let (mut scene, cube) = scene_with_cube();
        let mut history = History::default();
        for index in 0..(UNDO_LIMIT + 6) {
            let command = RenameNode::new(cube, format!("Cube {index}"));
            history
                .commit(&mut scene, Transaction::single("Rename", Box::new(command)))
                .unwrap();
        }
        assert_eq!(history.depth(), UNDO_LIMIT);
        assert!(history.can_undo());
        assert!(!history.can_redo());
        assert_eq!(history.state().undo_limit, UNDO_LIMIT as u32);
    }

    #[test]
    fn rename_round_trips_through_undo_and_redo() {
        let (mut scene, cube) = scene_with_cube();
        let mut history = History::default();
        history
            .commit(
                &mut scene,
                Transaction::single("Rename", Box::new(RenameNode::new(cube, "Body"))),
            )
            .unwrap();
        let id = scene.resolve(cube).unwrap();
        assert_eq!(scene.node(id).map(|node| node.name.clone()), Some("Body".to_owned()));
        history.undo(&mut scene).unwrap();
        assert_eq!(scene.node(id).map(|node| node.name.clone()), Some("Cube".to_owned()));
        history.redo(&mut scene).unwrap();
        assert_eq!(scene.node(id).map(|node| node.name.clone()), Some("Body".to_owned()));
        assert_eq!(history.last_label(), Some("Rename".to_owned()));
    }

    #[test]
    fn empty_stacks_report_errors_instead_of_panicking() {
        let (mut scene, _) = scene_with_cube();
        let mut history = History::default();
        assert_eq!(history.undo(&mut scene), Err(CmdError::NothingToUndo));
        assert_eq!(history.redo(&mut scene), Err(CmdError::NothingToRedo));
        assert_eq!(history.commit_preview(), Err(CmdError::NoPreview));
        assert_eq!(history.cancel_preview(&mut scene), Err(CmdError::NoPreview));
    }

    #[test]
    fn names_are_deduplicated_without_stacking_suffixes() {
        let (mut scene, _) = scene_with_cube();
        assert_eq!(unique_name(&scene, "Cube", &[]), "Cube.001");
        assert_eq!(unique_name(&scene, "Cube.001", &[]), "Cube.001");
        assert_eq!(
            unique_name(&scene, "Cube", &["Cube.001".to_owned()]),
            "Cube.002"
        );
        assert_eq!(unique_name(&scene, "Sphere", &[]), "Sphere");
        assert_eq!(name_stem("Cube.001"), "Cube");
        assert_eq!(name_stem("Cube.mesh"), "Cube.mesh");
        scene.add_node("Cube.001", None, NodeData::Empty);
        assert_eq!(unique_name(&scene, "Cube", &[]), "Cube.002");
    }

    #[test]
    fn errors_map_to_stable_wire_codes() {
        let error: ErrorDto = CmdError::Scene(SceneError::Cycle).into();
        assert_eq!(error.code, "SCENE_HIERARCHY_CYCLE");
        assert!(error.recoverable);
        let fatal: ErrorDto = CmdError::LockPoisoned.into();
        assert_eq!(fatal.code, "CORE_LOCK_POISONED");
        assert!(!fatal.recoverable);
        let missing: ErrorDto = CmdError::NodeNotFound.into();
        assert_eq!(missing.code, "SCENE_NODE_NOT_FOUND");
    }
}
