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

## Phase 5 — 页面细节

- 完成 T5.1–T5.19。模式 Tabs 56px（实际修正 shadcn 横向 group 高度覆盖）、统计 elevated 卡片与图标、真实连接数趋势、代理操作 Tooltip、引导空态、运行时反馈渐变与图标容器、状态栏胶囊/健康色、规则徽章、设置卡片分组、Switch、日志级别/等宽时间。
- T5.3 使用当前页面实际连接数前后变化，展示箭头和百分比；没有昨日统计时不伪造“比昨天”。首采样/零基线/采样中断有明确状态，新增回归测试覆盖。其余未支持的历史数据仍标记不可用。
- T5.15 适配实际支持的 domain/domain_suffix/ip_cidr；分别使用 info/success/warning，没有新增不支持的 GEOIP 业务规则。
- 本地独立预览使用 Tauri 官方 mockIPC 模拟数据，明确标注模拟；没有更改生产入口或接触实际代理配置。浏览器确认 Tabs 高度56px、四个统计卡片 elevated、操作键盘 Tooltip、规则三类徽章、日志 INFO/WARNING/ERROR 色和等宽字体。深浅色视图及真实无后端错误态已检查。
- typecheck、lint、158 项前端测试、生产构建通过。CSS 96.64kB。最后 Tabs 高度修正后类型检查通过；最终验收将复跑完整检查。
- 阶段提交沿用已记录 hook 例外。全应用文本对比度、响应式及性能仍待下一阶段。

## Phase 6 — 响应式与深色主题

- 完成 T6.1–T6.8。移动标题20px、桌面24px；移动页面/头部16px边距；内容限制max-w-7xl。375px统计与设置卡片单列，代理表格隐藏次要列并将协议/地址/端口合并到名称下方，操作组保留；粗指针按钮最小44px。移动导航点击后自动关闭抽屉，底栏隐藏。
- 实际浏览器遍历375/768/800/1024/1280/1366/1920px的五个页面，body宽度均等于视口。375px代理滚动内容scrollWidth/clientWidth均375，表格341px；五页移动导航抽屉均关闭。手机深色代理截图确认名称、状态、延迟及编辑/删除/测试按钮可见，编辑表单可打开。
- 深色侧栏统一OKLCH，边框与卡片阴影增强；渐变使用主题变量。primary-text独立于按钮填充，保证文字可读性。
- axe-core对五页浅色/深色各扫描一次，稳定状态color-contrast均0违规。最初主题切换瞬间扫描出现1.27徽章对比度，为颜色过渡中间态；等待450ms重扫后消失。没有据此修改正确的最终主题颜色。
- axe不能自动确定渐变背景：每页10–21个节点需补充核验。QA端点辅助测量读取实际computed颜色，用canvas转换sRGB、叠加祖先背景与头部装饰层，枚举渐变端点组合；十次扫描的端点最小比值浅色4.76、深色5.01，无低于4.5的节点。这是渐变端点核验，不代表完整WCAG扫描或所有交互状态；Phase7仍需检查弹窗、表单、hover与全规则扫描。
- 最后移动导航改动后lint、typecheck、31文件/158项前端测试、生产构建通过。CSS97.67kB。阶段提交使用前述格式hook例外；Phase7还需全套检查、基线性能和跨浏览器验证。
