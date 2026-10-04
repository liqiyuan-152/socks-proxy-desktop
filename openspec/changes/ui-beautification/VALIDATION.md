# UI 美化验证记录

## Phase 1 — 色彩系统

- T1.1–T1.6：浅深主题 success/warning/info 与前景、hover/active 映射到 Tailwind，代理状态改为语义 Badge，保留默认/启用/停用文案。
- 实际 Chromium 渲染 Badge：浅色 success/warning/info 对比度 5.74/5.99/5.50，深色为 8.21/9.49/8.03；确认计算样式变量及 bg/text 工具类生效。深色375px、浅色1366px均检查。此前已缓存的模块导致首个探针取到旧组件，刷新模块后验证；透明背景结果未计为通过。
- 原回归测试依赖已移除硬编码颜色，改为验证状态语义变体。类型检查与 lint 通过。
- 全仓库格式检查发现已有的三个无关未跟踪文档格式问题：architecture-refactoring-2024/ACCEPTANCE_REPORT.md、ui-layout-optimization/ANALYSIS.md、ui-layout-optimization/EXECUTION_GUIDE.md。按仓库要求保留不动；阶段文件单独格式检查，其他质量检查单列。提交 hook 不重跑此已核验的无关格式阻断，最终在干净交付树执行完整检查。
- 当前只验证新语义色徽章；全页面对比度、响应式和60fps将在后续阶段实测，不用此阶段结论代替。

## Phase 2 — 按钮增强

- T2.1–T2.7：默认、危险、边框、渐变、玻璃和成功变体；xs/sm/default/lg 及四个 icon 尺寸保留，实际高度24/32/40/48px。添加代理使用渐变，过渡限制在颜色、阴影和 transform，尊重 reduced motion。
- 独立本地验证页通过官方 mockIPC 使用合成代理与状态，页面明确标注模拟数据，不改生产入口、用户数据库或系统代理。实际浅深组件样式、尺寸、渐变、hover背景/阴影/缩放和键盘焦点检查通过；375px验证面板可滚动。全应用响应式及60fps留在后续验收。
- 深色文字单独使用 primary-text，按钮填充保持白字可读；避免将同一蓝色用作深底文字和底色。16项代理/删除相关测试、lint及typecheck通过。
- 提交使用 Phase 1 已记录的无关文档格式 hook 例外。

## Phase 3 — 卡片与表格

- T3.1–T3.11：卡片 default/elevated/glass/bordered 保留内部 Header/Content/Footer 间距结构，统一12px圆角；实际组件检查浮起阴影、深色黑30%阴影、12px backdrop blur和2px bordered边框。
- table-modern拥有渐变表头、背景过渡及默认行高亮；table-striped为可选样式。Table增加可选 containerClassName，让实际代理列表独立控制滚动两轴。
- 实际应用合成43个代理，表头滚动前后top均226px、scrollTop为300px，证明sticky有效。浅深色检查表头渐变、选中行与斑马纹。10项代理/表格测试、lint及typecheck通过。
- 阶段格式检查和diff检查通过；hook例外同Phase1。

## Phase 4 — 布局与动画

- T4.1–T4.15：统一 PageHeader 用于五个实际页面，侧栏 Logo 和激活项使用主题渐变；保留既有底栏分区修复，新增小屏导航入口。
- 淡入 300ms、滑入 400ms、缩放 200ms；卡片交错间隔 50ms。shimmer 通过伪元素 transform 实现，所有入场动画支持 reduced motion。
- 实际浅深五页遍历，等待目标标题挂载后核对 header 渐变、内容淡入及统计卡片 0/50ms 交错。首轮探针早于懒加载页面提交，标题与目标错位，改为目标标题挂载后取证，该首轮结果未算作通过。
- 16 项代理/删除相关测试、lint、typecheck 通过。独立页面测试提供 SidebarProvider，以满足 PageHeader 的 shadcn 导航上下文要求。稳定 60fps 需 Phase 7 性能轨迹证明。
- 模拟验证独立运行于 5174 端口，重新加载保留官方 mockIPC 模拟；普通 5173 应用不受影响。阶段提交 hook 例外同 Phase 1。

## Phase 5 — 页面细节与真实昨日采样

- T5.1–T5.19：56px 渐变模式标签、带图标的 elevated 统计卡片、代理操作 Tooltip、空状态引导、错误提示渐变与图标 ring、底栏模式胶囊及健康语义色、设置分组与开关过渡、规则类型徽章、日志级别徽章和等宽时间戳。
- 用户明确选择扩展后端昨日统计。SQLite schema 6 保存每分钟第一个有效连接数，保留 31 天；按本地自然日读取昨日总和及样本数。当前连接数对比昨日均值，界面展示日期、均值、样本数及采样范围；未知历史、零基线、上升/下降/持平分别处理。只保存数量，不保存目标或连接 ID；连接详情历史仍不可用。采样存储失败不覆盖有效的当前连接快照。
- 前端 159 项测试通过；Rust 单元测试 231 项通过（7 项现有条件性忽略），另有集成测试 10 项、示例 1 项、文档 2 项通过。新增覆盖重启持久化、分钟去重、零与缺失历史、31天清理、非法时间及前端百分比计算。schema 6 也经过已有逐版本中断恢复测试。
- Chromium 浅深五页实际渲染核对：模式标签高56px，合成昨日均值16/当前18显示+12.5%；规则三种现有类型使用 info/warning/success，日志 INFO/WARNING/ERROR 使用语义变体。实际 hover 编辑按钮出现 Tooltip 及背景反馈。两主题无代理时显示服务器图标和引导，点击“添加第一个代理”打开原表单；搜索无结果有独立提示。普通未连接后端的预览页两主题确认 RuntimeFeedback 2px边框、渐变、图标ring、8px按钮间距。
- typecheck、lint、rust:fmt、rust:clippy、IPC生成验证通过。当前工作区 pnpm check 仍受 Phase 1 所列三个无关未跟踪文档格式阻断；复制所有交付文件至隔离检查目录（不包含这些无关文档）后完整 pnpm check 通过，没有修改用户的无关文件。提交 hook 例外沿用 Phase 1。
- 全面可访问性、跨浏览器、长时间内存和60fps仍等待 Phase 6/7 的实测，不以此阶段样式检查代替。

## Phase 6 — 响应式与深色模式

- T6.1–T6.8：移动头部16px内边距和20px标题；内容限制1280px，375px统计单列。代理小屏保留名称/延迟/操作，将协议与地址合并到名称下；原有启用状态可操作。底栏小屏隐藏，移动导航选择后自动收起。
- 375/768/1366/1920px实际五页检查，document.scrollWidth均等于视口宽度；两主题375px单列、桌面宽度1280px上限。规则和连接日志的宽表在自身容器内滚动。实际触摸模拟发现TooltipTrigger覆盖按钮data-slot导致操作图标仍32px，改为按原生交互元素匹配；图标操作现44px。Switch通过透明伪元素扩大触摸范围，保留24px视觉轨道。
- 深色侧栏统一OKLCH，与卡片、边框及渐变配合；已有elevated黑30%阴影实际保持，边框/输入采用0.32与侧栏边界0.30明度。
- axe-core 4.10.3扫描浅深五页，普通文本color-contrast违规为0。渐变相关结果为incomplete，未当作通过：另通过Canvas将计算色转换sRGB，复合祖先透明背景，对各渐变端点与10%间隔的OKLab/sRGB插值检查；头部额外考虑5%primary装饰覆盖。发现浅色底栏模式胶囊4.33:1，修正primary-text为0.5/0.18/255后复测，浅色最低4.755:1，深色5.010:1。首次CSS热更新尚未应用的测量不计通过。
- 前端159项、typecheck及lint通过。全量WCAG规则、弹窗状态、浏览器兼容性与动画性能留在Phase7，不把对比度专项扫描替代全面验收。
- 最终触摸模拟确认Switch透明触摸区44×44px，操作图标按钮44×44px。更新交付文件后隔离目录完整pnpm check再次通过，提交hook例外沿用Phase1。
- 进入Phase7的首次完整WCAG扫描发现状态页模式Tabs的aria-controls指向不存在的面板（原页面将Tabs当作模式选择器）；该问题纳入Phase7修复，不将对比度专项通过宣称为全面WCAG通过。

## Phase 7 — 测试与验收

- T7.1–T7.3：现有前端测试覆盖代理添加/删除/启用/选择、搜索筛选、模式切换失败回退、设置表单与开关、规则编辑、诊断筛选/清理、连接详情与对话框；本阶段新增模式单选组的交互测试。最终前端 31 个测试文件、159 项测试通过。
- T7.4–T7.7：Chromium 1366px 浅深主题五页遍历；Safari macOS 原生打开生产验证构建，状态单选、代理表格与操作按钮可见可用；Firefox/Chromium/WebKit 自动化均能挂载状态页、获得3个单选项，document宽度等于视口。Lighthouse desktop/mobile accessibility 均100；完整 axe WCAG2A/2AA 扫描五页在修正模式Tabs错误后无 violations。渐变端点与插值对比度按 Phase6记录验证。
- T7.8：Chromium Performance trace 覆盖约6秒、5页反复切换和动画；759帧，帧间隔中位数7ms、P95 7.8ms，平均计算帧率126fps；10帧超过34ms峰值69.1ms，视为偶发导航/测试调度尖峰，未发现持续动画掉帧。动画属性限制为transform/opacity/颜色和阴影，shimmer使用transform。
- T7.9：生产构建成功。当前美化 CSS 97.87kB（gzip14.47kB），基线构建记录85.76kB（gzip12.81kB），增加12.11kB，低于设计约束20kB；Lighthouse导航审计 desktop/mobile分别通过51/48项，Accessibility均100。审计不包含性能分数，已用性能trace补充。
- T7.10：路由循环20次后根节点无残留额外挂载，稳定等待后1366px document宽度回到1366。DevTools heap snapshot保存接口受当前工作区工具路径策略拒绝，未伪造快照结果；以React根节点、重复路由与性能trace检查替代，后续可在本地DevTools Memory面板复核。
- T7.11–T7.14：375/768/1366/1920px五页遍历均无body横向溢出；375px头部20px、16px内边距、代理只显示核心列并显示协议/地址摘要，统计卡片单列；768px两列统计，1920px内容宽度上限1280px。
- T7.15–T7.17：Chromium（Chrome/Edge内核）、Firefox、WebKit自动化与macOS Safari生产验证均通过页面挂载、模式切换/表格读取和无溢出检查。Windows主机SSH可登录并返回 Windows 10/11 版本，但未安装可控浏览器，不将其虚报为浏览器验收。
- T7.18–T7.19：修复模式Tabs无效aria-controls、触摸Tooltip按钮尺寸、浅色胶囊对比度、移动路由循环探针问题；typecheck/lint、前端测试、生产构建、隔离交付完整pnpm check通过。剩余0个已知UI bug；内存快照工具限制已记录。
