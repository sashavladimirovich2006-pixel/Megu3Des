//! Tauri command handlers: the thin bridge between the webview and the core.
//!
//! Handlers do no logic of their own. They lock the session, delegate to
//! `megu3d-cmd`, and translate errors into the shared [`ErrorDto`] envelope.

use std::sync::Mutex;

use megu3d_cmd::dispatch::Session;
use megu3d_cmd::CmdError;
use megu3d_core::dto::{
    AppInfo, CommandRequestDto, CommandResultDto, ErrorDto, SceneSnapshotDto, SceneStats,
};
use tauri::{AppHandle, Emitter, State};

/// Emitted after every successful command so panels can refresh together.
pub const EVENT_SCENE_PATCH: &str = "megu3d.event.scenePatch";
/// Emitted with the current selection, the one piece of state every panel reads.
pub const EVENT_SELECTION: &str = "megu3d.event.selection";

type Ipc<T> = Result<T, ErrorDto>;

fn poisoned() -> ErrorDto {
    ErrorDto::from(CmdError::LockPoisoned)
}

#[tauri::command]
pub fn megu3d_query_app_info() -> AppInfo {
    AppInfo::current()
}

#[tauri::command]
pub fn megu3d_query_scene_stats(session: State<'_, Mutex<Session>>) -> Ipc<SceneStats> {
    let guard = session.lock().map_err(|_| poisoned())?;
    Ok(guard.scene().stats())
}

#[tauri::command]
pub fn megu3d_query_scene(session: State<'_, Mutex<Session>>) -> Ipc<SceneSnapshotDto> {
    let guard = session.lock().map_err(|_| poisoned())?;
    Ok(guard.snapshot())
}

#[tauri::command]
pub fn megu3d_cmd_dispatch(
    app: AppHandle,
    session: State<'_, Mutex<Session>>,
    request: CommandRequestDto,
) -> Ipc<CommandResultDto> {
    let result = {
        let mut guard = session.lock().map_err(|_| poisoned())?;
        guard.dispatch(request).map_err(ErrorDto::from)?
    };
    // Events are best effort: the caller already has the authoritative result.
    if let Err(error) = app.emit(EVENT_SCENE_PATCH, &result.patch) {
        tracing::warn!(%error, "scene patch event was not delivered");
    }
    if let Err(error) = app.emit(EVENT_SELECTION, &result.scene.selection) {
        tracing::warn!(%error, "selection event was not delivered");
    }
    Ok(result)
}
