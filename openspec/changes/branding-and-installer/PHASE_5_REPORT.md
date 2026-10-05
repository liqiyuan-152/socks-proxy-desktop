# Phase 5 发布准备记录（进行中）

日期：2026-10-05。README 和 CHANGELOG 已更新，记录品牌图标、中文 NSIS、三种 macOS 架构、Linux 包及实际功能限制。尚未完成发布。

## 实际产物汇总

源运行：[37259003588](https://github.com/liqiyuan-152/socks-proxy-desktop/actions/runs/37259003588)。五个目标均成功。验收源码提交：ab81d4bd46ed6436408532c7a1350354ddfe820e，版本 0.2.1。

下载真实 artifact 后执行：

```sh
node scripts/prepare-release.mjs /tmp/socks-brand-ci-37259003588 /tmp/socks-brand-ci-verified-37259003588 v0.2.1 ab81d4bd46ed6436408532c7a1350354ddfe820e
```

七个实际安装包均通过 SHA256、版本、源码及平台格式核验；输出含 SHA256SUMS.txt、SOURCE_REVISION.txt 和 VERSION.txt。中文 EXE 文件名可由汇总脚本正常生成，安装器界面语言则由 NSIS SimpChinese 独立控制。

## 后续修复源码产物复核

源运行：[37262569957](https://github.com/liqiyuan-152/socks-proxy-desktop/actions/runs/37262569957)。Windows x64、macOS Apple Silicon/Intel/Universal、Linux x64 五个构建目标全部成功；同源码 Quality 运行 37262570009 成功。源码为 290aec3c2928c05353af32e8397f8e0d62dd1fa7，包含 Windows 快捷方式品牌图标 hook 和代理状态标签移除。

下载五个真实 artifact 后执行：

```sh
node scripts/prepare-release.mjs /tmp/socks-brand-ci-37262569957 /tmp/socks-brand-ci-verified-37262569957 v0.2.1 290aec3c2928c05353af32e8397f8e0d62dd1fa7
```

命令成功核验并汇总 7 个安装包，平台及文件数量、扩展名、SHA256、版本和源码标识一致，输出完整的校验与追溯清单。该步骤核验产物一致性，不替代安装交互或系统图标视觉验收，亦未创建 GitHub Release。

## 尚未发布

以上仅为验收包，不替代已有 v0.2.1。全平台实际安装交互验收及新版本号确定完成后，须针对最终发布提交重新构建并核验产物；不得复用上述旧源码的安装包充当新版本。现有 v0.2.1 标签保持不变。5.3/5.4 保留待完成。

## 0.2.2 候选准备

为进行真实的 0.2.1→0.2.2 升级检查，候选源码版本统一调整为 0.2.2，现有 v0.2.1 标签不变。Linux 工作流增加可选旧包 artifact 输入；先验证旧包源码和校验清单，再启动旧包、写入非空代理配置与模式、重新启动确认，然后安装严格更高版本并逐记录比较。每次启动清理本套件临时目录中的旧日志，防止复用旧就绪记录。未修改宿主用户的 XDG 数据。

准确暂存的候选源码在隔离副本 /tmp/socks-brand-check.j1eUaC 完整 pnpm check 和 pnpm build 通过：159 个前端测试、19 个工具测试、format/lint/typecheck、Rust fmt/clippy、IPC 契约检查；版本一致性检查、升级状态脚本的未变化通过/模式变化拒绝及 bash 语法检查通过。真实 Linux 版本升级及候选全平台打包仍待 CI 执行；未创建标签或 Release。主工作区用户的窗口尺寸修改未暂存。

## 修正后 0.2.2 全平台候选产物

源码 8bc3b87deda47436e218df71d3b782aada65f893，成功运行 [37269117857](https://github.com/liqiyuan-152/socks-proxy-desktop/actions/runs/37269117857)。已下载全部五个 artifact，执行：

```sh
node scripts/prepare-release.mjs /tmp/socks-brand-ci-37269117857 /tmp/socks-brand-ci-verified-37269117857 v0.2.2 8bc3b87deda47436e218df71d3b782aada65f893
```

七个安装包及 SHA256、VERSION、SOURCE_REVISION 全部核验通过。此次三个 macOS DMG 均通过实际挂载布局检查，Apple Silicon 包另已通过 Finder 中文背景、拖拽替换和应用启动/非空配置保留验证。Windows EXE 已传至远端，上传前后 SHA256 一致（8c83ab91b6084e7c2e8898c83d44e8b0d3117f6cfa6ebc7cfef6fe1cdd550df7），尚待用户退出应用后执行新版本升级。已有 v0.2.1 不变，未创建 v0.2.2 标签或 Release；首次 Windows 配置缺失未定因，不将后续未复现作为修复证明。

同源码独立 Quality [37269103256](https://github.com/liqiyuan-152/socks-proxy-desktop/actions/runs/37269103256) 已全部成功。

## 最新验收文档提交的候选复核

源码 f19b8b7c867da2c4fe499e069c877f53e3fc9e33 已推送。Desktop packages 37271908223 五个目标成功，同源码 Quality 37271908293 全部成功；该提交仅更新验收文档，应用及安装资源与 8bc3b87 相同。已下载全部真实 artifact 并执行：

```sh
node scripts/prepare-release.mjs /tmp/socks-brand-ci-37271908223 /tmp/socks-brand-ci-verified-37271908223 v0.2.2 f19b8b7c867da2c4fe499e069c877f53e3fc9e33
```

七个安装包的版本、源码、格式、数量和 SHA256 全部通过。Windows 实际升级已完成且原配置、模式保留，细节见 Phase 4 报告。已准备包含首次未定因配置缺失及完整备份建议的本地发布预览，等待用户决定继续排查或接受明确记录的已知异常后发布；未创建 v0.2.2 标签或 Release，不将未定因异常标记为已修复。

## 配置保留加固后的候选与发布说明

用户已决定“继续排查后发布”，首次数据目录删除的 NTFS 证据及安装器加固见 Phase 4 报告。源码 5bc35c4c3cf6c8ee73e0e1545a441c7964721680 的 Desktop packages [37279969996](https://github.com/liqiyuan-152/socks-proxy-desktop/actions/runs/37279969996) 五个目标全部成功。已下载完整 artifact 并执行：

```sh
node scripts/prepare-release.mjs /tmp/socks-brand-ci-37279969996 /tmp/socks-brand-ci-verified-37279969996 v0.2.2 5bc35c4c3cf6c8ee73e0e1545a441c7964721680
```

七个安装包的版本、源码、格式、数量和 SHA256 全部通过。CI Windows EXE 已传至远端独立文件 data-retention-ci-setup.exe，本机与远端 SHA256 一致（edb429a56a830c1bf5953fe097290a98ab8df1b8da59b7f626ceb42ada3dcbc4），等待应用退出后实际复测，不替换运行中的程序或数据。该包不同于本机远端编译的加固包，后续实际验收以此已核验 CI 产物为准。

新增 docs/releases/v0.2.2.md，说明实际品牌改动、Windows 保留加固、首次删除证据及未记录的触发细节、完整备份和平台限制、sing-box 许可与对应源码。发布工作流优先使用对应 tag 的版本专属说明，其他版本保持通用说明回退；YAML、bash 语法、格式检查通过，用隔离 gh 模拟同时验证专属说明原样使用、回退说明及先草稿上传后发布的调用路径，未写入 GitHub。v0.2.1 标签仍为 990504a7065bf43d9e00cc27a7579a531651158b，v0.2.2 Release 不存在。5.3/5.4 保持未完成，实际 Windows 加固复测仍待用户退出应用。

## 0.2.2 最终发布启动

Windows 保留加固的真实旧版升级、独立卸载和重装均已完成，详见 Phase 4。新版卸载页面经用户确认中文及品牌正常、说明保留数据且无删除选项，重装后 0.2.2 实际运行，原配置及模式一致。README 示例和 CHANGELOG 更新为 0.2.2 / 2026-10-05。下一步将本阶段提交作为 v0.2.2 发布源码，经 tag 工作流重新构建全部七个安装包；发布后下载实际 Release 并核验清单和源码，5.3/5.4 在取得证据前保留未完成。已有 v0.2.1 保持不变。

首轮 tag 构建 37289501895 因 Windows 路由夹具时序失败，发布步骤跳过，v0.2.2 Release 未创建。修复后真实 Windows 单项内核测试通过，详见 Phase 4。将尚未发布的 v0.2.2 tag 更新至夹具修复提交后重新构建，最终源码与下载清单以后续记录为准，已发布 v0.2.1 不变。
