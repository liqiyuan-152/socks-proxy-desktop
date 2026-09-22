# 项目指南

## 概述

`socks-proxy` 是一个用于管理 SOCKS 代理的 Tauri 2 桌面应用。

- 前端：React 19、TypeScript、Vite、Tailwind CSS 和 shadcn/ui。
- 后端：`src-tauri/` 中的 Rust 2021。
- 包管理器：pnpm 10.33.x。
- Node.js：22.22.x（见 `.node-version`）。

## 仓库开发规范

- 前端改动放在 `src/` 下，原生 Tauri 改动放在 `src-tauri/` 下。
- 实现前端 UI 时，优先复用 `src/components/ui/` 和 `src/lib/` 中现有的 shadcn/ui 组件与工具；仅当没有合适的 shadcn/ui 组件，或现有组件无法满足需求时，才封装自定义组件或基础组件。
- 所有可复用的 shadcn/ui 基础组件统一维护在 `src/components/ui/`；不要创建平行的基础组件目录或重复封装已有的 `Button`、`Input`、`Select`、`Sidebar` 等组件。shadcn 组件内部使用语义化原生元素是正常实现，不应因此移除。
- 界面颜色使用 `src/index.css` 中的语义化主题令牌。侧栏使用 `sidebar` 系列令牌，并同时维护默认主题和 `.dark` 主题下的可读性；除明确的品牌或状态色外，避免在布局组件中硬编码背景和文字颜色。
- 源代码文件应控制在 400 行以内；达到或超过该限制时，按功能职责拆分为更小的模块或组件。
- 保持 TypeScript 严格类型检查并避免使用 `any`；行为变更时，在可行的情况下新增或更新 Vitest 覆盖。
- 在使用文本搜索理解或定位应用代码前，先运行 `codegraph explore <query>`；完成大范围改动后，使用 `codegraph sync` 保持索引最新。
- 不要提交生成产物（`dist/`）、依赖目录（`node_modules/`）、本地 IDE 配置或密钥。

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
```

## 验证要求

- 对聚焦的 TypeScript 或 UI 改动，运行相关测试，以及 `pnpm typecheck` 和 `pnpm lint`。
- 对 Rust 改动，运行 `pnpm rust:fmt` 和 `pnpm rust:clippy`。
- 完成跨模块改动前，运行 `pnpm check`；构建相关改动后，运行 `pnpm build` 或 `pnpm tauri:build:check`。
- 工作区存在未提交改动时，不要修改与当前任务无关的文件。
