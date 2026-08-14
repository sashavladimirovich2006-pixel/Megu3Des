import { en } from "./messages/en"
import { ru } from "./messages/ru"
import type { MessageKey } from "./messages/en"

export type { MessageKey }
export { en, ru }

export const LOCALES = ["en", "ru"] as const
export type Locale = (typeof LOCALES)[number]

export type MessageVars = Record<string, string | number>
export type Translate = (key: MessageKey, vars?: MessageVars) => string

const CATALOGS: Record<Locale, Record<MessageKey, string>> = { en, ru }

export function catalogFor(locale: Locale): Record<MessageKey, string> {
	return CATALOGS[locale]
}

/** RU is the default when the OS/browser locale is Russian, EN otherwise. */
export function detectLocale(language?: string): Locale {
	const tag =
		language ?? (typeof navigator === "undefined" ? undefined : navigator.language) ?? "en"
	return tag.toLowerCase().startsWith("ru") ? "ru" : "en"
}

export function isLocale(value: unknown): value is Locale {
	return typeof value === "string" && LOCALES.some((locale) => locale === value)
}

function interpolate(template: string, vars?: MessageVars): string {
	if (!vars) return template
	return template.replace(/\{(\w+)\}/g, (match, name: string) =>
		name in vars ? String(vars[name]) : match,
	)
}

export function createTranslator(locale: Locale): Translate {
	const catalog = CATALOGS[locale]
	return (key, vars) => interpolate(catalog[key], vars)
}
