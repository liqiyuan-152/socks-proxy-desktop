# Phase 5 发布准备记录（进行中）

日期：2026-10-05。README 和 CHANGELOG 已更新，记录品牌图标、中文 NSIS、三种 macOS 架构、Linux 包及实际功能限制。尚未完成发布。

## 实际产物汇总

源运行：[37259003588](https://github.com/liqiyuan-152/socks-proxy-desktop/actions/runs/37259003588)。五个目标均成功。验收源码提交：ab81d4bd46ed6436408532c7a1350354ddfe820e，版本 0.2.1。

下载真实 artifact 后执行：

```sh
node scripts/prepare-release.mjs /tmp/socks-brand-ci-37259003588 /tmp/socks-brand-ci-verified-37259003588 v0.2.1 ab81d4bd46ed6436408532c7a1350354ddfe820e
```

七个实际安装包均通过 SHA256、版本、源码及平台格式核验；输出含 SHA256SUMS.txt、SOURCE_REVISION.txt 和 VERSION.txt。中文 EXE 文件名可由汇总脚本正常生成，安装器界面语言则由 NSIS SimpChinese 独立控制。

## 尚未发布

以上仅为验收包，不替代已有 v0.2.1。全平台实际安装交互验收及新版本号确定完成后，须针对最终发布提交重新构建并核验产物；不得复用上述旧源码的安装包充当新版本。现有 v0.2.1 标签保持不变。5.3/5.4 保留待完成。
