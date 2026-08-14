import { useCallback, useEffect, useMemo, useState, type ReactNode } from "react"
import { createTranslator, type MessageKey, type MessageVars } from "@megu3d/i18n"
import { isDesktop, queryAppInfo, querySceneStats } from "@megu3d/ipc"
import type { AppInfo, SceneStats } from "@megu3d/types"

import { formatChord, parseChord } from "./commands/keybinding"
import { createCommands, panelTitleKey, type CommandContext } from "./commands/registry"
import { useKeybindings } from "./commands/useKeybindings"
import { DockView } from "./layout/DockView"
import { loadLayouts, saveLayouts } from "./layout/persistence"
import { WORKSPACE_LAYOUTS, presetFor } from "./layout/presets"
import { layoutReducer, type LayoutAction } from "./layout/reducer"
import type { Layout, PanelId } from "./layout/types"
import { CommandPalette } from "./palette/CommandPalette"
import { PANELS } from "./panels/registry"
import {
	applyPreferences,
	loadPreferences,
	nextLocale,
	nextTheme,
	savePreferences,
	type Preferences,
} from "./preferences"
import { WORKSPACES, type Workspace } from "./workspaces"

const PALETTE_BINDING = "Ctrl+Shift+P"
/** Tab group that receives panels toggled on from the palette. */
const DEFAULT_HOST = "main"

type IpcStatus = { kind: "connecting" } | { kind: "browser" } | { kind: "error"; message: string }
type Notice = { key: MessageKey; vars?: MessageVars }

function paletteShortcut(): string {
	const chord = parseChord(PALETTE_BINDING)
	return chord ? formatChord(chord) : PALETTE_BINDING
}

/**
 * M2 window frame: workspace switcher on top, dockable panel tree in the middle,
 * status bar at the bottom. Layout, theme, language and the keymap all live in
 * one place so panels stay dumb and testable.
 */
export function AppShell(): ReactNode {
	const [preferences, setPreferences] = useState<Preferences>(loadPreferences)
	const [workspace, setWorkspace] = useState<Workspace>("Layout")
	const [layouts, setLayouts] = useState<Record<Workspace, Layout>>(() => ({
		...WORKSPACE_LAYOUTS,
		...loadLayouts(),
	}))
	const [paletteOpen, setPaletteOpen] = useState(false)
	const [notice, setNotice] = useState<Notice | null>(null)
	const [appInfo, setAppInfo] = useState<AppInfo | null>(null)
	const [stats, setStats] = useState<SceneStats | null>(null)
	const [ipcStatus, setIpcStatus] = useState<IpcStatus>({ kind: "connecting" })

	const t = useMemo(() => createTranslator(preferences.locale), [preferences.locale])
	const commands = useMemo(() => createCommands(), [])
	const layout = layouts[workspace]

	useEffect(() => {
		applyPreferences(preferences)
		savePreferences(preferences)
	}, [preferences])

	useEffect(() => {
		saveLayouts(layouts)
	}, [layouts])

	useEffect(() => {
		if (!isDesktop()) {
			setIpcStatus({ kind: "browser" })
			return
		}
		let cancelled = false
		void (async () => {
			try {
				const [nextInfo, nextStats] = await Promise.all([queryAppInfo(), querySceneStats()])
				if (cancelled) return
				setAppInfo(nextInfo)
				setStats(nextStats)
			} catch (cause) {
				if (!cancelled) setIpcStatus({ kind: "error", message: String(cause) })
			}
		})()
		return () => {
			cancelled = true
		}
	}, [])

	const dispatchLayout = useCallback((action: LayoutAction) => {
		setLayouts((current) => ({
			...current,
			[workspace]: layoutReducer(current[workspace], action),
		}))
	}, [workspace])

	const context: CommandContext = useMemo(
		() => ({
			t,
			workspace,
			setWorkspace,
			dispatchLayout,
			resetLayout: () =>
				setLayouts((current) => ({ ...current, [workspace]: presetFor(workspace) })),
			togglePanel: (panel: PanelId) =>
				dispatchLayout({ type: "togglePanel", panel, hostNodeId: DEFAULT_HOST }),
			toggleTheme: () =>
				setPreferences((current) => ({ ...current, theme: nextTheme(current.theme) })),
			toggleLocale: () =>
				setPreferences((current) => ({ ...current, locale: nextLocale(current.locale) })),
			openPalette: () => setPaletteOpen(true),
			notify: (key: MessageKey, vars?: MessageVars) => setNotice({ key, vars }),
		}),
		[dispatchLayout, t, workspace],
	)

	useKeybindings(commands, context, paletteOpen)

	const shortcut = useMemo(paletteShortcut, [])

	const renderPanel = useCallback(
		(panel: PanelId) =>
			PANELS[panel].render({
				t,
				workspace,
				appInfo,
				stats,
				theme: preferences.theme,
				locale: preferences.locale,
				uiScale: preferences.uiScale,
				paletteShortcut: shortcut,
			}),
		[appInfo, preferences, shortcut, stats, t, workspace],
	)

	const panelTitle = useCallback((panel: PanelId) => t(panelTitleKey(panel)), [t])

	const statusText = appInfo
		? `${appInfo.name} ${appInfo.version} · schema ${appInfo.schemaVersion} · ${appInfo.buildProfile}`
		: ipcStatus.kind === "browser"
			? t("status.browserPreview")
			: ipcStatus.kind === "error"
				? ipcStatus.message
				: t("status.connecting")

	return (
		<div className="mg-shell">
			<header className="mg-top">
				<span className="mg-brand">{t("app.name")}</span>
				<nav className="mg-tabs" aria-label={t("command.category.workspace")}>
					{WORKSPACES.map((name) => (
						<button
							key={name}
							type="button"
							className={name === workspace ? "mg-tab mg-tab--active" : "mg-tab"}
							aria-current={name === workspace}
							onClick={() => setWorkspace(name)}
						>
							{name}
						</button>
					))}
				</nav>
				<button
					type="button"
					className="mg-palette-trigger"
					onClick={() => setPaletteOpen(true)}
					title={t("command.palette.open")}
				>
					{t("palette.title")}
					<kbd className="mg-kbd">{shortcut}</kbd>
				</button>
			</header>

			<main className="mg-dock-root">
				<DockView
					node={layout.root}
					dispatch={dispatchLayout}
					renderPanel={renderPanel}
					panelTitle={panelTitle}
					closeLabel={t("panel.close")}
					dropHint={t("panel.dropHint")}
				/>
			</main>

			<div className="mg-status" role="status">
				<span>{statusText}</span>
				{notice ? <span className="mg-muted">{t(notice.key, notice.vars)}</span> : null}
				<span className="mg-status-right mg-muted">
					{`${workspace} · ${preferences.locale.toUpperCase()} · ${
						preferences.theme === "dark" ? t("theme.dark") : t("theme.light")
					}`}
				</span>
			</div>

			{paletteOpen ? (
				<CommandPalette
					commands={commands}
					context={context}
					t={t}
					onClose={() => setPaletteOpen(false)}
				/>
			) : null}
		</div>
	)
}
