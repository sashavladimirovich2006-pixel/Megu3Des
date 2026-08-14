import { describe, expect, it } from "vitest"

import {
	IDENTITY,
	fromEulerDeg,
	fromTrs,
	invert3,
	linearPart,
	lookAt,
	multiply,
	normalized,
	perspective,
	project,
	transformMat3,
	transformPoint,
	type Mat4,
	type Vec3,
} from "./math"

const WIDTH = 800
const HEIGHT = 600

function testViewProjection(eye: Vec3): Mat4 {
	return multiply(perspective(50, WIDTH / HEIGHT, 0.1, 100), lookAt(eye, [0, 0, 0], [0, 0, 1]))
}

describe("viewport math", () => {
	it("keeps identity neutral", () => {
		const m = fromTrs([1, 2, 3], [10, 20, 30], [1, 2, 3])
		expect(multiply(IDENTITY, m)).toEqual(m)
		expect(multiply(m, IDENTITY)).toEqual(m)
	})

	it("rotates around Z the same way the core does", () => {
		const rotated = transformPoint(fromEulerDeg([0, 0, 90]), [1, 0, 0])
		expect(rotated[0]).toBeCloseTo(0, 6)
		expect(rotated[1]).toBeCloseTo(1, 6)
		expect(rotated[2]).toBeCloseTo(0, 6)
	})

	it("composes TRS as scale, then rotation, then translation", () => {
		const point = transformPoint(fromTrs([0, 0, 5], [0, 0, 90], [2, 2, 2]), [1, 0, 0])
		expect(point[0]).toBeCloseTo(0, 6)
		expect(point[1]).toBeCloseTo(2, 6)
		expect(point[2]).toBeCloseTo(5, 6)
	})

	it("composes parent and child transforms", () => {
		const parent = fromTrs([1, 0, 0], [0, 0, 90], [1, 1, 1])
		const child = fromTrs([1, 0, 0], [0, 0, 0], [1, 1, 1])
		const world = transformPoint(multiply(parent, child), [0, 0, 0])
		expect(world[0]).toBeCloseTo(1, 6)
		expect(world[1]).toBeCloseTo(1, 6)
		expect(world[2]).toBeCloseTo(0, 6)
	})

	it("inverts the linear part and refuses singular matrices", () => {
		const linear = linearPart(fromTrs([3, 1, 2], [15, 30, 45], [2, 0.5, 1.5]))
		const inverse = invert3(linear)
		expect(inverse).not.toBeNull()
		if (!inverse) return
		const source: Vec3 = [0.3, -1.2, 4]
		const roundTrip = transformMat3(inverse, transformMat3(linear, source))
		expect(roundTrip[0]).toBeCloseTo(source[0], 6)
		expect(roundTrip[1]).toBeCloseTo(source[1], 6)
		expect(roundTrip[2]).toBeCloseTo(source[2], 6)
		expect(invert3(linearPart(fromTrs([0, 0, 0], [0, 0, 0], [0, 1, 1])))).toBeNull()
	})

	it("projects the look-at target to the centre of the canvas", () => {
		const vp = testViewProjection([0, -5, 0])
		const centre = project(vp, [0, 0, 0], WIDTH, HEIGHT)
		expect(centre.behind).toBe(false)
		expect(centre.x).toBeCloseTo(WIDTH / 2, 4)
		expect(centre.y).toBeCloseTo(HEIGHT / 2, 4)
	})

	it("puts +X to the right, +Z up, and flags points behind the camera", () => {
		const vp = testViewProjection([0, -5, 0])
		expect(project(vp, [1, 0, 0], WIDTH, HEIGHT).x).toBeGreaterThan(WIDTH / 2)
		expect(project(vp, [0, 0, 1], WIDTH, HEIGHT).y).toBeLessThan(HEIGHT / 2)
		expect(project(vp, [0, -10, 0], WIDTH, HEIGHT).behind).toBe(true)
	})

	it("normalizes degenerate vectors to zero instead of NaN", () => {
		expect(normalized([0, 0, 0])).toEqual([0, 0, 0])
	})
})
