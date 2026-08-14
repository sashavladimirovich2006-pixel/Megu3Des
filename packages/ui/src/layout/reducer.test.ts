import { describe, expect, it } from "vitest"

import { WORKSPACE_LAYOUTS, presetFor } from "./presets"
import { findPanel, findTabsNode, layoutReducer, visiblePanels } from "./reducer"
import { MIN_SIZE, type Layout } from "./types"

const base = (): Layout => presetFor("Layout")

describe("layout reducer", () => {
	it("ships a preset for every workspace with a viewport or a dedicated editor", () => {
		for (const [workspace, layout] of Object.entries(WORKSPACE_LAYOUTS)) {
			expect(visiblePanels(layout.root).length, workspace).toBeGreaterThan(0)
		}
	})

	it("activates a tab and clamps out of range indices", () => {
		const withAssets = layoutReducer(base(), {
			type: "togglePanel",
			panel: "assets",
			hostNodeId: "main",
		})
		expect(findPanel(withAssets.root, "assets")).toEqual({ nodeId: "main", index: 1 })
		const high = layoutReducer(withAssets, { type: "activateTab", nodeId: "main", index: 99 })
		expect(findTabsNode(high.root, "main")?.activeIndex).toBe(1)
		const low = layoutReducer(high, { type: "activateTab", nodeId: "main", index: -5 })
		expect(findTabsNode(low.root, "main")?.activeIndex).toBe(0)
	})

	it("keeps split sizes normalized and respects the minimum size", () => {
		const layout = layoutReducer(base(), { type: "resize", nodeId: "root", index: 0, delta: -1 })
		if (layout.root.kind !== "split") throw new Error("expected a split root")
		expect(layout.root.sizes[0]).toBeCloseTo(MIN_SIZE)
		expect(layout.root.sizes.reduce((sum, size) => sum + size, 0)).toBeCloseTo(1)
	})

	it("moves a tab between groups", () => {
		const layout = layoutReducer(base(), {
			type: "moveTab",
			from: { nodeId: "bottom", index: 0 },
			to: { nodeId: "main", index: 1 },
		})
		expect(findPanel(layout.root, "timeline")).toEqual({ nodeId: "main", index: 1 })
	})

	it("collapses empty groups when the last tab is closed", () => {
		const layout = layoutReducer(base(), { type: "closeTab", nodeId: "bottom", index: 0 })
		expect(findPanel(layout.root, "timeline")).toBeNull()
		expect(findPanel(layout.root, "viewport")).toEqual({ nodeId: "main", index: 0 })
		if (layout.root.kind !== "split") throw new Error("expected a split root")
		expect(layout.root.sizes.reduce((sum, size) => sum + size, 0)).toBeCloseTo(1)
	})

	it("toggles a panel off and back on", () => {
		const hidden = layoutReducer(base(), {
			type: "togglePanel",
			panel: "properties",
			hostNodeId: "right-bottom",
		})
		expect(findPanel(hidden.root, "properties")).toBeNull()
		const shown = layoutReducer(hidden, {
			type: "togglePanel",
			panel: "properties",
			hostNodeId: "right-top",
		})
		expect(findPanel(shown.root, "properties")).toEqual({ nodeId: "right-top", index: 1 })
	})

	it("never duplicates a panel", () => {
		const layout = layoutReducer(base(), {
			type: "moveTab",
			from: { nodeId: "main", index: 0 },
			to: { nodeId: "main", index: 0 },
		})
		expect(visiblePanels(layout.root).filter((panel) => panel === "viewport")).toHaveLength(1)
	})
})
