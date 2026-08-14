/**
 * Keybindings are stored as physical `KeyboardEvent.code` values, so a shortcut
 * never depends on the keyboard layout or the interface language (D-70).
 */
export type Chord = { ctrl: boolean; shift: boolean; alt: boolean; code: string }

export type ChordEvent = {
	ctrlKey: boolean
	shiftKey: boolean
	altKey: boolean
	metaKey: boolean
	code: string
}

const NAMED_CODES: Record<string, string> = {
	esc: "Escape",
	escape: "Escape",
	enter: "Enter",
	tab: "Tab",
	space: "Space",
	backspace: "Backspace",
	delete: "Delete",
	insert: "Insert",
	home: "Home",
	end: "End",
	pageup: "PageUp",
	pagedown: "PageDown",
	left: "ArrowLeft",
	right: "ArrowRight",
	up: "ArrowUp",
	down: "ArrowDown",
	slash: "Slash",
	period: "Period",
	comma: "Comma",
	minus: "Minus",
	equal: "Equal",
	backquote: "Backquote",
}

function toCode(key: string): string | null {
	if (/^[a-z]$/i.test(key)) return `Key${key.toUpperCase()}`
	if (/^[0-9]$/.test(key)) return `Digit${key}`
	if (/^f([1-9]|1[0-2])$/i.test(key)) return `F${key.slice(1)}`
	return NAMED_CODES[key.toLowerCase()] ?? null
}

/** Returns `null` for malformed bindings instead of throwing: a bad keymap must not break the app. */
export function parseChord(binding: string): Chord | null {
	const parts = binding
		.split("+")
		.map((part) => part.trim())
		.filter((part) => part.length > 0)
	const key = parts.pop()
	if (key === undefined) return null

	let ctrl = false
	let shift = false
	let alt = false
	for (const part of parts) {
		switch (part.toLowerCase()) {
			case "ctrl":
			case "control":
				ctrl = true
				break
			case "shift":
				shift = true
				break
			case "alt":
				alt = true
				break
			default:
				return null
		}
	}

	const code = toCode(key)
	return code === null ? null : { ctrl, shift, alt, code }
}

export function formatChord(chord: Chord): string {
	const parts: string[] = []
	if (chord.ctrl) parts.push("Ctrl")
	if (chord.shift) parts.push("Shift")
	if (chord.alt) parts.push("Alt")
	if (chord.code.startsWith("Key")) parts.push(chord.code.slice(3))
	else if (chord.code.startsWith("Digit")) parts.push(chord.code.slice(5))
	else parts.push(chord.code)
	return parts.join("+")
}

export function matchesChord(chord: Chord, event: ChordEvent): boolean {
	return (
		!event.metaKey &&
		event.ctrlKey === chord.ctrl &&
		event.shiftKey === chord.shift &&
		event.altKey === chord.alt &&
		event.code === chord.code
	)
}

export function chordSignature(chord: Chord): string {
	return `${chord.ctrl ? "C" : ""}${chord.shift ? "S" : ""}${chord.alt ? "A" : ""}:${chord.code}`
}
