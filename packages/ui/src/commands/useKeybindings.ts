import { useEffect } from "react"

import { matchesChord, parseChord } from "./keybinding"
import type { Command, CommandContext } from "./registry"

function isTextEntry(target: EventTarget | null): boolean {
	if (!(target instanceof HTMLElement)) return false
	return (
		target.tagName === "INPUT" ||
		target.tagName === "TEXTAREA" ||
		target.tagName === "SELECT" ||
		target.isContentEditable
	)
}

/** Global keymap layer. Disabled while a modal (palette) owns the keyboard. */
export function useKeybindings(
	commands: Command[],
	context: CommandContext,
	disabled = false,
): void {
	useEffect(() => {
		if (disabled) return

		function handleKeyDown(event: globalThis.KeyboardEvent) {
			if (isTextEntry(event.target)) return
			for (const command of commands) {
				if (!command.keybinding) continue
				const chord = parseChord(command.keybinding)
				if (!chord || !matchesChord(chord, event)) continue
				event.preventDefault()
				command.run(context)
				return
			}
		}

		window.addEventListener("keydown", handleKeyDown)
		return () => window.removeEventListener("keydown", handleKeyDown)
	}, [commands, context, disabled])
}
