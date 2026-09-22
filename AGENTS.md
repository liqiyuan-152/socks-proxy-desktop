# Project Guidance

## Overview

`socks-proxy` is a Tauri 2 desktop application for managing SOCKS proxies.

- Frontend: React 19, TypeScript, Vite, Tailwind CSS, and shadcn/ui.
- Backend: Rust 2021 in `src-tauri/`.
- Package manager: pnpm 10.33.x.
- Node.js: 22.22.x (see `.node-version`).

## Working In This Repository

- Keep frontend changes under `src/` and native Tauri changes under `src-tauri/`.
- Prefer the existing shadcn/ui components and helpers in `src/components/ui/` and `src/lib/` over introducing duplicate primitives.
- Keep TypeScript strict and avoid `any`; add or update Vitest coverage for behavior changes where practical.
- Run `codegraph explore <query>` before using text search to understand or locate application code. Keep the index current with `codegraph sync` after broad changes.
- Do not commit generated output (`dist/`), dependencies (`node_modules/`), local IDE settings, or secrets.

## Commands

```powershell
pnpm dev                 # Start the Vite frontend
pnpm tauri dev           # Start the desktop application
pnpm test                # Run frontend tests once
pnpm lint                # Run oxlint, ESLint, and Stylelint
pnpm typecheck           # Type-check the frontend
pnpm format              # Format repository files with oxfmt
pnpm check               # Run the full CI quality suite
pnpm build               # Build the frontend
pnpm tauri:build:check   # Compile the Tauri app without bundling
```

For Rust-only changes, run:

```powershell
pnpm rust:fmt
pnpm rust:clippy
```

## Validation

- For a focused TypeScript or UI change, run the relevant test plus `pnpm typecheck` and `pnpm lint`.
- For Rust changes, run `pnpm rust:fmt` and `pnpm rust:clippy`.
- Before completing a cross-cutting change, run `pnpm check`. Run `pnpm build` or `pnpm tauri:build:check` when the relevant build surface changed.
- Do not modify unrelated files in a dirty working tree.
