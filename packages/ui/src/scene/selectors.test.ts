import { describe, expect, it } from "vitest"
import type { SceneNodeDto, SceneSnapshotDto, Vec3Dto } from "@megu3d/types"

import { transformPoint } from "../viewport/math"
import {
	activeNode,
	boundsOf,
	formatNumber,
	kindBadge,
	nodeByUuid,
	parentWorldMatrix,
	selectedNodes,
	selectionBounds,
	worldToLocalDelta,
} from "./selectors"

function v(x: number, y: number, z: number): Vec3Dto {
	return { x, y, z }
}

function node(uuid: string, overrides: Partial<SceneNodeDto> = {}): SceneNodeDto {
	return {
		uuid,
		name: uuid,
		parent: null,
		children: [],
		depth: 0,
		kind: "mesh",
		transform: { translation: v(0, 0, 0), rotationEulerDeg: v(0, 0, 0), scale: v(1, 1, 1) },
		worldTranslation: v(0, 0, 0),
		worldBoundsMin: v(-1, -1, -1),
		worldBoundsMax: v(1, 1, 1),
		mesh: null,
		visible: true,
		locked: false,
		selected: false,
		...overrides,
	}
}

function snapshot(nodes: SceneNodeDto[], active: string | null): SceneSnapshotDto {
	const selection = nodes.filter((entry) => entry.selected).map((entry) => entry.uuid)
	return {
		schemaVersion: "0.2.0",
		nodes,
		roots: nodes.filter((entry) => entry.parent === null).map((entry) => entry.uuid),
		selection,
		active,
		stats: {
			nodeCount: nodes.length,
			selectionCount: selection.length,
			triangleCount: 0,
			schemaVersion: "0.2.0",
		},
		history: {
			canUndo: false,
			canRedo: false,
			undoDepth: 0,
			undoLimit: 64,
			lastLabel: null,
			previewActive: false,
		},
	}
}

const parent = node("parent", {
	transform: { translation: v(1, 0, 0), rotationEulerDeg: v(0, 0, 90), scale: v(1, 1, 1) },
	children: ["child"],
	worldTranslation: v(1, 0, 0),
})
const child = node("child", {
	parent: "parent",
	depth: 1,
	selected: true,
	worldTranslation: v(1, 1, 0),
	worldBoundsMin: v(2, 2, 2),
	worldBoundsMax: v(4, 4, 4),
})
const scene = snapshot([parent, child], "child")

describe("scene selectors", () => {
	it("resolves nodes, the active node and the selection", () => {
		expect(nodeByUuid(scene, "parent")?.name).toBe("parent")
		expect(nodeByUuid(scene, "missing")).toBeNull()
		expect(nodeByUuid(null, "parent")).toBeNull()
		expect(activeNode(scene)?.uuid).toBe("child")
		expect(selectedNodes(scene).map((entry) => entry.uuid)).toEqual(["child"])
	})

	it("composes the parent world matrix from local transforms", () => {
		const world = transformPoint(parentWorldMatrix(scene, child), [1, 0, 0])
		expect(world[0]).toBeCloseTo(1, 6)
		expect(world[1]).toBeCloseTo(1, 6)
		expect(world[2]).toBeCloseTo(0, 6)
	})

	it("converts a world drag into the parent space of the node", () => {
		const local = worldToLocalDelta(scene, child, [1, 0, 0])
		expect(local[0]).toBeCloseTo(0, 6)
		expect(local[1]).toBeCloseTo(-1, 6)
		expect(local[2]).toBeCloseTo(0, 6)
		expect(worldToLocalDelta(scene, parent, [1, 2, 3])).toEqual([1, 2, 3])
	})

	it("unions world bounds and falls back to the whole scene", () => {
		expect(boundsOf([parent, child])).toEqual({ min: [-1, -1, -1], max: [4, 4, 4] })
		expect(boundsOf([])).toBeNull()
		expect(selectionBounds(scene)).toEqual({ min: [2, 2, 2], max: [4, 4, 4] })
		expect(selectionBounds(snapshot([parent], null))).toEqual({ min: [-1, -1, -1], max: [1, 1, 1] })
	})

	it("formats numbers for the inspector", () => {
		expect(formatNumber(1.23456789)).toBe("1.235")
		expect(formatNumber(-0)).toBe("0")
		expect(formatNumber(2)).toBe("2")
		expect(formatNumber(Number.NaN)).toBe("0")
		expect(kindBadge("camera")).toBe("C")
	})
})
