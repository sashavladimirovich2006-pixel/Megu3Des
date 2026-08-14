import { useCallback, useEffect, useMemo, useRef, useState } from "react"
import { desktopTransport, isDesktop, type SceneTransport } from "@megu3d/ipc"
import type {
	CommandRequestDto,
	PrimitiveKindDto,
	SceneSnapshotDto,
	SelectionModeDto,
	TransformDto,
} from "@megu3d/types"

export type SceneStatus = "loading" | "ready" | "browser" | "error"

export type SceneApi = {
	scene: SceneSnapshotDto | null
	status: SceneStatus
	error: string | null
	send: (request: CommandRequestDto) => void
	add: (primitive: PrimitiveKindDto, parent?: string | null) => void
	remove: () => void
	duplicate: () => void
	rename: (node: string, name: string) => void
	setTransform: (node: string, transform: TransformDto, preview?: boolean) => void
	commitPreview: () => void
	cancelPreview: () => void
	setVisible: (node: string, visible: boolean) => void
	reparent: (node: string, parent: string | null) => void
	select: (nodes: string[], mode: SelectionModeDto) => void
	undo: () => void
	redo: () => void
}

/**
 * The UI's only door to the scene. Requests are serialized through one promise
 * chain so a fast gizmo drag can never apply out of order, and the authoritative
 * snapshot that comes back from Rust replaces local state wholesale.
 */
export function useScene(transport: SceneTransport = desktopTransport): SceneApi {
	const [scene, setScene] = useState<SceneSnapshotDto | null>(null)
	const [status, setStatus] = useState<SceneStatus>("loading")
	const [error, setError] = useState<string | null>(null)
	const queue = useRef<Promise<void>>(Promise.resolve())
	const inFlight = useRef(0)
	const mounted = useRef(true)

	useEffect(() => {
		mounted.current = true
		return () => {
			mounted.current = false
		}
	}, [])

	const refresh = useCallback(async () => {
		try {
			const next = await transport.queryScene()
			if (!mounted.current) return
			setScene(next)
			setStatus("ready")
			setError(null)
		} catch (cause) {
			if (!mounted.current) return
			setStatus("error")
			setError(String(cause))
		}
	}, [transport])

	useEffect(() => {
		if (!isDesktop()) {
			setStatus("browser")
			return
		}
		void refresh()
	}, [refresh])

	useEffect(() => {
		if (!isDesktop()) return
		let dispose: (() => void) | null = null
		void transport
			.onScenePatch(() => {
				// Our own dispatch already returns the new snapshot; only refetch when
				// something else (autosave, another window) touched the scene.
				if (inFlight.current === 0) void refresh()
			})
			.then((unlisten) => {
				if (mounted.current) dispose = unlisten
				else unlisten()
			})
			.catch(() => undefined)
		return () => {
			dispose?.()
		}
	}, [refresh, transport])

	const send = useCallback(
		(request: CommandRequestDto) => {
			if (!isDesktop()) {
				setStatus("browser")
				return
			}
			inFlight.current += 1
			queue.current = queue.current
				.then(async () => {
					try {
						const result = await transport.dispatch(request)
						if (!mounted.current) return
						setScene(result.scene)
						setStatus("ready")
						setError(null)
					} catch (cause) {
						if (mounted.current) setError(String(cause))
					}
				})
				.finally(() => {
					inFlight.current = Math.max(0, inFlight.current - 1)
				})
		},
		[transport],
	)

	return useMemo<SceneApi>(
		() => ({
			scene,
			status,
			error,
			send,
			add: (primitive, parent = null) => send({ type: "add", primitive, parent }),
			remove: () => send({ type: "delete" }),
			duplicate: () => send({ type: "duplicate" }),
			rename: (node, name) => send({ type: "rename", node, name }),
			setTransform: (node, transform, preview = false) =>
				send({ type: "setTransform", node, transform, preview }),
			commitPreview: () => send({ type: "commitPreview" }),
			cancelPreview: () => send({ type: "cancelPreview" }),
			setVisible: (node, visible) => send({ type: "setVisible", node, visible }),
			reparent: (node, parent) => send({ type: "reparent", node, parent }),
			select: (nodes, mode) => send({ type: "select", nodes, mode }),
			undo: () => send({ type: "undo" }),
			redo: () => send({ type: "redo" }),
		}),
		[error, scene, send, status],
	)
}
