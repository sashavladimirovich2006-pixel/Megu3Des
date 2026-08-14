import {
	Fragment,
	useRef,
	type DragEvent,
	type KeyboardEvent,
	type PointerEvent,
	type ReactNode,
} from "react"

import type { LayoutAction, TabRef } from "./reducer"
import type { LayoutNode, PanelId, TabsNode } from "./types"

export const TAB_MIME = "application/x-megu3d-tab"

const KEYBOARD_STEP = 0.02

export type DockViewProps = {
	node: LayoutNode
	dispatch: (action: LayoutAction) => void
	renderPanel: (panel: PanelId) => ReactNode
	panelTitle: (panel: PanelId) => string
	closeLabel: string
	dropHint: string
}

/** Recursive dock renderer for the split/tabs tree. */
export function DockView(props: DockViewProps): ReactNode {
	const { node, dispatch } = props
	if (node.kind === "tabs") return <DockTabs {...props} node={node} />

	return (
		<div className={`mg-split mg-split--${node.direction}`}>
			{node.children.map((child, index) => (
				<Fragment key={child.id}>
					{index > 0 ? (
						<Splitter
							direction={node.direction}
							onResize={(delta) =>
								dispatch({ type: "resize", nodeId: node.id, index: index - 1, delta })
							}
						/>
					) : null}
					<div className="mg-split-item" style={{ flexGrow: node.sizes[index] ?? 1 }}>
						<DockView {...props} node={child} />
					</div>
				</Fragment>
			))}
		</div>
	)
}

function DockTabs({
	node,
	dispatch,
	renderPanel,
	panelTitle,
	closeLabel,
	dropHint,
}: DockViewProps & { node: TabsNode }): ReactNode {
	const active = node.children[node.activeIndex] ?? node.children[0]

	function handleDrop(event: DragEvent<HTMLElement>, index: number) {
		const from = readTabRef(event)
		if (!from) return
		event.preventDefault()
		dispatch({ type: "moveTab", from, to: { nodeId: node.id, index } })
	}

	return (
		<section className="mg-dock">
			<header
				className="mg-dock-tabs"
				onDragOver={(event) => {
					if (event.dataTransfer.types.includes(TAB_MIME)) event.preventDefault()
				}}
				onDrop={(event) => handleDrop(event, node.children.length)}
			>
				{node.children.map((child, index) => (
					<div
						key={child.id}
						className={
							index === node.activeIndex ? "mg-tab mg-tab--active mg-dock-tab" : "mg-tab mg-dock-tab"
						}
						draggable
						onDragStart={(event) => {
							event.dataTransfer.effectAllowed = "move"
							event.dataTransfer.setData(TAB_MIME, JSON.stringify({ nodeId: node.id, index }))
						}}
						onDrop={(event) => handleDrop(event, index)}
					>
						<button
							type="button"
							className="mg-dock-tab-label"
							aria-pressed={index === node.activeIndex}
							onClick={() => dispatch({ type: "activateTab", nodeId: node.id, index })}
						>
							{panelTitle(child.id)}
						</button>
						<button
							type="button"
							className="mg-dock-tab-close"
							title={closeLabel}
							aria-label={`${closeLabel}: ${panelTitle(child.id)}`}
							onClick={() => dispatch({ type: "closeTab", nodeId: node.id, index })}
						>
							&times;
						</button>
					</div>
				))}
			</header>
			<div className="mg-dock-body">{active ? renderPanel(active.id) : <p className="mg-muted">{dropHint}</p>}</div>
		</section>
	)
}

function Splitter({
	direction,
	onResize,
}: {
	direction: "row" | "column"
	onResize: (delta: number) => void
}): ReactNode {
	const ref = useRef<HTMLDivElement | null>(null)
	const origin = useRef(0)

	function handlePointerDown(event: PointerEvent<HTMLDivElement>) {
		event.preventDefault()
		event.currentTarget.setPointerCapture(event.pointerId)
		origin.current = direction === "row" ? event.clientX : event.clientY
	}

	function handlePointerMove(event: PointerEvent<HTMLDivElement>) {
		if (!event.currentTarget.hasPointerCapture(event.pointerId)) return
		const parent = ref.current?.parentElement
		if (!parent) return
		const total = direction === "row" ? parent.clientWidth : parent.clientHeight
		if (total <= 0) return
		const position = direction === "row" ? event.clientX : event.clientY
		const delta = (position - origin.current) / total
		if (Math.abs(delta) < 0.001) return
		origin.current = position
		onResize(delta)
	}

	function handlePointerUp(event: PointerEvent<HTMLDivElement>) {
		if (event.currentTarget.hasPointerCapture(event.pointerId)) {
			event.currentTarget.releasePointerCapture(event.pointerId)
		}
	}

	function handleKeyDown(event: KeyboardEvent<HTMLDivElement>) {
		const decrease = direction === "row" ? "ArrowLeft" : "ArrowUp"
		const increase = direction === "row" ? "ArrowRight" : "ArrowDown"
		if (event.key === decrease) {
			event.preventDefault()
			onResize(-KEYBOARD_STEP)
		} else if (event.key === increase) {
			event.preventDefault()
			onResize(KEYBOARD_STEP)
		}
	}

	return (
		<div
			ref={ref}
			className={`mg-splitter mg-splitter--${direction}`}
			role="separator"
			tabIndex={0}
			aria-orientation={direction === "row" ? "vertical" : "horizontal"}
			onPointerDown={handlePointerDown}
			onPointerMove={handlePointerMove}
			onPointerUp={handlePointerUp}
			onKeyDown={handleKeyDown}
		/>
	)
}

function readTabRef(event: DragEvent<HTMLElement>): TabRef | null {
	const raw = event.dataTransfer.getData(TAB_MIME)
	if (!raw) return null
	try {
		const parsed: unknown = JSON.parse(raw)
		if (typeof parsed !== "object" || parsed === null) return null
		const candidate = parsed as { nodeId?: unknown; index?: unknown }
		if (typeof candidate.nodeId !== "string" || typeof candidate.index !== "number") return null
		return { nodeId: candidate.nodeId, index: candidate.index }
	} catch (error) {
		console.warn("megu3d: invalid tab drag payload", error)
		return null
	}
}
