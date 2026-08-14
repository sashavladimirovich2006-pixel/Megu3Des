import type { Workspace } from "../workspaces"
import { LAYOUT_VERSION, type Layout, type LayoutNode, type PanelId, type TabsNode } from "./types"

export function tabs(id: string, panels: PanelId[], activeIndex = 0): TabsNode {
	return {
		kind: "tabs",
		id,
		children: panels.map((panel) => ({ kind: "panel", id: panel })),
		activeIndex,
	}
}

export function split(
	id: string,
	direction: "row" | "column",
	sizes: number[],
	children: LayoutNode[],
): LayoutNode {
	return { kind: "split", id, direction, sizes, children }
}

/** Window frame from `docs/04-ui-architecture.md`: tools left, editor center, Outliner/Properties right, context editor bottom. */
function standard(center: PanelId[], bottom: PanelId[]): Layout {
	return {
		version: LAYOUT_VERSION,
		root: split("root", "row", [0.1, 0.68, 0.22], [
			tabs("left", ["tools"]),
			split("center", "column", [0.74, 0.26], [tabs("main", center), tabs("bottom", bottom)]),
			split("right", "column", [0.42, 0.58], [
				tabs("right-top", ["outliner"]),
				tabs("right-bottom", ["properties"]),
			]),
		]),
	}
}

function single(panel: PanelId): Layout {
	return { version: LAYOUT_VERSION, root: tabs("main", [panel]) }
}

/** Only relevant panels per workspace: no kitchen-sink layouts. */
export const WORKSPACE_LAYOUTS: Record<Workspace, Layout> = {
	Home: single("home"),
	Layout: standard(["viewport"], ["timeline"]),
	Modeling: standard(["viewport"], ["timeline"]),
	Sculpting: standard(["viewport"], ["timeline"]),
	"UV & Textures": standard(["uv", "viewport"], ["timeline"]),
	Shading: standard(["viewport"], ["shader"]),
	"Geometry Nodes": standard(["viewport"], ["geometryNodes"]),
	Animation: standard(["viewport"], ["timeline"]),
	Rigging: standard(["viewport"], ["timeline"]),
	Simulation: standard(["viewport"], ["timeline"]),
	Rendering: standard(["viewport"], ["timeline"]),
	Compositing: standard(["viewport"], ["compositor"]),
	"Video Editing": standard(["viewport"], ["video"]),
	Assets: single("assets"),
	Preferences: single("preferences"),
}

export function presetFor(workspace: Workspace): Layout {
	return WORKSPACE_LAYOUTS[workspace]
}
