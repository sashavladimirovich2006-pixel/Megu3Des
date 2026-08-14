import type { SceneNodeDto } from "@megu3d/types"

import { boxCorners, project, type Mat4 } from "./math"

/** Click tolerance around a node origin, in CSS pixels. */
export const ORIGIN_PICK_RADIUS = 16

export type ScreenBox = { minX: number; minY: number; maxX: number; maxY: number; area: number }

export function projectedBox(
	node: SceneNodeDto,
	viewProjection: Mat4,
	width: number,
	height: number,
): ScreenBox | null {
	const corners = boxCorners(
		[node.worldBoundsMin.x, node.worldBoundsMin.y, node.worldBoundsMin.z],
		[node.worldBoundsMax.x, node.worldBoundsMax.y, node.worldBoundsMax.z],
	)
	let minX = Number.POSITIVE_INFINITY
	let minY = Number.POSITIVE_INFINITY
	let maxX = Number.NEGATIVE_INFINITY
	let maxY = Number.NEGATIVE_INFINITY
	for (const corner of corners) {
		const point = project(viewProjection, corner, width, height)
		if (point.behind) return null
		minX = Math.min(minX, point.x)
		minY = Math.min(minY, point.y)
		maxX = Math.max(maxX, point.x)
		maxY = Math.max(maxY, point.y)
	}
	return { minX, minY, maxX, maxY, area: Math.max(maxX - minX, 1) * Math.max(maxY - minY, 1) }
}

/**
 * Screen-space picking without a GPU id-buffer: origins win by proximity, and
 * overlapping boxes are resolved by area so a small object inside a big one is
 * still reachable. Replaced by GPU picking when the wgpu viewport lands.
 */
export function pickNodeAt(
	nodes: readonly SceneNodeDto[],
	viewProjection: Mat4,
	width: number,
	height: number,
	x: number,
	y: number,
	radius = ORIGIN_PICK_RADIUS,
): string | null {
	let nearest: string | null = null
	let nearestDistance = radius
	let boxed: string | null = null
	let boxedArea = Number.POSITIVE_INFINITY
	for (const node of nodes) {
		if (!node.visible) continue
		const origin = project(
			viewProjection,
			[node.worldTranslation.x, node.worldTranslation.y, node.worldTranslation.z],
			width,
			height,
		)
		if (!origin.behind) {
			const distance = Math.hypot(x - origin.x, y - origin.y)
			if (distance <= nearestDistance) {
				nearestDistance = distance
				nearest = node.uuid
			}
		}
		if (node.mesh === null) continue
		const box = projectedBox(node, viewProjection, width, height)
		if (!box) continue
		if (x >= box.minX && x <= box.maxX && y >= box.minY && y <= box.maxY && box.area < boxedArea) {
			boxedArea = box.area
			boxed = node.uuid
		}
	}
	return nearest ?? boxed
}
