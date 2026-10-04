# UI 美化任务清单

本文档列出所有 UI 美化相关任务，按阶段组织。

---

## Phase 1: 色彩系统升级

**目标**: 扩展色彩变量，建立语义化色彩系统  
**工期**: 0.5天

### 任务列表

- [x] **T1.1** 扩展 CSS 变量 - 浅色模式
  - 文件: `src/index.css`
  - 位置: `:root` 块
  - 内容: 添加 success/warning/info 色彩变量及其 foreground 色
  - 验证: 在开发者工具检查变量是否生效

- [x] **T1.2** 扩展 CSS 变量 - 深色模式
  - 文件: `src/index.css`
  - 位置: `.dark` 块
  - 内容: 添加深色模式对应的色彩变量
  - 验证: 切换深色模式检查颜色

- [x] **T1.3** 更新 @theme 配置
  - 文件: `src/index.css`
  - 位置: `@theme inline` 块
  - 内容: 将新色彩变量映射到 Tailwind 类
  - 验证: 测试 `bg-success`、`text-warning` 等类

- [x] **T1.4** 添加交互状态色
  - 文件: `src/index.css`
  - 内容: 添加 primary-hover、primary-active 变量
  - 验证: 测试按钮 hover 和 active 效果

- [x] **T1.5** 更新 Badge 组件变体
  - 文件: `src/components/ui/badge.tsx`
  - 内容: 在 badgeVariants 中添加 success/warning/info 变体
  - 验证: 测试 `<Badge variant="success">` 等

- [x] **T1.6** 应用语义色 Badge 到页面
  - 文件: `src/pages/proxies/index.tsx`
  - 内容: 将代理启用状态改用 success/secondary 变体
  - 验证: 查看代理列表，检查徽章颜色

---

## Phase 2: 按钮组件增强

**目标**: 提升按钮视觉效果和交互反馈  
**工期**: 0.5天

### 任务列表

- [ ] **T2.1** 增强按钮基础类
  - 文件: `src/components/ui/button.tsx`
  - 内容: 修改 buttonVariants 基础类，添加 transition-all、圆角优化
  - 验证: 检查所有按钮是否有平滑过渡

- [ ] **T2.2** 升级 default 变体
  - 文件: `src/components/ui/button.tsx`
  - 内容: 添加阴影、hover 缩放、使用 primary-hover 色
  - 验证: hover 按钮观察阴影和缩放效果

- [ ] **T2.3** 新增 gradient 变体
  - 文件: `src/components/ui/button.tsx`
  - 内容: 创建渐变背景按钮变体
  - 验证: `<Button variant="gradient">` 显示蓝色渐变

- [ ] **T2.4** 优化 outline 变体
  - 文件: `src/components/ui/button.tsx`
  - 内容: 增加边框粗细、hover 边框变色
  - 验证: hover outline 按钮检查边框变化

- [ ] **T2.5** 优化 destructive 变体
  - 文件: `src/components/ui/button.tsx`
  - 内容: 添加红色阴影和 hover 效果
  - 验证: 删除按钮有红色阴影

- [ ] **T2.6** 优化按钮尺寸
  - 文件: `src/components/ui/button.tsx`
  - 内容: 调整各尺寸的高度和内边距
  - 验证: 测试 xs/sm/default/lg 各尺寸

- [ ] **T2.7** 应用新按钮变体到关键页面
  - 文件: `src/pages/proxies/index.tsx`
  - 内容: 将"添加代理"改为 gradient 变体
  - 验证: 查看代理列表页按钮效果

---

## Phase 3: 卡片和表格美化

**目标**: 提升卡片层次感和表格可读性  
**工期**: 1天

### 任务列表

#### 卡片增强

- [ ] **T3.1** 为 Card 组件添加 variant 支持
  - 文件: `src/components/ui/card.tsx`
  - 内容: 添加 variant prop（default/elevated/glass/bordered）
  - 验证: `<Card variant="elevated">` 显示增强阴影

- [ ] **T3.2** 实现 elevated 变体
  - 文件: `src/components/ui/card.tsx`
  - 内容: 添加大阴影效果
  - 验证: 卡片有明显的浮起效果

- [ ] **T3.3** 实现 glass 变体
  - 文件: `src/components/ui/card.tsx`
  - 内容: 毛玻璃效果（backdrop-blur + 半透明背景）
  - 验证: 卡片有模糊透明效果

- [ ] **T3.4** 实现 bordered 变体
  - 文件: `src/components/ui/card.tsx`
  - 内容: 透明背景 + 双倍边框 + hover 变色
  - 验证: hover 卡片边框变为主色

- [ ] **T3.5** 统一卡片圆角
  - 文件: `src/components/ui/card.tsx`
  - 内容: 将所有卡片改用 `rounded-xl`
  - 验证: 检查各页面卡片圆角一致性

#### 表格美化

- [ ] **T3.6** 创建现代表格样式类
  - 文件: `src/index.css`
  - 内容: 添加 `.table-modern` 样式（表头渐变、行 hover、sticky 表头）
  - 验证: 在示例表格应用该类

- [ ] **T3.7** 实现表头 sticky 效果
  - 文件: `src/index.css`
  - 位置: `.table-modern thead th`
  - 内容: 添加 sticky top-0、backdrop-blur
  - 验证: 滚动表格时表头固定

- [ ] **T3.8** 优化表格行 hover 效果
  - 文件: `src/index.css`
  - 位置: `.table-modern tbody tr`
  - 内容: 添加背景色过渡和 hover 变色
  - 验证: hover 表格行有明显背景变化

- [ ] **T3.9** 实现选中行高亮
  - 文件: `src/index.css`
  - 位置: `.table-modern tbody tr[data-selected="true"]`
  - 内容: 选中行显示主色背景
  - 验证: 选中行有蓝色高亮

- [ ] **T3.10** 添加表格斑马纹（可选）
  - 文件: `src/index.css`
  - 内容: 创建 `.table-striped` 类
  - 验证: 偶数行有浅色背景

- [ ] **T3.11** 应用现代表格样式到代理列表
  - 文件: `src/pages/proxies/index.tsx`
  - 内容: 给 Table 组件添加 `table-modern` 类
  - 验证: 代理列表表格样式更新

---

## Phase 4: 布局和动画优化

**目标**: 添加页面动画和优化整体布局  
**工期**: 1天

### 任务列表

#### 动画系统

- [ ] **T4.1** 定义页面过渡动画关键帧
  - 文件: `src/index.css`
  - 内容: 添加 fadeIn/slideInRight/scaleIn 关键帧
  - 验证: 在开发者工具检查 @keyframes 定义

- [ ] **T4.2** 定义 shimmer 加载动画
  - 文件: `src/index.css`
  - 内容: 创建骨架屏闪烁效果动画
  - 验证: 应用到加载状态元素

- [ ] **T4.3** 创建动画工具类
  - 文件: `src/index.css`
  - 内容: `.animate-fade-in`、`.animate-slide-in`、`.animate-scale-in`
  - 验证: 给元素添加类名测试动画

- [ ] **T4.4** 实现列表交错动画
  - 文件: `src/index.css`
  - 内容: `.stagger-children` 及子元素延迟规则
  - 验证: 列表项依次淡入

- [ ] **T4.5** 应用淡入动画到页面内容区
  - 文件: 所有页面组件
  - 内容: 给主内容区添加 `animate-fade-in`
  - 验证: 切换页面有淡入效果

- [ ] **T4.6** 应用交错动画到卡片列表
  - 文件: `src/pages/status/index.tsx`
  - 内容: 统计卡片容器添加 `stagger-children`
  - 验证: 卡片依次出现

#### 侧边栏优化

- [ ] **T4.7** 美化侧边栏 Logo
  - 文件: `src/components/AppLayout.tsx`
  - 位置: SidebarHeader 内（第95-104行）
  - 内容: Logo 容器添加渐变、阴影、ring
  - 验证: Logo 有蓝色渐变和光晕

- [ ] **T4.8** 优化侧边栏导航项样式
  - 文件: `src/components/AppLayout.tsx`
  - 位置: SidebarMenuButton（第56-70行）
  - 内容: 激活状态显示渐变背景和阴影
  - 验证: 当前页导航项有蓝色渐变高亮

- [ ] **T4.9** 添加导航项 hover 缩放效果
  - 文件: `src/components/AppLayout.tsx`
  - 内容: hover 时 scale-[1.02]
  - 验证: hover 导航项有轻微放大

#### 页面头部优化

- [ ] **T4.10** 创建统一的页面头部样式
  - 文件: 所有页面组件
  - 内容: header 添加渐变背景、装饰层、backdrop-blur
  - 验证: 所有页面头部样式一致

- [ ] **T4.11** 优化状态页头部
  - 文件: `src/pages/status/index.tsx`
  - 内容: 应用新头部样式
  - 验证: 头部有渐变和层次感

- [ ] **T4.12** 优化代理列表页头部
  - 文件: `src/pages/proxies/index.tsx`
  - 内容: 应用新头部样式
  - 验证: 头部与状态页一致

- [ ] **T4.13** 优化路由页头部
  - 文件: `src/pages/routing/index.tsx`
  - 内容: 应用新头部样式
  - 验证: 头部样式统一

- [ ] **T4.14** 优化设置页头部
  - 文件: `src/pages/settings/index.tsx`
  - 内容: 应用新头部样式
  - 验证: 头部样式统一

- [ ] **T4.15** 优化日志页头部
  - 文件: `src/pages/logs/index.tsx`
  - 内容: 应用新头部样式
  - 验证: 头部样式统一

---

## Phase 5: 特定页面优化

**目标**: 优化关键页面的细节体验  
**工期**: 1-2天

### 任务列表

#### 状态页优化

- [ ] **T5.1** 美化代理模式切换标签
  - 文件: `src/pages/status/index.tsx`
  - 位置: TabsList（第100行左右）
  - 内容: 添加渐变背景、增强激活状态样式
  - 验证: 标签有圆角渐变容器，激活项有蓝色渐变

- [ ] **T5.2** 优化统计卡片布局
  - 文件: `src/pages/status/index.tsx`
  - 内容: 使用 elevated 卡片变体，添加图标
  - 验证: 卡片有浮起效果

- [ ] **T5.3** 添加统计数据趋势指示
  - 文件: `src/pages/status/index.tsx`
  - 内容: 添加 ↑ ↓ 箭头和百分比变化
  - 验证: 显示"比昨天 ↑ 12%"样式

#### 代理列表页优化

- [ ] **T5.4** 优化代理卡片间距
  - 文件: `src/pages/proxies/index.tsx`
  - 内容: 统一使用 gap-4 或 gap-6
  - 验证: 卡片间距视觉舒适

- [ ] **T5.5** 美化操作按钮组
  - 文件: `src/pages/proxies/index.tsx`
  - 内容: 使用 ghost 变体，添加 tooltip
  - 验证: 操作按钮 hover 有背景反馈

- [ ] **T5.6** 优化空状态提示
  - 文件: `src/pages/proxies/index.tsx`
  - 内容: 添加插图和引导文案
  - 验证: 无代理时显示友好提示

#### 错误提示优化

- [ ] **T5.7** 美化 RuntimeFeedback 组件
  - 文件: `src/components/RuntimeFeedback.tsx`
  - 位置: 返回的 JSX（第51-92行）
  - 内容: 添加渐变背景、优化图标容器、增强按钮样式
  - 验证: 错误提示有红色渐变边框和阴影

- [ ] **T5.8** 优化错误提示图标
  - 文件: `src/components/RuntimeFeedback.tsx`
  - 内容: 图标容器添加背景色和 ring
  - 验证: 图标有圆角容器和光晕

- [ ] **T5.9** 优化错误提示按钮布局
  - 文件: `src/components/RuntimeFeedback.tsx`
  - 内容: 使用 flex gap-2，统一按钮尺寸
  - 验证: 按钮排列整齐

#### 底部状态栏优化

- [ ] **T5.10** 优化状态栏背景
  - 文件: `src/components/AppLayout.tsx`
  - 位置: footer（第141-158行）
  - 内容: 添加渐变背景和 backdrop-blur
  - 验证: 状态栏有渐变效果

- [ ] **T5.11** 美化状态栏标签样式
  - 文件: `src/components/AppLayout.tsx`
  - 内容: 当前模式标签添加背景和 ring
  - 验证: 模式名有蓝色背景胶囊

- [ ] **T5.12** 添加状态指示图标
  - 文件: `src/components/AppLayout.tsx`
  - 内容: 在状态文本前添加图标（Activity、Globe2）
  - 验证: 状态栏有对应图标

- [ ] **T5.13** 优化健康状态颜色
  - 文件: `src/components/AppLayout.tsx`
  - 内容: 运行中显示绿色（text-success）
  - 验证: "健康"文本为绿色

#### 路由页优化

- [ ] **T5.14** 优化规则列表卡片
  - 文件: `src/pages/routing/index.tsx`
  - 内容: 使用新卡片变体，添加 hover 效果
  - 验证: 规则卡片有交互反馈

- [ ] **T5.15** 美化规则类型徽章
  - 文件: `src/pages/routing/index.tsx`
  - 内容: 不同规则类型使用不同颜色徽章
  - 验证: DOMAIN/IP/GEOIP 有不同颜色

#### 设置页优化

- [ ] **T5.16** 优化设置项分组
  - 文件: `src/pages/settings/index.tsx`
  - 内容: 使用卡片包裹设置组，添加标题
  - 验证: 设置项分组清晰

- [ ] **T5.17** 美化开关组件
  - 文件: `src/components/ui/switch.tsx`（如果存在）
  - 内容: 增强开关的视觉效果
  - 验证: 开关有平滑过渡

#### 日志页优化

- [ ] **T5.18** 优化日志级别标签
  - 文件: `src/pages/logs/index.tsx`
  - 内容: ERROR 红色、WARN 橙色、INFO 蓝色
  - 验证: 不同级别有对应颜色

- [ ] **T5.19** 美化日志时间戳
  - 文件: `src/pages/logs/index.tsx`
  - 内容: 使用等宽字体，浅色显示
  - 验证: 时间戳易读

---

## Phase 6: 响应式和深色模式优化

**目标**: 完善响应式布局和深色模式体验  
**工期**: 0.5天

### 任务列表

#### 响应式优化

- [ ] **T6.1** 优化移动端头部
  - 文件: 所有页面组件
  - 内容: 移动端减小标题字号和内边距
  - 验证: < 768px 时头部紧凑

- [ ] **T6.2** 优化移动端卡片网格
  - 文件: 所有页面组件
  - 内容: 移动端改为单列布局
  - 验证: 小屏幕卡片堆叠

- [ ] **T6.3** 优化移动端表格
  - 文件: `src/pages/proxies/index.tsx`
  - 内容: 隐藏次要列，保留核心信息
  - 验证: 移动端表格可用

- [ ] **T6.4** 优化移动端底部状态栏
  - 文件: `src/components/AppLayout.tsx`
  - 内容: 移动端隐藏或简化状态信息
  - 验证: < 768px 状态栏不占用过多空间

#### 深色模式优化

- [ ] **T6.5** 验证深色模式色彩对比度
  - 工具: 浏览器开发者工具 Accessibility 面板
  - 内容: 检查所有文本对比度 ≥ 4.5:1
  - 验证: 无可访问性警告

- [ ] **T6.6** 优化深色模式阴影
  - 文件: `src/index.css`
  - 内容: 深色模式使用更明显的阴影
  - 验证: 深色模式卡片有层次感

- [ ] **T6.7** 优化深色模式边框
  - 文件: `src/index.css`
  - 内容: 调整深色模式边框透明度
  - 验证: 边框清晰可见但不刺眼

- [ ] **T6.8** 优化深色模式渐变
  - 文件: 所有使用渐变的组件
  - 内容: 深色模式使用深色系渐变
  - 验证: 渐变效果自然

---

## Phase 7: 测试和验收

**目标**: 全面测试和问题修复  
**工期**: 1天

### 任务列表

#### 功能测试

- [ ] **T7.1** 测试所有页面功能
  - 内容: 确保代理添加、删除、启用等功能正常
  - 验证: 所有功能无回归

- [ ] **T7.2** 测试表单交互
  - 内容: 测试输入框、下拉框、开关等组件
  - 验证: 表单操作流畅

- [ ] **T7.3** 测试对话框和模态框
  - 内容: 测试各类弹窗的样式和功能
  - 验证: 对话框居中、样式统一

#### 视觉测试

- [ ] **T7.4** 测试浅色模式
  - 内容: 遍历所有页面检查视觉效果
  - 验证: 色彩和谐、层次清晰

- [ ] **T7.5** 测试深色模式
  - 内容: 切换深色模式遍历所有页面
  - 验证: 深色模式无可见性问题

- [ ] **T7.6** 测试色彩对比度
  - 工具: axe DevTools 或 WAVE
  - 内容: 扫描所有页面的可访问性
  - 验证: 无 WCAG AA 级别违规

- [ ] **T7.7** 测试视觉一致性
  - 内容: 检查圆角、间距、阴影是否统一
  - 验证: 设计规范一致执行

#### 性能测试

- [ ] **T7.8** 测试动画性能
  - 工具: 浏览器 Performance 面板
  - 内容: 录制页面切换和动画性能
  - 验证: 帧率稳定在 60fps

- [ ] **T7.9** 测试首屏渲染时间
  - 工具: Lighthouse
  - 内容: 对比美化前后的 FCP、LCP 指标
  - 验证: 无明显性能退化

- [ ] **T7.10** 测试内存占用
  - 工具: 浏览器 Memory 面板
  - 内容: 检查是否有内存泄漏
  - 验证: 长时间使用内存稳定

#### 响应式测试

- [ ] **T7.11** 测试桌面端布局（1920px）
  - 内容: 检查大屏幕下的布局
  - 验证: 内容不过度拉伸

- [ ] **T7.12** 测试笔记本布局（1366px）
  - 内容: 检查常见分辨率布局
  - 验证: 布局合理，无溢出

- [ ] **T7.13** 测试平板布局（768px）
  - 内容: 检查中等屏幕布局
  - 验证: 侧边栏和内容适配良好

- [ ] **T7.14** 测试移动端布局（375px）
  - 内容: 检查手机屏幕布局
  - 验证: 内容可用，不需要横向滚动

#### 浏览器兼容性测试

- [ ] **T7.15** 测试 Chrome/Edge
  - 内容: 在 Chromium 内核浏览器测试
  - 验证: 所有功能和样式正常

- [ ] **T7.16** 测试 Firefox
  - 内容: 在 Firefox 中测试
  - 验证: backdrop-filter 等特性正常

- [ ] **T7.17** 测试 Safari（macOS）
  - 内容: 在 Safari 中测试
  - 验证: WebKit 特定样式正常

#### 问题修复

- [ ] **T7.18** 修复测试中发现的所有 bug
  - 内容: 根据测试结果逐一修复问题
  - 验证: 所有问题已解决

- [ ] **T7.19** 优化性能瓶颈
  - 内容: 针对性能测试中的问题进行优化
  - 验证: 性能指标达标

---

## 验收标准

### 完成定义

所有 P0 任务完成 + 所有 P1 任务完成 + 测试通过

### 质量标准

- ✅ 无功能回归
- ✅ 色彩对比度 ≥ 4.5:1
- ✅ 动画帧率 ≥ 60fps
- ✅ 深色/浅色模式都正常
- ✅ 响应式布局无问题
- ✅ 代码遵循现有规范

---

## 任务统计

| 阶段     | 任务数 | P0     | P1     | P2    |
| -------- | ------ | ------ | ------ | ----- |
| Phase 1  | 6      | 6      | 0      | 0     |
| Phase 2  | 7      | 5      | 2      | 0     |
| Phase 3  | 11     | 8      | 3      | 0     |
| Phase 4  | 15     | 10     | 5      | 0     |
| Phase 5  | 19     | 10     | 9      | 0     |
| Phase 6  | 8      | 4      | 4      | 0     |
| Phase 7  | 19     | 19     | 0      | 0     |
| **总计** | **85** | **62** | **23** | **0** |

---

## 进度跟踪

**开始日期**: 待定  
**预计完成**: 开始后 7 个工作日  
**当前阶段**: Phase 2  
**完成度**: 7% (6/85)

---

**文档版本**: v1.0  
**最后更新**: 2024-10-04
