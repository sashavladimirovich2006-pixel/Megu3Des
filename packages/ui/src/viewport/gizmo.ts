import type { TransformDto, Vec3Dto } from "@megu3d/types"

import { project, type Mat4, type Vec3 } from "./math"

export const TRANSFORM_MODES = ["move", "rotate", "scale"] as const
export type TransformMode = (typeof TRANSFORM_MODES)[number]

export const AXES = ["x", "y", "z"] as const
export type Axis = (typeof AXES)[number]

export const AXIS_VECTOR: Record<Axis, Vec3> = {
	x: [1, 0, 0],
	y: [0, 1, 0],
	z: [0, 0, 1],
}

/** Blender-compatible axis colours: red X, green Y, blue Z. */
export const AXIS_COLOR: Record<Axis, string> = {
	x: "#ff6b5b",
	y: "#7ddc4a",
	z: "#4c8dff",
}

/** Screen-constant gizmo size, so it stays grabbable at any zoom level. */
export const HANDLE_LENGTH = 78
export const PICK_RADIUS = 10
export const MIN_SCALE_FACTOR = 0.01

export type AxisHandle = {
	axis: Axis
	originX: number
	originY: number
	tipX: number
	tipY: number
	/** Unit direction in screen space; `null` when the axis points at the camera. */
	dirX: number
	dirY: number
}

/**
 * Projects the three world axes at `origin` into screen space and rescales them
 * to a fixed pixel length. Axes pointing almost straight at the camera collapse
 * to a dot and are dropped: they cannot be dragged meaningfully.
 */
export function axisHandles(
	viewProjection: Mat4,
	origin: Vec3,
	width: number,
	height: number,
	length = HANDLE_LENGTH,
): AxisHandle[] {
	const base = project(viewProjection, origin, width, height)
	if (base.behind) return []
	const handles: AxisHandle[] = []
	for (const axis of AXES) {
		const direction = AXIS_VECTOR[axis]
		const tip = project(
			viewProjection,
			[origin[0] + direction[0], origin[1] + direction[1], origin[2] + direction[2]],
			width,
			height,
		)
		if (tip.behind) continue
		const dx = tip.x - base.x
		const dy = tip.y - base.y
		const len = Math.hypot(dx, dy)
		if (len < 1e-3) continue
		const unitX = dx / len
		const unitY = dy / len
		handles.push({
			axis,
			originX: base.x,
			originY: base.y,
			tipX: base.x + unitX * length,
			tipY: base.y + unitY * length,
			dirX: unitX,
			dirY: unitY,
		})
	}
	return handles
}

function distanceToSegment(handle: AxisHandle, x: number, y: number): number {
	const dx = handle.tipX - handle.originX
	const dy = handle.tipY - handle.originY
	const lengthSquared = dx * dx + dy * dy
	if (lengthSquared < 1e-6) return Math.hypot(x - handle.originX, y - handle.originY)
	const t = Math.min(
		1,
		Math.max(0, ((x - handle.originX) * dx + (y - handle.originY) * dy) / lengthSquared),
	)
	return Math.hypot(x - (handle.originX + dx * t), y - (handle.originY + dy * t))
}

/** Nearest axis handle under the cursor, or `null` when the click misses the gizmo. */
export function pickAxis(
	handles: AxisHandle[],
	x: number,
	y: number,
	radius = PICK_RADIUS,
): Axis | null {
	let best: Axis | null = null
	let bestDistance = radius
	for (const handle of handles) {
		const distance = distanceToSegment(handle, x, y)
		if (distance <= bestDistance) {
			bestDistance = distance
			best = handle.axis
		}
	}
	return best
}

/** Cursor travel projected onto the handle direction, in pixels. */
export function dragScalar(handle: AxisHandle, dx: number, dy: number): number {
	return dx * handle.dirX + dy * handle.dirY
}

export function axisIndex(axis: Axis): 0 | 1 | 2 {
	return axis === "x" ? 0 : axis === "y" ? 1 : 2
}

function componentOf(value: Vec3Dto, axis: Axis): number {
	return axis === "x" ? value.x : axis === "y" ? value.y : value.z
}

function withComponent(value: Vec3Dto, axis: Axis, next: number): Vec3Dto {
	return {
		x: axis === "x" ? next : value.x,
		y: axis === "y" ? next : value.y,
		z: axis === "z" ? next : value.z,
	}
}

export function withTranslation(transform: TransformDto, delta: Vec3): TransformDto {
	return {
		...transform,
		translation: {
			x: transform.translation.x + delta[0],
			y: transform.translation.y + delta[1],
			z: transform.translation.z + delta[2],
		},
	}
}

/** Rotation is applied around the node's own axis, the same value the Properties panel edits. */
export function withRotation(transform: TransformDto, axis: Axis, degrees: number): TransformDto {
	const current = componentOf(transform.rotationEulerDeg, axis)
	return {
		...transform,
		rotationEulerDeg: withComponent(transform.rotationEulerDeg, axis, current + degrees),
	}
}

/** Zero scale is rejected by the core, so the factor is clamped before it is sent. */
export function withScale(transform: TransformDto, axis: Axis, factor: number): TransformDto {
	const current = componentOf(transform.scale, axis)
	const next = current * factor
	const safe =
		Math.abs(next) < MIN_SCALE_FACTOR ? Math.sign(next || 1) * MIN_SCALE_FACTOR : next
	return { ...transform, scale: withComponent(transform.scale, axis, safe) }
}
