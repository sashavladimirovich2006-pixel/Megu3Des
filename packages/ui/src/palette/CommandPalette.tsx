import {
	useEffect,
	useMemo,
	useRef,
	useState,
	type KeyboardEvent,
	type ReactNode,
} from "react"
import type { Translate } from "@megu3d/i18n"

import { formatChord, parseChord } from "../commands/keybinding"
import { commandTitle, type Command, type CommandContext } from "../commands/registry"
import { highlightRuns, rankItems } from "./fuzzy"

const MAX_RESULTS = 40

export type CommandPaletteProps = {
	commands: Command[]
	context: CommandContext
	t: Translate
	onClose: () => void
}

/** Ctrl+Shift+P palette: keyboard first, fuzzy ranked, grouped by category. */
export function CommandPalette({
	commands,
	context,
	t,
	onClose,
}: CommandPaletteProps): ReactNode {
	const [query, setQuery] = useState("")
	const [selected, setSelected] = useState(0)
	const inputRef = useRef<HTMLInputElement | null>(null)

	useEffect(() => {
		inputRef.current?.focus()
	}, [])

	const results = useMemo(
		() =>
			rankItems(query, commands, (command) => `${commandTitle(command, t)} ${command.id}`).slice(
				0,
				MAX_RESULTS,
			),
		[commands, query, t],
	)

	useEffect(() => {
		setSelected(0)
	}, [query])

	function runAt(index: number) {
		const entry = results[index]
		if (!entry) return
		onClose()
		entry.item.run(context)
	}

	function handleKeyDown(event: KeyboardEvent<HTMLDivElement>) {
		if (event.key === "Escape") {
			event.preventDefault()
			onClose()
		} else if (event.key === "ArrowDown") {
			event.preventDefault()
			setSelected((current) => Math.min(current + 1, Math.max(results.length - 1, 0)))
		} else if (event.key === "ArrowUp") {
			event.preventDefault()
			setSelected((current) => Math.max(current - 1, 0))
		} else if (event.key === "Enter") {
			event.preventDefault()
			runAt(selected)
		}
	}

	return (
		<div className="mg-overlay" role="presentation" onMouseDown={onClose}>
			<div
				className="mg-palette"
				role="dialog"
				aria-modal="true"
				aria-label={t("palette.title")}
				onMouseDown={(event) => event.stopPropagation()}
				onKeyDown={handleKeyDown}
			>
				<input
					ref={inputRef}
					className="mg-palette-input"
					type="text"
					value={query}
					placeholder={t("palette.placeholder")}
					aria-label={t("palette.placeholder")}
					onChange={(event) => setQuery(event.target.value)}
				/>
				{results.length === 0 ? (
					<p className="mg-palette-empty mg-muted">{t("palette.empty", { query })}</p>
				) : (
					<ul className="mg-palette-list" role="listbox" aria-label={t("palette.title")}>
						{results.map((entry, index) => {
							const title = commandTitle(entry.item, t)
							const chord = entry.item.keybinding ? parseChord(entry.item.keybinding) : null
							return (
								<li key={entry.item.id} role="none">
									<button
										type="button"
										role="option"
										aria-selected={index === selected}
										className={
											index === selected ? "mg-palette-item mg-palette-item--active" : "mg-palette-item"
										}
										onMouseEnter={() => setSelected(index)}
										onClick={() => runAt(index)}
									>
										<span className="mg-palette-category mg-muted">{t(entry.item.categoryKey)}</span>
										<span className="mg-palette-label">
											{highlightRuns(title, entry.match.indices).map((run, runIndex) =>
												run.matched ? (
													<mark key={runIndex} className="mg-palette-hit">
														{run.text}
													</mark>
												) : (
													<span key={runIndex}>{run.text}</span>
												),
											)}
										</span>
										{chord ? <kbd className="mg-kbd">{formatChord(chord)}</kbd> : null}
									</button>
								</li>
							)
						})}
					</ul>
				)}
			</div>
		</div>
	)
}
