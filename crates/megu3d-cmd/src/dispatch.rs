//! The session: scene plus history behind one dispatch call.
//!
//! Both the Tauri IPC layer and the headless tests go through
//! [`Session::dispatch`], so what CI exercises is exactly what the app runs.

use megu3d_core::dto::{
    CommandRequestDto, CommandResultDto, ScenePatch, SceneSnapshotDto, SelectionModeDto,
};
use megu3d_core::scene::{Scene, Transform};
use uuid::Uuid;

use crate::{
    unique_name, AddNode, CmdError, DeleteNodes, DuplicateNodes, History, RenameNode, Reparent,
    SetTransform, SetVisible, Transaction,
};

/// Longest node name the UI accepts; keeps the outliner readable and the
/// project file bounded.
pub const MAX_NAME_LENGTH: usize = 120;

#[derive(Debug)]
pub struct Session {
    scene: Scene,
    history: History,
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

impl Session {
    /// A session with the startup scene (camera, sun, cube).
    pub fn new() -> Self {
        Self {
            scene: Scene::startup(),
            history: History::default(),
        }
    }

    /// A session with an empty scene; used by tests and, later, by "New scene".
    pub fn empty() -> Self {
        Self {
            scene: Scene::new(),
            history: History::default(),
        }
    }

    pub fn scene(&self) -> &Scene {
        &self.scene
    }

    pub fn history(&self) -> &History {
        &self.history
    }

    pub fn snapshot(&self) -> SceneSnapshotDto {
        self.scene.snapshot(self.history.state())
    }

    /// Applies one intent and returns the patch plus a fresh snapshot.
    pub fn dispatch(&mut self, request: CommandRequestDto) -> Result<CommandResultDto, CmdError> {
        let patch = self.run(request)?;
        Ok(CommandResultDto {
            patch,
            scene: self.snapshot(),
            label: self.history.last_label(),
        })
    }

    fn run(&mut self, request: CommandRequestDto) -> Result<ScenePatch, CmdError> {
        match request {
            CommandRequestDto::Add { primitive, parent } => {
                let parent = match parent {
                    Some(value) => Some(parse_uuid(&value)?),
                    None => None,
                };
                let name = unique_name(&self.scene, primitive.label(), &[]);
                let label = format!("Add {}", primitive.label());
                let command = AddNode::new(primitive, name, parent);
                self.history
                    .commit(&mut self.scene, Transaction::single(label, Box::new(command)))
            }
            CommandRequestDto::Delete => {
                let nodes = self.selected_roots();
                if nodes.is_empty() {
                    return Err(CmdError::Invalid("nothing is selected".to_owned()));
                }
                self.history.commit(
                    &mut self.scene,
                    Transaction::single("Delete", Box::new(DeleteNodes::new(nodes))),
                )
            }
            CommandRequestDto::Duplicate => {
                let nodes = self.selected_roots();
                if nodes.is_empty() {
                    return Err(CmdError::Invalid("nothing is selected".to_owned()));
                }
                self.history.commit(
                    &mut self.scene,
                    Transaction::single("Duplicate", Box::new(DuplicateNodes::new(nodes))),
                )
            }
            CommandRequestDto::Rename { node, name } => {
                let uuid = parse_uuid(&node)?;
                let trimmed = name.trim();
                if trimmed.is_empty() {
                    return Err(CmdError::Invalid("name must not be empty".to_owned()));
                }
                if trimmed.chars().count() > MAX_NAME_LENGTH {
                    return Err(CmdError::Invalid(format!(
                        "name must be at most {MAX_NAME_LENGTH} characters"
                    )));
                }
                self.history.commit(
                    &mut self.scene,
                    Transaction::single("Rename", Box::new(RenameNode::new(uuid, trimmed))),
                )
            }
            CommandRequestDto::SetTransform {
                node,
                transform,
                preview,
            } => {
                let uuid = parse_uuid(&node)?;
                let transform = Transform::from_dto(&transform)?;
                let transaction = Transaction::single(
                    "Transform",
                    Box::new(SetTransform::new(uuid, transform)),
                );
                if preview {
                    self.history.preview(&mut self.scene, transaction)
                } else {
                    self.history.commit(&mut self.scene, transaction)
                }
            }
            CommandRequestDto::CommitPreview => self.history.commit_preview(),
            CommandRequestDto::CancelPreview => self.history.cancel_preview(&mut self.scene),
            CommandRequestDto::SetVisible { node, visible } => {
                let uuid = parse_uuid(&node)?;
                self.history.commit(
                    &mut self.scene,
                    Transaction::single(
                        "Visibility",
                        Box::new(SetVisible::new(uuid, visible)),
                    ),
                )
            }
            CommandRequestDto::Reparent { node, parent } => {
                let uuid = parse_uuid(&node)?;
                let parent = match parent {
                    Some(value) => Some(parse_uuid(&value)?),
                    None => None,
                };
                self.history.commit(
                    &mut self.scene,
                    Transaction::single("Reparent", Box::new(Reparent::new(uuid, parent, None))),
                )
            }
            CommandRequestDto::Select { nodes, mode } => {
                let uuids = nodes
                    .iter()
                    .map(|value| parse_uuid(value))
                    .collect::<Result<Vec<Uuid>, CmdError>>()?;
                let mut changed = self.scene.selection_uuids();
                self.scene.select(&uuids, mode);
                changed.extend(self.scene.selection_uuids());
                Ok(ScenePatch::nodes(changed))
            }
            CommandRequestDto::Undo => self.history.undo(&mut self.scene),
            CommandRequestDto::Redo => self.history.redo(&mut self.scene),
        }
    }

    fn selected_roots(&self) -> Vec<Uuid> {
        self.scene
            .top_level_selection()
            .iter()
            .filter_map(|id| self.scene.uuid_of(*id))
            .collect()
    }
}

fn parse_uuid(value: &str) -> Result<Uuid, CmdError> {
    Uuid::parse_str(value).map_err(|_| CmdError::BadUuid(value.to_owned()))
}

/// Convenience for the IPC layer and tests: the selection mode used when the
/// UI clicks a single node.
pub const CLICK_SELECTION: SelectionModeDto = SelectionModeDto::Replace;

#[cfg(test)]
mod tests {
    use super::*;
    use megu3d_core::dto::{PrimitiveKindDto, SceneNodeDto, TransformDto, Vec3Dto};

    fn find<'a>(snapshot: &'a SceneSnapshotDto, uuid: &str) -> &'a SceneNodeDto {
        snapshot
            .nodes
            .iter()
            .find(|node| node.uuid == uuid)
            .expect("node is in the snapshot")
    }

    fn selected(snapshot: &SceneSnapshotDto) -> String {
        snapshot
            .selection
            .first()
            .cloned()
            .expect("something is selected")
    }

    fn translation(x: f32) -> TransformDto {
        TransformDto {
            translation: Vec3Dto::new(x, 0.0, 0.0),
            rotation_euler_deg: Vec3Dto::default(),
            scale: Vec3Dto::new(1.0, 1.0, 1.0),
        }
    }

    #[test]
    fn adding_a_primitive_survives_undo_and_redo_with_the_same_uuid() {
        let mut session = Session::new();
        let before = session.snapshot();
        assert_eq!(before.nodes.len(), 3);
        let result = session
            .dispatch(CommandRequestDto::Add {
                primitive: PrimitiveKindDto::Sphere,
                parent: None,
            })
            .unwrap();
        assert!(result.patch.full_reload);
        assert_eq!(result.scene.nodes.len(), 4);
        assert_eq!(result.label, Some("Add Sphere".to_owned()));
        let sphere = selected(&result.scene);
        assert_eq!(find(&result.scene, &sphere).name, "Sphere");
        assert!(find(&result.scene, &sphere).mesh.is_some());
        assert!(result.scene.stats.triangle_count > before.stats.triangle_count);

        let undone = session.dispatch(CommandRequestDto::Undo).unwrap();
        assert_eq!(undone.scene.nodes.len(), 3);
        assert_eq!(undone.scene.selection, before.selection);
        assert!(!undone.scene.history.can_undo);

        let redone = session.dispatch(CommandRequestDto::Redo).unwrap();
        assert_eq!(redone.scene.nodes.len(), 4);
        assert_eq!(selected(&redone.scene), sphere, "redo keeps the identity");
        assert!(redone.scene.history.can_undo);
    }

    #[test]
    fn deleting_a_subtree_restores_name_parent_and_order() {
        let mut session = Session::new();
        let cube = selected(&session.snapshot());
        session
            .dispatch(CommandRequestDto::Add {
                primitive: PrimitiveKindDto::Empty,
                parent: Some(cube.clone()),
            })
            .unwrap();
        let child = selected(&session.snapshot());
        let before = session.snapshot();

        session
            .dispatch(CommandRequestDto::Select {
                nodes: vec![cube.clone(), child.clone()],
                mode: SelectionModeDto::Replace,
            })
            .unwrap();
        let deleted = session.dispatch(CommandRequestDto::Delete).unwrap();
        assert_eq!(deleted.scene.nodes.len(), 2, "the subtree goes as one");

        let restored = session.dispatch(CommandRequestDto::Undo).unwrap();
        assert_eq!(restored.scene.nodes.len(), before.nodes.len());
        assert_eq!(
            restored.scene.nodes.iter().map(|node| node.uuid.clone()).collect::<Vec<_>>(),
            before.nodes.iter().map(|node| node.uuid.clone()).collect::<Vec<_>>()
        );
        assert_eq!(find(&restored.scene, &child).parent, Some(cube));
        assert_eq!(find(&restored.scene, &child).depth, 1);
    }

    #[test]
    fn a_drag_streams_previews_and_commits_one_undo_step() {
        let mut session = Session::new();
        let cube = selected(&session.snapshot());
        for step in 1..=4 {
            let result = session
                .dispatch(CommandRequestDto::SetTransform {
                    node: cube.clone(),
                    transform: translation(step as f32 * 0.5),
                    preview: true,
                })
                .unwrap();
            assert!(result.scene.history.preview_active);
            assert_eq!(result.scene.history.undo_depth, 0, "previews stay out of history");
        }
        let committed = session.dispatch(CommandRequestDto::CommitPreview).unwrap();
        assert!(!committed.scene.history.preview_active);
        assert_eq!(committed.scene.history.undo_depth, 1);
        assert_eq!(find(&committed.scene, &cube).transform.translation.x, 2.0);

        let undone = session.dispatch(CommandRequestDto::Undo).unwrap();
        assert_eq!(find(&undone.scene, &cube).transform.translation.x, 0.0);
        let redone = session.dispatch(CommandRequestDto::Redo).unwrap();
        assert_eq!(find(&redone.scene, &cube).transform.translation.x, 2.0);
    }

    #[test]
    fn cancelling_a_drag_leaves_no_trace() {
        let mut session = Session::new();
        let cube = selected(&session.snapshot());
        session
            .dispatch(CommandRequestDto::SetTransform {
                node: cube.clone(),
                transform: translation(3.0),
                preview: true,
            })
            .unwrap();
        let cancelled = session.dispatch(CommandRequestDto::CancelPreview).unwrap();
        assert_eq!(find(&cancelled.scene, &cube).transform.translation.x, 0.0);
        assert_eq!(cancelled.scene.history.undo_depth, 0);
        assert!(!cancelled.scene.history.can_undo);
    }

    #[test]
    fn world_translation_follows_the_parent() {
        let mut session = Session::new();
        let cube = selected(&session.snapshot());
        session
            .dispatch(CommandRequestDto::SetTransform {
                node: cube.clone(),
                transform: translation(2.0),
                preview: false,
            })
            .unwrap();
        session
            .dispatch(CommandRequestDto::Add {
                primitive: PrimitiveKindDto::Cube,
                parent: Some(cube.clone()),
            })
            .unwrap();
        let child = selected(&session.snapshot());
        let result = session
            .dispatch(CommandRequestDto::SetTransform {
                node: child.clone(),
                transform: translation(1.0),
                preview: false,
            })
            .unwrap();
        let node = find(&result.scene, &child);
        assert_eq!(node.world_translation.x, 3.0);
        assert_eq!(node.world_bounds_min.x, 2.5);
        assert_eq!(node.world_bounds_max.x, 3.5);
    }

    #[test]
    fn duplicate_copies_the_subtree_with_fresh_names() {
        let mut session = Session::new();
        let cube = selected(&session.snapshot());
        session
            .dispatch(CommandRequestDto::Add {
                primitive: PrimitiveKindDto::Empty,
                parent: Some(cube.clone()),
            })
            .unwrap();
        session
            .dispatch(CommandRequestDto::Select {
                nodes: vec![cube.clone()],
                mode: SelectionModeDto::Replace,
            })
            .unwrap();
        let result = session.dispatch(CommandRequestDto::Duplicate).unwrap();
        assert_eq!(result.scene.nodes.len(), 6);
        let copy = selected(&result.scene);
        assert_ne!(copy, cube);
        assert_eq!(find(&result.scene, &copy).name, "Cube.001");
        assert_eq!(find(&result.scene, &copy).children.len(), 1);
        assert_eq!(result.scene.roots.len(), 4, "the copy sits next to its source");
        assert_eq!(result.scene.roots[3], copy);

        let undone = session.dispatch(CommandRequestDto::Undo).unwrap();
        assert_eq!(undone.scene.nodes.len(), 4);
        assert_eq!(undone.scene.selection, vec![cube]);
    }

    #[test]
    fn reparenting_under_a_descendant_is_rejected() {
        let mut session = Session::new();
        let cube = selected(&session.snapshot());
        session
            .dispatch(CommandRequestDto::Add {
                primitive: PrimitiveKindDto::Empty,
                parent: Some(cube.clone()),
            })
            .unwrap();
        let child = selected(&session.snapshot());
        let error = session
            .dispatch(CommandRequestDto::Reparent {
                node: cube.clone(),
                parent: Some(child.clone()),
            })
            .unwrap_err();
        assert!(matches!(
            error,
            CmdError::Scene(megu3d_core::scene::SceneError::Cycle)
        ));

        let moved = session
            .dispatch(CommandRequestDto::Reparent {
                node: child.clone(),
                parent: None,
            })
            .unwrap();
        assert_eq!(find(&moved.scene, &child).parent, None);
        let undone = session.dispatch(CommandRequestDto::Undo).unwrap();
        assert_eq!(find(&undone.scene, &child).parent, Some(cube));
    }

    #[test]
    fn selection_and_visibility_behave_as_specified() {
        let mut session = Session::new();
        let snapshot = session.snapshot();
        let cube = selected(&snapshot);
        let sun = snapshot
            .nodes
            .iter()
            .find(|node| node.name == "Sun")
            .map(|node| node.uuid.clone())
            .unwrap();

        let result = session
            .dispatch(CommandRequestDto::Select {
                nodes: vec![sun.clone()],
                mode: SelectionModeDto::Add,
            })
            .unwrap();
        assert_eq!(result.scene.selection.len(), 2);
        assert_eq!(result.scene.active, Some(sun.clone()));
        assert_eq!(
            result.scene.history.undo_depth, 0,
            "selection is not undoable"
        );

        let hidden = session
            .dispatch(CommandRequestDto::SetVisible {
                node: cube.clone(),
                visible: false,
            })
            .unwrap();
        assert!(!find(&hidden.scene, &cube).visible);
        let shown = session.dispatch(CommandRequestDto::Undo).unwrap();
        assert!(find(&shown.scene, &cube).visible);

        let cleared = session
            .dispatch(CommandRequestDto::Select {
                nodes: Vec::new(),
                mode: SelectionModeDto::Clear,
            })
            .unwrap();
        assert!(cleared.scene.selection.is_empty());
        assert_eq!(cleared.scene.active, None);
        assert!(session.dispatch(CommandRequestDto::Delete).is_err());
    }

    #[test]
    fn invalid_payloads_are_rejected_with_typed_errors() {
        let mut session = Session::new();
        let cube = selected(&session.snapshot());
        assert!(matches!(
            session
                .dispatch(CommandRequestDto::Rename {
                    node: "not-a-uuid".to_owned(),
                    name: "Body".to_owned(),
                })
                .unwrap_err(),
            CmdError::BadUuid(_)
        ));
        assert!(matches!(
            session
                .dispatch(CommandRequestDto::Rename {
                    node: cube.clone(),
                    name: "   ".to_owned(),
                })
                .unwrap_err(),
            CmdError::Invalid(_)
        ));
        let mut zero = translation(0.0);
        zero.scale = Vec3Dto::new(0.0, 1.0, 1.0);
        assert!(session
            .dispatch(CommandRequestDto::SetTransform {
                node: cube.clone(),
                transform: zero,
                preview: false,
            })
            .is_err());
        assert!(matches!(
            session.dispatch(CommandRequestDto::Undo).unwrap_err(),
            CmdError::NothingToUndo
        ));
        assert_eq!(session.snapshot().nodes.len(), 3, "failures change nothing");
    }

    #[test]
    fn renaming_keeps_history_labels_for_the_ui() {
        let mut session = Session::new();
        let cube = selected(&session.snapshot());
        let result = session
            .dispatch(CommandRequestDto::Rename {
                node: cube.clone(),
                name: "  Body  ".to_owned(),
            })
            .unwrap();
        assert_eq!(find(&result.scene, &cube).name, "Body");
        assert_eq!(result.scene.history.last_label, Some("Rename".to_owned()));
        assert_eq!(result.scene.history.undo_limit, crate::UNDO_LIMIT as u32);
    }
}
