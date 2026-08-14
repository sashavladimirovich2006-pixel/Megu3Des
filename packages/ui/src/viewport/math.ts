/**
 * Minimal Z-up math for the viewport. Column-major, right-handed, degrees on the
 * API surface: the same conventions as `crates/megu3d-core/src/math.rs`, so the
 * preview and the future wgpu renderer agree on what a transform means.
 *
 * Tuples instead of arrays on purpose: `noUncheckedIndexedAccess` makes loose
 * `number[]` math a minefield of `undefined` checks.
 */

export type Vec3 = readonly [number, number, number]

export type Mat3 = readonly [number, number, number, number, number, number, number, number, number]

export type Mat4 = readonly [
	number,
	number,
	number,
	number,
	number,
	number,
	number,
	number,
	number,
	number,
	number,
	number,
	number,
	number,
	number,
	number,
]

export const DEG_TO_RAD = Math.PI / 180
export const RAD_TO_DEG = 180 / Math.PI

export const ZERO: Vec3 = [0, 0, 0]
export const UP: Vec3 = [0, 0, 1]

export const IDENTITY: Mat4 = [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]

function at(values: readonly number[], index: number): number {
	return values[index] ?? 0
}

function mat4(values: readonly number[]): Mat4 {
	return [
		at(values, 0),
		at(values, 1),
		at(values, 2),
		at(values, 3),
		at(values, 4),
		at(values, 5),
		at(values, 6),
		at(values, 7),
		at(values, 8),
		at(values, 9),
		at(values, 10),
		at(values, 11),
		at(values, 12),
		at(values, 13),
		at(values, 14),
		at(values, 15),
	]
}

export function vec3(x: number, y: number, z: number): Vec3 {
	return [x, y, z]
}

export function add(a: Vec3, b: Vec3): Vec3 {
	return [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

export function sub(a: Vec3, b: Vec3): Vec3 {
	return [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

export function scaled(v: Vec3, k: number): Vec3 {
	return [v[0] * k, v[1] * k, v[2] * k]
}

export function dot(a: Vec3, b: Vec3): number {
	return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

export function cross(a: Vec3, b: Vec3): Vec3 {
	return [
		a[1] * b[2] - a[2] * b[1],
		a[2] * b[0] - a[0] * b[2],
		a[0] * b[1] - a[1] * b[0],
	]
}

export function length(v: Vec3): number {
	return Math.sqrt(dot(v, v))
}

export function normalized(v: Vec3): Vec3 {
	const len = length(v)
	return len < 1e-8 ? ZERO : scaled(v, 1 / len)
}

export function multiply(a: Mat4, b: Mat4): Mat4 {
	const out: number[] = new Array<number>(16).fill(0)
	for (let column = 0; column < 4; column += 1) {
		for (let row = 0; row < 4; row += 1) {
			let sum = 0
			for (let k = 0; k < 4; k += 1) {
				sum += at(a, k * 4 + row) * at(b, column * 4 + k)
			}
			out[column * 4 + row] = sum
		}
	}
	return mat4(out)
}

function rotationX(radians: number): Mat4 {
	const c = Math.cos(radians)
	const s = Math.sin(radians)
	return [1, 0, 0, 0, 0, c, s, 0, 0, -s, c, 0, 0, 0, 0, 1]
}

function rotationY(radians: number): Mat4 {
	const c = Math.cos(radians)
	const s = Math.sin(radians)
	return [c, 0, -s, 0, 0, 1, 0, 0, s, 0, c, 0, 0, 0, 0, 1]
}

function rotationZ(radians: number): Mat4 {
	const c = Math.cos(radians)
	const s = Math.sin(radians)
	return [c, s, 0, 0, -s, c, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]
}

/** Euler XYZ applied as Rz * Ry * Rx, matching `quat_from_euler_deg` in the core. */
export function fromEulerDeg(euler: Vec3): Mat4 {
	return multiply(
		rotationZ(euler[2] * DEG_TO_RAD),
		multiply(rotationY(euler[1] * DEG_TO_RAD), rotationX(euler[0] * DEG_TO_RAD)),
	)
}

export function fromTrs(translation: Vec3, eulerDeg: Vec3, scale: Vec3): Mat4 {
	const rotation = fromEulerDeg(eulerDeg)
	const out: number[] = new Array<number>(16).fill(0)
	for (let column = 0; column < 3; column += 1) {
		const factor = at(scale, column)
		for (let row = 0; row < 3; row += 1) {
			out[column * 4 + row] = at(rotation, column * 4 + row) * factor
		}
	}
	out[12] = translation[0]
	out[13] = translation[1]
	out[14] = translation[2]
	out[15] = 1
	return mat4(out)
}

export function translationOf(m: Mat4): Vec3 {
	return [m[12], m[13], m[14]]
}

export function transformPoint(m: Mat4, point: Vec3): Vec3 {
	return [
		m[0] * point[0] + m[4] * point[1] + m[8] * point[2] + m[12],
		m[1] * point[0] + m[5] * point[1] + m[9] * point[2] + m[13],
		m[2] * point[0] + m[6] * point[1] + m[10] * point[2] + m[14],
	]
}

export function linearPart(m: Mat4): Mat3 {
	return [m[0], m[1], m[2], m[4], m[5], m[6], m[8], m[9], m[10]]
}

export function transformMat3(m: Mat3, v: Vec3): Vec3 {
	return [
		m[0] * v[0] + m[3] * v[1] + m[6] * v[2],
		m[1] * v[0] + m[4] * v[1] + m[7] * v[2],
		m[2] * v[0] + m[5] * v[1] + m[8] * v[2],
	]
}

/** Returns `null` for singular matrices instead of producing NaN geometry. */
export function invert3(m: Mat3): Mat3 | null {
	const a00 = m[0]
	const a01 = m[1]
	const a02 = m[2]
	const a10 = m[3]
	const a11 = m[4]
	const a12 = m[5]
	const a20 = m[6]
	const a21 = m[7]
	const a22 = m[8]
	const b01 = a22 * a11 - a12 * a21
	const b11 = -a22 * a10 + a12 * a20
	const b21 = a21 * a10 - a11 * a20
	const determinant = a00 * b01 + a01 * b11 + a02 * b21
	if (Math.abs(determinant) < 1e-12) return null
	const inverse = 1 / determinant
	return [
		b01 * inverse,
		(-a22 * a01 + a02 * a21) * inverse,
		(a12 * a01 - a02 * a11) * inverse,
		b11 * inverse,
		(a22 * a00 - a02 * a20) * inverse,
		(-a12 * a00 + a02 * a10) * inverse,
		b21 * inverse,
		(-a21 * a00 + a01 * a20) * inverse,
		(a11 * a00 - a01 * a10) * inverse,
	]
}

export function perspective(fovDeg: number, aspect: number, near: number, far: number): Mat4 {
	const f = 1 / Math.tan((fovDeg * DEG_TO_RAD) / 2)
	const safeAspect = aspect > 1e-6 ? aspect : 1
	const range = near - far
	return [f / safeAspect, 0, 0, 0, 0, f, 0, 0, 0, 0, (far + near) / range, -1, 0, 0, (2 * far * near) / range, 0]
}

export function lookAt(eye: Vec3, target: Vec3, up: Vec3): Mat4 {
	const forward = normalized(sub(target, eye))
	let side = normalized(cross(forward, up))
	if (length(side) < 1e-6) {
		// Looking straight down the up axis: pick any stable side vector.
		side = normalized(cross(forward, [0, 1, 0]))
	}
	const realUp = cross(side, forward)
	return [
		side[0],
		realUp[0],
		-forward[0],
		0,
		side[1],
		realUp[1],
		-forward[1],
		0,
		side[2],
		realUp[2],
		-forward[2],
		0,
		-dot(side, eye),
		-dot(realUp, eye),
		dot(forward, eye),
		1,
	]
}

export type Projected = { x: number; y: number; depth: number; behind: boolean }

/** Projects a world point to canvas pixels. `behind` points must not be drawn. */
export function project(viewProjection: Mat4, point: Vec3, width: number, height: number): Projected {
	const x = viewProjection[0] * point[0] + viewProjection[4] * point[1] + viewProjection[8] * point[2] + viewProjection[12]
	const y = viewProjection[1] * point[0] + viewProjection[5] * point[1] + viewProjection[9] * point[2] + viewProjection[13]
	const w = viewProjection[3] * point[0] + viewProjection[7] * point[1] + viewProjection[11] * point[2] + viewProjection[15]
	if (w <= 1e-6) {
		return { x: 0, y: 0, depth: w, behind: true }
	}
	return {
		x: (x / w * 0.5 + 0.5) * width,
		y: (1 - (y / w * 0.5 + 0.5)) * height,
		depth: w,
		behind: false,
	}
}

/** The eight corners of an axis-aligned box, in world space. */
export function boxCorners(min: Vec3, max: Vec3): Vec3[] {
	return [
		[min[0], min[1], min[2]],
		[max[0], min[1], min[2]],
		[max[0], max[1], min[2]],
		[min[0], max[1], min[2]],
		[min[0], min[1], max[2]],
		[max[0], min[1], max[2]],
		[max[0], max[1], max[2]],
		[min[0], max[1], max[2]],
	]
}

/** Corner index pairs for the twelve edges of a box. */
export const BOX_EDGES: ReadonlyArray<readonly [number, number]> = [
	[0, 1],
	[1, 2],
	[2, 3],
	[3, 0],
	[4, 5],
	[5, 6],
	[6, 7],
	[7, 4],
	[0, 4],
	[1, 5],
	[2, 6],
	[3, 7],
]
