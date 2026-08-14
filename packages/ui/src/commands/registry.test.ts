import { describe, expect, it, vi } from "vitest"
import { catalogFor, createTranslator } from "@megu3d/i18n"
import type { SceneSnapshotDto } from "@megu3d/types"

import { PANEL_IDS } from "../layout/types"
import type { SceneApi, SceneStatus } from "../scene/useScene"
import { WORKSPACES } from "../workspaces"
import { commandTitle, createCommands, findConflicts, type CommandContext } from "./registry"

function stubScene(selection: string[]): SceneSnapshotDto {
	return {
		schemaVersion: "0.2.0",
		nodes: [],
		roots: [],
		selection,
		active: selection[0] ?? null,
		stats: {
			nodeCount: selection.length,
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

function stubContext(options: { selection?: string[]; status?: SceneStatus } = {}) {
	const scene = stubScene(options.selection ?? ["node-1"])
	const api: SceneApi = {
		scene,
		status: options.status ?? "ready",
		error: null,
		send: vi.fn(),
		add: vi.fn(),
		remove: vi.fn(),
		duplicate: vi.fn(),
		rename: vi.fn(),
		setTransform: vi.fn(),
		commitPreview: vi.fn(),
		cancelPreview: vi.fn(),
		setVisible: vi.fn(),
		reparent: vi.fn(),
		select: vi.fn(),
		undo: vi.fn(),
		redo: vi.fn(),
	}
	const spies = {
		setWorkspace: vi.fn(),
		dispatchLayout: vi.fn(),
		resetLayout: vi.fn(),
		togglePanel: vi.fn(),
		toggleTheme: vi.fn(),
		toggleLocale: vi.fn(),
		openPalette: vi.fn(),
		notify: vi.fn(),
		setTransformMode: vi.fn(),
		frameSelected: vi.fn(),
	}
	const context: CommandContext = {
		t: createTranslator("en"),
		workspace: "Layout",
		scene,
		api,
		transformMode: "move",
		...spies,
	}
	return { api, context, spies }
}

function run(id: string, context: CommandContext): void {
	createCommands()
		.find((command) => command.id === id)
		?.run(context)
}

describe("command registry", () => {
	const commands = createCommands()

	it("exposes unique command ids", () => {
		const ids = commands.map((command) => command.id)
		expect(new Set(ids).size).toBe(ids.length)
	})

	it("has no conflicting or malformed keybindings", () => {
		expect(findConflicts(commands)).toEqual([])
	})

	it("covers every workspace and every panel", () => {
		expect(commands.filter((command) => command.category === "workspace")).toHaveLength(
			WORKSPACES.length,
		)
		expect(commands.filter((command) => command.category === "panel")).toHaveLength(
			PANEL_IDS.length,
		)
	})

	it("only uses message keys that exist in both catalogs", () => {
		for (const locale of ["en", "ru"] as const) {
			const catalog = catalogFor(locale)
			for (const command of commands) {
				expect(catalog[command.titleKey], `${command.id} title`).toBeTruthy()
				expect(catalog[command.categoryKey], `${command.id} category`).toBeTruthy()
				for (const key of Object.values(command.titleVarKeys ?? {})) {
					expect(catalog[key], `${command.id} var`).toBeTruthy()
				}
			}
		}
	})

	it("localizes titles including interpolated names", () => {
		const t = createTranslator("en")
		const workspace = commands.find((command) => command.id === "workspace.goto.modeling")
		const panel = commands.find((command) => command.id === "panel.toggle.outliner")
		const add = commands.find((command) => command.id === "scene.add.cube")
		expect(workspace && commandTitle(workspace, t)).toBe("Go to workspace: Modeling")
		expect(panel && commandTitle(panel, t)).toBe("Toggle panel: Outliner")
		expect(panel && commandTitle(panel, createTranslator("ru"))).toContain("Структура")
		expect(add && commandTitle(add, t)).toBe("Add: Cube")
	})

	it("keeps the hidden Delete alias out of the palette but inside the keymap", () => {
		const alias = commands.find((command) => command.id === "scene.delete.key")
		expect(alias?.hidden).toBe(true)
		expect(alias?.keybinding).toBe("Delete")
		expect(commands.filter((command) => command.hidden).length).toBe(1)
	})

	it("routes shell commands through the context", () => {
		const { context, spies } = stubContext()
		for (const id of ["view.theme.toggle", "palette.open", "panel.toggle.timeline", "view.frame.selected", "transform.rotate"]) {
			run(id, context)
		}
		expect(spies.toggleTheme).toHaveBeenCalledOnce()
		expect(spies.openPalette).toHaveBeenCalledOnce()
		expect(spies.togglePanel).toHaveBeenCalledWith("timeline")
		expect(spies.frameSelected).toHaveBeenCalledOnce()
		expect(spies.setTransformMode).toHaveBeenCalledWith("rotate")
	})

	it("drives the scene core for edit commands", () => {
		const { api, context } = stubContext()
		for (const id of ["edit.undo", "edit.redo", "scene.add.cube", "scene.delete", "scene.duplicate"]) {
			run(id, context)
		}
		expect(api.undo).toHaveBeenCalledOnce()
		expect(api.redo).toHaveBeenCalledOnce()
		expect(api.add).toHaveBeenCalledWith("cube")
		expect(api.remove).toHaveBeenCalledOnce()
		expect(api.duplicate).toHaveBeenCalledOnce()
	})

	it("refuses to act without a selection or without the desktop core", () => {
		const empty = stubContext({ selection: [] })
		run("scene.delete", empty.context)
		expect(empty.api.remove).not.toHaveBeenCalled()
		expect(empty.spies.notify).toHaveBeenCalledWith("status.noSelection")

		const browser = stubContext({ status: "browser" })
		run("scene.add.cube", browser.context)
		expect(browser.api.add).not.toHaveBeenCalled()
		expect(browser.spies.notify).toHaveBeenCalledWith("status.desktopOnly")
	})
})
