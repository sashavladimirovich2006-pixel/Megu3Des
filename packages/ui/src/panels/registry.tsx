import type { ReactNode } from "react"
import type { Locale, MessageKey, Translate } from "@megu3d/i18n"
import type { AppInfo, SceneStats } from "@megu3d/types"

import type { PanelId } from "../layout/types"
import type { Theme } from "../preferences"
import type { SceneApi } from "../scene/useScene"
import type { TransformMode } from "../viewport/gizmo"
import { Viewport } from "../viewport/Viewport"
import type { Workspace } from "../workspaces"
import { Outliner } from "./Outliner"
import { Properties } from "./Properties"

export type PanelContext = {
	t: Translate
	workspace: Workspace
	appInfo: AppInfo | null
	stats: SceneStats | null
	theme: Theme
	locale: Locale
	uiScale: number
	paletteShortcut: string
	api: SceneApi
	transformMode: TransformMode
	/** Bumped by the "frame selected" command; the viewport reacts to the change. */
	frameToken: number
}

export type PanelDefinition = {
	titleKey: MessageKey
	render: (context: PanelContext) => ReactNode
}

/** Placeholder panels state the milestone that fills them: no fake UI, no fake promises. */
function pendingPanel(titleKey: MessageKey, messageKey: MessageKey): PanelDefinition {
	return {
		titleKey,
		render: ({ t }) => <p className="mg-pending mg-muted">{t(messageKey)}</p>,
	}
}

export const PANELS: Record<PanelId, PanelDefinition> = {
	home: {
		titleKey: "panel.home",
		render: ({ t, appInfo, paletteShortcut }) => (
			<div className="mg-home">
				<h2>{t("home.title")}</h2>
				<p className="mg-muted">{t("home.subtitle", { shortcut: paletteShortcut })}</p>
				{appInfo ? (
					<p className="mg-muted">{`${appInfo.name} ${appInfo.version} · schema ${appInfo.schemaVersion}`}</p>
				) : null}
			</div>
		),
	},
	viewport: {
		titleKey: "panel.viewport",
		render: ({ t, api, transformMode, frameToken, theme }) => (
			<Viewport t={t} api={api} transformMode={transformMode} frameToken={frameToken} theme={theme} />
		),
	},
	outliner: {
		titleKey: "panel.outliner",
		render: ({ t, api }) => <Outliner t={t} api={api} />,
	},
	properties: {
		titleKey: "panel.properties",
		render: ({ t, api }) => <Properties t={t} api={api} />,
	},
	timeline: pendingPanel("panel.timeline", "timeline.pending"),
	assets: pendingPanel("panel.assets", "assets.pending"),
	tools: pendingPanel("panel.tools", "tools.pending"),
	shader: pendingPanel("panel.shader", "shader.pending"),
	geometryNodes: pendingPanel("panel.geometryNodes", "geometryNodes.pending"),
	uv: pendingPanel("panel.uv", "uv.pending"),
	compositor: pendingPanel("panel.compositor", "compositor.pending"),
	video: pendingPanel("panel.video", "video.pending"),
	preferences: {
		titleKey: "panel.preferences",
		render: ({ t, theme, locale, uiScale }) => (
			<dl className="mg-props">
				<dt>{t("preferences.theme")}</dt>
				<dd>{theme === "dark" ? t("theme.dark") : t("theme.light")}</dd>
				<dt>{t("preferences.language")}</dt>
				<dd>{locale.toUpperCase()}</dd>
				<dt>{t("preferences.uiScale")}</dt>
				<dd>{`${Math.round(uiScale * 100)}%`}</dd>
			</dl>
		),
	},
}

export function panelDefinition(panel: PanelId): PanelDefinition {
	return PANELS[panel]
}
