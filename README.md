# Socks Proxy

Private desktop SOCKS proxy manager built with Tauri, React, Tailwind CSS, shadcn/ui, and Rust.

## Prerequisites

- Node.js 22.22.0 and pnpm 10.33.0
- Rust stable with the Windows MSVC toolchain
- WebView2 Runtime on Windows

## Development

```powershell
pnpm install
pnpm tauri dev
```

For browser-only frontend development, run `pnpm dev` and open `http://127.0.0.1:5173/`.

## Quality Checks

```powershell
pnpm check
pnpm format
pnpm tauri:build:check
```

`pnpm check` runs formatting checks, Oxlint, ESLint, Stylelint, TypeScript, Vitest, Rust formatting, and Clippy.

## Production Build

```powershell
pnpm tauri build
```

No signing, updater, or release publishing is configured.
