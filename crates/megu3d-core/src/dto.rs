//! Wire types for the `megu3d.*` IPC surface.
//!
//! Rust is the source of truth: every type here is exported to
//! `packages/types/src/generated/` by ts-rs (`pnpm ipc:types`), so the UI never
//! hand-writes a payload shape. The `Dto` suffix marks the IPC boundary; scene
//! internals (quaternions, slotmap keys) never cross it.

use std::collections::BTreeMap;

use megu3d_mesh::Primitive;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::math::Vec3;
use crate::SCHEMA_VERSION;

/// Application metadata, exposed as `megu3d.query.appInfo`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../packages/types/src/generated/")]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub schema_version: String,
    pub build_profile: String,
}

impl AppInfo {
    pub fn current() -> Self {
        Self {
            name: "Megu3D".to_owned(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
            schema_version: SCHEMA_VERSION.to_owned(),
            build_profile: if cfg!(debug_assertions) {
                "debug".to_owned()
            } else {
                "release".to_owned()
            },
        }
    }
}

/// Lightweight scene summary for the status bar, exposed as `megu3d.query.sceneStats`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../packages/types/src/generated/")]
pub struct SceneStats {
    pub node_count: u32,
    pub selection_count: u32,
    pub triangle_count: u32,
    pub schema_version: String,
}

/// What changed after a command was applied; the `megu3d.event.scenePatch` payload.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../packages/types/src/generated/")]
pub struct ScenePatch {
    pub changed_nodes: Vec<String>,
    pub full_reload: bool,
}

impl ScenePatch {
    pub fn nodes(uuids: impl IntoIterator<Item = Uuid>) -> Self {
        Self {
            changed_nodes: uuids.into_iter().map(|id| id.to_string()).collect(),
            full_reload: false,
        }
    }

    /// Structural change: the UI rebuilds its tree from the snapshot.
    pub fn structural(uuids: impl IntoIterator<Item = Uuid>) -> Self {
        Self {
            changed_nodes: uuids.into_iter().map(|id| id.to_string()).collect(),
            full_reload: true,
        }
    }

    pub fn merge(&mut self, other: ScenePatch) {
        self.changed_nodes.extend(other.changed_nodes);
        self.full_reload |= other.full_reload;
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../packages/types/src/generated/")]
pub struct Vec3Dto {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3Dto {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn from_array(value: Vec3) -> Self {
        Self {
            x: value[0],
            y: value[1],
            z: value[2],
        }
    }

    pub fn to_array(self) -> Vec3 {
        [self.x, self.y, self.z]
    }
}

/// Transform as the UI edits it: metres and degrees (`D-32`). The scene stores
/// the rotation as a quaternion and converts on the boundary.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../packages/types/src/generated/")]
pub struct TransformDto {
    pub translation: Vec3Dto,
    pub rotation_euler_deg: Vec3Dto,
    pub scale: Vec3Dto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../packages/types/src/generated/")]
pub enum NodeKindDto {
    Empty,
    Mesh,
    Camera,
    Light,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../packages/types/src/generated/")]
pub struct MeshInfoDto {
    pub name: String,
    pub vertex_count: u32,
    pub triangle_count: u32,
    pub bounds_min: Vec3Dto,
    pub bounds_max: Vec3Dto,
}

/// One scene node, flattened in outliner order with its `depth`, so the UI can
/// render the hierarchy without recursion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../packages/types/src/generated/")]
pub struct SceneNodeDto {
    pub uuid: String,
    pub name: String,
    pub parent: Option<String>,
    pub children: Vec<String>,
    pub depth: u32,
    pub kind: NodeKindDto,
    pub transform: TransformDto,
    pub world_translation: Vec3Dto,
    /// World-space bounds; a zero-size box for empties, cameras and lights.
    pub world_bounds_min: Vec3Dto,
    pub world_bounds_max: Vec3Dto,
    pub mesh: Option<MeshInfoDto>,
    pub visible: bool,
    pub locked: bool,
    pub selected: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../packages/types/src/generated/")]
pub struct HistoryStateDto {
    pub can_undo: bool,
    pub can_redo: bool,
    pub undo_depth: u32,
    pub undo_limit: u32,
    pub last_label: Option<String>,
    /// True while a drag is previewing an uncommitted transform.
    pub preview_active: bool,
}

/// Everything the UI needs to draw the scene. M3 returns a full snapshot after
/// every command: correctness first, incremental patching lands with the
/// renderer once scenes are big enough to need it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../packages/types/src/generated/")]
pub struct SceneSnapshotDto {
    pub schema_version: String,
    pub nodes: Vec<SceneNodeDto>,
    pub roots: Vec<String>,
    pub selection: Vec<String>,
    pub active: Option<String>,
    pub stats: SceneStats,
    pub history: HistoryStateDto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../packages/types/src/generated/")]
pub enum PrimitiveKindDto {
    Plane,
    Cube,
    Sphere,
    Cylinder,
    Cone,
    Torus,
    Empty,
    Camera,
    Light,
}

impl PrimitiveKindDto {
    /// `None` for non-mesh nodes (empty, camera, light).
    pub fn mesh(self) -> Option<Primitive> {
        match self {
            PrimitiveKindDto::Plane => Some(Primitive::Plane),
            PrimitiveKindDto::Cube => Some(Primitive::Cube),
            PrimitiveKindDto::Sphere => Some(Primitive::Sphere),
            PrimitiveKindDto::Cylinder => Some(Primitive::Cylinder),
            PrimitiveKindDto::Cone => Some(Primitive::Cone),
            PrimitiveKindDto::Torus => Some(Primitive::Torus),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self.mesh() {
            Some(primitive) => primitive.label(),
            None => match self {
                PrimitiveKindDto::Camera => "Camera",
                PrimitiveKindDto::Light => "Light",
                _ => "Empty",
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../packages/types/src/generated/")]
pub enum SelectionModeDto {
    Replace,
    Add,
    Toggle,
    Clear,
}

/// The single mutation entry point: `megu3d.cmd.dispatch`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(export, export_to = "../../packages/types/src/generated/")]
pub enum CommandRequestDto {
    Add {
        primitive: PrimitiveKindDto,
        parent: Option<String>,
    },
    Delete,
    Duplicate,
    Rename {
        node: String,
        name: String,
    },
    /// `preview: true` applies without pushing an undo step; the drag ends with
    /// `commitPreview` or `cancelPreview`.
    SetTransform {
        node: String,
        transform: TransformDto,
        preview: bool,
    },
    CommitPreview,
    CancelPreview,
    SetVisible {
        node: String,
        visible: bool,
    },
    Reparent {
        node: String,
        parent: Option<String>,
    },
    /// Selection is deliberately not undoable, like every other DCC.
    Select {
        nodes: Vec<String>,
        mode: SelectionModeDto,
    },
    Undo,
    Redo,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../packages/types/src/generated/")]
pub struct CommandResultDto {
    pub patch: ScenePatch,
    pub scene: SceneSnapshotDto,
    /// Undo label of the step this command produced, when it produced one.
    pub label: Option<String>,
}

/// Error envelope shared by every IPC handler (`docs/02-architecture.md`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../packages/types/src/generated/")]
pub struct ErrorDto {
    pub code: String,
    pub message: String,
    pub details: BTreeMap<String, String>,
    pub recoverable: bool,
}

impl ErrorDto {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: BTreeMap::new(),
            recoverable: true,
        }
    }

    pub fn fatal(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            recoverable: false,
            ..Self::new(code, message)
        }
    }

    pub fn with_detail(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.details.insert(key.into(), value.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_are_tagged_for_the_typescript_union() {
        let json = serde_json::to_string(&CommandRequestDto::Add {
            primitive: PrimitiveKindDto::Cube,
            parent: None,
        })
        .unwrap();
        assert!(json.contains("\"type\":\"add\""), "{json}");
        assert!(json.contains("\"primitive\":\"cube\""), "{json}");
        let undo = serde_json::to_string(&CommandRequestDto::Undo).unwrap();
        assert_eq!(undo, "{\"type\":\"undo\"}");
    }

    #[test]
    fn primitive_labels_cover_every_variant() {
        for primitive in [
            PrimitiveKindDto::Plane,
            PrimitiveKindDto::Cube,
            PrimitiveKindDto::Sphere,
            PrimitiveKindDto::Cylinder,
            PrimitiveKindDto::Cone,
            PrimitiveKindDto::Torus,
            PrimitiveKindDto::Empty,
            PrimitiveKindDto::Camera,
            PrimitiveKindDto::Light,
        ] {
            assert!(!primitive.label().is_empty());
        }
        assert!(PrimitiveKindDto::Cube.mesh().is_some());
        assert!(PrimitiveKindDto::Light.mesh().is_none());
    }

    #[test]
    fn error_envelope_carries_code_and_details() {
        let error = ErrorDto::new("SCENE_NODE_NOT_FOUND", "node not found").with_detail("node", "x");
        assert_eq!(error.details.get("node").map(String::as_str), Some("x"));
        assert!(error.recoverable);
        assert!(!ErrorDto::fatal("CORE_LOCK_POISONED", "poisoned").recoverable);
    }
}
