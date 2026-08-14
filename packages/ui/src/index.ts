export { AppShell } from "./AppShell"
export { WORKSPACES } from "./workspaces"
export type { Workspace } from "./workspaces"

export { DockView, TAB_MIME } from "./layout/DockView"
export { WORKSPACE_LAYOUTS, presetFor, split, tabs } from "./layout/presets"
export { loadLayouts, saveLayouts } from "./layout/persistence"
export { findPanel, findTabsNode, layoutReducer, normalizeSizes, visiblePanels } from "./layout/reducer"
export type { LayoutAction, TabRef } from "./layout/reducer"
export { LAYOUT_VERSION, MIN_SIZE, PANEL_IDS, isPanelId } from "./layout/types"
export type { Layout, LayoutNode, PanelId, PanelNode, SplitNode, TabsNode } from "./layout/types"

export { chordSignature, formatChord, matchesChord, parseChord } from "./commands/keybinding"
export type { Chord, ChordEvent } from "./commands/keybinding"
export { commandTitle, createCommands, findConflicts, panelTitleKey } from "./commands/registry"
export type { Command, CommandCategory, CommandContext } from "./commands/registry"
export { useKeybindings } from "./commands/useKeybindings"

export { CommandPalette } from "./palette/CommandPalette"
export { fuzzyMatch, highlightRuns, rankItems } from "./palette/fuzzy"
export type { FuzzyMatch } from "./palette/fuzzy"

export { PANELS, panelDefinition } from "./panels/registry"
export type { PanelContext, PanelDefinition } from "./panels/registry"

export {
	MAX_UI_SCALE,
	MIN_UI_SCALE,
	THEMES,
	applyPreferences,
	clampUiScale,
	defaultPreferences,
	isTheme,
	loadPreferences,
	nextLocale,
	nextTheme,
	savePreferences,
} from "./preferences"
export type { Preferences, Theme } from "./preferences"
