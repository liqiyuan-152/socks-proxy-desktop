# UI 美化实施指南

你是一个精通现代 Web 设计的前端工程师，负责实施 socks-proxy-desktop 的 UI 美化升级。请严格遵循本指南。

---

## 项目上下文

**技术栈**: React 19 + TypeScript + TailwindCSS v4 + OKLCH 色彩空间  
**组件库**: shadcn/ui + Radix UI  
**设计文档**: `openspec/changes/ui-beautification/DESIGN_PROPOSAL.md`

**美化目标**:

- 提升视觉层次和设计一致性
- 增强交互反馈和动效
- 打造现代专业的界面体验
- 保持性能和可访问性

---

## 实施原则

### 渐进式升级

✅ 分阶段实施，每个阶段独立可发布  
✅ 优先修复布局问题，再进行美化  
✅ 保持向后兼容，不破坏现有功能  
✅ 每次改动都经过测试验证

### 代码质量

✅ 遵循 TailwindCSS 最佳实践  
✅ 保持类名顺序一致性  
✅ 提取可复用的样式模式  
✅ 添加必要的注释说明设计意图

---

## 第一阶段：色彩系统升级（2小时）

### 步骤 1.1：扩展色彩变量

**文件**: `src/index.css`

**位置**: `:root` 和 `.dark` 块内

**添加语义色彩**:

```css
:root {
  /* 在现有颜色后添加 */

  /* === 成功色 === */
  --success: oklch(0.6 0.18 145);
  --success-foreground: oklch(0.99 0 0);
  --success-light: oklch(0.95 0.1 145);

  /* === 警告色 === */
  --warning: oklch(0.72 0.18 70);
  --warning-foreground: oklch(0.2 0.02 70);
  --warning-light: oklch(0.96 0.1 70);

  /* === 信息色 === */
  --info: oklch(0.58 0.2 230);
  --info-foreground: oklch(0.99 0 0);
  --info-light: oklch(0.95 0.12 230);

  /* === 交互状态 === */
  --primary-hover: oklch(0.48 0.23 255);
  --primary-active: oklch(0.43 0.24 255);
}

.dark {
  /* 对应的深色模式颜色 */
  --success: oklch(0.65 0.18 145);
  --success-foreground: oklch(0.99 0 0);

  --warning: oklch(0.75 0.18 70);
  --warning-foreground: oklch(0.12 0.02 70);

  --info: oklch(0.62 0.2 230);
  --info-foreground: oklch(0.99 0 0);

  --primary-hover: oklch(0.68 0.23 255);
  --primary-active: oklch(0.63 0.24 255);
}
```

**更新 @theme**:

```css
@theme inline {
  /* 在现有主题变量后添加 */
  --color-success: var(--success);
  --color-success-foreground: var(--success-foreground);
  --color-warning: var(--warning);
  --color-warning-foreground: var(--warning-foreground);
  --color-info: var(--info);
  --color-info-foreground: var(--info-foreground);
  --color-primary-hover: var(--primary-hover);
  --color-primary-active: var(--primary-active);
}
```

**验证**:

```bash
# 启动开发服务器
pnpm tauri dev

# 在浏览器开发者工具 Console 中测试
getComputedStyle(document.documentElement).getPropertyValue('--success')
// 应返回: oklch(0.60 0.18 145)
```

---

### 步骤 1.2：创建 Badge 成功/警告变体

**文件**: `src/components/ui/badge.tsx`

**找到 `badgeVariants` 定义**，添加新变体:

```tsx
const badgeVariants = cva(
  "inline-flex items-center rounded-md border px-2.5 py-0.5 text-xs font-semibold transition-colors focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2",
  {
    variants: {
      variant: {
        default: "border-transparent bg-primary text-primary-foreground shadow hover:bg-primary/80",
        secondary:
          "border-transparent bg-secondary text-secondary-foreground hover:bg-secondary/80",
        destructive:
          "border-transparent bg-destructive text-destructive-foreground shadow hover:bg-destructive/80",
        outline: "text-foreground",

        // 新增语义色变体
        success:
          "border-transparent bg-success text-success-foreground shadow-sm hover:bg-success/90",
        warning:
          "border-transparent bg-warning text-warning-foreground shadow-sm hover:bg-warning/90",
        info: "border-transparent bg-info text-info-foreground shadow-sm hover:bg-info/90",
      },
    },
    defaultVariants: {
      variant: "default",
    },
  },
);
```

**使用示例**:

```tsx
// 在代理列表中使用
<Badge variant={proxy.enabled ? "success" : "secondary"}>
  {proxy.enabled ? "已启用" : "未启用"}
</Badge>

// 在状态页面使用
<Badge variant={running ? "success" : "secondary"}>
  {running ? "运行中" : "未运行"}
</Badge>
```

---

## 第二阶段：按钮升级（1小时）

### 步骤 2.1：增强按钮样式

**文件**: `src/components/ui/button.tsx`

**修改 `buttonVariants` 的基础类**:

```tsx
const buttonVariants = cva(
  // 基础类 - 增强圆角和过渡
  "inline-flex shrink-0 items-center justify-center gap-2 rounded-lg text-sm font-medium whitespace-nowrap transition-all duration-200 outline-none focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50 disabled:pointer-events-none disabled:opacity-50 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4",
  {
    variants: {
      variant: {
        // 增强 default - 添加阴影和 hover 效果
        default:
          "bg-primary text-primary-foreground shadow-sm shadow-primary/20 hover:bg-primary-hover hover:shadow-md hover:shadow-primary/30 active:bg-primary-active active:shadow-sm transition-shadow",

        // 新增 gradient 变体
        gradient:
          "bg-gradient-to-br from-[oklch(0.58_0.22_255)] to-[oklch(0.52_0.24_265)] text-white shadow-lg shadow-primary/30 hover:shadow-xl hover:shadow-primary/40 active:scale-[0.98]",

        destructive:
          "bg-destructive text-white shadow-sm shadow-destructive/20 hover:bg-destructive/90 hover:shadow-md hover:shadow-destructive/30",

        outline:
          "border-2 border-input bg-background hover:bg-accent hover:border-primary/50 hover:text-primary dark:bg-input/30",

        secondary: "bg-secondary text-secondary-foreground shadow-sm hover:bg-secondary/80",

        ghost: "hover:bg-accent/80 hover:text-accent-foreground",

        link: "text-primary underline-offset-4 hover:underline",
      },
      size: {
        default: "h-10 px-5 py-2.5 has-[>svg]:px-4",
        xs: "h-6 gap-1 rounded-md px-2 text-xs has-[>svg]:px-1.5",
        sm: "h-8 gap-1.5 rounded-lg px-3 text-xs has-[>svg]:px-2.5",
        lg: "h-12 rounded-lg px-7 text-base has-[>svg]:px-5",
        icon: "size-10 rounded-lg",
        "icon-xs": "size-6 rounded-md",
        "icon-sm": "size-8 rounded-lg",
        "icon-lg": "size-12 rounded-lg",
      },
    },
  },
);
```

**测试新按钮**:

```tsx
// 在任意页面测试
<div className="flex gap-2">
  <Button variant="default">默认按钮</Button>
  <Button variant="gradient">渐变按钮</Button>
  <Button variant="outline">边框按钮</Button>
  <Button variant="ghost">幽灵按钮</Button>
</div>
```

---

## 第三阶段：卡片美化（1.5小时）

### 步骤 3.1：增强 Card 组件

**文件**: `src/components/ui/card.tsx`

**添加 variant 支持**:

```tsx
import { cn } from "cn";
import { forwardRef } from "react";

const Card = forwardRef<
  HTMLDivElement,
  React.HTMLAttributes<HTMLDivElement> & {
    variant?: "default" | "elevated" | "glass" | "bordered";
  }
>(({ className, variant = "default", ...props }, ref) => {
  const variants = {
    default: "bg-card border border-border shadow-sm",
    elevated: "bg-card border border-border/50 shadow-lg shadow-black/5",
    glass:
      "bg-white/50 dark:bg-white/5 backdrop-blur-md border border-white/20 dark:border-white/10",
    bordered: "bg-transparent border-2 border-border hover:border-primary/50 transition-colors",
  };

  return (
    <div
      ref={ref}
      className={cn("rounded-xl p-6 transition-all duration-200", variants[variant], className)}
      {...props}
    />
  );
});
Card.displayName = "Card";

// 保持其他子组件不变
const CardHeader = forwardRef<HTMLDivElement, React.HTMLAttributes<HTMLDivElement>>(
  ({ className, ...props }, ref) => (
    <div ref={ref} className={cn("flex flex-col space-y-1.5 p-6", className)} {...props} />
  ),
);
CardHeader.displayName = "CardHeader";

// ... 其他组件保持不变
```

**使用示例**:

```tsx
// 状态页面的统计卡片
<Card variant="elevated" className="group overflow-hidden">
  <CardHeader className="pb-3">
    <CardTitle className="text-sm font-medium text-muted-foreground">当前活跃连接</CardTitle>
  </CardHeader>
  <CardContent>
    <div className="text-3xl font-bold">128</div>
    <p className="text-xs text-muted-foreground mt-1">
      比昨天 <span className="text-success">↑ 12%</span>
    </p>
  </CardContent>
</Card>
```

---

### 步骤 3.2：优化表格样式

**文件**: `src/index.css`

**添加表格样式类**:

```css
/* 在文件末尾添加 */

/* === 现代表格样式 === */
.table-modern {
  /* 表头样式 */
  & thead th {
    @apply bg-gradient-to-b from-muted/80 to-muted/50;
    @apply text-xs font-semibold uppercase tracking-wider text-muted-foreground;
    @apply border-b-2 border-border/80;
    @apply sticky top-0 z-10;
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
  }

  /* 行 hover 效果 */
  & tbody tr {
    @apply transition-colors duration-150;
    @apply hover:bg-accent/50;
    @apply border-b border-border/50;
  }

  & tbody tr:last-child {
    @apply border-b-0;
  }

  /* 选中行高亮 */
  & tbody tr[data-selected="true"] {
    @apply bg-primary/10 border-primary/30;
  }

  /* 单元格间距 */
  & td,
  & th {
    @apply px-4 py-3;
  }
}

/* 表格斑马纹（可选） */
.table-striped tbody tr:nth-child(even) {
  @apply bg-muted/20;
}
```

**应用到代理列表**:

**文件**: `src/pages/proxies/index.tsx`

找到 `<Table>` 标签，添加类名:

```tsx
<Table className="table-modern table-fixed text-sm">{/* 表格内容保持不变 */}</Table>
```

---

## 第四阶段：布局和动画（2小时）

### 步骤 4.1：优化页面头部

**文件**: `src/pages/status/index.tsx`, `src/pages/proxies/index.tsx` 等所有页面

**统一头部样式**:

```tsx
<header className="relative overflow-hidden border-b border-sidebar-border bg-gradient-to-br from-sidebar via-sidebar to-sidebar/80 px-6 py-5 text-sidebar-foreground backdrop-blur-sm">
  {/* 可选：添加装饰性渐变 */}
  <div className="pointer-events-none absolute inset-0 bg-gradient-to-br from-primary/5 via-transparent to-transparent" />

  <div className="relative z-10">
    <h1 className="text-2xl font-bold tracking-tight">页面标题</h1>
    <p className="mt-1.5 text-sm text-muted-foreground">页面描述</p>
  </div>
</header>
```

---

### 步骤 4.2：添加页面过渡动画

**文件**: `src/index.css`

**添加动画定义**:

```css
/* === 页面过渡动画 === */
@keyframes fadeIn {
  from {
    opacity: 0;
    transform: translateY(8px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

@keyframes slideInRight {
  from {
    opacity: 0;
    transform: translateX(16px);
  }
  to {
    opacity: 1;
    transform: translateX(0);
  }
}

@keyframes scaleIn {
  from {
    opacity: 0;
    transform: scale(0.96);
  }
  to {
    opacity: 1;
    transform: scale(1);
  }
}

@keyframes shimmer {
  from {
    background-position: -200% 0;
  }
  to {
    background-position: 200% 0;
  }
}

/* 动画工具类 */
.animate-fade-in {
  animation: fadeIn 0.3s ease-out;
}

.animate-slide-in {
  animation: slideInRight 0.4s ease-out;
}

.animate-scale-in {
  animation: scaleIn 0.2s ease-out;
}

/* 列表项依次出现 */
.stagger-children > * {
  animation: fadeIn 0.3s ease-out backwards;
}

.stagger-children > *:nth-child(n + 1) {
  animation-delay: 0s;
}
.stagger-children > *:nth-child(n + 2) {
  animation-delay: 0.05s;
}
.stagger-children > *:nth-child(n + 3) {
  animation-delay: 0.1s;
}
.stagger-children > *:nth-child(n + 4) {
  animation-delay: 0.15s;
}
.stagger-children > *:nth-child(n + 5) {
  animation-delay: 0.2s;
}
.stagger-children > *:nth-child(n + 6) {
  animation-delay: 0.25s;
}
```

**应用动画**:

```tsx
// 页面主内容添加淡入动画
<div className="content-scroll flex-1 overflow-y-auto px-6 py-6 animate-fade-in">
  {/* 内容 */}
</div>

// 卡片列表添加交错动画
<div className="grid gap-4 stagger-children">
  {items.map(item => <Card key={item.id}>{/* ... */}</Card>)}
</div>
```

---

### 步骤 4.3：美化侧边栏

**文件**: `src/components/AppLayout.tsx`

**优化 Logo 区域** (第 95-104 行):

```tsx
<SidebarHeader className="p-4">
  <div className="flex h-12 items-center gap-3 px-2">
    {/* 增强 Logo 样式 */}
    <div className="grid size-10 shrink-0 place-items-center rounded-xl bg-gradient-to-br from-blue-600 to-blue-700 text-white shadow-lg shadow-blue-600/30 ring-2 ring-blue-500/20 ring-offset-2 ring-offset-background">
      <ShieldCheck className="size-5" aria-hidden="true" />
    </div>
    <div className="group-data-[collapsible=icon]:hidden">
      <span className="text-base font-bold">Socks Proxy</span>
      <p className="text-xs text-muted-foreground">代理管理工具</p>
    </div>
  </div>
</SidebarHeader>
```

**优化导航项样式** (第 56-70 行):

```tsx
<SidebarMenuButton
  asChild
  isActive={isActive}
  tooltip={label}
  className="
    h-11 rounded-lg text-sm font-medium transition-all duration-200
    text-sidebar-foreground/70
    hover:bg-sidebar-accent/60 hover:text-sidebar-foreground hover:scale-[1.02]
    data-[active=true]:bg-gradient-to-r 
    data-[active=true]:from-blue-600 
    data-[active=true]:to-blue-700
    data-[active=true]:text-white
    data-[active=true]:shadow-lg 
    data-[active=true]:shadow-blue-600/30
    data-[active=true]:scale-[1.02]
  "
>
  <NavLink to={to} end={to === "/"}>
    <Icon className="size-5" aria-hidden="true" />
    <span>{label}</span>
  </NavLink>
</SidebarMenuButton>
```

---

## 第五阶段：特定页面优化（2-3小时）

### 步骤 5.1：美化状态页面的模式切换

**文件**: `src/pages/status/index.tsx`

**找到 `<TabsList>` (第100行)**，替换为:

```tsx
<TabsList className="grid h-14 w-full grid-cols-3 overflow-hidden rounded-xl border-2 border-border bg-gradient-to-b from-muted/30 to-muted/60 p-1.5 backdrop-blur-sm sm:mx-auto sm:max-w-2xl">
  {(Object.keys(proxyModes) as ProxyMode[]).map((mode) => (
    <TabsTrigger
      key={mode}
      value={mode}
      className="
        h-full rounded-lg border-0 text-sm font-semibold
        bg-transparent text-muted-foreground
        transition-all duration-300
        hover:bg-white/50 hover:text-foreground dark:hover:bg-white/10
        data-[state=active]:bg-gradient-to-br
        data-[state=active]:from-blue-600
        data-[state=active]:to-blue-700
        data-[state=active]:text-white
        data-[state=active]:shadow-lg
        data-[state=active]:shadow-blue-600/40
        data-[state=active]:scale-105
      "
      disabled={loading || !snapshot || !capabilities?.proxy_runtime}
    >
      {proxyModes[mode].label}
    </TabsTrigger>
  ))}
</TabsList>
```

---

### 步骤 5.2：优化错误提示组件

**文件**: `src/components/RuntimeFeedback.tsx`

**找到返回的 JSX (第51-92行)**，优化样式:

```tsx
<section
  aria-label="运行时反馈"
  className="animate-fade-in rounded-xl border-2 border-destructive/50 bg-gradient-to-br from-destructive/5 via-destructive/8 to-destructive/10 p-5 backdrop-blur-sm"
>
  <div className="flex items-start gap-4">
    {/* 图标容器 */}
    <div className="shrink-0 rounded-lg bg-destructive/10 p-2.5 ring-1 ring-destructive/20">
      <AlertCircle className="size-5 text-destructive" />
    </div>

    {/* 内容区域 */}
    <div className="flex-1 space-y-3">
      {errors.map((reason) => (
        <ErrorAlert
          key={reason.context?.error_id ?? reason.message}
          error={reason}
          onOpenDiagnostics={() => navigate("/logs")}
        />
      ))}
      {recoveryRequired && (
        <p role="alert" className="text-sm text-destructive">
          恢复未完成，请检查 Windows 系统代理设置；应用不会覆盖外部修改。
        </p>
      )}

      <p className="text-sm text-muted-foreground">
        当前仍生效的模式：
        {snapshot?.applied_mode ? proxyModes[snapshot.applied_mode].label : "未应用"}
        {snapshot?.session_health === "healthy" && "，原内核仍在运行。"}
      </p>

      {/* 操作按钮 */}
      <div className="flex gap-2 pt-1">
        {capabilities?.proxy_runtime && snapshot && !recoveryRequired && (
          <Button
            variant="outline"
            size="sm"
            disabled={pending || recovering}
            onClick={() => void switchMode(snapshot.desired_mode)}
          >
            重试模式切换
          </Button>
        )}
        {capabilities?.network_recovery && (
          <Button
            variant="outline"
            size="sm"
            disabled={pending || recovering}
            onClick={() => void recover()}
          >
            {recovering ? "正在恢复…" : "恢复系统代理"}
          </Button>
        )}
      </div>
    </div>
  </div>
</section>
```

---

### 步骤 5.3：优化底部状态栏

**文件**: `src/components/AppLayout.tsx`

**找到 `<footer>` (第141-158行)**，优化样式:

```tsx
<footer className="hidden shrink-0 items-center gap-4 border-t border-sidebar-border/80 bg-gradient-to-r from-sidebar/95 via-sidebar to-sidebar/95 px-6 py-3.5 text-xs text-muted-foreground backdrop-blur-sm lg:flex">
  <span className="flex items-center gap-2">
    <span className="text-sidebar-foreground/60">当前模式</span>
    <strong className="rounded-md bg-primary/10 px-2 py-0.5 font-semibold text-primary ring-1 ring-primary/20">
      {activeModeLabel}
    </strong>
  </span>

  <Separator orientation="vertical" className="h-4" />

  <span className="flex items-center gap-2">
    <span className="text-sidebar-foreground/60">当前代理</span>
    <strong className="font-semibold text-foreground">{profileName}</strong>
  </span>

  <Separator orientation="vertical" className="h-4" />

  <span className="flex items-center gap-2">
    <Activity className="size-3.5" aria-hidden="true" />
    内核运行状态：
    <span className={cn("font-medium", running ? "text-success" : "text-muted-foreground")}>
      {snapshot ? (running ? "健康" : "未运行") : "不可用"}
    </span>
  </span>

  <span className="ml-auto flex items-center gap-2">
    <Globe2 className="size-3.5" aria-hidden="true" />
    {snapshot?.coverage === "system_proxy_apps" ? "系统代理应用流量" : "未接管系统代理"}
  </span>
</footer>
```

---

## 验收清单

### 视觉质量

- [ ] 色彩对比度符合 WCAG AA 标准（至少 4.5:1）
- [ ] 圆角统一使用 `rounded-lg` (12px) 或 `rounded-xl` (16px)
- [ ] 所有按钮有明显的 hover/active 反馈
- [ ] 卡片有适当的阴影和间距
- [ ] 表格行 hover 效果明显

### 动画性能

- [ ] 页面切换流畅，无卡顿
- [ ] 动画帧率稳定在 60fps
- [ ] 无不必要的重绘和回流

### 响应式

- [ ] 不同窗口大小下布局正常
- [ ] 移动端（< 768px）体验良好
- [ ] 侧边栏折叠/展开流畅

### 深色模式

- [ ] 深色模式下所有颜色可见且舒适
- [ ] 无过亮或过暗的元素
- [ ] 阴影和边框在深色模式下适配良好

---

## 测试流程

### 1. 启动开发服务器

```bash
pnpm tauri dev
```

### 2. 测试色彩

```javascript
// 在浏览器控制台
const root = document.documentElement;
["--success", "--warning", "--info"].forEach((color) => {
  console.log(color, getComputedStyle(root).getPropertyValue(color));
});
```

### 3. 测试各种按钮变体

- 访问 `/proxies` 页面
- 测试"添加代理"按钮 (gradient 变体)
- 测试表格中的操作按钮 (ghost 变体)
- 测试确认对话框按钮 (default 和 outline)

### 4. 测试动画

- 切换不同页面，观察淡入效果
- 添加多个代理，观察列表项交错动画
- hover 表格行，观察背景变化

### 5. 测试响应式

- 调整窗口宽度: 800px → 1024px → 1280px → 1920px
- 检查布局在不同尺寸下是否正常
- 检查表格列隐藏/显示是否符合预期

### 6. 测试深色模式

- 切换系统深色模式
- 检查所有颜色是否适配
- 检查阴影和边框是否可见

---

## 常见问题

### 问题 1: 动画不流畅

**检查**: CSS 动画是否使用了 transform 和 opacity（GPU 加速）  
**避免**: 动画 width、height、top、left（触发重排）

### 问题 2: 颜色在深色模式下不可见

**检查**: 是否同时定义了 `:root` 和 `.dark` 下的颜色  
**修复**: 补充深色模式对应的颜色变量

### 问题 3: 圆角不一致

**统一**: 小元素用 `rounded-lg` (12px)，大容器用 `rounded-xl` (16px)

### 问题 4: Tailwind 类名不生效

**检查**: 类名是否完整（不要动态拼接）  
**修复**: 使用 `cn()` 函数或完整的类名字符串

---

## 提交规范

```
feat(ui): [阶段名称] UI 美化

- 具体改动1
- 具体改动2
- 具体改动3

视觉效果:
- 改进了色彩层次
- 增强了交互反馈
- 统一了视觉规范

测试:
- [x] 深色/浅色模式
- [x] 响应式布局
- [x] 动画性能
```

---

## 开始实施

现在请：

1. 阅读 `DESIGN_PROPOSAL.md` 了解设计理念
2. 从第一阶段"色彩系统升级"开始
3. 每完成一个步骤就测试验证
4. 遇到问题参考"常见问题"章节

准备好了吗？开始美化 UI！
