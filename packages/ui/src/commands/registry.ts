import type { MessageKey, MessageVars, Translate } from "@megu3d/i18n"

import type { PrimitiveKindDto, SceneSnapshotDto } from "@megu3d/types"

import type { LayoutAction } from "../layout/reducer"
import { PANEL_IDS, type PanelId } from "../layout/types"
import type { SceneApi } from "../scene/useScene"
import { TRANSFORM_MODES, type TransformMode } from "../viewport/gizmo"
import { WORKSPACES, type Workspace } from "../workspaces"
import { chordSignature, parseChord } from "./keybinding"

export type CommandCategory = "workspace" | "panel" | "view" | "edit" | "scene"

/** Everything a command may touch. Commands never reach into React state directly. */
export type CommandContext = {
	t: Translate
	workspace: Workspace
	setWorkspace: (workspace: Workspace) => void
	dispatchLayout: (action: LayoutAction) => void
	resetLayout: () => void
	togglePanel: (panel: PanelId) => void
	toggleTheme: () => void
	toggleLocale: () => void
	openPalette: () => void
	notify: (key: MessageKey, vars?: MessageVars) => void
	scene: SceneSnapshotDto | null
	api: SceneApi
	transformMode: TransformMode
	setTransformMode: (mode: TransformMode) => void
	frameSelected: () => void
}

export type Command = {
	id: string
	titleKey: MessageKey
	categoryKey: MessageKey
	category: CommandCategory
	/** Literal interpolation values, e.g. a workspace name. */
	titleVars?: MessageVars
	/** Interpolation values that must be translated first, e.g. a panel name. */
	titleVarKeys?: Record<string, MessageKey>
	keybinding?: string
	/** Hidden commands stay in the keymap but out of the palette (e.g. the Delete alias). */
	hidden?: boolean
	run: (context: CommandContext) => void
}

const PANEL_TITLE_KEYS: Record<PanelId, MessageKey> = {
	home: "panel.home",
	viewport: "panel.viewport",
	outliner: "panel.outliner",
	properties: "panel.properties",
	timeline: "panel.timeline",
	assets: "panel.assets",
	tools: "panel.tools",
	shader: "panel.shader",
	geometryNodes: "panel.geometryNodes",
	uv: "panel.uv",
	compositor: "panel.compositor",
	video: "panel.video",
	preferences: "panel.preferences",
}

export function panelTitleKey(panel: PanelId): MessageKey {
	return PANEL_TITLE_KEYS[panel]
}

export function commandTitle(command: Command, t: Translate): string {
	const vars: MessageVars = { ...command.titleVars }
	for (const [name, key] of Object.entries(command.titleVarKeys ?? {})) {
		vars[name] = t(key)
	}
	return t(command.titleKey, vars)
}

function slug(value: string): string {
	return value
		.toLowerCase()
		.replace(/[^a-z0-9]+/g, "-")
		.replace(/^-|-$/g, "")
}

const PRIMITIVE_KINDS = [
	"cube",
	"sphere",
	"cylinder",
	"cone",
	"torus",
	"plane",
	"empty",
	"camera",
	"light",
] as const satisfies readonly PrimitiveKindDto[]

const PRIMITIVE_KEYS: Record<PrimitiveKindDto, MessageKey> = {
	plane: "primitive.plane",
	cube: "primitive.cube",
	sphere: "primitive.sphere",
	cylinder: "primitive.cylinder",
	cone: "primitive.cone",
	torus: "primitive.torus",
	empty: "primitive.empty",
	camera: "primitive.camera",
	light: "primitive.light",
}

const TRANSFORM_KEYS: Record<TransformMode, MessageKey> = {
	move: "command.transform.move",
	rotate: "command.transform.rotate",
	scale: "command.transform.scale",
}

/** Blender-compatible transform shortcuts; safe because the keymap ignores text fields. */
const TRANSFORM_BINDINGS: Record<TransformMode, string> = { move: "G", rotate: "R", scale: "S" }

/** Commands fail loudly in the status bar instead of silently doing nothing. */
function sceneReady(context: CommandContext): boolean {
	if (context.api.status === "ready") return true
	context.notify("status.desktopOnly")
	return false
}

function hasSelection(context: CommandContext): boolean {
	if (!sceneReady(context)) return false
	if ((context.scene?.selection.length ?? 0) > 0) return true
	context.notify("status.noSelection")
	return false
}

/** The single source of truth for the keymap. M3 extends it with scene commands. */
export function createCommands(): Command[] {
	const commands: Command[] = [
		{
			id: "palette.open",
			titleKey: "command.palette.open",
			categoryKey: "command.category.view",
			category: "view",
			keybinding: "Ctrl+Shift+P",
			run: (context) => context.openPalette(),
		},
		{
			id: "view.theme.toggle",
			titleKey: "command.theme.toggle",
			categoryKey: "command.category.view",
			category: "view",
			keybinding: "Ctrl+Alt+T",
			run: (context) => context.toggleTheme(),
		},
		{
			id: "view.locale.toggle",
			titleKey: "command.locale.toggle",
			categoryKey: "command.category.view",
			category: "view",
			keybinding: "Ctrl+Alt+L",
			run: (context) => context.toggleLocale(),
		},
		{
			id: "view.layout.reset",
			titleKey: "command.layout.reset",
			categoryKey: "command.category.view",
			category: "view",
			keybinding: "Ctrl+Alt+R",
			run: (context) => context.resetLayout(),
		},
		{
			id: "edit.undo",
			titleKey: "command.edit.undo",
			categoryKey: "command.category.edit",
			category: "edit",
			keybinding: "Ctrl+Z",
			run: (context) => {
				if (sceneReady(context)) context.api.undo()
			},
		},
		{
			id: "edit.redo",
			titleKey: "command.edit.redo",
			categoryKey: "command.category.edit",
			category: "edit",
			keybinding: "Ctrl+Shift+Z",
			run: (context) => {
				if (sceneReady(context)) context.api.redo()
			},
		},
	]

	WORKSPACES.forEach((workspace, index) => {
		commands.push({
			id: `workspace.goto.${slug(workspace)}`,
			titleKey: "command.workspace.switch",
			categoryKey: "command.category.workspace",
			category: "workspace",
			titleVars: { workspace },
			keybinding: index < 9 ? `Alt+${index + 1}` : undefined,
			run: (context) => context.setWorkspace(workspace),
		})
	})

	for (const panel of PANEL_IDS) {
		commands.push({
			id: `panel.toggle.${panel}`,
			titleKey: "command.panel.toggle",
			categoryKey: "command.category.panel",
			category: "panel",
			titleVarKeys: { panel: PANEL_TITLE_KEYS[panel] },
			run: (context) => context.togglePanel(panel),
		})
	}

	for (const primitive of PRIMITIVE_KINDS) {
		commands.push({
			id: `scene.add.${primitive}`,
			titleKey: "command.scene.add",
			categoryKey: "command.category.scene",
			category: "scene",
			titleVarKeys: { primitive: PRIMITIVE_KEYS[primitive] },
			keybinding: primitive === "cube" ? "Shift+A" : undefined,
			run: (context) => {
				if (sceneReady(context)) context.api.add(primitive)
			},
		})
	}

	commands.push(
		{
			id: "scene.delete",
			titleKey: "command.scene.delete",
			categoryKey: "command.category.scene",
			category: "scene",
			keybinding: "X",
			run: (context) => {
				if (hasSelection(context)) context.api.remove()
			},
		},
		{
			id: "scene.delete.key",
			titleKey: "command.scene.delete",
			categoryKey: "command.category.scene",
			category: "scene",
			keybinding: "Delete",
			hidden: true,
			run: (context) => {
				if (hasSelection(context)) context.api.remove()
			},
		},
		{
			id: "scene.duplicate",
			titleKey: "command.scene.duplicate",
			categoryKey: "command.category.scene",
			category: "scene",
			keybinding: "Shift+D",
			run: (context) => {
				if (hasSelection(context)) context.api.duplicate()
			},
		},
		{
			id: "scene.deselect",
			titleKey: "command.scene.deselect",
			categoryKey: "command.category.scene",
			category: "scene",
			keybinding: "Alt+A",
			run: (context) => {
				if (sceneReady(context)) context.api.select([], "clear")
			},
		},
		{
			id: "view.frame.selected",
			titleKey: "command.scene.frame",
			categoryKey: "command.category.view",
			category: "view",
			keybinding: "F",
			run: (context) => context.frameSelected(),
		},
	)

	for (const mode of TRANSFORM_MODES) {
		commands.push({
			id: `transform.${mode}`,
			titleKey: TRANSFORM_KEYS[mode],
			categoryKey: "command.category.scene",
			category: "scene",
			keybinding: TRANSFORM_BINDINGS[mode],
			run: (context) => context.setTransformMode(mode),
		})
	}

	return commands
}

/** Guards the keymap: duplicated or malformed shortcuts are a build/test failure, not a surprise. */
export function findConflicts(commands: Command[]): string[] {
	const owners = new Map<string, string>()
	const conflicts: string[] = []
	for (const command of commands) {
		if (!command.keybinding) continue
		const chord = parseChord(command.keybinding)
		if (!chord) {
			conflicts.push(`${command.id}: invalid binding "${command.keybinding}"`)
			continue
		}
		const signature = chordSignature(chord)
		const owner = owners.get(signature)
		if (owner) conflicts.push(`${command.keybinding}: ${owner} vs ${command.id}`)
		else owners.set(signature, command.id)
	}
	return conflicts
}
