import {
  Activity,
  FileText,
  GitBranch,
  Globe2,
  Home,
  Server,
  Settings,
  ShieldCheck,
} from "lucide-react";
import { useBackend } from "@/lib/backend-context";
import { proxyModes } from "@/lib/proxy-mode";
import { NavLink, Outlet, useMatch } from "react-router-dom";
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupContent,
  SidebarHeader,
  SidebarInset,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarProvider,
  SidebarRail,
} from "@/components/ui/sidebar";
import { Separator } from "@/components/ui/separator";

const navigationItems = [
  { label: "状态", icon: Home, to: "/" },
  { label: "代理", icon: Server, to: "/proxies" },
  { label: "分流规则", icon: GitBranch, to: "/rules" },
  { label: "连接日志", icon: FileText, to: "/logs" },
  { label: "设置", icon: Settings, to: "/settings" },
];

type NavigationItem = (typeof navigationItems)[number];

function StatusDot({ running }: { running: boolean }) {
  return (
    <span
      aria-hidden="true"
      className={`size-2.5 rounded-full ${running ? "bg-emerald-500" : "bg-muted-foreground"}`}
    />
  );
}

function SidebarNavigationItem({ label, icon: Icon, to }: NavigationItem) {
  const isActive = useMatch({ path: to, end: to === "/" }) !== null;

  return (
    <SidebarMenuItem>
      <SidebarMenuButton
        asChild
        isActive={isActive}
        tooltip={label}
        className="h-10 text-sm text-sidebar-foreground/70 hover:bg-sidebar-accent hover:text-sidebar-accent-foreground data-[active=true]:bg-blue-500/15 data-[active=true]:text-blue-700 data-[active=true]:hover:bg-blue-500/20 data-[active=true]:hover:text-blue-800 dark:data-[active=true]:bg-blue-500/20 dark:data-[active=true]:text-blue-300 dark:data-[active=true]:hover:bg-blue-500/25 dark:data-[active=true]:hover:text-blue-200"
      >
        <NavLink to={to} end={to === "/"}>
          <Icon className="size-5" aria-hidden="true" />
          <span>{label}</span>
        </NavLink>
      </SidebarMenuButton>
    </SidebarMenuItem>
  );
}

export function AppLayout() {
  const { snapshot, profiles } = useBackend();
  const running = snapshot?.phase === "running";
  const activeModeLabel = snapshot?.applied_mode
    ? proxyModes[snapshot.applied_mode].label
    : "未应用";
  const profileName =
    profiles.find((profile) => profile.id === snapshot?.active_profile_id)?.name ?? "未选择";
  return (
    <SidebarProvider className="h-svh overflow-hidden bg-background text-foreground">
      <Sidebar collapsible="icon" className="border-sidebar-border">
        <SidebarHeader className="p-3">
          <div className="flex h-10 items-center gap-2 px-1">
            <div className="grid size-9 shrink-0 place-items-center rounded-md bg-blue-600 text-white shadow-sm shadow-blue-600/30">
              <ShieldCheck className="size-[18px]" aria-hidden="true" />
            </div>
            <span className="truncate text-base font-semibold group-data-[collapsible=icon]:hidden">
              Socks Proxy
            </span>
          </div>
        </SidebarHeader>

        <SidebarContent>
          <SidebarGroup>
            <SidebarGroupContent>
              <SidebarMenu aria-label="主导航">
                {navigationItems.map((item) => (
                  <SidebarNavigationItem key={item.label} {...item} />
                ))}
              </SidebarMenu>
            </SidebarGroupContent>
          </SidebarGroup>
        </SidebarContent>

        <SidebarFooter className="p-3 text-xs text-sidebar-foreground/70">
          <div className="flex items-center gap-2 font-medium group-data-[collapsible=icon]:justify-center">
            <StatusDot running={running} />
            <span className="group-data-[collapsible=icon]:hidden">
              内核{running ? "运行中" : "未运行"}
            </span>
          </div>
          <p className="mt-3 group-data-[collapsible=icon]:hidden">v0.1.0</p>
        </SidebarFooter>
        <SidebarRail />
      </Sidebar>

      <SidebarInset className="min-w-0 bg-background">
        <section className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden">
          <Outlet />
          <footer className="hidden items-center gap-4 px-6 py-3 text-xs text-muted-foreground lg:flex">
            <span>
              当前模式：<strong className="font-semibold text-primary">{activeModeLabel}</strong>
            </span>
            <Separator orientation="vertical" className="h-4" />
            <span>
              当前代理名称：<strong className="font-semibold text-foreground">{profileName}</strong>
            </span>
            <Separator orientation="vertical" className="h-4" />
            <span className="flex items-center gap-2">
              <Activity className="size-3.5" aria-hidden="true" />
              内核运行状态：{snapshot?.phase ?? "不可用"}
            </span>
            <span className="ml-auto flex items-center gap-2">
              <Globe2 className="size-3.5" aria-hidden="true" />
              {snapshot?.coverage === "system_proxy_apps" ? "系统代理应用流量" : "未接管系统代理"}
            </span>
          </footer>
        </section>
      </SidebarInset>
    </SidebarProvider>
  );
}
