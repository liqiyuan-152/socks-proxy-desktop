import { appVersion } from "@/lib/app-version";
import { useEffect } from "react";
import { scheduleStartupReady } from "@/lib/startup-ready";
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
import { useBackendStore } from "@/store/backend-store";
import { useShallow } from "zustand/react/shallow";
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
      className={`size-2.5 rounded-full ${running ? "bg-success" : "bg-muted-foreground"}`}
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
        className="h-11 rounded-lg text-sm font-medium text-sidebar-foreground transition-[background-color,box-shadow,scale] duration-200 motion-reduce:transition-none hover:bg-sidebar-accent hover:text-sidebar-accent-foreground motion-safe:hover:scale-[1.02] data-[active=true]:bg-[image:var(--gradient-primary)] data-[active=true]:text-primary-foreground data-[active=true]:shadow-md data-[active=true]:shadow-primary/25"
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
  const { snapshot, profiles, error } = useBackendStore(
    useShallow((state) => ({
      snapshot: state.snapshot,
      profiles: state.profiles,
      error: state.error,
    })),
  );
  const loading = useBackendStore((state) => state.loading);
  const ready = !loading && snapshot !== null;
  useEffect(() => {
    if (ready) return scheduleStartupReady();
  }, [ready]);
  const running = snapshot?.session_health === "healthy";
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
            <div className="grid size-9 shrink-0 place-items-center rounded-xl bg-[image:var(--gradient-primary)] text-primary-foreground shadow-lg shadow-primary/25 ring-2 ring-primary/20">
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
          {(error ||
            snapshot?.last_operation.outcome === "failed" ||
            snapshot?.session_health === "recovery_required") && (
            <NavLink to="/" className="text-destructive group-data-[collapsible=icon]:hidden">
              {snapshot?.session_health === "recovery_required" ? "恢复未完成" : "最近操作失败"} ·{" "}
              {activeModeLabel} · 查看原因
            </NavLink>
          )}
          <p className="mt-3 group-data-[collapsible=icon]:hidden">v{appVersion}</p>
        </SidebarFooter>
        <SidebarRail />
      </Sidebar>

      <SidebarInset className="min-w-0 bg-background">
        <div className="flex min-h-0 min-w-0 flex-1 flex-col">
          <div className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden">
            <Outlet />
          </div>
          <footer className="hidden shrink-0 items-center gap-4 border-t border-sidebar-border/80 bg-gradient-to-r from-sidebar via-sidebar to-sidebar-accent/30 px-6 py-3.5 backdrop-blur-sm text-xs text-muted-foreground lg:flex">
            <span>
              当前模式：
              <strong className="rounded-md bg-primary/10 px-2 py-0.5 font-semibold text-primary ring-1 ring-primary/20">
                {activeModeLabel}
              </strong>
            </span>
            <Separator orientation="vertical" className="h-4" />
            <span>
              当前代理名称：<strong className="font-semibold text-foreground">{profileName}</strong>
            </span>
            <Separator orientation="vertical" className="h-4" />
            <span className="flex items-center gap-2">
              <Activity className="size-3.5" aria-hidden="true" />
              内核运行状态：
              <span className={running ? "font-medium text-success" : "text-muted-foreground"}>
                {snapshot ? (running ? "健康" : "未运行") : "不可用"}
              </span>
            </span>
            <span className="ml-auto flex items-center gap-2">
              <Globe2 className="size-3.5" aria-hidden="true" />
              {snapshot?.coverage === "system_proxy_apps" ? "系统代理应用流量" : "未接管系统代理"}
            </span>
          </footer>
        </div>
      </SidebarInset>
    </SidebarProvider>
  );
}
