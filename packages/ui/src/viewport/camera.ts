import {
	DEG_TO_RAD,
	UP,
	add,
	cross,
	lookAt,
	multiply,
	normalized,
	perspective,
	scaled,
	sub,
	type Mat4,
	type Vec3,
} from "./math"

/** Orbit camera state. Angles in degrees, distance in metres, Z-up. */
export type Camera = {
	target: Vec3
	yawDeg: number
	pitchDeg: number
	distance: number
	fovDeg: number
}

export const DEFAULT_CAMERA: Camera = {
	target: [0, 0, 0],
	yawDeg: 40,
	pitchDeg: 24,
	distance: 8,
	fovDeg: 50,
}

export const MIN_DISTANCE = 0.1
export const MAX_DISTANCE = 2000
/** Never reach the pole: the up vector would flip and the view would spin. */
export const MAX_PITCH = 89
export const ORBIT_DEG_PER_PIXEL = 0.35
export const DOLLY_FACTOR = 1.12
export const NEAR_PLANE = 0.05
export const FAR_PLANE = 5000

function clamp(value: number, min: number, max: number): number {
	return Math.min(Math.max(value, min), max)
}

export function cameraEye(camera: Camera): Vec3 {
	const yaw = camera.yawDeg * DEG_TO_RAD
	const pitch = camera.pitchDeg * DEG_TO_RAD
	const horizontal = Math.cos(pitch) * camera.distance
	return [
		camera.target[0] + horizontal * Math.cos(yaw),
		camera.target[1] + horizontal * Math.sin(yaw),
		camera.target[2] + Math.sin(pitch) * camera.distance,
	]
}

export function cameraForward(camera: Camera): Vec3 {
	return normalized(sub(camera.target, cameraEye(camera)))
}

export function cameraRight(camera: Camera): Vec3 {
	return normalized(cross(cameraForward(camera), UP))
}

export function cameraUp(camera: Camera): Vec3 {
	return normalized(cross(cameraRight(camera), cameraForward(camera)))
}

export function orbit(camera: Camera, dx: number, dy: number): Camera {
	return {
		...camera,
		yawDeg: (camera.yawDeg - dx * ORBIT_DEG_PER_PIXEL) % 360,
		pitchDeg: clamp(camera.pitchDeg + dy * ORBIT_DEG_PER_PIXEL, -MAX_PITCH, MAX_PITCH),
	}
}

/** Metres covered by one pixel at the orbit target: keeps panning 1:1 with the cursor. */
export function worldPerPixel(camera: Camera, viewportHeight: number): number {
	if (viewportHeight <= 0) return 0
	return (2 * Math.tan((camera.fovDeg * DEG_TO_RAD) / 2) * camera.distance) / viewportHeight
}

export function pan(camera: Camera, dx: number, dy: number, viewportHeight: number): Camera {
	const unit = worldPerPixel(camera, viewportHeight)
	const offset = add(scaled(cameraRight(camera), -dx * unit), scaled(cameraUp(camera), dy * unit))
	return { ...camera, target: add(camera.target, offset) }
}

export function dolly(camera: Camera, steps: number): Camera {
	const distance = camera.distance * Math.pow(DOLLY_FACTOR, steps)
	return { ...camera, distance: clamp(distance, MIN_DISTANCE, MAX_DISTANCE) }
}

/** Frames a world box, the "F" command. Empty boxes fall back to a sane close-up. */
export function frame(camera: Camera, min: Vec3, max: Vec3): Camera {
	const center: Vec3 = [(min[0] + max[0]) / 2, (min[1] + max[1]) / 2, (min[2] + max[2]) / 2]
	const extent = sub(max, min)
	const radius = Math.max(extent[0], extent[1], extent[2], 0.001) / 2
	const fit = radius / Math.sin((camera.fovDeg * DEG_TO_RAD) / 2)
	return {
		...camera,
		target: center,
		distance: clamp(fit * 1.6, MIN_DISTANCE, MAX_DISTANCE),
	}
}

export function viewMatrix(camera: Camera): Mat4 {
	return lookAt(cameraEye(camera), camera.target, UP)
}

export function viewProjection(camera: Camera, width: number, height: number): Mat4 {
	const aspect = height > 0 ? width / height : 1
	return multiply(perspective(camera.fovDeg, aspect, NEAR_PLANE, FAR_PLANE), viewMatrix(camera))
}
