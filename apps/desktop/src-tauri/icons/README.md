# Icons

Binary icons are not committed yet. Generate the full set from a 1024x1024 PNG:

```bash
pnpm --filter @megu3d/desktop exec tauri icon ../../assets/logo.png
```

Until then build without installers: `pnpm build` (runs `tauri build --no-bundle`).
