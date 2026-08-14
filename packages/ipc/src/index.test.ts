import { describe, expect, it } from "vitest"

import {
	EVENTS,
	IPC,
	TAURI_COMMAND,
	desktopTransport,
	dispatch,
	isDesktop,
	onScenePatch,
	queryAppInfo,
	queryScene,
} from "./index"

describe("ipc contract", () => {
	it("maps every contract name to a snake_case tauri command", () => {
		const commands = Object.values(IPC).map((name) => TAURI_COMMAND[name])
		for (const command of commands) {
			expect(command).toMatch(/^megu3d_[a-z0-9_]+$/)
		}
		expect(new Set(commands).size).toBe(commands.length)
	})

	it("names every event under the megu3d.event namespace", () => {
		for (const name of Object.values(EVENTS)) {
			expect(name).toMatch(/^megu3d\.event\.[a-zA-Z]+$/)
		}
	})

	it("reports a non-tauri context as not desktop", () => {
		expect(isDesktop()).toBe(false)
	})

	it("fails loudly outside the desktop shell", async () => {
		await expect(queryAppInfo()).rejects.toThrow(/desktop shell/)
		await expect(queryScene()).rejects.toThrow(/desktop shell/)
		await expect(dispatch({ type: "undo" })).rejects.toThrow(/desktop shell/)
	})

	it("subscribes to nothing in browser preview", async () => {
		const unlisten = await onScenePatch(() => undefined)
		expect(typeof unlisten).toBe("function")
		expect(unlisten()).toBeUndefined()
	})

	it("exposes a transport the ui can stub", () => {
		expect(Object.keys(desktopTransport).sort()).toEqual([
			"dispatch",
			"onScenePatch",
			"queryScene",
		])
	})
})
