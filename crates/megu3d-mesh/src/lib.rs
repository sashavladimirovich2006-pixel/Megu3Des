//! Primitive mesh generation for Megu3D.
//!
//! Pure geometry: no scene, no IPC, no GPU. Conventions (`docs/assumptions.md`):
//! Z-up right-handed, meters (`D-30`, `D-31`), UV origin bottom-left (`D-33`),
//! triangles wound counter-clockwise as seen from outside the surface.

use std::f32::consts::{PI, TAU};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Default radial resolution of round primitives.
pub const DEFAULT_SEGMENTS: u32 = 32;
/// Default ring resolution of spheres and tori.
pub const DEFAULT_RINGS: u32 = 16;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum MeshError {
    #[error("index {index} is out of range for {vertices} vertices")]
    IndexOutOfRange { index: u32, vertices: usize },
    #[error("index count {0} is not a multiple of three")]
    NotTriangles(usize),
    #[error("attribute arrays have different lengths")]
    AttributeMismatch,
}

/// Indexed triangle mesh. Attribute vectors are parallel: one entry per vertex.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MeshData {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
}

impl MeshData {
    pub fn vertex_count(&self) -> usize {
        self.positions.len()
    }

    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    /// Axis-aligned bounds as `(min, max)`. An empty mesh reports zeroes.
    pub fn bounds(&self) -> ([f32; 3], [f32; 3]) {
        let mut min = [f32::INFINITY; 3];
        let mut max = [f32::NEG_INFINITY; 3];
        for position in &self.positions {
            for axis in 0..3 {
                min[axis] = min[axis].min(position[axis]);
                max[axis] = max[axis].max(position[axis]);
            }
        }
        if self.positions.is_empty() {
            return ([0.0; 3], [0.0; 3]);
        }
        (min, max)
    }

    /// Structural check shared by tests today and by importers in M4.
    pub fn validate(&self) -> Result<(), MeshError> {
        if self.normals.len() != self.positions.len() || self.uvs.len() != self.positions.len() {
            return Err(MeshError::AttributeMismatch);
        }
        if self.indices.len() % 3 != 0 {
            return Err(MeshError::NotTriangles(self.indices.len()));
        }
        let vertices = self.positions.len();
        for &index in &self.indices {
            if index as usize >= vertices {
                return Err(MeshError::IndexOutOfRange { index, vertices });
            }
        }
        Ok(())
    }

    fn push_vertex(&mut self, position: [f32; 3], normal: [f32; 3], uv: [f32; 2]) -> u32 {
        let index = self.positions.len() as u32;
        self.positions.push(position);
        self.normals.push(normal);
        self.uvs.push(uv);
        index
    }

    fn push_triangle(&mut self, a: u32, b: u32, c: u32) {
        self.indices.extend_from_slice(&[a, b, c]);
    }
}

/// Mesh primitives available from the Add menu and the command palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Primitive {
    Plane,
    Cube,
    Sphere,
    Cylinder,
    Cone,
    Torus,
}

impl Primitive {
    /// Every primitive is one metre across, centred on its own origin.
    pub fn build(self) -> MeshData {
        match self {
            Primitive::Plane => plane(1.0),
            Primitive::Cube => cube(1.0),
            Primitive::Sphere => sphere(0.5, DEFAULT_SEGMENTS, DEFAULT_RINGS),
            Primitive::Cylinder => cylinder(0.5, 1.0, DEFAULT_SEGMENTS),
            Primitive::Cone => cone(0.5, 1.0, DEFAULT_SEGMENTS),
            Primitive::Torus => torus(0.5, 0.15, DEFAULT_SEGMENTS, DEFAULT_RINGS),
        }
    }

    /// English base name; the UI translates node names on creation, not after.
    pub fn label(self) -> &'static str {
        match self {
            Primitive::Plane => "Plane",
            Primitive::Cube => "Cube",
            Primitive::Sphere => "Sphere",
            Primitive::Cylinder => "Cylinder",
            Primitive::Cone => "Cone",
            Primitive::Torus => "Torus",
        }
    }
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let length = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if length <= f32::EPSILON {
        return [0.0, 0.0, 1.0];
    }
    [v[0] / length, v[1] / length, v[2] / length]
}

/// Unit quad in the XY plane, normal `+Z`.
pub fn plane(size: f32) -> MeshData {
    let half = size.max(f32::EPSILON) / 2.0;
    let normal = [0.0, 0.0, 1.0];
    let mut mesh = MeshData::default();
    let a = mesh.push_vertex([-half, -half, 0.0], normal, [0.0, 0.0]);
    let b = mesh.push_vertex([half, -half, 0.0], normal, [1.0, 0.0]);
    let c = mesh.push_vertex([half, half, 0.0], normal, [1.0, 1.0]);
    let d = mesh.push_vertex([-half, half, 0.0], normal, [0.0, 1.0]);
    mesh.push_triangle(a, b, c);
    mesh.push_triangle(a, c, d);
    mesh
}

/// Face basis: `(normal, u, v)` with `u x v == normal`, which keeps winding outward.
const CUBE_FACES: [([f32; 3], [f32; 3], [f32; 3]); 6] = [
    ([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]),
    ([-1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0, 0.0]),
    ([0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
    ([0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]),
    ([0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
    ([0.0, 0.0, -1.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]),
];

/// Cube with hard edges: 24 vertices, per-face normals, per-face UV square.
pub fn cube(size: f32) -> MeshData {
    let half = size.max(f32::EPSILON) / 2.0;
    let mut mesh = MeshData::default();
    for (normal, u, v) in CUBE_FACES {
        let corner = |su: f32, sv: f32| {
            [
                (normal[0] + u[0] * su + v[0] * sv) * half,
                (normal[1] + u[1] * su + v[1] * sv) * half,
                (normal[2] + u[2] * su + v[2] * sv) * half,
            ]
        };
        let a = mesh.push_vertex(corner(-1.0, -1.0), normal, [0.0, 0.0]);
        let b = mesh.push_vertex(corner(1.0, -1.0), normal, [1.0, 0.0]);
        let c = mesh.push_vertex(corner(1.0, 1.0), normal, [1.0, 1.0]);
        let d = mesh.push_vertex(corner(-1.0, 1.0), normal, [0.0, 1.0]);
        mesh.push_triangle(a, b, c);
        mesh.push_triangle(a, c, d);
    }
    mesh
}

/// UV sphere with poles on the Z axis; `rings` spans pole to pole.
pub fn sphere(radius: f32, segments: u32, rings: u32) -> MeshData {
    let radius = radius.max(f32::EPSILON);
    let segments = segments.max(3);
    let rings = rings.max(2);
    let mut mesh = MeshData::default();
    for ring in 0..=rings {
        let v = ring as f32 / rings as f32;
        let (sin_phi, cos_phi) = (v * PI).sin_cos();
        for segment in 0..=segments {
            let u = segment as f32 / segments as f32;
            let (sin_theta, cos_theta) = (u * TAU).sin_cos();
            let normal = [sin_phi * cos_theta, sin_phi * sin_theta, cos_phi];
            let position = [normal[0] * radius, normal[1] * radius, normal[2] * radius];
            mesh.push_vertex(position, normal, [u, 1.0 - v]);
        }
    }
    let stride = segments + 1;
    for ring in 0..rings {
        for segment in 0..segments {
            let a = ring * stride + segment;
            let b = a + 1;
            let c = a + stride;
            let d = c + 1;
            if ring > 0 {
                mesh.push_triangle(a, c, b);
            }
            if ring + 1 < rings {
                mesh.push_triangle(b, c, d);
            }
        }
    }
    mesh
}

fn push_cap(mesh: &mut MeshData, radius: f32, z: f32, segments: u32, up: bool) {
    let normal = if up { [0.0, 0.0, 1.0] } else { [0.0, 0.0, -1.0] };
    let center = mesh.push_vertex([0.0, 0.0, z], normal, [0.5, 0.5]);
    let first = mesh.positions.len() as u32;
    for segment in 0..=segments {
        let u = segment as f32 / segments as f32;
        let (sin, cos) = (u * TAU).sin_cos();
        let uv = [0.5 + cos * 0.5, 0.5 + sin * 0.5];
        mesh.push_vertex([cos * radius, sin * radius, z], normal, uv);
    }
    for segment in 0..segments {
        let a = first + segment;
        let b = a + 1;
        if up {
            mesh.push_triangle(center, a, b);
        } else {
            mesh.push_triangle(center, b, a);
        }
    }
}

/// Cylinder along Z with flat caps and smooth side normals.
pub fn cylinder(radius: f32, height: f32, segments: u32) -> MeshData {
    let radius = radius.max(f32::EPSILON);
    let half = height.max(f32::EPSILON) / 2.0;
    let segments = segments.max(3);
    let mut mesh = MeshData::default();
    for segment in 0..=segments {
        let u = segment as f32 / segments as f32;
        let (sin, cos) = (u * TAU).sin_cos();
        let normal = [cos, sin, 0.0];
        mesh.push_vertex([cos * radius, sin * radius, -half], normal, [u, 0.0]);
        mesh.push_vertex([cos * radius, sin * radius, half], normal, [u, 1.0]);
    }
    for segment in 0..segments {
        let bottom = segment * 2;
        let top = bottom + 1;
        let next_bottom = bottom + 2;
        let next_top = bottom + 3;
        mesh.push_triangle(bottom, next_bottom, top);
        mesh.push_triangle(next_bottom, next_top, top);
    }
    push_cap(&mut mesh, radius, half, segments, true);
    push_cap(&mut mesh, radius, -half, segments, false);
    mesh
}

/// Cone along Z, apex at `+height/2`, flat base cap.
pub fn cone(radius: f32, height: f32, segments: u32) -> MeshData {
    let radius = radius.max(f32::EPSILON);
    let height = height.max(f32::EPSILON);
    let half = height / 2.0;
    let segments = segments.max(3);
    let mut mesh = MeshData::default();
    for segment in 0..segments {
        let u0 = segment as f32 / segments as f32;
        let u1 = (segment + 1) as f32 / segments as f32;
        let um = (u0 + u1) / 2.0;
        let (sin0, cos0) = (u0 * TAU).sin_cos();
        let (sin1, cos1) = (u1 * TAU).sin_cos();
        let (sinm, cosm) = (um * TAU).sin_cos();
        let a = mesh.push_vertex(
            [cos0 * radius, sin0 * radius, -half],
            normalize([cos0 * height, sin0 * height, radius]),
            [u0, 0.0],
        );
        let b = mesh.push_vertex(
            [cos1 * radius, sin1 * radius, -half],
            normalize([cos1 * height, sin1 * height, radius]),
            [u1, 0.0],
        );
        let apex = mesh.push_vertex(
            [0.0, 0.0, half],
            normalize([cosm * height, sinm * height, radius]),
            [um, 1.0],
        );
        mesh.push_triangle(a, b, apex);
    }
    push_cap(&mut mesh, radius, -half, segments, false);
    mesh
}

/// Torus in the XY plane; the hole axis is Z.
pub fn torus(major: f32, minor: f32, major_segments: u32, minor_segments: u32) -> MeshData {
    let major = major.max(f32::EPSILON);
    let minor = minor.max(f32::EPSILON).min(major);
    let major_segments = major_segments.max(3);
    let minor_segments = minor_segments.max(3);
    let mut mesh = MeshData::default();
    for i in 0..=major_segments {
        let u = i as f32 / major_segments as f32;
        let (sin_u, cos_u) = (u * TAU).sin_cos();
        for j in 0..=minor_segments {
            let v = j as f32 / minor_segments as f32;
            let (sin_v, cos_v) = (v * TAU).sin_cos();
            let normal = [cos_v * cos_u, cos_v * sin_u, sin_v];
            let radial = major + minor * cos_v;
            mesh.push_vertex([radial * cos_u, radial * sin_u, minor * sin_v], normal, [u, v]);
        }
    }
    let stride = minor_segments + 1;
    for i in 0..major_segments {
        for j in 0..minor_segments {
            let a = i * stride + j;
            let c = a + 1;
            let b = a + stride;
            let d = b + 1;
            mesh.push_triangle(a, b, c);
            mesh.push_triangle(b, d, c);
        }
    }
    mesh
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Primitive; 6] = [
        Primitive::Plane,
        Primitive::Cube,
        Primitive::Sphere,
        Primitive::Cylinder,
        Primitive::Cone,
        Primitive::Torus,
    ];

    fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    }

    fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
        [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
    }

    fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
        a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
    }

    #[test]
    fn every_primitive_is_structurally_valid() {
        for primitive in ALL {
            let mesh = primitive.build();
            mesh.validate()
                .unwrap_or_else(|error| panic!("{} is invalid: {error}", primitive.label()));
            assert!(mesh.triangle_count() > 0, "{} has no faces", primitive.label());
        }
    }

    #[test]
    fn triangles_are_wound_outward() {
        for primitive in ALL {
            let mesh = primitive.build();
            for triangle in mesh.indices.chunks_exact(3) {
                let (i, j, k) = (
                    triangle[0] as usize,
                    triangle[1] as usize,
                    triangle[2] as usize,
                );
                let face = cross(
                    sub(mesh.positions[j], mesh.positions[i]),
                    sub(mesh.positions[k], mesh.positions[i]),
                );
                let vertex = [
                    (mesh.normals[i][0] + mesh.normals[j][0] + mesh.normals[k][0]) / 3.0,
                    (mesh.normals[i][1] + mesh.normals[j][1] + mesh.normals[k][1]) / 3.0,
                    (mesh.normals[i][2] + mesh.normals[j][2] + mesh.normals[k][2]) / 3.0,
                ];
                assert!(
                    dot(face, vertex) > 0.0,
                    "{} has an inward triangle {triangle:?}",
                    primitive.label()
                );
            }
        }
    }

    #[test]
    fn primitives_are_one_metre_and_centred() {
        for primitive in ALL {
            let (min, max) = primitive.build().bounds();
            for axis in 0..3 {
                assert!(max[axis] <= 0.501, "{} is too large", primitive.label());
                assert!(min[axis] >= -0.501, "{} is too large", primitive.label());
                assert!(
                    (min[axis] + max[axis]).abs() < 1e-4,
                    "{} is off-centre on axis {axis}",
                    primitive.label()
                );
            }
        }
    }

    #[test]
    fn topology_counts_match_the_grid_formulas() {
        assert_eq!(plane(1.0).triangle_count(), 2);
        assert_eq!(cube(1.0).vertex_count(), 24);
        assert_eq!(cube(1.0).triangle_count(), 12);
        assert_eq!(sphere(0.5, 32, 16).triangle_count(), 32 * (16 * 2 - 2));
        assert_eq!(cylinder(0.5, 1.0, 32).triangle_count(), 32 * 4);
        assert_eq!(cone(0.5, 1.0, 32).triangle_count(), 32 * 2);
        assert_eq!(torus(0.5, 0.15, 32, 16).triangle_count(), 32 * 16 * 2);
    }

    #[test]
    fn degenerate_parameters_are_clamped_instead_of_panicking() {
        assert_eq!(sphere(0.0, 0, 0).triangle_count(), 3 * (2 * 2 - 2));
        assert!(cylinder(-1.0, 0.0, 1).validate().is_ok());
    }
}
