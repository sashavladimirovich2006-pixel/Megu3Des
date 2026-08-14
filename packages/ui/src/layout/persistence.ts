import type { Workspace } from "../workspaces"
import { LAYOUT_VERSION, type Layout } from "./types"

const STORAGE_KEY = "megu3d.layouts.v1"

export type StoredLayouts = Partial<Record<Workspace, Layout>>

export function isLayout(value: unknown): value is Layout {
	if (typeof value !== "object" || value === null) return false
	const candidate = value as { version?: unknown; root?: unknown }
	return candidate.version === LAYOUT_VERSION && typeof candidate.root === "object"
}

/**
 * M2 persists dock layouts in localStorage. M4 moves them to
 * `%APPDATA%/Megu3D/layout.json` through the Rust settings IPC.
 */
export function loadLayouts(): StoredLayouts {
	if (typeof localStorage === "undefined") return {}
	try {
		const raw = localStorage.getItem(STORAGE_KEY)
		if (!raw) return {}
		const parsed: unknown = JSON.parse(raw)
		if (typeof parsed !== "object" || parsed === null) return {}
		const stored: StoredLayouts = {}
		for (const [workspace, layout] of Object.entries(parsed as Record<string, unknown>)) {
			if (isLayout(layout)) stored[workspace as Workspace] = layout
		}
		return stored
	} catch (error) {
		console.warn("megu3d: ignoring corrupted stored layouts", error)
		return {}
	}
}

export function saveLayouts(layouts: StoredLayouts): void {
	if (typeof localStorage === "undefined") return
	try {
		localStorage.setItem(STORAGE_KEY, JSON.stringify(layouts))
	} catch (error) {
		console.warn("megu3d: failed to persist layouts", error)
	}
}
