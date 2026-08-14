import {
	MIN_SIZE,
	type Layout,
	type LayoutNode,
	type PanelId,
	type PanelNode,
	type TabsNode,
} from "./types"

export type TabRef = { nodeId: string; index: number }

export type LayoutAction =
	| { type: "activateTab"; nodeId: string; index: number }
	| { type: "resize"; nodeId: string; index: number; delta: number }
	| { type: "moveTab"; from: TabRef; to: TabRef }
	| { type: "closeTab"; nodeId: string; index: number }
	| { type: "togglePanel"; panel: PanelId; hostNodeId: string }
	| { type: "replace"; layout: Layout }

/** Pure: the dock never mutates its tree, which keeps undo of layout changes trivial later. */
export function layoutReducer(layout: Layout, action: LayoutAction): Layout {
	switch (action.type) {
		case "replace":
			return action.layout
		case "activateTab":
			return {
				...layout,
				root: mapNode(layout.root, action.nodeId, (node) =>
					node.kind === "tabs"
						? { ...node, activeIndex: clamp(action.index, 0, node.children.length - 1) }
						: node,
				),
			}
		case "resize":
			return {
				...layout,
				root: mapNode(layout.root, action.nodeId, (node) =>
					resizeSplit(node, action.index, action.delta),
				),
			}
		case "closeTab":
			return withPrunedRoot(
				layout,
				mapNode(layout.root, action.nodeId, (node) =>
					node.kind === "tabs" ? removeTab(node, action.index) : node,
				),
			)
		case "moveTab":
			return moveTab(layout, action.from, action.to)
		case "togglePanel":
			return togglePanel(layout, action.panel, action.hostNodeId)
	}
}

export function findPanel(node: LayoutNode, panel: PanelId): TabRef | null {
	if (node.kind === "tabs") {
		const index = node.children.findIndex((child) => child.id === panel)
		return index === -1 ? null : { nodeId: node.id, index }
	}
	for (const child of node.children) {
		const found = findPanel(child, panel)
		if (found) return found
	}
	return null
}

export function visiblePanels(node: LayoutNode): PanelId[] {
	if (node.kind === "tabs") return node.children.map((child) => child.id)
	return node.children.flatMap((child) => visiblePanels(child))
}

export function findTabsNode(node: LayoutNode, nodeId: string): TabsNode | null {
	if (node.kind === "tabs") return node.id === nodeId ? node : null
	for (const child of node.children) {
		const found = findTabsNode(child, nodeId)
		if (found) return found
	}
	return null
}

function firstTabsNode(node: LayoutNode): TabsNode | null {
	if (node.kind === "tabs") return node
	for (const child of node.children) {
		const found = firstTabsNode(child)
		if (found) return found
	}
	return null
}

function mapNode(
	node: LayoutNode,
	nodeId: string,
	update: (node: LayoutNode) => LayoutNode,
): LayoutNode {
	if (node.id === nodeId) return update(node)
	if (node.kind === "split") {
		return { ...node, children: node.children.map((child) => mapNode(child, nodeId, update)) }
	}
	return node
}

function resizeSplit(node: LayoutNode, index: number, delta: number): LayoutNode {
	if (node.kind !== "split") return node
	const sizes = [...node.sizes]
	const current = sizes[index]
	const next = sizes[index + 1]
	if (current === undefined || next === undefined) return node
	const shift = clamp(delta, MIN_SIZE - current, next - MIN_SIZE)
	sizes[index] = current + shift
	sizes[index + 1] = next - shift
	return { ...node, sizes }
}

function removeTab(node: TabsNode, index: number): TabsNode {
	const children = node.children.filter((_, position) => position !== index)
	const active = node.activeIndex > index ? node.activeIndex - 1 : node.activeIndex
	return { ...node, children, activeIndex: clamp(active, 0, children.length - 1) }
}

function moveTab(layout: Layout, from: TabRef, to: TabRef): Layout {
	const source = findTabsNode(layout.root, from.nodeId)
	const panel = source?.children[from.index]
	if (!source || !panel) return layout
	if (findTabsNode(layout.root, to.nodeId) === null) return layout

	const without = mapNode(layout.root, from.nodeId, (node) =>
		node.kind === "tabs" ? removeTab(node, from.index) : node,
	)
	const inserted = mapNode(without, to.nodeId, (node) => {
		if (node.kind !== "tabs") return node
		if (node.children.some((child) => child.id === panel.id)) return node
		const index = clamp(to.index, 0, node.children.length)
		const children: PanelNode[] = [
			...node.children.slice(0, index),
			panel,
			...node.children.slice(index),
		]
		return { ...node, children, activeIndex: index }
	})
	return withPrunedRoot(layout, inserted)
}

function togglePanel(layout: Layout, panel: PanelId, hostNodeId: string): Layout {
	const existing = findPanel(layout.root, panel)
	if (existing) {
		return withPrunedRoot(
			layout,
			mapNode(layout.root, existing.nodeId, (node) =>
				node.kind === "tabs" ? removeTab(node, existing.index) : node,
			),
		)
	}
	const host = findTabsNode(layout.root, hostNodeId) ?? firstTabsNode(layout.root)
	if (!host) return layout
	return {
		...layout,
		root: mapNode(layout.root, host.id, (node) => {
			if (node.kind !== "tabs") return node
			const children: PanelNode[] = [...node.children, { kind: "panel", id: panel }]
			return { ...node, children, activeIndex: children.length - 1 }
		}),
	}
}

/** Drops empty tab groups and collapses single-child splits so the tree stays clean. */
function prune(node: LayoutNode): LayoutNode | null {
	if (node.kind === "tabs") return node.children.length > 0 ? node : null
	const kept: Array<{ child: LayoutNode; size: number }> = []
	node.children.forEach((child, index) => {
		const pruned = prune(child)
		if (pruned !== null) kept.push({ child: pruned, size: node.sizes[index] ?? 0 })
	})
	const first = kept[0]
	if (first === undefined) return null
	if (kept.length === 1) return first.child
	return {
		...node,
		children: kept.map((entry) => entry.child),
		sizes: normalizeSizes(kept.map((entry) => entry.size)),
	}
}

function withPrunedRoot(layout: Layout, root: LayoutNode): Layout {
	const pruned = prune(root)
	return pruned === null ? layout : { ...layout, root: pruned }
}

export function normalizeSizes(sizes: number[]): number[] {
	const total = sizes.reduce((sum, size) => sum + size, 0)
	if (total <= 0) return sizes.map(() => 1 / Math.max(sizes.length, 1))
	return sizes.map((size) => size / total)
}

function clamp(value: number, min: number, max: number): number {
	if (max < min) return min
	return Math.min(Math.max(value, min), max)
}
