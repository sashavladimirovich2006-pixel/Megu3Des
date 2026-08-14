import { useEffect, useState, type ReactNode } from "react"
import type { MessageKey, Translate } from "@megu3d/i18n"
import type { TransformDto, Vec3Dto } from "@megu3d/types"

import { activeNode, formatNumber, kindMessageKey } from "../scene/selectors"
import type { SceneApi } from "../scene/useScene"

export type PropertiesProps = {
	t: Translate
	api: SceneApi
}

type NumberFieldProps = {
	label: string
	value: number
	step: number
	onCommit: (value: number) => void
}

/** Text-first numeric input: the draft is free-form until it parses and commits. */
function NumberField({ label, value, step, onCommit }: NumberFieldProps): ReactNode {
	const [draft, setDraft] = useState(() => formatNumber(value))
	const [editing, setEditing] = useState(false)

	useEffect(() => {
		if (!editing) setDraft(formatNumber(value))
	}, [editing, value])

	function commit() {
		setEditing(false)
		const parsed = Number.parseFloat(draft.replace(",", "."))
		if (Number.isFinite(parsed) && parsed !== value) onCommit(parsed)
		else setDraft(formatNumber(value))
	}

	return (
		<label className="mg-number">
			<span className="mg-number-label mg-muted">{label}</span>
			<input
				className="mg-input"
				type="number"
				step={step}
				value={draft}
				onChange={(event) => {
					setEditing(true)
					setDraft(event.target.value)
				}}
				onBlur={commit}
				onKeyDown={(event) => {
					if (event.key === "Enter") event.currentTarget.blur()
					if (event.key === "Escape") {
						setEditing(false)
						setDraft(formatNumber(value))
					}
				}}
			/>
		</label>
	)
}

type VectorFieldProps = {
	title: string
	value: Vec3Dto
	step: number
	onCommit: (value: Vec3Dto) => void
}

function VectorField({ title, value, step, onCommit }: VectorFieldProps): ReactNode {
	return (
		<div className="mg-vector">
			<span className="mg-section-title">{title}</span>
			<div className="mg-vector-row">
				<NumberField label="X" value={value.x} step={step} onCommit={(x) => onCommit({ ...value, x })} />
				<NumberField label="Y" value={value.y} step={step} onCommit={(y) => onCommit({ ...value, y })} />
				<NumberField label="Z" value={value.z} step={step} onCommit={(z) => onCommit({ ...value, z })} />
			</div>
		</div>
	)
}

/** Inspector for the active node. Every edit is a command, so every edit undoes. */
export function Properties({ t, api }: PropertiesProps): ReactNode {
	const node = activeNode(api.scene)
	const [name, setName] = useState(node?.name ?? "")
	const [renaming, setRenaming] = useState(false)

	useEffect(() => {
		if (!renaming) setName(node?.name ?? "")
	}, [node?.name, node?.uuid, renaming])

	if (api.status === "browser") {
		return <p className="mg-pending mg-muted">{t("viewport.browserPreview")}</p>
	}
	if (!node) {
		return <p className="mg-pending mg-muted">{t("properties.empty")}</p>
	}

	const history = api.scene?.history ?? null

	function applyTransform(next: TransformDto) {
		if (node) api.setTransform(node.uuid, next, false)
	}

	function commitName() {
		setRenaming(false)
		const trimmed = name.trim()
		if (node && trimmed.length > 0 && trimmed !== node.name) api.rename(node.uuid, trimmed)
		else setName(node?.name ?? "")
	}

	const rows: Array<{ key: MessageKey; value: string }> = [
		{ key: "properties.kind", value: t(kindMessageKey(node.kind)) },
	]
	if (node.mesh) {
		rows.push(
			{ key: "properties.vertices", value: String(node.mesh.vertexCount) },
			{ key: "properties.triangles", value: String(node.mesh.triangleCount) },
		)
	}

	return (
		<div className="mg-properties">
			<label className="mg-field">
				<span className="mg-section-title">{t("properties.name")}</span>
				<input
					className="mg-input"
					type="text"
					value={name}
					onChange={(event) => {
						setRenaming(true)
						setName(event.target.value)
					}}
					onBlur={commitName}
					onKeyDown={(event) => {
						if (event.key === "Enter") event.currentTarget.blur()
						if (event.key === "Escape") {
							setRenaming(false)
							setName(node.name)
						}
					}}
				/>
			</label>

			<VectorField
				title={t("properties.translation")}
				value={node.transform.translation}
				step={0.1}
				onCommit={(translation) => applyTransform({ ...node.transform, translation })}
			/>
			<VectorField
				title={t("properties.rotation")}
				value={node.transform.rotationEulerDeg}
				step={1}
				onCommit={(rotationEulerDeg) => applyTransform({ ...node.transform, rotationEulerDeg })}
			/>
			<VectorField
				title={t("properties.scale")}
				value={node.transform.scale}
				step={0.1}
				onCommit={(scale) => applyTransform({ ...node.transform, scale })}
			/>

			<label className="mg-field mg-field--inline">
				<input
					type="checkbox"
					checked={node.visible}
					onChange={(event) => api.setVisible(node.uuid, event.target.checked)}
				/>
				<span>{t("properties.visible")}</span>
			</label>

			<dl className="mg-props">
				{rows.map((row) => (
					<div key={row.key} className="mg-props-row">
						<dt>{t(row.key)}</dt>
						<dd>{row.value}</dd>
					</div>
				))}
				<div className="mg-props-row">
					<dt>{t("properties.history")}</dt>
					<dd>
						{history
							? `${history.undoDepth}/${history.undoLimit}${history.lastLabel ? ` \u00b7 ${history.lastLabel}` : ""}`
							: "\u2014"}
					</dd>
				</div>
			</dl>

			{api.error ? <p className="mg-error">{api.error}</p> : null}
		</div>
	)
}
