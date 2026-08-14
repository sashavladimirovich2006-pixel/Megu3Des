/** Workspace tabs from `docs/04-ui-architecture.md`. Only Layout is wired in M1. */
export const WORKSPACES = [
	"Home",
	"Layout",
	"Modeling",
	"Sculpting",
	"UV & Textures",
	"Shading",
	"Geometry Nodes",
	"Animation",
	"Rigging",
	"Simulation",
	"Rendering",
	"Compositing",
	"Video Editing",
	"Assets",
	"Preferences",
] as const

export type Workspace = (typeof WORKSPACES)[number]
