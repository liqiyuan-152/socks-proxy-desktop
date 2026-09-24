# 项目指南

## 概述

`socks-proxy` 是一个用于管理 SOCKS 代理的 Tauri 2 桌面应用。

- 前端：React 19、TypeScript、Vite、Tailwind CSS 和 shadcn/ui。
- 后端：`src-tauri/` 中的 Rust 2021。
- 包管理器：pnpm `>=10.33.0`（推荐版本见 `packageManager`）。
- Node.js：`>=22.22.0`；`.node-version` 指定推荐版本 `22.22.0`。

## 仓库开发规范

- 前端改动放在 `src/` 下，原生 Tauri 改动放在 `src-tauri/` 下。
- 实现前端 UI 时，优先复用 `src/components/ui/` 和 `src/lib/` 中现有的 shadcn/ui 组件与工具；仅当没有合适的 shadcn/ui 组件，或现有组件无法满足需求时，才封装自定义组件或基础组件。
- 所有可复用的 shadcn/ui 基础组件统一维护在 `src/components/ui/`；不要创建平行的基础组件目录或重复封装已有的 `Button`、`Input`、`Select`、`Sidebar` 等组件。shadcn 组件内部使用语义化原生元素是正常实现，不应因此移除。
- 界面颜色使用 `src/index.css` 中的语义化主题令牌。侧栏使用 `sidebar` 系列令牌，并同时维护默认主题和 `.dark` 主题下的可读性；除明确的品牌或状态色外，避免在布局组件中硬编码背景和文字颜色。
- 新增或实质修改的源代码文件应控制在 400 行以内；达到或超过该限制时，按功能职责拆分为更小的模块或组件。生成文件和测试夹具不适用此限制，且不要只为满足行数限制重构无关代码。
- 保持 TypeScript 严格类型检查并避免使用 `any`；行为变更时，在可行的情况下新增或更新 Vitest 覆盖。
- 当仓库根目录存在 `.codegraph/` 时，在使用文本搜索理解或定位应用代码前先运行 `codegraph explore <query>`；完成大范围改动后使用 `codegraph sync` 保持索引最新。未建立索引时跳过 CodeGraph，改用 `rg` 和有针对性的文件读取。
- 不要提交生成产物（`dist/`）、依赖目录（`node_modules/`）、本地 IDE 配置或密钥。

## 环境预检

- 执行 `pnpm` 命令前，确认 `node --version` 满足 `>=22.22.0`、`pnpm --version` 满足 `>=10.33.0`。
- 当前 shell 版本不符合要求时，先通过已配置的版本管理器切换；不要绕过 `package.json` 的 engines 约束。

## 常用命令

```powershell
pnpm dev                 # 启动 Vite 前端
pnpm tauri dev           # 启动桌面应用
pnpm test                # 运行一次前端测试
pnpm lint                # 运行 oxlint、ESLint 和 Stylelint
pnpm typecheck           # 对前端进行类型检查
pnpm format              # 使用 oxfmt 格式化仓库文件
pnpm check               # 运行完整的 CI 质量检查
pnpm build               # 构建前端
pnpm tauri:build:check   # 编译 Tauri 应用但不打包
```

仅修改 Rust 代码时，运行：

```powershell
pnpm rust:fmt
pnpm rust:clippy
cargo test --manifest-path src-tauri/Cargo.toml
```

## 验证要求

- 对聚焦的 TypeScript 或 UI 改动，运行相关测试，以及 `pnpm typecheck` 和 `pnpm lint`。
- 对 Rust 改动，运行 `pnpm rust:fmt` 和 `pnpm rust:clippy`；涉及 Rust 行为或测试时，再运行 `cargo test --manifest-path src-tauri/Cargo.toml`。
- 完成跨模块改动前，运行 `pnpm check`；构建相关改动后，运行 `pnpm build` 或 `pnpm tauri:build:check`。
- 工作区存在未提交改动时，不要修改与当前任务无关的文件。
