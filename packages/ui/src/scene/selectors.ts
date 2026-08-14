import type { MessageKey } from "@megu3d/i18n"
import type { NodeKindDto, SceneNodeDto, SceneSnapshotDto, Vec3Dto } from "@megu3d/types"

import { IDENTITY, fromTrs, invert3, linearPart, multiply, transformMat3, type Mat4, type Vec3 } from "../viewport/math"

export type Bounds = { min: Vec3; max: Vec3 }

export function toVec3(value: Vec3Dto): Vec3 {
	return [value.x, value.y, value.z]
}

export function toVec3Dto(value: Vec3): Vec3Dto {
	return { x: value[0], y: value[1], z: value[2] }
}

export function nodeIndex(scene: SceneSnapshotDto | null): Map<string, SceneNodeDto> {
	const index = new Map<string, SceneNodeDto>()
	for (const node of scene?.nodes ?? []) index.set(node.uuid, node)
	return index
}

export function nodeByUuid(scene: SceneSnapshotDto | null, uuid: string | null): SceneNodeDto | null {
	if (!scene || !uuid) return null
	return scene.nodes.find((node) => node.uuid === uuid) ?? null
}

export function activeNode(scene: SceneSnapshotDto | null): SceneNodeDto | null {
	return nodeByUuid(scene, scene?.active ?? null)
}

export function selectedNodes(scene: SceneSnapshotDto | null): SceneNodeDto[] {
	return scene ? scene.nodes.filter((node) => node.selected) : []
}

export function localMatrix(node: SceneNodeDto): Mat4 {
	return fromTrs(
		toVec3(node.transform.translation),
		toVec3(node.transform.rotationEulerDeg),
		toVec3(node.transform.scale),
	)
}

/**
 * World matrix of a node's parent, composed from the local transforms in the
 * snapshot. Needed to convert a world-space gizmo drag into the local delta the
 * core expects.
 */
export function parentWorldMatrix(scene: SceneSnapshotDto | null, node: SceneNodeDto): Mat4 {
	const index = nodeIndex(scene)
	const chain: SceneNodeDto[] = []
	let current = node.parent ? index.get(node.parent) : undefined
	while (current) {
		chain.push(current)
		current = current.parent ? index.get(current.parent) : undefined
	}
	let world: Mat4 = IDENTITY
	for (const ancestor of chain.reverse()) {
		world = multiply(world, localMatrix(ancestor))
	}
	return world
}

/** Converts a world-space translation delta into the node's parent space. */
export function worldToLocalDelta(
	scene: SceneSnapshotDto | null,
	node: SceneNodeDto,
	worldDelta: Vec3,
): Vec3 {
	if (!node.parent) return worldDelta
	const inverse = invert3(linearPart(parentWorldMatrix(scene, node)))
	return inverse ? transformMat3(inverse, worldDelta) : worldDelta
}

export function boundsOf(nodes: SceneNodeDto[]): Bounds | null {
	if (nodes.length === 0) return null
	let min: Vec3 = [Number.POSITIVE_INFINITY, Number.POSITIVE_INFINITY, Number.POSITIVE_INFINITY]
	let max: Vec3 = [Number.NEGATIVE_INFINITY, Number.NEGATIVE_INFINITY, Number.NEGATIVE_INFINITY]
	for (const node of nodes) {
		const nodeMin = toVec3(node.worldBoundsMin)
		const nodeMax = toVec3(node.worldBoundsMax)
		min = [Math.min(min[0], nodeMin[0]), Math.min(min[1], nodeMin[1]), Math.min(min[2], nodeMin[2])]
		max = [Math.max(max[0], nodeMax[0]), Math.max(max[1], nodeMax[1]), Math.max(max[2], nodeMax[2])]
	}
	return { min, max }
}

export function selectionBounds(scene: SceneSnapshotDto | null): Bounds | null {
	const selected = selectedNodes(scene)
	return boundsOf(selected.length > 0 ? selected : (scene?.nodes ?? []))
}

const KIND_KEYS: Record<NodeKindDto, MessageKey> = {
	empty: "kind.empty",
	mesh: "kind.mesh",
	camera: "kind.camera",
	light: "kind.light",
}

export function kindMessageKey(kind: NodeKindDto): MessageKey {
	return KIND_KEYS[kind]
}

/** One-letter outliner badge; readable at 13 px without an icon font. */
export function kindBadge(kind: NodeKindDto): string {
	return kind === "mesh" ? "M" : kind === "camera" ? "C" : kind === "light" ? "L" : "E"
}

export function formatNumber(value: number, digits = 3): string {
	if (!Number.isFinite(value)) return "0"
	const rounded = Number(value.toFixed(digits))
	return Object.is(rounded, -0) ? "0" : String(rounded)
}
