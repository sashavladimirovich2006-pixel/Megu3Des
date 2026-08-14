import {
	useCallback,
	useEffect,
	useMemo,
	useRef,
	useState,
	type PointerEvent as ReactPointerEvent,
	type ReactNode,
} from "react"
import type { Translate } from "@megu3d/i18n"
import type { SceneNodeDto, TransformDto } from "@megu3d/types"

import type { Theme } from "../preferences"
import { activeNode, selectionBounds, toVec3, worldToLocalDelta } from "../scene/selectors"
import type { SceneApi } from "../scene/useScene"
import {
	DEFAULT_CAMERA,
	dolly,
	frame,
	orbit,
	pan,
	viewProjection,
	worldPerPixel,
	type Camera,
} from "./camera"
import {
	AXIS_COLOR,
	AXIS_VECTOR,
	axisHandles,
	dragScalar,
	pickAxis,
	withRotation,
	withScale,
	withTranslation,
	type Axis,
	type AxisHandle,
	type TransformMode,
} from "./gizmo"
import { BOX_EDGES, boxCorners, project, scaled, type Mat4, type Vec3 } from "./math"
import { pickNodeAt } from "./pick"

const GRID_EXTENT = 10
const ROTATE_DEG_PER_PIXEL = 0.5
const SCALE_PER_PIXEL = 0.01
const CLICK_SLOP = 3

type Palette = {
	background: string
	grid: string
	gridStrong: string
	node: string
	hidden: string
	selected: string
	active: string
	marker: string
}

const DARK: Palette = {
	background: "#16181c",
	grid: "#23272e",
	gridStrong: "#333944",
	node: "#8d97a6",
	hidden: "#3a4049",
	selected: "#ffa348",
	active: "#ffd24a",
	marker: "#c8cedb",
}

const LIGHT: Palette = {
	background: "#eef1f5",
	grid: "#dbe0e8",
	gridStrong: "#c2c9d4",
	node: "#5b6472",
	hidden: "#c7ccd4",
	selected: "#d9791f",
	active: "#b8860b",
	marker: "#2a2f38",
}

type Drag =
	| { kind: "orbit" | "pan"; lastX: number; lastY: number }
	| {
			kind: "gizmo"
			axis: Axis
			handle: AxisHandle
			node: string
			start: TransformDto
			startX: number
			startY: number
			moved: boolean
	  }

export type ViewportProps = {
	t: Translate
	api: SceneApi
	transformMode: TransformMode
	frameToken: number
	theme: Theme
}

function drawGrid(ctx: CanvasRenderingContext2D, vp: Mat4, width: number, height: number, palette: Palette): void {
	ctx.lineWidth = 1
	for (let i = -GRID_EXTENT; i <= GRID_EXTENT; i += 1) {
		const isAxis = i === 0
		for (const horizontal of [true, false]) {
			const from: Vec3 = horizontal ? [-GRID_EXTENT, i, 0] : [i, -GRID_EXTENT, 0]
			const to: Vec3 = horizontal ? [GRID_EXTENT, i, 0] : [i, GRID_EXTENT, 0]
			const a = project(vp, from, width, height)
			const b = project(vp, to, width, height)
			if (a.behind || b.behind) continue
			ctx.strokeStyle = isAxis
				? horizontal
					? AXIS_COLOR.x
					: AXIS_COLOR.y
				: i % 5 === 0
					? palette.gridStrong
					: palette.grid
			ctx.beginPath()
			ctx.moveTo(a.x, a.y)
			ctx.lineTo(b.x, b.y)
			ctx.stroke()
		}
	}
}

function drawNode(
	ctx: CanvasRenderingContext2D,
	node: SceneNodeDto,
	vp: Mat4,
	width: number,
	height: number,
	palette: Palette,
	isActive: boolean,
): void {
	const color = !node.visible
		? palette.hidden
		: isActive
			? palette.active
			: node.selected
				? palette.selected
				: palette.node
	ctx.strokeStyle = color
	ctx.lineWidth = node.selected ? 2 : 1
	if (node.mesh) {
		const corners = boxCorners(toVec3(node.worldBoundsMin), toVec3(node.worldBoundsMax)).map(
			(corner) => project(vp, corner, width, height),
		)
		for (const [from, to] of BOX_EDGES) {
			const a = corners[from]
			const b = corners[to]
			if (!a || !b || a.behind || b.behind) continue
			ctx.beginPath()
			ctx.moveTo(a.x, a.y)
			ctx.lineTo(b.x, b.y)
			ctx.stroke()
		}
	}
	const origin = project(vp, toVec3(node.worldTranslation), width, height)
	if (origin.behind) return
	const size = node.mesh ? 4 : 7
	ctx.strokeStyle = node.selected || isActive ? color : palette.marker
	ctx.beginPath()
	ctx.moveTo(origin.x - size, origin.y)
	ctx.lineTo(origin.x + size, origin.y)
	ctx.moveTo(origin.x, origin.y - size)
	ctx.lineTo(origin.x, origin.y + size)
	ctx.stroke()
}

function drawGizmo(ctx: CanvasRenderingContext2D, handles: AxisHandle[], mode: TransformMode): void {
	for (const handle of handles) {
		ctx.strokeStyle = AXIS_COLOR[handle.axis]
		ctx.fillStyle = AXIS_COLOR[handle.axis]
		ctx.lineWidth = 2
		ctx.beginPath()
		ctx.moveTo(handle.originX, handle.originY)
		ctx.lineTo(handle.tipX, handle.tipY)
		ctx.stroke()
		ctx.beginPath()
		if (mode === "move") {
			ctx.arc(handle.tipX, handle.tipY, 4.5, 0, Math.PI * 2)
			ctx.fill()
		} else if (mode === "scale") {
			ctx.fillRect(handle.tipX - 4, handle.tipY - 4, 8, 8)
		} else {
			ctx.arc(handle.tipX, handle.tipY, 5, 0, Math.PI * 2)
			ctx.stroke()
		}
	}
}

/**
 * Wireframe preview viewport: real camera, real picking, real gizmo drags that
 * stream through the core's preview API. Shading, textures and GPU picking
 * arrive with the wgpu renderer; nothing here pretends otherwise.
 */
export function Viewport({ t, api, transformMode, frameToken, theme }: ViewportProps): ReactNode {
	const [camera, setCamera] = useState<Camera>(DEFAULT_CAMERA)
	const [size, setSize] = useState({ width: 640, height: 360 })
	const containerRef = useRef<HTMLDivElement | null>(null)
	const canvasRef = useRef<HTMLCanvasElement | null>(null)
	const dragRef = useRef<Drag | null>(null)
	const pendingRef = useRef<{ node: string; transform: TransformDto } | null>(null)
	const rafRef = useRef(0)
	const lastFrameToken = useRef(0)

	const scene = api.scene
	const active = activeNode(scene)
	const palette = theme === "dark" ? DARK : LIGHT
	const vp = useMemo(
		() => viewProjection(camera, size.width, size.height),
		[camera, size.height, size.width],
	)
	const handles = useMemo(
		() => (active ? axisHandles(vp, toVec3(active.worldTranslation), size.width, size.height) : []),
		[active, size.height, size.width, vp],
	)

	useEffect(() => {
		const container = containerRef.current
		if (!container) return
		const observer = new ResizeObserver((entries) => {
			const entry = entries[0]
			if (!entry) return
			setSize({
				width: Math.max(1, Math.round(entry.contentRect.width)),
				height: Math.max(1, Math.round(entry.contentRect.height)),
			})
		})
		observer.observe(container)
		return () => observer.disconnect()
	}, [])

	useEffect(() => {
		const canvas = canvasRef.current
		if (!canvas) return
		const ratio = typeof window === "undefined" ? 1 : Math.min(window.devicePixelRatio || 1, 2)
		canvas.width = Math.round(size.width * ratio)
		canvas.height = Math.round(size.height * ratio)
		const ctx = canvas.getContext("2d")
		if (!ctx) return
		ctx.setTransform(ratio, 0, 0, ratio, 0, 0)
		ctx.fillStyle = palette.background
		ctx.fillRect(0, 0, size.width, size.height)
		drawGrid(ctx, vp, size.width, size.height, palette)
		for (const node of scene?.nodes ?? []) {
			drawNode(ctx, node, vp, size.width, size.height, palette, node.uuid === scene?.active)
		}
		if (handles.length > 0) drawGizmo(ctx, handles, transformMode)
	}, [handles, palette, scene, size.height, size.width, transformMode, vp])

	useEffect(() => {
		if (frameToken === lastFrameToken.current) return
		lastFrameToken.current = frameToken
		const bounds = selectionBounds(scene)
		if (bounds) setCamera((current) => frame(current, bounds.min, bounds.max))
	}, [frameToken, scene])

	const flush = useCallback(() => {
		rafRef.current = 0
		const pending = pendingRef.current
		pendingRef.current = null
		if (pending) api.setTransform(pending.node, pending.transform, true)
	}, [api])

	const queuePreview = useCallback(
		(node: string, transform: TransformDto) => {
			pendingRef.current = { node, transform }
			if (rafRef.current !== 0) return
			rafRef.current =
				typeof window === "undefined" ? 0 : window.requestAnimationFrame(() => flush())
		},
		[flush],
	)

	useEffect(() => {
		const canvas = canvasRef.current
		if (!canvas) return
		function handleWheel(event: WheelEvent) {
			event.preventDefault()
			setCamera((current) => dolly(current, event.deltaY > 0 ? 1 : -1))
		}
		canvas.addEventListener("wheel", handleWheel, { passive: false })
		return () => canvas.removeEventListener("wheel", handleWheel)
	}, [])

	useEffect(() => {
		function handleKeyDown(event: globalThis.KeyboardEvent) {
			const drag = dragRef.current
			if (event.key !== "Escape" || !drag || drag.kind !== "gizmo") return
			dragRef.current = null
			pendingRef.current = null
			api.cancelPreview()
		}
		window.addEventListener("keydown", handleKeyDown)
		return () => window.removeEventListener("keydown", handleKeyDown)
	}, [api])

	const handlePointerDown = useCallback(
		(event: ReactPointerEvent<HTMLCanvasElement>) => {
			const canvas = event.currentTarget
			const rect = canvas.getBoundingClientRect()
			const x = event.clientX - rect.left
			const y = event.clientY - rect.top
			canvas.setPointerCapture(event.pointerId)
			if (event.button === 1 || (event.button === 0 && event.altKey)) {
				dragRef.current = {
					kind: event.shiftKey ? "pan" : "orbit",
					lastX: event.clientX,
					lastY: event.clientY,
				}
				return
			}
			if (event.button !== 0) return
			const axis = pickAxis(handles, x, y)
			const handle = handles.find((entry) => entry.axis === axis)
			if (active && axis && handle) {
				dragRef.current = {
					kind: "gizmo",
					axis,
					handle,
					node: active.uuid,
					start: active.transform,
					startX: event.clientX,
					startY: event.clientY,
					moved: false,
				}
				return
			}
			const hit = pickNodeAt(scene?.nodes ?? [], vp, size.width, size.height, x, y)
			const additive = event.ctrlKey || event.metaKey || event.shiftKey
			if (hit) api.select([hit], additive ? "toggle" : "replace")
			else if (!additive) api.select([], "clear")
		},
		[active, api, handles, scene, size.height, size.width, vp],
	)

	const handlePointerMove = useCallback(
		(event: ReactPointerEvent<HTMLCanvasElement>) => {
			const drag = dragRef.current
			if (!drag) return
			if (drag.kind === "orbit" || drag.kind === "pan") {
				const dx = event.clientX - drag.lastX
				const dy = event.clientY - drag.lastY
				drag.lastX = event.clientX
				drag.lastY = event.clientY
				setCamera((current) =>
					drag.kind === "orbit" ? orbit(current, dx, dy) : pan(current, dx, dy, size.height),
				)
				return
			}
			const dx = event.clientX - drag.startX
			const dy = event.clientY - drag.startY
			if (!drag.moved && Math.hypot(dx, dy) < CLICK_SLOP) return
			drag.moved = true
			const travel = dragScalar(drag.handle, dx, dy)
			const node = scene?.nodes.find((entry) => entry.uuid === drag.node)
			if (!node) return
			let next: TransformDto
			if (transformMode === "move") {
				const worldDelta = scaled(AXIS_VECTOR[drag.axis], travel * worldPerPixel(camera, size.height))
				next = withTranslation(drag.start, worldToLocalDelta(scene, node, worldDelta))
			} else if (transformMode === "rotate") {
				next = withRotation(drag.start, drag.axis, travel * ROTATE_DEG_PER_PIXEL)
			} else {
				next = withScale(drag.start, drag.axis, Math.max(0.01, 1 + travel * SCALE_PER_PIXEL))
			}
			queuePreview(drag.node, next)
		},
		[camera, queuePreview, scene, size.height, transformMode],
	)

	const endDrag = useCallback(
		(event: ReactPointerEvent<HTMLCanvasElement>) => {
			const drag = dragRef.current
			dragRef.current = null
			if (event.currentTarget.hasPointerCapture(event.pointerId)) {
				event.currentTarget.releasePointerCapture(event.pointerId)
			}
			if (!drag || drag.kind !== "gizmo" || !drag.moved) return
			flush()
			api.commitPreview()
		},
		[api, flush],
	)

	const overlay =
		api.status === "browser"
			? t("viewport.browserPreview")
			: api.status === "loading"
				? t("status.connecting")
				: api.error !== null
					? api.error
					: null

	return (
		<div className="mg-viewport" ref={containerRef}>
			<canvas
				ref={canvasRef}
				className="mg-viewport-canvas"
				style={{ width: `${size.width}px`, height: `${size.height}px` }}
				aria-label={t("panel.viewport")}
				onPointerDown={handlePointerDown}
				onPointerMove={handlePointerMove}
				onPointerUp={endDrag}
				onPointerCancel={endDrag}
			/>
			<div className="mg-viewport-hud">
				<span className="mg-badge">{t(`viewport.mode.${transformMode}`)}</span>
				<span className="mg-muted">{t("viewport.wireframe")}</span>
			</div>
			{overlay ? <p className="mg-viewport-overlay mg-muted">{overlay}</p> : null}
			<p className="mg-viewport-hint mg-muted">{t("viewport.hint")}</p>
		</div>
	)
}
