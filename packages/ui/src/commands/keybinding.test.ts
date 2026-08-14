import { describe, expect, it } from "vitest"

import { formatChord, matchesChord, parseChord } from "./keybinding"

const event = (overrides: Partial<Parameters<typeof matchesChord>[1]>) => ({
	ctrlKey: false,
	shiftKey: false,
	altKey: false,
	metaKey: false,
	code: "KeyA",
	...overrides,
})

describe("keybinding", () => {
	it("parses modifiers and physical key codes", () => {
		expect(parseChord("Ctrl+Shift+P")).toEqual({
			ctrl: true,
			shift: true,
			alt: false,
			code: "KeyP",
		})
		expect(parseChord("Alt+1")?.code).toBe("Digit1")
		expect(parseChord("F12")?.code).toBe("F12")
		expect(parseChord("Ctrl+Slash")?.code).toBe("Slash")
	})

	it("rejects malformed bindings", () => {
		expect(parseChord("Ctrl+Meta+K")).toBeNull()
		expect(parseChord("Ctrl+")).toBeNull()
		expect(parseChord("")).toBeNull()
	})

	it("round-trips through the display format", () => {
		for (const binding of ["Ctrl+Shift+P", "Ctrl+Alt+T", "Alt+9", "Ctrl+Z"]) {
			const chord = parseChord(binding)
			expect(chord).not.toBeNull()
			if (chord) expect(formatChord(chord)).toBe(binding)
		}
	})

	it("matches on physical code so layout and locale do not matter", () => {
		const chord = parseChord("Ctrl+Shift+P")
		expect(chord).not.toBeNull()
		if (!chord) return
		expect(matchesChord(chord, event({ ctrlKey: true, shiftKey: true, code: "KeyP" }))).toBe(true)
		expect(matchesChord(chord, event({ ctrlKey: true, code: "KeyP" }))).toBe(false)
		expect(
			matchesChord(chord, event({ ctrlKey: true, shiftKey: true, metaKey: true, code: "KeyP" })),
		).toBe(false)
	})
})
