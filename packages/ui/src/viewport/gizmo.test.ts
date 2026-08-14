import { describe, expect, it } from "vitest"
import type { SceneNodeDto, TransformDto } from "@megu3d/types"

import {
	AXES,
	HANDLE_LENGTH,
	MIN_SCALE_FACTOR,
	axisHandles,
	dragScalar,
	pickAxis,
	withRotation,
	withScale,
	withTranslation,
	type AxisHandle,
} from "./gizmo"
import { lookAt, multiply, perspective, type Mat4 } from "./math"
import { pickNodeAt } from "./pick"

const WIDTH = 800
const HEIGHT = 600
const VP: Mat4 = multiply(
	perspective(50, WIDTH / HEIGHT, 0.1, 100),
	lookAt([6, -6, 4], [0, 0, 0], [0, 0, 1]),
)

const TRANSFORM: TransformDto = {
	translation: { x: 1, y: 2, z: 3 },
	rotationEulerDeg: { x: 0, y: 0, z: 45 },
	scale: { x: 2, y: 2, z: 2 },
}

function node(uuid: string, x: number, visible = true): SceneNodeDto {
	return {
		uuid,
		name: uuid,
		parent: null,
		children: [],
		depth: 0,
		kind: "mesh",
		transform: {
			translation: { x, y: 0, z: 0 },
			rotationEulerDeg: { x: 0, y: 0, z: 0 },
			scale: { x: 1, y: 1, z: 1 },
		},
		worldTranslation: { x, y: 0, z: 0 },
		worldBoundsMin: { x: x - 0.5, y: -0.5, z: -0.5 },
		worldBoundsMax: { x: x + 0.5, y: 0.5, z: 0.5 },
		mesh: null,
		visible,
		locked: false,
		selected: false,
	}
}

describe("transform gizmo", () => {
	const handles = axisHandles(VP, [0, 0, 0], WIDTH, HEIGHT)

	it("draws one screen-constant handle per axis", () => {
		expect(handles.map((handle) => handle.axis).sort()).toEqual([...AXES].sort())
		for (const handle of handles) {
			expect(Math.hypot(handle.tipX - handle.originX, handle.tipY - handle.originY)).toBeCloseTo(
				HANDLE_LENGTH,
				6,
			)
		}
	})

	it("picks the axis under the cursor and nothing far from it", () => {
		const target = handles[0]
		expect(target).toBeDefined()
		if (!target) return
		const midX = (target.originX + target.tipX) / 2
		const midY = (target.originY + target.tipY) / 2
		expect(pickAxis(handles, midX, midY)).toBe(target.axis)
		expect(pickAxis(handles, -400, -400)).toBeNull()
	})

	it("measures drag along the handle direction only", () => {
		const handle: AxisHandle = {
			axis: "x",
			originX: 0,
			originY: 0,
			tipX: HANDLE_LENGTH,
			tipY: 0,
			dirX: 1,
			dirY: 0,
		}
		expect(dragScalar(handle, 10, 5)).toBe(10)
		expect(dragScalar(handle, -10, 100)).toBe(-10)
	})

	it("applies translation, rotation and scale without mutating the input", () => {
		expect(withTranslation(TRANSFORM, [1, 0, -1]).translation).toEqual({ x: 2, y: 2, z: 2 })
		expect(withRotation(TRANSFORM, "z", 45).rotationEulerDeg.z).toBe(90)
		expect(withScale(TRANSFORM, "y", 1.5).scale).toEqual({ x: 2, y: 3, z: 2 })
		expect(TRANSFORM.translation).toEqual({ x: 1, y: 2, z: 3 })
	})

	it("never lets an axis scale collapse to zero", () => {
		expect(withScale(TRANSFORM, "x", 0).scale.x).toBe(MIN_SCALE_FACTOR)
	})
})

describe("viewport picking", () => {
	const nodes = [node("a", 0), node("b", 3), node("hidden", 0, false)]

	it("returns the node whose origin is under the cursor", () => {
		const handles = axisHandles(VP, [0, 0, 0], WIDTH, HEIGHT)
		expect(handles.length).toBeGreaterThan(0)
		const origin = handles[0]
		if (!origin) return
		expect(pickNodeAt(nodes, VP, WIDTH, HEIGHT, origin.originX, origin.originY)).toBe("a")
	})

	it("returns null on empty space and skips hidden nodes", () => {
		expect(pickNodeAt(nodes, VP, WIDTH, HEIGHT, 5, 5)).toBeNull()
		expect(pickNodeAt([node("ghost", 0, false)], VP, WIDTH, HEIGHT, WIDTH / 2, HEIGHT / 2)).toBeNull()
	})
})
