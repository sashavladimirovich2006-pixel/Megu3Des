import { describe, expect, it } from "vitest"

import {
	DEFAULT_CAMERA,
	MAX_DISTANCE,
	MAX_PITCH,
	MIN_DISTANCE,
	cameraEye,
	dolly,
	frame,
	orbit,
	pan,
	viewProjection,
	worldPerPixel,
	type Camera,
} from "./camera"
import { project } from "./math"

const FLAT: Camera = { target: [0, 0, 0], yawDeg: 0, pitchDeg: 0, distance: 5, fovDeg: 50 }

describe("orbit camera", () => {
	it("places the eye on the Z-up sphere around the target", () => {
		const eye = cameraEye(FLAT)
		expect(eye[0]).toBeCloseTo(5, 6)
		expect(eye[1]).toBeCloseTo(0, 6)
		expect(eye[2]).toBeCloseTo(0, 6)
		expect(cameraEye({ ...FLAT, pitchDeg: 90 })[2]).toBeCloseTo(5, 6)
	})

	it("clamps pitch so the view never flips over the pole", () => {
		expect(orbit(FLAT, 0, 10_000).pitchDeg).toBe(MAX_PITCH)
		expect(orbit(FLAT, 0, -10_000).pitchDeg).toBe(-MAX_PITCH)
	})

	it("clamps dolly distance", () => {
		expect(dolly(FLAT, 500).distance).toBe(MAX_DISTANCE)
		expect(dolly(FLAT, -500).distance).toBe(MIN_DISTANCE)
		expect(dolly(FLAT, -1).distance).toBeLessThan(FLAT.distance)
	})

	it("pans the target across the view plane, not along it", () => {
		const panned = pan(FLAT, 10, 0, 600)
		expect(panned.target[1]).toBeLessThan(0)
		expect(panned.target[0]).toBeCloseTo(0, 6)
		expect(panned.distance).toBe(FLAT.distance)
	})

	it("frames a box by centring it and backing off far enough", () => {
		const framed = frame(DEFAULT_CAMERA, [1, 1, 1], [3, 3, 3])
		expect(framed.target).toEqual([2, 2, 2])
		expect(framed.distance).toBeGreaterThan(1)
		expect(framed.distance).toBeLessThanOrEqual(MAX_DISTANCE)
		expect(frame(DEFAULT_CAMERA, [0, 0, 0], [0, 0, 0]).distance).toBeGreaterThanOrEqual(MIN_DISTANCE)
	})

	it("scales pixel-to-metre conversion with distance", () => {
		expect(worldPerPixel(FLAT, 600)).toBeGreaterThan(0)
		expect(worldPerPixel({ ...FLAT, distance: 10 }, 600)).toBeCloseTo(
			worldPerPixel(FLAT, 600) * 2,
			9,
		)
		expect(worldPerPixel(FLAT, 0)).toBe(0)
	})

	it("keeps the target centred in the projection", () => {
		const camera: Camera = { ...DEFAULT_CAMERA, target: [1, 2, 3] }
		const centre = project(viewProjection(camera, 800, 600), camera.target, 800, 600)
		expect(centre.x).toBeCloseTo(400, 4)
		expect(centre.y).toBeCloseTo(300, 4)
	})
})
