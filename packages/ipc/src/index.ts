import { invoke } from "@tauri-apps/api/core"
import { listen, type UnlistenFn } from "@tauri-apps/api/event"
import {
	EVENTS,
	IPC,
	type AppInfo,
	type CommandRequestDto,
	type CommandResultDto,
	type EventName,
	type IpcName,
	type ScenePatch,
	type SceneSnapshotDto,
	type SceneStats,
} from "@megu3d/types"

/**
 * Contract name -> Tauri command name. Tauri command names cannot contain dots,
 * so the dotted contract names are mapped to snake_case handlers in
 * `apps/desktop/src-tauri/src/ipc.rs`.
 */
export const TAURI_COMMAND: Record<IpcName, string> = {
	[IPC.queryAppInfo]: "megu3d_query_app_info",
	[IPC.querySceneStats]: "megu3d_query_scene_stats",
	[IPC.queryScene]: "megu3d_query_scene",
	[IPC.cmdDispatch]: "megu3d_cmd_dispatch",
}

/** True inside the Tauri shell, false in a plain browser or in unit tests. */
export function isDesktop(): boolean {
	return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window
}

async function call<T>(name: IpcName, args: Record<string, unknown> = {}): Promise<T> {
	if (!isDesktop()) {
		throw new Error(`IPC "${name}" is only available inside the Megu3D desktop shell`)
	}
	return invoke<T>(TAURI_COMMAND[name], args)
}

async function subscribe<T>(name: EventName, handler: (payload: T) => void): Promise<UnlistenFn> {
	if (!isDesktop()) {
		// Browser preview has no core to listen to; unsubscribing is a no-op.
		return () => undefined
	}
	return listen<T>(name, (event) => handler(event.payload))
}

export function queryAppInfo(): Promise<AppInfo> {
	return call<AppInfo>(IPC.queryAppInfo)
}

export function querySceneStats(): Promise<SceneStats> {
	return call<SceneStats>(IPC.querySceneStats)
}

/** Full scene snapshot: what the outliner, properties and viewport render. */
export function queryScene(): Promise<SceneSnapshotDto> {
	return call<SceneSnapshotDto>(IPC.queryScene)
}

/** The single mutation entry point. Every UI intent goes through here. */
export function dispatch(request: CommandRequestDto): Promise<CommandResultDto> {
	return call<CommandResultDto>(IPC.cmdDispatch, { request })
}

export function onScenePatch(handler: (patch: ScenePatch) => void): Promise<UnlistenFn> {
	return subscribe<ScenePatch>(EVENTS.scenePatch, handler)
}

export function onSelection(handler: (selection: string[]) => void): Promise<UnlistenFn> {
	return subscribe<string[]>(EVENTS.selection, handler)
}

/**
 * What the UI depends on instead of the module itself, so panels can be tested
 * with a stub instead of a running desktop shell.
 */
export type SceneTransport = {
	queryScene: () => Promise<SceneSnapshotDto>
	dispatch: (request: CommandRequestDto) => Promise<CommandResultDto>
	onScenePatch: (handler: (patch: ScenePatch) => void) => Promise<UnlistenFn>
}

export const desktopTransport: SceneTransport = {
	queryScene,
	dispatch,
	onScenePatch,
}

export { EVENTS, IPC }
export type {
	AppInfo,
	CommandRequestDto,
	CommandResultDto,
	EventName,
	IpcName,
	ScenePatch,
	SceneSnapshotDto,
	SceneStats,
	UnlistenFn,
}
