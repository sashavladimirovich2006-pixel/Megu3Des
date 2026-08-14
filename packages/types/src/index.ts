export type { AppInfo } from "./generated/AppInfo"
export type { CommandRequestDto } from "./generated/CommandRequestDto"
export type { CommandResultDto } from "./generated/CommandResultDto"
export type { ErrorDto } from "./generated/ErrorDto"
export type { HistoryStateDto } from "./generated/HistoryStateDto"
export type { MeshInfoDto } from "./generated/MeshInfoDto"
export type { NodeKindDto } from "./generated/NodeKindDto"
export type { PrimitiveKindDto } from "./generated/PrimitiveKindDto"
export type { ScenePatch } from "./generated/ScenePatch"
export type { SceneNodeDto } from "./generated/SceneNodeDto"
export type { SceneSnapshotDto } from "./generated/SceneSnapshotDto"
export type { SceneStats } from "./generated/SceneStats"
export type { SelectionModeDto } from "./generated/SelectionModeDto"
export type { TransformDto } from "./generated/TransformDto"
export type { Vec3Dto } from "./generated/Vec3Dto"

/**
 * Public IPC contract names. Rust is the source of truth: the payload types in
 * `./generated` are exported by ts-rs from `crates/megu3d-core/src/dto.rs`.
 */
export const IPC = {
	queryAppInfo: "megu3d.query.appInfo",
	querySceneStats: "megu3d.query.sceneStats",
	queryScene: "megu3d.query.scene",
	cmdDispatch: "megu3d.cmd.dispatch",
} as const

export type IpcName = (typeof IPC)[keyof typeof IPC]

/** Events pushed by the core after a command changes the scene. */
export const EVENTS = {
	scenePatch: "megu3d.event.scenePatch",
	selection: "megu3d.event.selection",
} as const

export type EventName = (typeof EVENTS)[keyof typeof EVENTS]
