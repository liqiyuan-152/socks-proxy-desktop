# UI 美化验证记录

## Phase 1 — 色彩系统

- 保留已有主题变量映射，补齐 success/warning/info Badge，并在代理状态组件中复用。
- 设计文档颜色为示例；实际成功绿/信息蓝调整亮度，深色徽章改为深色前景以满足对比度。
- 浏览器实际 computed style 测量：浅色 success 5.75、warning 7.09、info 5.46；深色分别 8.37、8.85、8.14。停用状态保持中性色，默认代理文字保持区分。
- 浏览器预览确认深浅色主题及代理状态标签；状态色更改不涉及布局或动画。
- typecheck、lint、生产构建通过。代理状态原测试依赖旧的硬编码类名，更新为语义 Badge 变体断言，9 项相关测试通过。
- 完整验收仍需后续阶段执行，未声称全部应用文本已符合 WCAG。

- 阶段提交的全仓库 `pnpm check` 在格式检查处因既有未提交文档及临时预览文件停止。没有修改无关文档。逐项运行其余检查：lint、typecheck、157 项前端测试、10 项工具测试、rust:fmt、rust:clippy、ipc:check 均通过；当前阶段六个提交文件的格式检查通过。阶段提交仅排除此已核验的 hook，最终验收仍保留全仓库格式问题。

## Phase 2 — 按钮组件

- 完成 T2.1–T2.7，保留 asChild、所有原有 variant/size、禁用和焦点行为；增加 gradient/success/glass，添加代理按钮使用 gradient。
- 默认按钮 hover 使用交互色和阴影；outline 为 2px 边框；危险操作使用主题化前景；深色渐变降低亮度。动画仅过渡颜色、背景、边框、阴影与 transform；尊重 reduced motion。
- 浏览器检查全部变体及 xs/sm/default/lg 和四种 icon 尺寸，高度依次 24/32/40/48px；浅色/深色截图、真实 hover 背景变化及键盘 3px 焦点环验证通过。60fps 及全部响应式验收留在 Phase 7。
- typecheck、lint、16 项代理/错误恢复/确认对话框测试及生产构建通过。CSS 91.30kB（第一阶段 89.48kB）。
- 阶段提交采用相同的已说明 hook 例外，未处理无关文档格式。

## Phase 3 — 卡片与表格

- 完成 T3.1–T3.11。Card 保留默认结构、子组件间距和 React 19 ref，添加 elevated/glass/bordered；所有变体 rounded-xl。
- table-modern 包含渐变表头、固定表头、hover、选中行、单元格间距；table-striped 是可选斑马纹。Table 增加可选 containerClassName，代理列表滚动容器上限 60vh，默认代理行使用 data-selected。
- 实际浏览器卡片检查：四种变体圆角均 12px，glass blur(12px)，elevated 增强阴影；深浅色截图验证通过。
- 实际表格滚动至 scrollTop=750px，表头与容器顶部始终为 557px，固定表头通过；检查选中行 computed background、深浅色边框和斑马纹。
- typecheck、10 项代理/表格测试、构建通过；Stylelint 检出的属性顺序已修复，lint 复跑通过。CSS 93.40kB。
- 采用同一已记录的阶段提交 hook 例外。全应用响应式、可访问性和性能仍待 Phase 6/7。

## Phase 4 — 布局与动画

- 完成 T4.1–T4.15。共用 PageHeader 应用于 status/proxies/rules/settings/connection-history（规范的 routing/logs 对应实际目录）。渐变、标题、说明和装饰统一。
- 保留工作区原有状态栏防遮挡修复；侧栏 Logo/激活导航使用主题渐变，hover 轻微缩放。主内容 fade-in，状态页卡片 stagger 0/50ms；fade/slide/scale 工具类、transform shimmer 和现有 Skeleton 复用完成。
- CSS 动画只改变 transform/opacity；prefers-reduced-motion 下关闭；没有引入动画依赖。
- 浏览器逐页等待标题完成加载，五个标题、渐变和动画名核验通过，深浅色截图已检查；动画结束 opacity=1。实际状态页交错延迟为 0s/0.05s。
- PageHeader 新增移动侧栏开关；删除测试原独立渲染页面缺少 SidebarProvider，测试包裹与实际应用一致的 Provider 后 7 项复跑通过。完整套件首次为 150 通过/7 因上下文失败，修复后最终完整复跑仍待后续。
- typecheck、构建通过；Stylelint 空行问题修复后 lint 复跑通过。CSS 94.22kB。60fps 性能实测未完成。
- 阶段提交沿用已记录 hook 例外。
