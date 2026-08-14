//! Minimal Z-up, right-handed linear algebra: vectors, quaternions, 4x4
//! matrices and axis-aligned bounds.
//!
//! Dependency-free on purpose (`docs/assumptions.md`, `D-80`): the scene graph
//! needs a few hundred flops per frame, and a hand-rolled module keeps the
//! math conventions (`D-30`..`D-32`) visible and unit-tested. If profiling ever
//! shows this is hot, swap the internals for `glam` behind the same API.

use serde::{Deserialize, Serialize};

/// `[x, y, z]` in metres.
pub type Vec3 = [f32; 3];
/// Quaternion `[x, y, z, w]`.
pub type Quat = [f32; 4];

pub const DEG_TO_RAD: f32 = std::f32::consts::PI / 180.0;
pub const RAD_TO_DEG: f32 = 180.0 / std::f32::consts::PI;
pub const IDENTITY_QUAT: Quat = [0.0, 0.0, 0.0, 1.0];

pub fn add(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

pub fn sub(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

pub fn scaled(v: Vec3, factor: f32) -> Vec3 {
    [v[0] * factor, v[1] * factor, v[2] * factor]
}

pub fn dot(a: Vec3, b: Vec3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

pub fn cross(a: Vec3, b: Vec3) -> Vec3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

pub fn length(v: Vec3) -> f32 {
    dot(v, v).sqrt()
}

pub fn normalized(v: Vec3) -> Vec3 {
    let len = length(v);
    if len <= f32::EPSILON {
        return [0.0, 0.0, 0.0];
    }
    scaled(v, 1.0 / len)
}

/// Hamilton product: `quat_mul(a, b)` rotates by `b` first, then by `a`.
pub fn quat_mul(a: Quat, b: Quat) -> Quat {
    [
        a[3] * b[0] + a[0] * b[3] + a[1] * b[2] - a[2] * b[1],
        a[3] * b[1] - a[0] * b[2] + a[1] * b[3] + a[2] * b[0],
        a[3] * b[2] + a[0] * b[1] - a[1] * b[0] + a[2] * b[3],
        a[3] * b[3] - a[0] * b[0] - a[1] * b[1] - a[2] * b[2],
    ]
}

pub fn quat_normalized(q: Quat) -> Quat {
    let len = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
    if len <= f32::EPSILON {
        return IDENTITY_QUAT;
    }
    [q[0] / len, q[1] / len, q[2] / len, q[3] / len]
}

pub fn quat_from_axis_angle(axis: Vec3, radians: f32) -> Quat {
    let axis = normalized(axis);
    let half = radians / 2.0;
    let sin = half.sin();
    [axis[0] * sin, axis[1] * sin, axis[2] * sin, half.cos()]
}

/// Euler angles in degrees, XYZ order: X is applied first, then Y, then Z
/// (`R = Rz * Ry * Rx`). Degrees are the UI unit; the scene stores quaternions
/// and all internal math is in radians (`D-32`).
pub fn quat_from_euler_deg(euler_deg: Vec3) -> Quat {
    let qx = quat_from_axis_angle([1.0, 0.0, 0.0], euler_deg[0] * DEG_TO_RAD);
    let qy = quat_from_axis_angle([0.0, 1.0, 0.0], euler_deg[1] * DEG_TO_RAD);
    let qz = quat_from_axis_angle([0.0, 0.0, 1.0], euler_deg[2] * DEG_TO_RAD);
    quat_mul(quat_mul(qz, qy), qx)
}

/// Inverse of [`quat_from_euler_deg`]. At gimbal lock the roll is folded into X.
pub fn quat_to_euler_deg(q: Quat) -> Vec3 {
    let [x, y, z, w] = quat_normalized(q);
    let r00 = 1.0 - 2.0 * (y * y + z * z);
    let r01 = 2.0 * (x * y - z * w);
    let r10 = 2.0 * (x * y + z * w);
    let r11 = 1.0 - 2.0 * (x * x + z * z);
    let r20 = 2.0 * (x * z - y * w);
    let r21 = 2.0 * (y * z + x * w);
    let r22 = 1.0 - 2.0 * (x * x + y * y);
    if r20.abs() > 0.999_999 {
        let sign = -r20.signum();
        let pitch = sign * std::f32::consts::FRAC_PI_2;
        let roll = if sign > 0.0 {
            r01.atan2(r11)
        } else {
            (-r01).atan2(r11)
        };
        return [roll * RAD_TO_DEG, pitch * RAD_TO_DEG, 0.0];
    }
    [
        r21.atan2(r22) * RAD_TO_DEG,
        (-r20).asin() * RAD_TO_DEG,
        r10.atan2(r00) * RAD_TO_DEG,
    ]
}

/// Column-major 4x4 matrix: the same layout the GPU expects, so viewport
/// uploads in M3.2 need no transpose.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mat4 {
    pub cols: [f32; 16],
}

impl Default for Mat4 {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Mat4 {
    pub const IDENTITY: Mat4 = Mat4 {
        cols: [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ],
    };

    /// `M = T * R * S`, the only composition order Megu3D uses.
    pub fn from_trs(translation: Vec3, rotation: Quat, scale: Vec3) -> Mat4 {
        let [x, y, z, w] = quat_normalized(rotation);
        let rotation_cols = [
            [
                1.0 - 2.0 * (y * y + z * z),
                2.0 * (x * y + z * w),
                2.0 * (x * z - y * w),
            ],
            [
                2.0 * (x * y - z * w),
                1.0 - 2.0 * (x * x + z * z),
                2.0 * (y * z + x * w),
            ],
            [
                2.0 * (x * z + y * w),
                2.0 * (y * z - x * w),
                1.0 - 2.0 * (x * x + y * y),
            ],
        ];
        let mut cols = [0.0f32; 16];
        for axis in 0..3 {
            let column = rotation_cols[axis];
            let factor = scale[axis];
            cols[axis * 4] = column[0] * factor;
            cols[axis * 4 + 1] = column[1] * factor;
            cols[axis * 4 + 2] = column[2] * factor;
        }
        cols[12] = translation[0];
        cols[13] = translation[1];
        cols[14] = translation[2];
        cols[15] = 1.0;
        Mat4 { cols }
    }

    /// `self * rhs`: `rhs` is applied to the point first.
    pub fn mul(&self, rhs: &Mat4) -> Mat4 {
        let mut cols = [0.0f32; 16];
        for column in 0..4 {
            for row in 0..4 {
                let mut sum = 0.0;
                for step in 0..4 {
                    sum += self.cols[step * 4 + row] * rhs.cols[column * 4 + step];
                }
                cols[column * 4 + row] = sum;
            }
        }
        Mat4 { cols }
    }

    pub fn transform_point(&self, point: Vec3) -> Vec3 {
        [
            self.cols[0] * point[0] + self.cols[4] * point[1] + self.cols[8] * point[2] + self.cols[12],
            self.cols[1] * point[0] + self.cols[5] * point[1] + self.cols[9] * point[2] + self.cols[13],
            self.cols[2] * point[0] + self.cols[6] * point[1] + self.cols[10] * point[2] + self.cols[14],
        ]
    }

    pub fn translation(&self) -> Vec3 {
        [self.cols[12], self.cols[13], self.cols[14]]
    }
}

/// Axis-aligned bounding box. An empty box has `min > max` and is skipped by
/// framing and picking.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Default for Aabb {
    fn default() -> Self {
        Self::EMPTY
    }
}

impl Aabb {
    pub const EMPTY: Aabb = Aabb {
        min: [f32::INFINITY; 3],
        max: [f32::NEG_INFINITY; 3],
    };

    pub fn point(point: Vec3) -> Aabb {
        Aabb {
            min: point,
            max: point,
        }
    }

    pub fn from_min_max(min: Vec3, max: Vec3) -> Aabb {
        Aabb { min, max }
    }

    pub fn is_empty(&self) -> bool {
        (0..3).any(|axis| self.min[axis] > self.max[axis])
    }

    pub fn expand(&mut self, point: Vec3) {
        for axis in 0..3 {
            self.min[axis] = self.min[axis].min(point[axis]);
            self.max[axis] = self.max[axis].max(point[axis]);
        }
    }

    pub fn union(&self, other: &Aabb) -> Aabb {
        if self.is_empty() {
            return *other;
        }
        if other.is_empty() {
            return *self;
        }
        let mut result = *self;
        result.expand(other.min);
        result.expand(other.max);
        result
    }

    /// Bounds of the transformed box: all eight corners, so rotation is handled.
    pub fn transformed(&self, matrix: &Mat4) -> Aabb {
        if self.is_empty() {
            return *self;
        }
        let mut result = Aabb::EMPTY;
        for corner in 0..8u8 {
            let point = [
                if corner & 1 == 0 { self.min[0] } else { self.max[0] },
                if corner & 2 == 0 { self.min[1] } else { self.max[1] },
                if corner & 4 == 0 { self.min[2] } else { self.max[2] },
            ];
            result.expand(matrix.transform_point(point));
        }
        result
    }

    pub fn center(&self) -> Vec3 {
        if self.is_empty() {
            return [0.0; 3];
        }
        scaled(add(self.min, self.max), 0.5)
    }

    pub fn size(&self) -> Vec3 {
        if self.is_empty() {
            return [0.0; 3];
        }
        sub(self.max, self.min)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    fn close_vec(a: Vec3, b: Vec3) -> bool {
        (0..3).all(|axis| close(a[axis], b[axis]))
    }

    #[test]
    fn yaw_of_ninety_degrees_maps_x_to_y() {
        let matrix = Mat4::from_trs([0.0; 3], quat_from_euler_deg([0.0, 0.0, 90.0]), [1.0; 3]);
        assert!(close_vec(matrix.transform_point([1.0, 0.0, 0.0]), [0.0, 1.0, 0.0]));
    }

    #[test]
    fn euler_survives_a_round_trip() {
        for euler in [
            [0.0, 0.0, 0.0],
            [30.0, 45.0, 60.0],
            [-15.0, 80.0, 170.0],
            [12.0, -89.0, -33.0],
        ] {
            let restored = quat_to_euler_deg(quat_from_euler_deg(euler));
            let a = Mat4::from_trs([0.0; 3], quat_from_euler_deg(euler), [1.0; 3]);
            let b = Mat4::from_trs([0.0; 3], quat_from_euler_deg(restored), [1.0; 3]);
            for probe in [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]] {
                assert!(
                    close_vec(a.transform_point(probe), b.transform_point(probe)),
                    "euler {euler:?} restored as {restored:?}"
                );
            }
        }
    }

    #[test]
    fn trs_scales_then_rotates_then_translates() {
        let matrix = Mat4::from_trs(
            [1.0, 2.0, 3.0],
            quat_from_euler_deg([0.0, 0.0, 90.0]),
            [2.0, 2.0, 2.0],
        );
        assert!(close_vec(matrix.transform_point([1.0, 0.0, 0.0]), [1.0, 4.0, 3.0]));
        assert!(close_vec(matrix.translation(), [1.0, 2.0, 3.0]));
    }

    #[test]
    fn parent_child_matrices_compose() {
        let parent = Mat4::from_trs([0.0, 0.0, 1.0], IDENTITY_QUAT, [1.0; 3]);
        let child = Mat4::from_trs([1.0, 0.0, 0.0], IDENTITY_QUAT, [1.0; 3]);
        let world = parent.mul(&child);
        assert!(close_vec(world.transform_point([0.0; 3]), [1.0, 0.0, 1.0]));
    }

    #[test]
    fn rotated_bounds_cover_every_corner() {
        let box3 = Aabb::from_min_max([-1.0, -1.0, -1.0], [1.0, 1.0, 1.0]);
        let rotated = box3.transformed(&Mat4::from_trs(
            [0.0; 3],
            quat_from_euler_deg([0.0, 0.0, 45.0]),
            [1.0; 3],
        ));
        assert!(close(rotated.max[0], 2.0f32.sqrt()));
        assert!(close_vec(rotated.center(), [0.0; 3]));
        assert!(Aabb::EMPTY.is_empty());
        assert!(!box3.is_empty());
    }
}
