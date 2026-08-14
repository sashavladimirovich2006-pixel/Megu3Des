import { describe, expect, it } from "vitest"

import { LOCALES, catalogFor, createTranslator, detectLocale, en, isLocale } from "./index"

describe("i18n", () => {
	it("keeps every locale in sync with the source catalog", () => {
		const keys = Object.keys(en).sort()
		for (const locale of LOCALES) {
			expect(Object.keys(catalogFor(locale)).sort()).toEqual(keys)
		}
	})

	it("has no empty translations", () => {
		for (const locale of LOCALES) {
			for (const [key, value] of Object.entries(catalogFor(locale))) {
				expect(value.trim(), key).not.toBe("")
			}
		}
	})

	it("interpolates variables and keeps unknown placeholders", () => {
		const t = createTranslator("en")
		expect(t("status.nodes", { count: 3 })).toBe("3 node(s)")
		expect(t("status.nodes")).toBe("{count} node(s)")
	})

	it("detects russian locales and falls back to english", () => {
		expect(detectLocale("ru-RU")).toBe("ru")
		expect(detectLocale("RU")).toBe("ru")
		expect(detectLocale("de-DE")).toBe("en")
		expect(isLocale("fr")).toBe(false)
	})
})
