# Megu3D

Professional full-pipeline 3D suite for Windows 10/11 x64: modeling, sculpting,
procedural nodes, UV, texturing, rigging, animation, simulation, realtime PBR
viewport, path tracing, compositing and video editing.

**Status:** M3.2 — scene core plus the UI on top of it: canvas viewport with orbit/pan/dolly navigation, screen-space picking, move/rotate/scale gizmos, outliner with drag-and-drop reparenting, properties inspector and undo/redo wired to the command palette. The wgpu PBR renderer, project I/O and interop land in M4.

## Docs

- `docs/assumptions.md` — decisions, defaults, performance budgets, open questions
- `docs/01-prd.md` — product requirements, P0/P1/P2/P3 scope
- `docs/02-architecture.md` — modules, IPC contract, scene model, undo/redo, project format
- `docs/03-roadmap.md` — milestones M0…M10 with exit criteria
- `docs/04-ui-architecture.md` — workspaces, docking, command registry, design tokens, a11y

## Requirements

- Windows 10/11 x64 (dev also works on Linux/macOS for the Rust crates)
- Node.js >= 20.11 and pnpm 9 (`corepack enable`)
- Rust stable (`rustup`), MSVC Build Tools with C++ workload
- WebView2 runtime (preinstalled on Windows 11)

## Quickstart

```bash
pnpm install
pnpm ipc:types      # regenerate TS types from Rust (ts-rs)
pnpm dev            # Tauri dev window + Vite HMR
pnpm ci             # typecheck, lint, tests, cargo fmt/clippy/test
```

Release build (no installer bundle): `pnpm build`.

## Layout

```text
apps/desktop        Tauri v2 shell + Vite/React frontend
packages/ui         React UI shell and panels
packages/ipc        typed IPC client (intents, queries, events)
packages/types      shared TS types, incl. generated from Rust
crates/megu3d-core  scene model, ids, app metadata (source of truth)
crates/megu3d-cmd   command pattern, transactions, undo/redo history
.github/workflows   CI gates
```

## License

MIT — see `LICENSE`.
