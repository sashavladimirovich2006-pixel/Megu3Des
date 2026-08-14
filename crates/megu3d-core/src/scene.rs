//! The scene graph: the single source of truth for document state.
//!
//! The UI never mutates this directly. It sends intents that become commands in
//! `megu3d-cmd`, which are the only writers (`docs/02-architecture.md`).

use std::collections::{HashMap, HashSet};

use megu3d_mesh::{MeshData, Primitive};
use serde::{Deserialize, Serialize};
use slotmap::{new_key_type, SlotMap};
use thiserror::Error;
use uuid::Uuid;

use crate::dto::{
    HistoryStateDto, MeshInfoDto, NodeKindDto, SceneNodeDto, SceneSnapshotDto, SceneStats,
    SelectionModeDto, TransformDto, Vec3Dto,
};
use crate::math::{quat_from_euler_deg, quat_to_euler_deg, Aabb, Mat4, Quat, Vec3, IDENTITY_QUAT};
use crate::SCHEMA_VERSION;

new_key_type! {
    /// Session-stable handle for a scene node. Commands reference nodes by
    /// [`Uuid`] instead, so undo history survives delete/restore cycles.
    pub struct NodeId;
}

new_key_type! {
    /// Handle for mesh data owned by the scene.
    pub struct MeshId;
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SceneError {
    #[error("node not found")]
    NodeNotFound,
    #[error("mesh not found")]
    MeshNotFound,
    #[error("a node cannot be parented under its own descendant")]
    Cycle,
    #[error("invalid argument: {0}")]
    Invalid(String),
}

/// Local transform. Z-up, right-handed, metres; rotation is a quaternion.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translation: [0.0; 3],
            rotation: IDENTITY_QUAT,
            scale: [1.0; 3],
        }
    }
}

impl Transform {
    pub fn matrix(&self) -> Mat4 {
        Mat4::from_trs(self.translation, self.rotation, self.scale)
    }

    pub fn to_dto(&self) -> TransformDto {
        TransformDto {
            translation: Vec3Dto::from_array(self.translation),
            rotation_euler_deg: Vec3Dto::from_array(quat_to_euler_deg(self.rotation)),
            scale: Vec3Dto::from_array(self.scale),
        }
    }

    /// Rejects non-finite values and zero scale: both would produce a singular
    /// matrix and break picking, normals and export.
    pub fn from_dto(dto: &TransformDto) -> Result<Transform, SceneError> {
        let translation = dto.translation.to_array();
        let euler = dto.rotation_euler_deg.to_array();
        let scale = dto.scale.to_array();
        let finite = translation
            .iter()
            .chain(euler.iter())
            .chain(scale.iter())
            .all(|value| value.is_finite());
        if !finite {
            return Err(SceneError::Invalid("transform must be finite".to_owned()));
        }
        if scale.iter().any(|value| value.abs() < 1e-6) {
            return Err(SceneError::Invalid("scale must not be zero".to_owned()));
        }
        Ok(Transform {
            translation,
            rotation: quat_from_euler_deg(euler),
            scale,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CameraData {
    pub fov_deg: f32,
    pub near: f32,
    pub far: f32,
}

impl Default for CameraData {
    fn default() -> Self {
        Self {
            fov_deg: 50.0,
            near: 0.05,
            far: 1000.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LightKind {
    Point,
    Sun,
    Spot,
    Area,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LightData {
    pub kind: LightKind,
    pub color: Vec3,
    /// Irradiance for `Sun`, watts for the other kinds. Physical units land with
    /// the shading work in M4.
    pub intensity: f32,
}

impl Default for LightData {
    fn default() -> Self {
        Self {
            kind: LightKind::Sun,
            color: [1.0, 1.0, 1.0],
            intensity: 3.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NodeData {
    Empty,
    Mesh(MeshId),
    Camera(CameraData),
    Light(LightData),
}

impl NodeData {
    pub fn kind(&self) -> NodeKindDto {
        match self {
            NodeData::Empty => NodeKindDto::Empty,
            NodeData::Mesh(_) => NodeKindDto::Mesh,
            NodeData::Camera(_) => NodeKindDto::Camera,
            NodeData::Light(_) => NodeKindDto::Light,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub uuid: Uuid,
    pub name: String,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
    pub transform: Transform,
    pub visible: bool,
    pub locked: bool,
    pub data: NodeData,
}

/// Detached copy of a node, used by delete/undo and duplicate. Parenting is
/// stored by [`Uuid`] and sibling index so a restore is order-preserving.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoredNode {
    pub uuid: Uuid,
    pub parent: Option<Uuid>,
    pub index: usize,
    pub name: String,
    pub transform: Transform,
    pub visible: bool,
    pub locked: bool,
    pub data: NodeData,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeshEntry {
    pub name: String,
    pub data: MeshData,
    pub bounds: Aabb,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Scene {
    pub schema_version: String,
    nodes: SlotMap<NodeId, Node>,
    meshes: SlotMap<MeshId, MeshEntry>,
    roots: Vec<NodeId>,
    selection: Vec<NodeId>,
    active: Option<NodeId>,
    /// Rebuilt from `nodes` after load; see [`Scene::rebuild_index`].
    #[serde(skip)]
    by_uuid: HashMap<Uuid, NodeId>,
}

impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}

impl Scene {
    pub fn new() -> Self {
        Self {
            schema_version: SCHEMA_VERSION.to_owned(),
            nodes: SlotMap::with_key(),
            meshes: SlotMap::with_key(),
            roots: Vec::new(),
            selection: Vec::new(),
            active: None,
            by_uuid: HashMap::new(),
        }
    }

    /// A new document is not empty: camera, sun and a cube, like every DCC
    /// (`docs/assumptions.md`, `D-81`). Nothing here is special-cased later.
    pub fn startup() -> Self {
        let mut scene = Self::new();
        let camera = scene.add_node("Camera", None, NodeData::Camera(CameraData::default()));
        if let Some(node) = scene.node_mut(camera) {
            node.transform.translation = [5.0, -7.0, 4.5];
            node.transform.rotation = quat_from_euler_deg([63.0, 0.0, 35.0]);
        }
        let sun = scene.add_node("Sun", None, NodeData::Light(LightData::default()));
        if let Some(node) = scene.node_mut(sun) {
            node.transform.translation = [2.0, -2.0, 6.0];
            node.transform.rotation = quat_from_euler_deg([40.0, 0.0, 30.0]);
        }
        let mesh = scene.insert_mesh(Primitive::Cube.label(), Primitive::Cube.build());
        let cube = scene.add_node(Primitive::Cube.label(), None, NodeData::Mesh(mesh));
        let uuid = scene.node(cube).map(|node| node.uuid);
        if let Some(uuid) = uuid {
            scene.select(&[uuid], SelectionModeDto::Replace);
        }
        scene
    }

    /// Call after deserializing a scene: the uuid index is not persisted.
    pub fn rebuild_index(&mut self) {
        self.by_uuid = self
            .nodes
            .iter()
            .map(|(id, node)| (node.uuid, id))
            .collect();
    }

    pub fn insert_mesh(&mut self, name: impl Into<String>, data: MeshData) -> MeshId {
        let (min, max) = data.bounds();
        self.meshes.insert(MeshEntry {
            name: name.into(),
            data,
            bounds: Aabb::from_min_max(min, max),
        })
    }

    pub fn mesh(&self, id: MeshId) -> Option<&MeshEntry> {
        self.meshes.get(id)
    }

    pub fn mesh_count(&self) -> usize {
        self.meshes.len()
    }

    /// True when a node already uses this exact name. Drives the `Name.001`
    /// scheme when adding and duplicating nodes.
    pub fn has_name(&self, name: &str) -> bool {
        self.nodes.values().any(|node| node.name == name)
    }

    pub fn iter(&self) -> impl Iterator<Item = (NodeId, &Node)> {
        self.nodes.iter()
    }

    pub fn add_node(
        &mut self,
        name: impl Into<String>,
        parent: Option<NodeId>,
        data: NodeData,
    ) -> NodeId {
        let node = Node {
            uuid: Uuid::new_v4(),
            name: name.into(),
            parent,
            children: Vec::new(),
            transform: Transform::default(),
            visible: true,
            locked: false,
            data,
        };
        self.attach(node, parent, None)
    }

    /// Adds a node with a caller-supplied uuid: used by commands so redo keeps
    /// the same identity the first apply created.
    pub fn add_node_with_uuid(
        &mut self,
        uuid: Uuid,
        name: impl Into<String>,
        parent: Option<NodeId>,
        data: NodeData,
    ) -> Result<NodeId, SceneError> {
        if self.by_uuid.contains_key(&uuid) {
            return Err(SceneError::Invalid("uuid is already in the scene".to_owned()));
        }
        let node = Node {
            uuid,
            name: name.into(),
            parent,
            children: Vec::new(),
            transform: Transform::default(),
            visible: true,
            locked: false,
            data,
        };
        Ok(self.attach(node, parent, None))
    }

    fn attach(&mut self, node: Node, parent: Option<NodeId>, index: Option<usize>) -> NodeId {
        let uuid = node.uuid;
        let id = self.nodes.insert(node);
        self.by_uuid.insert(uuid, id);
        let siblings = match parent.and_then(|parent_id| self.nodes.get(parent_id)) {
            Some(parent_node) => parent_node.children.len(),
            None => self.roots.len(),
        };
        let at = index.unwrap_or(siblings).min(siblings);
        match parent.and_then(|parent_id| self.nodes.get_mut(parent_id)) {
            Some(parent_node) => parent_node.children.insert(at, id),
            None => self.roots.insert(at, id),
        }
        if let Some(inserted) = self.nodes.get_mut(id) {
            inserted.parent = parent;
        }
        id
    }

    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(id)
    }

    pub fn node_mut(&mut self, id: NodeId) -> Option<&mut Node> {
        self.nodes.get_mut(id)
    }

    pub fn resolve(&self, uuid: Uuid) -> Result<NodeId, SceneError> {
        self.by_uuid
            .get(&uuid)
            .copied()
            .ok_or(SceneError::NodeNotFound)
    }

    pub fn uuid_of(&self, id: NodeId) -> Option<Uuid> {
        self.nodes.get(id).map(|node| node.uuid)
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn roots(&self) -> &[NodeId] {
        &self.roots
    }

    pub fn selection(&self) -> &[NodeId] {
        &self.selection
    }

    pub fn selection_uuids(&self) -> Vec<Uuid> {
        self.selection
            .iter()
            .filter_map(|id| self.uuid_of(*id))
            .collect()
    }

    pub fn active_uuid(&self) -> Option<Uuid> {
        self.active.and_then(|id| self.uuid_of(id))
    }

    /// Selection is intentionally outside the undo history, like other DCCs.
    pub fn select(&mut self, uuids: &[Uuid], mode: SelectionModeDto) {
        let ids: Vec<NodeId> = uuids
            .iter()
            .filter_map(|uuid| self.by_uuid.get(uuid).copied())
            .collect();
        match mode {
            SelectionModeDto::Clear => {
                self.selection.clear();
                self.active = None;
            }
            SelectionModeDto::Replace => {
                self.selection = ids.clone();
                self.active = ids.last().copied();
            }
            SelectionModeDto::Add => {
                for id in &ids {
                    if !self.selection.contains(id) {
                        self.selection.push(*id);
                    }
                }
                if let Some(last) = ids.last() {
                    self.active = Some(*last);
                }
            }
            SelectionModeDto::Toggle => {
                for id in &ids {
                    match self.selection.iter().position(|current| current == id) {
                        Some(position) => {
                            self.selection.remove(position);
                            if self.active == Some(*id) {
                                self.active = self.selection.last().copied();
                            }
                        }
                        None => {
                            self.selection.push(*id);
                            self.active = Some(*id);
                        }
                    }
                }
            }
        }
    }

    /// Selected nodes whose ancestors are not also selected: what delete and
    /// duplicate operate on, so subtrees are handled exactly once.
    pub fn top_level_selection(&self) -> Vec<NodeId> {
        self.selection
            .iter()
            .copied()
            .filter(|id| {
                !self
                    .selection
                    .iter()
                    .any(|other| other != id && self.is_ancestor(*other, *id))
            })
            .collect()
    }

    pub fn is_ancestor(&self, ancestor: NodeId, node: NodeId) -> bool {
        let mut current = self.nodes.get(node).and_then(|entry| entry.parent);
        while let Some(id) = current {
            if id == ancestor {
                return true;
            }
            current = self.nodes.get(id).and_then(|entry| entry.parent);
        }
        false
    }

    pub fn sibling_index(&self, id: NodeId) -> Option<usize> {
        let siblings = match self.nodes.get(id).and_then(|node| node.parent) {
            Some(parent) => &self.nodes.get(parent)?.children,
            None => &self.roots,
        };
        siblings.iter().position(|child| *child == id)
    }

    fn collect_stored(&self, id: NodeId, out: &mut Vec<StoredNode>) {
        let Some(node) = self.nodes.get(id) else {
            return;
        };
        out.push(StoredNode {
            uuid: node.uuid,
            parent: node
                .parent
                .and_then(|parent| self.nodes.get(parent))
                .map(|parent| parent.uuid),
            index: self.sibling_index(id).unwrap_or(0),
            name: node.name.clone(),
            transform: node.transform,
            visible: node.visible,
            locked: node.locked,
            data: node.data.clone(),
        });
        for child in &node.children {
            self.collect_stored(*child, out);
        }
    }

    /// Copies a subtree without touching the scene: the source of duplicate.
    pub fn export_subtree(&self, root: NodeId) -> Vec<StoredNode> {
        let mut stored = Vec::new();
        self.collect_stored(root, &mut stored);
        stored
    }

    /// Removes a subtree and returns it in pre-order so it can be restored
    /// exactly by [`Scene::restore_subtree`].
    pub fn detach_subtree(&mut self, root: NodeId) -> Vec<StoredNode> {
        let stored = self.export_subtree(root);
        let parent = self.nodes.get(root).and_then(|node| node.parent);
        match parent.and_then(|id| self.nodes.get_mut(id)) {
            Some(parent_node) => parent_node.children.retain(|child| *child != root),
            None => self.roots.retain(|child| *child != root),
        }
        for entry in &stored {
            if let Some(id) = self.by_uuid.remove(&entry.uuid) {
                self.nodes.remove(id);
                self.selection.retain(|selected| *selected != id);
                if self.active == Some(id) {
                    self.active = self.selection.last().copied();
                }
            }
        }
        stored
    }

    /// Re-inserts detached nodes, preserving uuids, order and hierarchy.
    /// Returns the uuids of the restored subtree roots.
    pub fn restore_subtree(&mut self, stored: &[StoredNode]) -> Result<Vec<Uuid>, SceneError> {
        let contained: HashSet<Uuid> = stored.iter().map(|entry| entry.uuid).collect();
        let mut restored_roots = Vec::new();
        for entry in stored {
            let parent = match entry.parent {
                Some(uuid) => Some(self.resolve(uuid)?),
                None => None,
            };
            if entry
                .parent
                .map_or(true, |parent_uuid| !contained.contains(&parent_uuid))
            {
                restored_roots.push(entry.uuid);
            }
            if self.by_uuid.contains_key(&entry.uuid) {
                return Err(SceneError::Invalid(
                    "uuid is already in the scene".to_owned(),
                ));
            }
            let node = Node {
                uuid: entry.uuid,
                name: entry.name.clone(),
                parent,
                children: Vec::new(),
                transform: entry.transform,
                visible: entry.visible,
                locked: entry.locked,
                data: entry.data.clone(),
            };
            self.attach(node, parent, Some(entry.index));
        }
        Ok(restored_roots)
    }

    /// Moves a node under a new parent. Returns the previous `(parent, index)`
    /// so the command can invert itself.
    pub fn reparent(
        &mut self,
        node: NodeId,
        parent: Option<NodeId>,
        index: Option<usize>,
    ) -> Result<(Option<Uuid>, usize), SceneError> {
        if !self.nodes.contains_key(node) {
            return Err(SceneError::NodeNotFound);
        }
        if let Some(target) = parent {
            if !self.nodes.contains_key(target) {
                return Err(SceneError::NodeNotFound);
            }
            if target == node || self.is_ancestor(node, target) {
                return Err(SceneError::Cycle);
            }
        }
        let previous_parent = self.nodes.get(node).and_then(|entry| entry.parent);
        let previous_parent_uuid = previous_parent.and_then(|id| self.uuid_of(id));
        let previous_index = self.sibling_index(node).unwrap_or(0);
        match previous_parent.and_then(|id| self.nodes.get_mut(id)) {
            Some(parent_node) => parent_node.children.retain(|child| *child != node),
            None => self.roots.retain(|child| *child != node),
        }
        let siblings = match parent.and_then(|id| self.nodes.get(id)) {
            Some(parent_node) => parent_node.children.len(),
            None => self.roots.len(),
        };
        let at = index.unwrap_or(siblings).min(siblings);
        match parent.and_then(|id| self.nodes.get_mut(id)) {
            Some(parent_node) => parent_node.children.insert(at, node),
            None => self.roots.insert(at, node),
        }
        if let Some(entry) = self.nodes.get_mut(node) {
            entry.parent = parent;
        }
        Ok((previous_parent_uuid, previous_index))
    }

    /// World matrix of a single node, walking up the parent chain.
    pub fn world_matrix(&self, id: NodeId) -> Mat4 {
        let mut chain = Vec::new();
        let mut current = Some(id);
        while let Some(node_id) = current {
            let Some(node) = self.nodes.get(node_id) else {
                break;
            };
            chain.push(node.transform.matrix());
            current = node.parent;
        }
        chain
            .iter()
            .rev()
            .fold(Mat4::IDENTITY, |world, local| world.mul(local))
    }

    /// World matrices for the whole scene in one top-down pass. Called once per
    /// snapshot; a dirty-tracked cache lands with the renderer, where it pays off.
    fn world_matrices(&self) -> HashMap<NodeId, Mat4> {
        let mut out = HashMap::with_capacity(self.nodes.len());
        let mut stack: Vec<(NodeId, Mat4)> = self
            .roots
            .iter()
            .rev()
            .map(|id| (*id, Mat4::IDENTITY))
            .collect();
        while let Some((id, parent_world)) = stack.pop() {
            let Some(node) = self.nodes.get(id) else {
                continue;
            };
            let world = parent_world.mul(&node.transform.matrix());
            for child in node.children.iter().rev() {
                stack.push((*child, world));
            }
            out.insert(id, world);
        }
        out
    }

    pub fn triangle_count(&self) -> u32 {
        self.nodes
            .values()
            .filter_map(|node| match &node.data {
                NodeData::Mesh(mesh) => self.meshes.get(*mesh),
                _ => None,
            })
            .map(|entry| entry.data.triangle_count() as u32)
            .sum()
    }

    pub fn stats(&self) -> SceneStats {
        SceneStats {
            node_count: self.nodes.len() as u32,
            selection_count: self.selection.len() as u32,
            triangle_count: self.triangle_count(),
            schema_version: self.schema_version.clone(),
        }
    }

    /// Flattens the scene in outliner order (pre-order, siblings in place).
    pub fn snapshot(&self, history: HistoryStateDto) -> SceneSnapshotDto {
        let worlds = self.world_matrices();
        let mut nodes = Vec::with_capacity(self.nodes.len());
        let mut stack: Vec<(NodeId, u32)> = self.roots.iter().rev().map(|id| (*id, 0)).collect();
        while let Some((id, depth)) = stack.pop() {
            let Some(node) = self.nodes.get(id) else {
                continue;
            };
            for child in node.children.iter().rev() {
                stack.push((*child, depth + 1));
            }
            let world = worlds.get(&id).copied().unwrap_or(Mat4::IDENTITY);
            let entry = match &node.data {
                NodeData::Mesh(mesh) => self.meshes.get(*mesh),
                _ => None,
            };
            let bounds = match entry {
                Some(mesh) => mesh.bounds.transformed(&world),
                None => Aabb::point(world.translation()),
            };
            nodes.push(SceneNodeDto {
                uuid: node.uuid.to_string(),
                name: node.name.clone(),
                parent: node
                    .parent
                    .and_then(|parent| self.nodes.get(parent))
                    .map(|parent| parent.uuid.to_string()),
                children: node
                    .children
                    .iter()
                    .filter_map(|child| self.nodes.get(*child))
                    .map(|child| child.uuid.to_string())
                    .collect(),
                depth,
                kind: node.data.kind(),
                transform: node.transform.to_dto(),
                world_translation: Vec3Dto::from_array(world.translation()),
                world_bounds_min: Vec3Dto::from_array(bounds.min),
                world_bounds_max: Vec3Dto::from_array(bounds.max),
                mesh: entry.map(|mesh| MeshInfoDto {
                    name: mesh.name.clone(),
                    vertex_count: mesh.data.vertex_count() as u32,
                    triangle_count: mesh.data.triangle_count() as u32,
                    bounds_min: Vec3Dto::from_array(mesh.bounds.min),
                    bounds_max: Vec3Dto::from_array(mesh.bounds.max),
                }),
                visible: node.visible,
                locked: node.locked,
                selected: self.selection.contains(&id),
            });
        }
        SceneSnapshotDto {
            schema_version: self.schema_version.clone(),
            nodes,
            roots: self
                .roots
                .iter()
                .filter_map(|id| self.uuid_of(*id))
                .map(|uuid| uuid.to_string())
                .collect(),
            selection: self
                .selection_uuids()
                .into_iter()
                .map(|uuid| uuid.to_string())
                .collect(),
            active: self.active_uuid().map(|uuid| uuid.to_string()),
            stats: self.stats(),
            history,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube_scene() -> (Scene, NodeId) {
        let mut scene = Scene::new();
        let mesh = scene.insert_mesh("Cube", Primitive::Cube.build());
        let node = scene.add_node("Cube", None, NodeData::Mesh(mesh));
        (scene, node)
    }

    #[test]
    fn startup_scene_has_camera_sun_and_cube() {
        let scene = Scene::startup();
        let snapshot = scene.snapshot(HistoryStateDto::default());
        let names: Vec<&str> = snapshot
            .nodes
            .iter()
            .map(|node| node.name.as_str())
            .collect();
        assert_eq!(names, vec!["Camera", "Sun", "Cube"]);
        assert_eq!(snapshot.stats.triangle_count, 12);
        assert_eq!(snapshot.selection.len(), 1);
        assert_eq!(snapshot.active, snapshot.selection.first().cloned());
    }

    #[test]
    fn child_transforms_compose_into_world_space() {
        let (mut scene, parent) = cube_scene();
        if let Some(node) = scene.node_mut(parent) {
            node.transform.translation = [0.0, 0.0, 2.0];
        }
        let child = scene.add_node("Child", Some(parent), NodeData::Empty);
        if let Some(node) = scene.node_mut(child) {
            node.transform.translation = [1.0, 0.0, 0.0];
        }
        let world = scene.world_matrix(child).translation();
        assert!((world[0] - 1.0).abs() < 1e-5);
        assert!((world[2] - 2.0).abs() < 1e-5);
        let snapshot = scene.snapshot(HistoryStateDto::default());
        assert_eq!(snapshot.nodes[1].depth, 1);
        assert_eq!(
            snapshot.nodes[1].parent,
            Some(snapshot.nodes[0].uuid.clone())
        );
    }

    #[test]
    fn detach_and_restore_round_trip_preserves_identity_and_order() {
        let (mut scene, parent) = cube_scene();
        let first = scene.add_node("First", Some(parent), NodeData::Empty);
        let second = scene.add_node("Second", Some(parent), NodeData::Empty);
        let before = scene.snapshot(HistoryStateDto::default());
        let stored = scene.detach_subtree(first);
        assert_eq!(scene.len(), 2);
        assert_eq!(
            scene.node(parent).map(|node| node.children.clone()),
            Some(vec![second])
        );
        scene.restore_subtree(&stored).unwrap();
        let after = scene.snapshot(HistoryStateDto::default());
        assert_eq!(before.nodes, after.nodes);
        assert_eq!(before.roots, after.roots);
    }

    #[test]
    fn reparenting_under_a_descendant_is_rejected() {
        let (mut scene, parent) = cube_scene();
        let child = scene.add_node("Child", Some(parent), NodeData::Empty);
        assert_eq!(
            scene.reparent(parent, Some(child), None),
            Err(SceneError::Cycle)
        );
        assert_eq!(
            scene.reparent(parent, Some(parent), None),
            Err(SceneError::Cycle)
        );
        let (previous_parent, previous_index) = scene.reparent(child, None, Some(0)).unwrap();
        assert_eq!(previous_parent, scene.uuid_of(parent));
        assert_eq!(previous_index, 0);
        assert_eq!(scene.roots().len(), 2);
    }

    #[test]
    fn selection_modes_behave_like_a_dcc() {
        let (mut scene, cube) = cube_scene();
        let other = scene.add_node("Empty", None, NodeData::Empty);
        let cube_uuid = scene.uuid_of(cube).unwrap();
        let other_uuid = scene.uuid_of(other).unwrap();
        scene.select(&[cube_uuid], SelectionModeDto::Replace);
        assert_eq!(scene.selection_uuids(), vec![cube_uuid]);
        scene.select(&[other_uuid], SelectionModeDto::Add);
        assert_eq!(scene.selection_uuids(), vec![cube_uuid, other_uuid]);
        scene.select(&[cube_uuid], SelectionModeDto::Toggle);
        assert_eq!(scene.selection_uuids(), vec![other_uuid]);
        assert_eq!(scene.active_uuid(), Some(other_uuid));
        scene.select(&[], SelectionModeDto::Clear);
        assert!(scene.selection_uuids().is_empty());
        assert_eq!(scene.active_uuid(), None);
    }

    #[test]
    fn top_level_selection_drops_nested_nodes() {
        let (mut scene, parent) = cube_scene();
        let child = scene.add_node("Child", Some(parent), NodeData::Empty);
        let parent_uuid = scene.uuid_of(parent).unwrap();
        let child_uuid = scene.uuid_of(child).unwrap();
        scene.select(&[parent_uuid, child_uuid], SelectionModeDto::Replace);
        assert_eq!(scene.top_level_selection(), vec![parent]);
    }

    #[test]
    fn transform_dto_round_trips_and_rejects_zero_scale() {
        let transform = Transform {
            translation: [1.0, 2.0, 3.0],
            rotation: quat_from_euler_deg([15.0, 30.0, 45.0]),
            scale: [2.0, 1.0, 0.5],
        };
        let restored = Transform::from_dto(&transform.to_dto()).unwrap();
        assert!((restored.translation[1] - 2.0).abs() < 1e-5);
        let euler = quat_to_euler_deg(restored.rotation);
        assert!((euler[2] - 45.0).abs() < 1e-3, "{euler:?}");
        let mut broken = transform.to_dto();
        broken.scale.x = 0.0;
        assert!(Transform::from_dto(&broken).is_err());
    }

    #[test]
    fn names_and_uuid_index_are_queryable() {
        let (mut scene, cube) = cube_scene();
        let uuid = scene.uuid_of(cube).unwrap();
        assert!(scene.has_name("Cube"));
        assert!(!scene.has_name("Cube.001"));
        assert_eq!(scene.iter().count(), 1);
        scene.rebuild_index();
        assert_eq!(scene.resolve(uuid), Ok(cube));
        assert_eq!(scene.resolve(Uuid::nil()), Err(SceneError::NodeNotFound));
    }
}
