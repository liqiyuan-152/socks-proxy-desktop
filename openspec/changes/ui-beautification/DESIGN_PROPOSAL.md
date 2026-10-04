# UI 整体美化方案

**项目**: socks-proxy-desktop  
**设计日期**: 2026-10-04  
**目标**: 提升视觉设计、改善用户体验、打造现代专业的界面

---

## 设计理念

### 核心原则

1. **清晰层次** - 明确的视觉层级，让信息易于理解
2. **现代简约** - 去除冗余，保持界面简洁
3. **一致性** - 统一的设计语言和交互模式
4. **专业感** - 适合专业用户的技术工具美学
5. **可访问性** - 符合 WCAG 标准的对比度和交互

### 设计关键词

- **专业 (Professional)** - 系统工具应有的专业形象
- **高效 (Efficient)** - 减少操作步骤，提高信息密度
- **现代 (Modern)** - 符合 2024 年设计趋势
- **可靠 (Reliable)** - 视觉传达稳定可靠的感觉

---

## 当前设计分析

### ✅ 优点

- shadcn/ui 组件库提供了良好的基础
- TailwindCSS v4 + OKLCH 色彩空间（先进）
- 侧边栏导航清晰
- 响应式基础良好

### ⚠️ 改进空间

1. **色彩层次不够丰富** - 过于依赖蓝色和灰色
2. **视觉重点不突出** - 关键操作和状态不够醒目
3. **间距节奏单一** - 缺少视觉呼吸感
4. **交互反馈不足** - hover、active 状态不够明显
5. **信息层级模糊** - 卡片、表格、状态区分度低
6. **缺少品牌特色** - 设计过于通用

---

## 美化方案

## 一、色彩系统重构

### 1.1 品牌色优化

**当前品牌色**：

```css
--primary: oklch(0.52 0.21 255); /* 蓝色 */
```

**优化方案**：引入渐变和语义色

```css
/* src/index.css */
:root {
  /* === 品牌色升级 === */
  --primary: oklch(0.55 0.22 255); /* 主品牌蓝（更亮） */
  --primary-hover: oklch(0.5 0.23 255); /* hover 状态 */
  --primary-active: oklch(0.45 0.24 255); /* active 状态 */

  /* === 语义色增强 === */
  --success: oklch(0.6 0.18 145); /* 成功绿 */
  --success-foreground: oklch(0.99 0 0);
  --warning: oklch(0.72 0.18 70); /* 警告橙 */
  --warning-foreground: oklch(0.2 0.02 70);
  --info: oklch(0.58 0.2 230); /* 信息蓝 */
  --info-foreground: oklch(0.99 0 0);

  /* === 中性色细化 === */
  --gray-50: oklch(0.985 0.003 250);
  --gray-100: oklch(0.97 0.005 250);
  --gray-200: oklch(0.94 0.008 250);
  --gray-300: oklch(0.88 0.012 250);
  --gray-400: oklch(0.72 0.015 250);
  --gray-500: oklch(0.56 0.018 250);
  --gray-600: oklch(0.45 0.02 250);
  --gray-700: oklch(0.34 0.02 250);
  --gray-800: oklch(0.26 0.02 250);
  --gray-900: oklch(0.18 0.02 250);

  /* === 渐变色 === */
  --gradient-primary: linear-gradient(135deg, oklch(0.58 0.22 255) 0%, oklch(0.52 0.24 265) 100%);
  --gradient-success: linear-gradient(135deg, oklch(0.62 0.18 145) 0%, oklch(0.56 0.2 155) 100%);
  --gradient-glass: linear-gradient(135deg, oklch(1 0 0 / 0.05) 0%, oklch(1 0 0 / 0.02) 100%);
}

.dark {
  /* 暗色模式优化 */
  --primary: oklch(0.65 0.22 255);
  --primary-hover: oklch(0.7 0.23 255);
  --primary-active: oklch(0.6 0.24 255);

  --success: oklch(0.65 0.18 145);
  --warning: oklch(0.75 0.18 70);
  --info: oklch(0.62 0.2 230);

  /* 暗色模式渐变更柔和 */
  --gradient-primary: linear-gradient(135deg, oklch(0.62 0.2 255) 0%, oklch(0.58 0.22 265) 100%);
}
```

### 1.2 状态色彩映射

| 状态        | 颜色                 | 使用场景               |
| ----------- | -------------------- | ---------------------- |
| 运行中/健康 | `success` (绿色)     | 代理运行状态、成功提示 |
| 警告/恢复中 | `warning` (橙色)     | 恢复状态、警告信息     |
| 错误/失败   | `destructive` (红色) | 错误提示、危险操作     |
| 信息/处理中 | `info` (蓝色)        | 加载状态、信息提示     |
| 已启用      | `primary` (品牌蓝)   | 已选中、主要操作       |
| 未启用/禁用 | `muted` (灰色)       | 禁用状态、次要信息     |

---

## 二、组件视觉升级

### 2.1 按钮增强

**新增按钮变体**：

```tsx
// src/components/ui/button.tsx
const buttonVariants = cva(
  "inline-flex shrink-0 items-center justify-center gap-2 rounded-lg text-sm font-medium whitespace-nowrap transition-all duration-200 outline-none focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50 disabled:pointer-events-none disabled:opacity-50 [&_svg]:pointer-events-none [&_svg]:shrink-0",
  {
    variants: {
      variant: {
        default:
          "bg-primary text-primary-foreground shadow-sm shadow-primary/20 hover:bg-primary-hover hover:shadow-md hover:shadow-primary/30 active:bg-primary-active active:shadow-sm",

        gradient:
          "bg-gradient-to-br from-[oklch(0.58_0.22_255)] to-[oklch(0.52_0.24_265)] text-white shadow-lg shadow-primary/30 hover:shadow-xl hover:shadow-primary/40 active:scale-[0.98]",

        success: "bg-success text-success-foreground shadow-sm shadow-success/20 hover:opacity-90",

        outline:
          "border-2 border-input bg-background hover:bg-accent hover:border-primary/50 hover:text-primary",

        ghost: "hover:bg-accent/80 hover:text-accent-foreground",

        glass:
          "bg-white/10 backdrop-blur-sm border border-white/20 hover:bg-white/20 hover:border-white/30",
      },
      size: {
        default: "h-10 px-5 py-2.5 has-[>svg]:px-4",
        sm: "h-8 gap-1.5 rounded-lg px-3 text-xs has-[>svg]:px-2.5",
        lg: "h-12 rounded-lg px-7 text-base has-[>svg]:px-5",
        icon: "size-10 rounded-lg",
        "icon-sm": "size-8 rounded-lg",
        "icon-lg": "size-12 rounded-lg",
      },
    },
  },
);
```

**效果**：

- ✨ 添加阴影和 hover 效果
- ✨ 渐变按钮用于主要 CTA
- ✨ 玻璃态按钮用于覆层场景
- ✨ 更大的圆角（8px → 12px）

### 2.2 卡片升级

```tsx
// src/components/ui/card.tsx
const Card = React.forwardRef<
  HTMLDivElement,
  React.ComponentProps<"div"> & {
    variant?: "default" | "elevated" | "glass" | "bordered";
  }
>(({ className, variant = "default", ...props }, ref) => {
  const variants = {
    default: "bg-card border border-border",
    elevated: "bg-card shadow-lg shadow-black/5 border border-border/50",
    glass:
      "bg-white/50 dark:bg-white/5 backdrop-blur-md border border-white/20 dark:border-white/10",
    bordered: "bg-transparent border-2 border-border hover:border-primary/50",
  };

  return (
    <div
      ref={ref}
      className={cn("rounded-xl p-6 transition-all duration-200", variants[variant], className)}
      {...props}
    />
  );
});
```

**效果**：

- ✨ 多种卡片风格选择
- ✨ 更大的圆角（8px → 12px）
- ✨ 玻璃态效果（毛玻璃背景）
- ✨ hover 交互反馈

### 2.3 表格美化

```css
/* src/index.css 添加 */
.table-modern {
  /* 表头优化 */
  & thead th {
    @apply bg-gradient-to-b from-muted/80 to-muted/50 text-xs font-semibold uppercase tracking-wider text-muted-foreground;
    @apply border-b-2 border-border/80;
    @apply sticky top-0 z-10;
    backdrop-filter: blur(8px);
  }

  /* 行优化 */
  & tbody tr {
    @apply transition-colors duration-150;
    @apply hover:bg-accent/50;
    @apply border-b border-border/50;
  }

  & tbody tr:last-child {
    @apply border-b-0;
  }

  /* 斑马纹（可选） */
  & tbody tr:nth-child(even) {
    @apply bg-muted/20;
  }

  /* 选中行高亮 */
  & tbody tr[data-selected="true"] {
    @apply bg-primary/10 border-primary/30;
  }
}
```

### 2.4 输入框增强

```tsx
// src/components/ui/input.tsx 添加
const Input = React.forwardRef<HTMLInputElement, React.ComponentProps<"input">>(
  ({ className, type, ...props }, ref) => {
    return (
      <input
        type={type}
        className={cn(
          "flex h-10 w-full rounded-lg border-2 border-input bg-background px-3 py-2 text-sm",
          "transition-all duration-200",
          "placeholder:text-muted-foreground",
          "focus-visible:outline-none focus-visible:border-primary focus-visible:ring-4 focus-visible:ring-primary/10",
          "hover:border-input-hover",
          "disabled:cursor-not-allowed disabled:opacity-50",
          className,
        )}
        ref={ref}
        {...props}
      />
    );
  },
);
```

---

## 三、布局优化

### 3.1 间距系统规范

```css
/* 统一间距变量 */
:root {
  --spacing-page: 1.5rem; /* 24px - 页面边距 */
  --spacing-section: 2rem; /* 32px - 区块间距 */
  --spacing-card: 1.25rem; /* 20px - 卡片内边距 */
  --spacing-item: 0.75rem; /* 12px - 列表项间距 */

  --radius-sm: 0.5rem; /* 8px */
  --radius-md: 0.75rem; /* 12px */
  --radius-lg: 1rem; /* 16px */
  --radius-xl: 1.5rem; /* 24px */
}
```

### 3.2 页面容器优化

```tsx
// 标准页面容器
<div className="flex min-h-0 flex-1 flex-col">
  {/* 页面头部 - 带渐变背景 */}
  <header className="relative border-b border-sidebar-border bg-gradient-to-br from-sidebar to-sidebar-accent/30 px-6 py-5 text-sidebar-foreground backdrop-blur-sm">
    <div className="relative z-10">
      <h1 className="text-2xl font-bold tracking-tight">页面标题</h1>
      <p className="mt-1.5 text-sm text-muted-foreground">页面描述</p>
    </div>
  </header>

  {/* 页面内容 */}
  <div className="content-scroll flex-1 overflow-y-auto px-6 py-6">
    <div className="mx-auto w-full max-w-7xl space-y-6">{/* 内容区域 */}</div>
  </div>
</div>
```

### 3.3 侧边栏美化

```tsx
// src/components/AppLayout.tsx 侧边栏样式升级
<Sidebar
  collapsible="icon"
  className="border-r border-sidebar-border bg-gradient-to-b from-sidebar via-sidebar to-sidebar-accent/20"
>
  {/* Logo 区域增强 */}
  <SidebarHeader className="p-4">
    <div className="flex h-12 items-center gap-3 px-2">
      <div className="grid size-10 shrink-0 place-items-center rounded-xl bg-gradient-to-br from-blue-600 to-blue-700 text-white shadow-lg shadow-blue-600/30 ring-2 ring-blue-500/20">
        <ShieldCheck className="size-5" aria-hidden="true" />
      </div>
      <div className="group-data-[collapsible=icon]:hidden">
        <span className="text-base font-bold">Socks Proxy</span>
        <p className="text-xs text-muted-foreground">代理管理工具</p>
      </div>
    </div>
  </SidebarHeader>

  {/* 导航项增强 */}
  <SidebarMenuButton
    className="
      h-11 rounded-lg text-sm font-medium
      text-sidebar-foreground/70
      hover:bg-sidebar-accent/60 hover:text-sidebar-foreground
      data-[active=true]:bg-gradient-to-r data-[active=true]:from-blue-600 data-[active=true]:to-blue-700
      data-[active=true]:text-white data-[active=true]:shadow-lg data-[active=true]:shadow-blue-600/30
      transition-all duration-200
    "
  >
    <Icon className="size-5" />
    <span>{label}</span>
  </SidebarMenuButton>
</Sidebar>
```

---

## 四、交互动效

### 4.1 页面过渡动画

```css
/* src/index.css */
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
    transform: scale(0.95);
  }
  to {
    opacity: 1;
    transform: scale(1);
  }
}

/* 应用动画 */
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

.stagger-children > *:nth-child(1) {
  animation-delay: 0s;
}
.stagger-children > *:nth-child(2) {
  animation-delay: 0.05s;
}
.stagger-children > *:nth-child(3) {
  animation-delay: 0.1s;
}
.stagger-children > *:nth-child(4) {
  animation-delay: 0.15s;
}
.stagger-children > *:nth-child(5) {
  animation-delay: 0.2s;
}
```

### 4.2 Hover 效果增强

```css
/* 卡片 hover 效果 */
.card-interactive {
  @apply transition-all duration-200;
  @apply hover:shadow-lg hover:shadow-black/5;
  @apply hover:-translate-y-0.5;
  @apply hover:border-primary/30;
}

/* 按钮 hover 脉冲效果 */
.button-pulse:hover {
  animation: pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite;
}

@keyframes pulse {
  0%,
  100% {
    opacity: 1;
  }
  50% {
    opacity: 0.8;
  }
}

/* 图标旋转效果 */
.icon-spin-on-hover:hover svg {
  animation: spin 0.6s ease-in-out;
}

@keyframes spin {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
}
```

### 4.3 加载状态优化

```tsx
// src/components/ui/loading-spinner.tsx (新建)
export function LoadingSpinner({ size = "default" }) {
  const sizes = {
    sm: "size-4",
    default: "size-6",
    lg: "size-8",
  };

  return (
    <div className={cn("animate-spin rounded-full border-2 border-current border-t-transparent", sizes[size])} />
  );
}

// 骨架屏组件
export function Skeleton({ className, ...props }) {
  return (
    <div
      className={cn(
        "animate-pulse rounded-lg bg-gradient-to-r from-muted via-muted-foreground/20 to-muted",
        "bg-[length:200%_100%]",
        className
      )}
      style={{
        animation: "shimmer 2s ease-in-out infinite",
      }}
      {...props}
    />
  );
}

/* CSS */
@keyframes shimmer {
  0% {
    background-position: -200% 0;
  }
  100% {
    background-position: 200% 0;
  }
}
```

---

## 五、特定页面优化

### 5.1 状态页面

**模式切换 Tabs 美化**：

```tsx
// src/pages/status/index.tsx
<Tabs value={selectedMode} onValueChange={selectMode}>
  <TabsList className="grid h-14 w-full grid-cols-3 overflow-hidden rounded-xl border-2 border-border bg-gradient-to-b from-muted/30 to-muted/60 p-1 backdrop-blur-sm sm:mx-auto sm:max-w-2xl">
    {modes.map((mode) => (
      <TabsTrigger
        key={mode}
        value={mode}
        className="
          h-full rounded-lg border-0 text-sm font-semibold
          bg-transparent text-muted-foreground
          transition-all duration-300
          hover:bg-white/50 hover:text-foreground dark:hover:bg-white/10
          data-[state=active]:bg-gradient-to-br data-[state=active]:from-blue-600 data-[state=active]:to-blue-700
          data-[state=active]:text-white data-[state=active]:shadow-lg data-[state=active]:shadow-blue-600/40
          data-[state=active]:scale-105
        "
      >
        <span className="flex items-center gap-2">
          {getModeIcon(mode)}
          {proxyModes[mode].label}
        </span>
      </TabsTrigger>
    ))}
  </TabsList>
</Tabs>
```

**统计卡片优化**：

```tsx
<Card variant="elevated" className="group overflow-hidden">
  <div className="flex items-start justify-between">
    <div className="space-y-2">
      <p className="text-sm font-medium text-muted-foreground">当前活跃连接</p>
      <p className="text-3xl font-bold tracking-tight">{connections?.active_count ?? 0}</p>
      <p className="text-xs text-muted-foreground">
        比昨天 <span className="text-success">↑ 12%</span>
      </p>
    </div>
    <div className="rounded-lg bg-gradient-to-br from-blue-500/10 to-blue-600/10 p-3 ring-1 ring-blue-500/20">
      <Activity className="size-6 text-blue-600" />
    </div>
  </div>
  <div className="mt-4 h-1 rounded-full bg-muted">
    <div className="h-full w-2/3 rounded-full bg-gradient-to-r from-blue-600 to-blue-700 transition-all duration-500" />
  </div>
</Card>
```

### 5.2 代理列表页面

**搜索栏美化**：

```tsx
<div className="relative">
  <Search className="absolute left-3 top-1/2 size-5 -translate-y-1/2 text-muted-foreground" />
  <Input
    type="search"
    placeholder="搜索代理名称、服务器地址..."
    value={search}
    onChange={(e) => setSearch(e.target.value)}
    className="pl-10 h-12 rounded-xl border-2 bg-background/50 backdrop-blur-sm"
  />
</div>
```

**代理卡片（移动端）**：

```tsx
<Card variant="elevated" className="card-interactive">
  <div className="flex items-start justify-between">
    <div className="flex items-center gap-3">
      <div className="rounded-lg bg-gradient-to-br from-blue-500/10 to-blue-600/10 p-2.5 ring-1 ring-blue-500/20">
        <Server className="size-5 text-blue-600" />
      </div>
      <div>
        <h3 className="font-semibold">{proxy.name}</h3>
        <p className="text-sm text-muted-foreground">
          {proxy.protocol.toUpperCase()} · {proxy.host}:{proxy.port}
        </p>
      </div>
    </div>
    <Badge variant={proxy.enabled ? "success" : "secondary"}>
      {proxy.enabled ? "已启用" : "未启用"}
    </Badge>
  </div>
</Card>
```

### 5.3 错误/成功提示优化

**RuntimeFeedback 组件美化**：

```tsx
// src/components/RuntimeFeedback.tsx
<div className="rounded-xl border-2 border-destructive/50 bg-gradient-to-br from-destructive/5 to-destructive/10 p-5 backdrop-blur-sm">
  <div className="flex items-start gap-4">
    <div className="rounded-lg bg-destructive/10 p-2.5 ring-1 ring-destructive/20">
      <AlertCircle className="size-5 text-destructive" />
    </div>
    <div className="flex-1 space-y-2">
      <h3 className="font-semibold text-destructive">操作失败</h3>
      <p className="text-sm text-muted-foreground">{error.message}</p>
      <div className="flex gap-2">
        <Button variant="outline" size="sm">
          重试
        </Button>
        <Button variant="ghost" size="sm">
          查看详情
        </Button>
      </div>
    </div>
  </div>
</div>
```

---

## 六、响应式优化

### 6.1 断点策略

```tsx
// 移动优先设计
const breakpoints = {
  sm: "640px", // 手机横屏
  md: "768px", // 平板
  lg: "1024px", // 小桌面
  xl: "1280px", // 大桌面
  "2xl": "1536px", // 超大桌面
};

// 使用示例
<div
  className="
  grid gap-4
  grid-cols-1
  sm:grid-cols-2
  lg:grid-cols-3
  xl:grid-cols-4
"
>
  {/* 卡片内容 */}
</div>;
```

### 6.2 移动端优化

```css
/* 触摸设备优化 */
@media (hover: none) {
  /* 增大点击区域 */
  button,
  a {
    min-height: 44px;
    min-width: 44px;
  }

  /* 移除 hover 效果 */
  .hover\:scale-105:hover {
    transform: none;
  }
}

/* 移动端专属样式 */
@media (max-width: 768px) {
  .mobile-stack {
    @apply flex flex-col space-y-2;
  }

  .mobile-hide {
    @apply hidden;
  }

  .mobile-full {
    @apply w-full;
  }
}
```

---

## 七、深色模式优化

### 7.1 自动切换主题

```tsx
// src/lib/theme.ts (新建)
export function useTheme() {
  const [theme, setTheme] = useState<"light" | "dark" | "system">("system");

  useEffect(() => {
    const root = window.document.documentElement;
    root.classList.remove("light", "dark");

    if (theme === "system") {
      const systemTheme = window.matchMedia("(prefers-color-scheme: dark)").matches
        ? "dark"
        : "light";
      root.classList.add(systemTheme);
    } else {
      root.classList.add(theme);
    }
  }, [theme]);

  return { theme, setTheme };
}
```

### 7.2 深色模式颜色调整

```css
/* 深色模式下降低对比度，更舒适 */
.dark {
  /* 背景层次更丰富 */
  --background: oklch(0.14 0.015 250); /* 更深的主背景 */
  --card: oklch(0.18 0.018 250); /* 卡片背景 */
  --sidebar: oklch(0.12 0.018 250); /* 侧边栏背景 */

  /* 文字对比度优化 */
  --foreground: oklch(0.92 0.01 250); /* 主文字（降低亮度） */
  --muted-foreground: oklch(0.65 0.015 250); /* 次要文字 */

  /* 品牌色在深色模式下更亮 */
  --primary: oklch(0.68 0.22 255);

  /* 边框更柔和 */
  --border: oklch(0.28 0.02 250);
}
```

---

## 八、微交互细节

### 8.1 按钮点击反馈

```css
.button-press {
  @apply active:scale-95 active:brightness-90;
  transition:
    transform 0.1s ease,
    filter 0.1s ease;
}
```

### 8.2 列表项拖拽反馈

```tsx
// 规则列表拖拽时的视觉反馈
<div
  className={cn(
    "rounded-lg border-2 p-4 transition-all",
    isDragging && "opacity-50 scale-95 rotate-2",
    isOver && "border-primary bg-primary/5",
  )}
>
  {/* 规则内容 */}
</div>
```

### 8.3 状态切换动画

```tsx
// Switch 组件增强
<Switch
  className="
    data-[state=checked]:bg-gradient-to-r
    data-[state=checked]:from-success
    data-[state=checked]:to-success/80
    data-[state=checked]:shadow-lg
    data-[state=checked]:shadow-success/30
  "
/>
```

---

## 实施优先级

### 🔴 P0 - 立即实施（1-2天）

1. ✅ 修复底部状态栏遮挡
2. ✅ 色彩系统优化（成功/警告/错误色）
3. ✅ 按钮样式升级（渐变、阴影）
4. ✅ 卡片圆角和阴影优化

### 🟡 P1 - 近期实施（3-5天）

5. ✅ 侧边栏导航美化
6. ✅ 表格样式优化
7. ✅ 输入框增强
8. ✅ 页面过渡动画
9. ✅ 加载状态优化

### 🟢 P2 - 后续优化（1-2周）

10. ✅ 移动端响应式优化
11. ✅ 深色模式精细调整
12. ✅ 微交互细节
13. ✅ 骨架屏和加载动画
14. ✅ 特殊页面定制优化

---

## 验收标准

### 视觉质量

- [ ] 色彩对比度符合 WCAG AA 标准
- [ ] 圆角、间距、字体大小保持一致
- [ ] 深色模式下视觉舒适，无过亮元素
- [ ] 所有交互元素有明显的 hover/active 反馈

### 性能

- [ ] 动画帧率稳定在 60fps
- [ ] 页面切换无明显卡顿
- [ ] CSS 文件大小增加 < 20KB

### 兼容性

- [ ] Windows 10/11 正常显示
- [ ] macOS 正常显示
- [ ] 不同窗口大小下布局自适应
- [ ] 侧边栏折叠/展开流畅

---

## 参考资源

### 设计灵感

- **Vercel Dashboard** - 简洁专业的开发者工具界面
- **Linear** - 优雅的任务管理界面
- **Raycast** - 现代的桌面工具美学
- **Tailwind UI** - 组件设计参考

### 技术文档

- [TailwindCSS v4 文档](https://tailwindcss.com)
- [OKLCH 色彩空间](https://oklch.com)
- [shadcn/ui 组件](https://ui.shadcn.com)
- [Radix UI](https://www.radix-ui.com)

---

**下一步**: 请阅读 `EXECUTION_GUIDE_BEAUTIFY.md` 获取详细的实施步骤。
