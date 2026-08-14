/** Panels that can live in the dock. Panel ids are stable and used in persisted layouts. */
export const PANEL_IDS = [
	"home",
	"viewport",
	"outliner",
	"properties",
	"timeline",
	"assets",
	"tools",
	"shader",
	"geometryNodes",
	"uv",
	"compositor",
	"video",
	"preferences",
] as const

export type PanelId = (typeof PANEL_IDS)[number]

export type PanelNode = { kind: "panel"; id: PanelId }

/** A tab group: the only place a panel can live. */
export type TabsNode = {
	kind: "tabs"
	id: string
	children: PanelNode[]
	activeIndex: number
}

/** A split container. `sizes` are fractions of the parent and sum to 1. */
export type SplitNode = {
	kind: "split"
	id: string
	direction: "row" | "column"
	sizes: number[]
	children: LayoutNode[]
}

export type LayoutNode = TabsNode | SplitNode

export type Layout = { version: typeof LAYOUT_VERSION; root: LayoutNode }

export const LAYOUT_VERSION = 1

/** Smallest fraction a split child may shrink to. */
export const MIN_SIZE = 0.08

export function isPanelId(value: unknown): value is PanelId {
	return typeof value === "string" && PANEL_IDS.some((panel) => panel === value)
}
