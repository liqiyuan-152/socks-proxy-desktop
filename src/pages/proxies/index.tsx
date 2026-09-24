import { useState } from "react";
import { Circle, CircleDot, Edit3, Plus, Search, Trash2, X } from "lucide-react";
import { Field } from "@/components/forms/Field";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { command, errorMessage, type BackendError, type ProxyProfile } from "@/lib/backend";
import { useBackend } from "@/lib/backend-context";
import { ProxyAuthenticationFields, type ProxyDraft } from "./ProxyAuthenticationFields";

function createProxyDraft(proxy?: ProxyProfile): ProxyDraft {
  const authentication = proxy?.authentication_enabled ?? false;

  return {
    name: proxy?.name ?? "",
    protocol: proxy?.protocol ?? "socks5",
    server: proxy?.host ?? "",
    port: proxy?.port?.toString() ?? "",
    authentication,
    username: "",
    password: "",
  };
}

export default function ProxyList() {
  const { profiles, snapshot, refresh, loading } = useBackend();
  const [dialogOpen, setDialogOpen] = useState(false);
  const [editingProxy, setEditingProxy] = useState<ProxyProfile | null>(null);
  const [draft, setDraft] = useState<ProxyDraft>(() => createProxyDraft());
  const [search, setSearch] = useState("");
  const [protocolFilter, setProtocolFilter] = useState("all");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const filtered = profiles.filter(
    (profile) =>
      (protocolFilter === "all" || profile.protocol === protocolFilter) &&
      `${profile.name} ${profile.host}`.toLocaleLowerCase().includes(search.toLocaleLowerCase()),
  );

  async function saveProfile() {
    setBusy(true);
    setError(null);
    try {
      const credential = !draft.authentication
        ? { action: "delete" }
        : draft.password
          ? { action: "replace", username: draft.username, password: draft.password }
          : { action: "preserve" };
      await command("save_profile", {
        input: {
          id: editingProxy?.id ?? null,
          name: draft.name,
          protocol: draft.protocol,
          host: draft.server,
          port: Number(draft.port),
          authentication_enabled: draft.authentication,
          enabled: editingProxy?.enabled ?? true,
          credential,
        },
      });
      setDialogOpen(false);
      await refresh();
    } catch (reason) {
      const typed = reason as Partial<BackendError>;
      setError(
        typed.fields?.map((field) => `${field.field}: ${field.message}`).join("；") ||
          errorMessage(reason),
      );
    } finally {
      setBusy(false);
    }
  }

  async function changeProfile(commandName: "select_profile" | "delete_profile", id: string) {
    setBusy(true);
    setError(null);
    try {
      await command(commandName, { id });
      await refresh();
    } catch (reason) {
      setError(errorMessage(reason));
    } finally {
      setBusy(false);
    }
  }

  const openDialog = (proxy?: ProxyProfile) => {
    setEditingProxy(proxy ?? null);
    setDraft(createProxyDraft(proxy));
    setDialogOpen(true);
  };

  const isEditing = editingProxy !== null;

  return (
    <>
      <header className="border-b border-sidebar-border bg-sidebar px-5 py-4 text-sidebar-foreground sm:px-6">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">代理</h1>
          <p className="mt-1 text-sm text-muted-foreground">管理多个 SOCKS5 或 HTTP 代理</p>
        </div>
      </header>

      <div className="content-scroll min-h-0 flex-1 overflow-y-auto px-5 py-5 sm:px-6">
        <div className="w-full space-y-5">
          <div className="flex flex-col gap-3 sm:flex-row">
            <div className="relative flex-1">
              <Search
                className="pointer-events-none absolute top-1/2 left-3 size-5 -translate-y-1/2 text-muted-foreground"
                aria-hidden="true"
              />
              <Input
                className="h-11 bg-card pl-10"
                placeholder="搜索代理名称、服务器地址..."
                value={search}
                onChange={(event) => setSearch(event.target.value)}
              />
            </div>
            <Select value={protocolFilter} onValueChange={setProtocolFilter}>
              <SelectTrigger aria-label="按协议筛选" className="h-11 bg-card sm:w-44">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="all">全部协议</SelectItem>
                <SelectItem value="socks5">SOCKS5</SelectItem>
                <SelectItem value="http">HTTP</SelectItem>
              </SelectContent>
            </Select>
            <Button
              className="h-11 sm:min-w-36"
              onClick={() => openDialog()}
              disabled={loading || busy}
            >
              <Plus className="size-5" aria-hidden="true" />
              添加代理
            </Button>
          </div>
          {error && (
            <p role="alert" className="text-destructive">
              {error}
            </p>
          )}
          {loading && <p role="status">正在加载代理档案…</p>}

          <Card className="gap-0 overflow-hidden border-white/10 bg-card py-0 shadow-none">
            <Table className="table-fixed text-sm">
              <TableHeader className="border-white/10 bg-white/[0.04] [&_th]:h-12 [&_th]:px-3 [&_th]:text-xs [&_th]:font-medium [&_th]:text-muted-foreground sm:[&_th]:px-4">
                <TableRow className="border-white/10 hover:bg-transparent">
                  <TableHead className="w-[28%]">名称</TableHead>
                  <TableHead className="w-[18%]">协议</TableHead>
                  <TableHead className="w-[32%]">服务器</TableHead>
                  <TableHead className="w-[22%]">端口</TableHead>
                  <TableHead className="hidden w-[13%] lg:table-cell">认证</TableHead>
                  <TableHead className="hidden w-[15%] lg:table-cell">状态</TableHead>
                  <TableHead className="hidden w-24 text-right lg:table-cell">操作</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {filtered.map((proxy) => {
                  const active = snapshot?.active_profile_id === proxy.id;
                  const SelectionIcon = active ? CircleDot : Circle;

                  return (
                    <TableRow key={proxy.id} className="border-white/10 hover:bg-white/[0.035]">
                      <TableCell className="px-3 py-4 font-medium sm:px-4">
                        <span className="flex items-center gap-3 truncate">
                          <SelectionIcon
                            className={
                              active
                                ? "size-5 shrink-0 text-blue-500"
                                : "size-5 shrink-0 text-muted-foreground"
                            }
                            aria-label={active ? "当前使用的代理" : "未选中的代理"}
                          />
                          <span className="truncate">{proxy.name}</span>
                        </span>
                      </TableCell>
                      <TableCell className="px-3 py-4 font-medium sm:px-4">
                        {proxy.protocol.toUpperCase()}
                      </TableCell>
                      <TableCell className="truncate px-3 py-4 text-muted-foreground sm:px-4">
                        {proxy.host}
                      </TableCell>
                      <TableCell className="px-3 py-4 text-muted-foreground sm:px-4">
                        {proxy.port}
                      </TableCell>
                      <TableCell className="hidden px-4 py-4 lg:table-cell">
                        {proxy.authentication_enabled ? "已配置" : "未启用"}
                      </TableCell>
                      <TableCell className="hidden px-4 py-4 font-medium lg:table-cell">
                        {active ? "当前选择" : proxy.enabled ? "已启用" : "已停用"}
                      </TableCell>
                      <TableCell className="hidden px-4 py-4 lg:table-cell">
                        <div className="flex justify-end gap-1">
                          {!active && (
                            <Button
                              variant="ghost"
                              size="icon-sm"
                              disabled={busy}
                              aria-label={`选择${proxy.name}`}
                              onClick={() => void changeProfile("select_profile", proxy.id)}
                            >
                              <CircleDot className="size-4" aria-hidden="true" />
                            </Button>
                          )}
                          <Button
                            variant="ghost"
                            size="icon-sm"
                            aria-label={`编辑${proxy.name}`}
                            disabled={busy}
                            onClick={() => openDialog(proxy)}
                          >
                            <Edit3 className="size-4" aria-hidden="true" />
                          </Button>
                          <Button
                            variant="ghost"
                            size="icon-sm"
                            className="text-rose-400 hover:bg-rose-500/10 hover:text-rose-300"
                            aria-label={`删除${proxy.name}`}
                            disabled={busy}
                            onClick={() => {
                              if (window.confirm(`确认删除代理「${proxy.name}」？`)) {
                                void changeProfile("delete_profile", proxy.id);
                              }
                            }}
                          >
                            <Trash2 className="size-4" aria-hidden="true" />
                          </Button>
                        </div>
                      </TableCell>
                    </TableRow>
                  );
                })}
                {!loading && filtered.length === 0 && (
                  <TableRow>
                    <TableCell colSpan={7} className="text-center text-muted-foreground">
                      暂无匹配的代理档案
                    </TableCell>
                  </TableRow>
                )}
              </TableBody>
            </Table>
          </Card>
        </div>
      </div>

      <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
        <DialogContent className="max-h-[calc(100vh-2rem)] p-0 sm:max-h-[680px]">
          <form
            onSubmit={(event) => {
              event.preventDefault();
              void saveProfile();
            }}
          >
            <DialogHeader className="relative border-b border-white/10 px-6 py-5 sm:px-7">
              <DialogTitle>{isEditing ? "编辑代理" : "添加代理"}</DialogTitle>
              <DialogClose asChild>
                <Button
                  variant="ghost"
                  size="icon-sm"
                  className="absolute top-3 right-4 text-muted-foreground hover:text-foreground"
                  aria-label="关闭代理表单"
                >
                  <X className="size-5" aria-hidden="true" />
                </Button>
              </DialogClose>
            </DialogHeader>

            <div className="space-y-5 px-6 py-6 sm:px-7">
              <div className="grid gap-5 sm:grid-cols-[1.1fr_0.9fr]">
                <Field label="名称" required htmlFor="proxy-name">
                  <Input
                    id="proxy-name"
                    placeholder="请输入代理名称"
                    value={draft.name}
                    onChange={(event) =>
                      setDraft((current) => ({ ...current, name: event.target.value }))
                    }
                  />
                </Field>
                <Field label="协议" required htmlFor="proxy-protocol">
                  <Select
                    value={draft.protocol}
                    onValueChange={(protocol) => setDraft((current) => ({ ...current, protocol }))}
                  >
                    <SelectTrigger id="proxy-protocol">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="socks5">SOCKS5</SelectItem>
                      <SelectItem value="http">HTTP</SelectItem>
                    </SelectContent>
                  </Select>
                </Field>
              </div>

              <div className="grid gap-5 sm:grid-cols-[1.7fr_0.8fr]">
                <Field label="服务器" required htmlFor="proxy-server">
                  <Input
                    id="proxy-server"
                    placeholder="例如：proxy.example.com"
                    value={draft.server}
                    onChange={(event) =>
                      setDraft((current) => ({ ...current, server: event.target.value }))
                    }
                  />
                </Field>
                <Field label="端口" required htmlFor="proxy-port">
                  <Input
                    id="proxy-port"
                    inputMode="numeric"
                    placeholder="例如：1080"
                    value={draft.port}
                    onChange={(event) =>
                      setDraft((current) => ({ ...current, port: event.target.value }))
                    }
                  />
                </Field>
              </div>

              <ProxyAuthenticationFields draft={draft} setDraft={setDraft} />
            </div>

            <DialogFooter className="border-t border-white/10 px-6 py-5 sm:px-7">
              <DialogClose asChild>
                <Button type="button" variant="secondary" className="min-w-28">
                  取消
                </Button>
              </DialogClose>
              <Button type="submit" className="min-w-28" disabled={busy}>
                保存
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>
    </>
  );
}
