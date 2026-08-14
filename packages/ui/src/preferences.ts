import { detectLocale, isLocale, type Locale } from "@megu3d/i18n"

export const THEMES = ["dark", "light"] as const
export type Theme = (typeof THEMES)[number]

export type Preferences = { theme: Theme; locale: Locale; uiScale: number }

const STORAGE_KEY = "megu3d.preferences.v1"

/** A-06: UI scale range for HiDPI displays and accessibility. */
export const MIN_UI_SCALE = 0.8
export const MAX_UI_SCALE = 2

export function isTheme(value: unknown): value is Theme {
	return typeof value === "string" && THEMES.some((theme) => theme === value)
}

export function clampUiScale(scale: number): number {
	if (!Number.isFinite(scale)) return 1
	return Math.min(Math.max(scale, MIN_UI_SCALE), MAX_UI_SCALE)
}

export function defaultPreferences(): Preferences {
	return { theme: "dark", locale: detectLocale(), uiScale: 1 }
}

export function loadPreferences(): Preferences {
	const defaults = defaultPreferences()
	if (typeof localStorage === "undefined") return defaults
	try {
		const raw = localStorage.getItem(STORAGE_KEY)
		if (!raw) return defaults
		const parsed: unknown = JSON.parse(raw)
		if (typeof parsed !== "object" || parsed === null) return defaults
		const candidate = parsed as { theme?: unknown; locale?: unknown; uiScale?: unknown }
		return {
			theme: isTheme(candidate.theme) ? candidate.theme : defaults.theme,
			locale: isLocale(candidate.locale) ? candidate.locale : defaults.locale,
			uiScale:
				typeof candidate.uiScale === "number" ? clampUiScale(candidate.uiScale) : defaults.uiScale,
		}
	} catch (error) {
		console.warn("megu3d: ignoring corrupted preferences", error)
		return defaults
	}
}

export function savePreferences(preferences: Preferences): void {
	if (typeof localStorage === "undefined") return
	try {
		localStorage.setItem(STORAGE_KEY, JSON.stringify(preferences))
	} catch (error) {
		console.warn("megu3d: failed to persist preferences", error)
	}
}

/** Themes are plain data attributes so CSS owns the tokens (no inline style soup). */
export function applyPreferences(preferences: Preferences): void {
	if (typeof document === "undefined") return
	const root = document.documentElement
	root.dataset.theme = preferences.theme
	root.lang = preferences.locale
	root.style.setProperty("--mg-scale", String(clampUiScale(preferences.uiScale)))
}

export function nextTheme(theme: Theme): Theme {
	return theme === "dark" ? "light" : "dark"
}

export function nextLocale(locale: Locale): Locale {
	return locale === "ru" ? "en" : "ru"
}
