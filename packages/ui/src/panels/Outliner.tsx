import { useCallback, useState, type DragEvent as ReactDragEvent, type KeyboardEvent as ReactKeyboardEvent, type ReactNode } from "react"
import type { Translate } from "@megu3d/i18n"
import type { SceneNodeDto } from "@megu3d/types"

import { kindBadge, kindMessageKey } from "../scene/selectors"
import type { SceneApi } from "../scene/useScene"

/** Drag payload for reparenting rows; distinct from the dock's tab MIME. */
export const NODE_MIME = "application/x-megu3d-node"

const INDENT_PX = 14

export type OutlinerProps = {
	t: Translate
	api: SceneApi
}

/**
 * Scene hierarchy. The list is already in pre-order with depth from the core, so
 * the panel stays a pure projection: no local tree state to drift out of sync.
 */
export function Outliner({ t, api }: OutlinerProps): ReactNode {
	const [dropTarget, setDropTarget] = useState<string | null>(null)
	const scene = api.scene
	const nodes = scene?.nodes ?? []

	const onDragStart = useCallback((event: ReactDragEvent<HTMLLIElement>, uuid: string) => {
		event.dataTransfer.setData(NODE_MIME, uuid)
		event.dataTransfer.effectAllowed = "move"
	}, [])

	const onDrop = useCallback(
		(event: ReactDragEvent<HTMLElement>, parent: string | null) => {
			event.preventDefault()
			setDropTarget(null)
			const uuid = event.dataTransfer.getData(NODE_MIME)
			if (!uuid || uuid === parent) return
			api.reparent(uuid, parent)
		},
		[api],
	)

	const onKeyDown = useCallback(
		(event: ReactKeyboardEvent<HTMLUListElement>) => {
			if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return
			event.preventDefault()
			const current = nodes.findIndex((node) => node.uuid === scene?.active)
			const step = event.key === "ArrowDown" ? 1 : -1
			const next = nodes[Math.min(Math.max(current + step, 0), nodes.length - 1)]
			if (next) api.select([next.uuid], "replace")
		},
		[api, nodes, scene],
	)

	if (api.status === "browser") {
		return <p className="mg-pending mg-muted">{t("viewport.browserPreview")}</p>
	}
	if (nodes.length === 0) {
		return <p className="mg-pending mg-muted">{t("outliner.empty")}</p>
	}

	return (
		<div
			className="mg-outliner"
			onDragOver={(event) => event.preventDefault()}
			onDrop={(event) => onDrop(event, null)}
		>
			<ul className="mg-outliner-list" role="tree" aria-label={t("panel.outliner")} tabIndex={0} onKeyDown={onKeyDown}>
				{nodes.map((node: SceneNodeDto) => {
					const isActive = node.uuid === scene?.active
					const classes = ["mg-outliner-row"]
					if (node.selected) classes.push("mg-outliner-row--selected")
					if (isActive) classes.push("mg-outliner-row--active")
					if (dropTarget === node.uuid) classes.push("mg-outliner-row--drop")
					return (
						<li
							key={node.uuid}
							className={classes.join(" ")}
							role="treeitem"
							aria-level={node.depth + 1}
							aria-selected={node.selected}
							draggable
							onDragStart={(event) => onDragStart(event, node.uuid)}
							onDragOver={(event) => {
								event.preventDefault()
								event.stopPropagation()
								setDropTarget(node.uuid)
							}}
							onDragLeave={() => setDropTarget((current) => (current === node.uuid ? null : current))}
							onDrop={(event) => {
								event.stopPropagation()
								onDrop(event, node.uuid)
							}}
						>
							<button
								type="button"
								className="mg-outliner-name"
								style={{ paddingLeft: `${node.depth * INDENT_PX}px` }}
								title={t(kindMessageKey(node.kind))}
								onClick={(event) =>
									api.select([node.uuid], event.ctrlKey || event.metaKey ? "toggle" : "replace")
								}
							>
								<span className="mg-outliner-kind" aria-hidden="true">
									{kindBadge(node.kind)}
								</span>
								<span className={node.visible ? undefined : "mg-muted"}>{node.name}</span>
							</button>
							<button
								type="button"
								className="mg-outliner-toggle"
								aria-pressed={node.visible}
								title={node.visible ? t("outliner.hide") : t("outliner.show")}
								onClick={() => api.setVisible(node.uuid, !node.visible)}
							>
								{node.visible ? "\u25c9" : "\u25cb"}
							</button>
						</li>
					)
				})}
			</ul>
			<p className="mg-outliner-hint mg-muted">{t("outliner.hint")}</p>
		</div>
	)
}
