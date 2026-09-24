# sing-box Windows 发行清单

此清单记录首期 Windows amd64 内核的来源与验证证据；打包所用的机器可在 `pnpm prepare:core` 时重新获取和校验文件。此文件不包含内核二进制或代理凭据。

| 项目                    | 锁定值                                                             |
| ----------------------- | ------------------------------------------------------------------ |
| 上游                    | `SagerNet/sing-box` GitHub Release `v1.14.1`                       |
| 标签对应提交            | `1ac1a339cb1223e9c70eae14c44411c75033c02d`                         |
| 目标资产                | `sing-box-1.14.1-windows-amd64.zip`                                |
| ZIP SHA-256             | `5197f16d492d93202dc623622149a6ed040f8eca263128f91d603f2b901baa89` |
| `sing-box.exe` SHA-256  | `b838de45bd0b2e6ddbed1977e4745622f7dffab3b293807ff4c6b1b640fed909` |
| `libcronet.dll` SHA-256 | `3217c6260fbca5f16072e0b79735742f40109a63bb0ff88fd6b96dd6b54a2928` |
| `LICENSE` SHA-256       | `bb3805862b583aee73ad6f7805ec634747a37257a637a3069857843f05ea589c` |
| 许可证                  | GPLv3；Windows 包同时包含上游 `LICENSE`                            |

## 发布提交签名记录

2026-09-24 核对上游 [v1.14.1 Release](https://github.com/SagerNet/sing-box/releases/tag/v1.14.1) 和 [提交](https://github.com/SagerNet/sing-box/commit/1ac1a339cb1223e9c70eae14c44411c75033c02d)：GitHub Commit API 的 `commit.verification` 返回 `verified: true`、`reason: valid`，并包含以 `-----BEGIN PGP SIGNATURE-----` 开头的签名及签名载荷；`git ls-remote --tags` 的 `refs/tags/v1.14.1` 与上述完整提交 ID 一致。这是 GitHub 提供的 PGP 验证记录，不等同于在本地独立建立密钥信任并运行 `git verify-commit`。签名证明提交身份，**不代替**发行 ZIP 和包内文件的 SHA-256 校验。

## 构建和分发复核

1. `pnpm prepare:core -- --target-windows` 仅从锁定的上游 Release 下载；先核对 ZIP SHA-256，再核对解压后三个文件的 SHA-256，不符即拒绝暂存。Windows 构建会由 Tauri 的 `beforeBuildCommand` 运行 `pnpm prepare:core`。
2. 三个文件仅暂存在被 `.gitignore` 排除的 `src-tauri/resources/sing-box/windows-amd64/`；Windows 专用的 `src-tauri/tauri.windows.conf.json` 把它们作为资源放入应用包。运行前再次核对 `sing-box.exe` 的 SHA-256。
3. 2026-09-24 本机复核暂存文件的三个 SHA-256 与本清单一致。Windows 验收所用 NSIS 包的 SHA-256 为 `8494b25d8fd95ac0b8c269efc4ef115ca7fbc99948811835d027f5e9ecc5675a`；此前已与传回本机的副本比对一致。该包仅供当前验收；正式对外分发前须另行完成 GPLv3 对应源代码提供方式、许可证通知和最终包内容的复核。
