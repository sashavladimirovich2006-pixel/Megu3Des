//! Megu3D core: the scene graph, the math it is built on, and the wire types
//! shared with the UI.
//!
//! Rust owns document state. The UI sends intents, `megu3d-cmd` turns them into
//! undoable commands, and this crate is the only place scene data lives
//! (`docs/02-architecture.md`).

pub mod dto;
pub mod math;
pub mod scene;

/// Scene schema version, independent of the app version.
///
/// `0.2.0` adds the real node graph (meshes, cameras, lights, hierarchy).
/// No `.megu3d` files exist in the wild yet, so no migration is needed; the
/// first migration lands with save/load in M4 (`docs/02-architecture.md`).
pub const SCHEMA_VERSION: &str = "0.2.0";

pub use dto::{
    AppInfo, CommandRequestDto, CommandResultDto, ErrorDto, HistoryStateDto, MeshInfoDto,
    NodeKindDto, PrimitiveKindDto, SceneNodeDto, ScenePatch, SceneSnapshotDto, SceneStats,
    SelectionModeDto, TransformDto, Vec3Dto,
};
pub use math::{quat_from_euler_deg, quat_to_euler_deg, Aabb, Mat4, Quat, Vec3};
pub use megu3d_mesh::{MeshData, MeshError, Primitive};
pub use scene::{
    CameraData, LightData, LightKind, MeshEntry, MeshId, Node, NodeData, NodeId, Scene, SceneError,
    StoredNode, Transform,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_info_reports_the_schema_version() {
        let info = AppInfo::current();
        assert_eq!(info.name, "Megu3D");
        assert_eq!(info.schema_version, SCHEMA_VERSION);
        assert!(!info.version.is_empty());
        assert!(matches!(info.build_profile.as_str(), "debug" | "release"));
    }

    #[test]
    fn schema_version_is_semver() {
        let parts: Vec<&str> = SCHEMA_VERSION.split('.').collect();
        assert_eq!(parts.len(), 3, "{SCHEMA_VERSION}");
        assert!(parts.iter().all(|part| part.parse::<u32>().is_ok()));
    }

    #[test]
    fn scene_patches_merge_into_the_widest_scope() {
        let mut patch = ScenePatch::default();
        patch.merge(ScenePatch::nodes([uuid::Uuid::nil()]));
        assert_eq!(patch.changed_nodes.len(), 1);
        assert!(!patch.full_reload);
        patch.merge(ScenePatch::structural([]));
        assert!(patch.full_reload);
    }

    #[test]
    fn primitives_are_reachable_through_the_core_facade() {
        let mesh: MeshData = Primitive::Sphere.build();
        assert!(mesh.validate().is_ok());
        assert!(mesh.triangle_count() > 0);
    }
}
