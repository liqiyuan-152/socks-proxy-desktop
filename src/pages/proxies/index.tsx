import { useEffect, useRef, useState } from "react";
import { Circle, CircleDot, Edit3, Timer, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Switch } from "@/components/ui/switch";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import {
  command,
  errorMessage,
  type BackendError,
  type ProfileCredential,
  type ProxyProfile,
} from "@/lib/backend";
import { useBackend } from "@/lib/backend-context";
import type { ProxyDraft } from "./ProxyAuthenticationFields";
import { ProxyFormDialog } from "./ProxyFormDialog";
import { ProxyStatus } from "./ProxyStatus";
import { ProxyToolbar } from "./ProxyToolbar";
import { useProxyLatency } from "./useProxyLatency";
import { LatencyCell } from "./LatencyCell";
import { createProxyDraft } from "./proxyDraft";
import { parseProxyLink } from "./parseProxyLink";

export default function ProxyList() {
  const { profiles, snapshot, refresh, loading } = useBackend();
  const [dialogOpen, setDialogOpen] = useState(false);
  const [editingProxy, setEditingProxy] = useState<ProxyProfile | null>(null);
  const [draft, setDraft] = useState<ProxyDraft>(() => createProxyDraft());
  const [proxyLink, setProxyLink] = useState("");
  const [linkError, setLinkError] = useState<string | null>(null);
  const [search, setSearch] = useState("");
  const [protocolFilter, setProtocolFilter] = useState("all");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [credentialLoading, setCredentialLoading] = useState(false);
  const [credentialError, setCredentialError] = useState<string | null>(null);
  const [originalCredential, setOriginalCredential] = useState<ProfileCredential | null>(null);
  const credentialRequest = useRef(0);
  const latency = useProxyLatency(profiles);
  useEffect(
    () => () => {
      credentialRequest.current++;
    },
    [],
  );

  async function loadCredential(id: string) {
    const request = ++credentialRequest.current;
    setCredentialLoading(true);
    setCredentialError(null);
    try {
      const credential = await command<ProfileCredential>("get_profile_credential", { id });
      if (request !== credentialRequest.current) return;
      setOriginalCredential(credential);
      setDraft((current) => ({ ...current, ...credential }));
    } catch (reason) {
      if (request === credentialRequest.current) setCredentialError(errorMessage(reason));
    } finally {
      if (request === credentialRequest.current) setCredentialLoading(false);
    }
  }

  const filtered = profiles.filter(
    (profile) =>
      (protocolFilter === "all" || profile.protocol === protocolFilter) &&
      `${profile.name} ${profile.host}`.toLocaleLowerCase().includes(search.toLocaleLowerCase()),
  );
  async function saveProfile() {
    if (credentialLoading || credentialError || busy) return;
    setBusy(true);
    setError(null);
    try {
      const credential = !draft.authentication
        ? { action: "delete" }
        : originalCredential &&
            draft.username === originalCredential.username &&
            draft.password === originalCredential.password
          ? { action: "preserve" }
          : { action: "replace", username: draft.username, password: draft.password };
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
      if (editingProxy) latency.clear(editingProxy.id);
      closeDialog();
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

  async function changeProfile(
    commandName: "select_profile" | "delete_profile",
    id: string | null,
  ) {
    setBusy(true);
    setError(null);
    try {
      await command(commandName, { id });
      if (commandName === "delete_profile" && id) latency.clear(id);
      await refresh();
    } catch (reason) {
      setError(errorMessage(reason));
    } finally {
      setBusy(false);
    }
  }

  async function changeEnabled(proxy: ProxyProfile, enabled: boolean) {
    setBusy(true);
    setError(null);
    try {
      await command("save_profile", {
        input: {
          id: proxy.id,
          name: proxy.name,
          protocol: proxy.protocol,
          host: proxy.host,
          port: proxy.port,
          authentication_enabled: proxy.authentication_enabled,
          enabled,
          credential: { action: "preserve" },
        },
      });
      latency.clear(proxy.id);
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

  const openDialog = (proxy?: ProxyProfile) => {
    credentialRequest.current++;
    setEditingProxy(proxy ?? null);
    setDraft(createProxyDraft(proxy));
    setOriginalCredential(null);
    setCredentialError(null);
    setCredentialLoading(false);
    setProxyLink("");
    setLinkError(null);
    setDialogOpen(true);
    if (proxy?.authentication_enabled) void loadCredential(proxy.id);
  };

  const closeDialog = () => {
    credentialRequest.current++;
    setDialogOpen(false);
    setEditingProxy(null);
    setOriginalCredential(null);
    setDraft(createProxyDraft());
    setCredentialError(null);
    setCredentialLoading(false);
  };

  const applyProxyLink = () => {
    try {
      setDraft(parseProxyLink(proxyLink));
      setProxyLink("");
      setLinkError(null);
    } catch (reason) {
      setLinkError(reason instanceof Error ? reason.message : "代理链接格式无效");
    }
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
          <ProxyToolbar
            search={search}
            onSearch={setSearch}
            protocolFilter={protocolFilter}
            onProtocolFilter={setProtocolFilter}
            batchPending={latency.batchPending}
            canTest={profiles.some((profile) => profile.enabled)}
            loading={loading}
            busy={busy}
            onTestAll={() => void latency.testAll()}
            onAdd={() => openDialog()}
          />
          {snapshot?.active_profile_id && (
            <Button
              variant="outline"
              size="sm"
              disabled={busy}
              onClick={() => void changeProfile("select_profile", null)}
            >
              取消默认代理
            </Button>
          )}
          {error && (
            <p role="alert" className="text-destructive">
              {error}
            </p>
          )}
          {loading && <p role="status">正在加载代理档案…</p>}

          <Card className="gap-0 overflow-hidden border-border bg-card py-0 shadow-none">
            <Table className="table-fixed text-sm">
              <TableHeader className="bg-muted/50 [&_th]:h-12 [&_th]:px-3 [&_th]:text-xs [&_th]:font-medium [&_th]:text-muted-foreground sm:[&_th]:px-4">
                <TableRow className="hover:bg-transparent">
                  <TableHead className="w-[45%] sm:w-[22%]">名称</TableHead>
                  <TableHead className="hidden w-[12%] sm:table-cell">协议</TableHead>
                  <TableHead className="hidden w-[24%] md:table-cell">服务器</TableHead>
                  <TableHead className="hidden w-[10%] md:table-cell">端口</TableHead>
                  <TableHead className="w-[25%] sm:w-[16%]">延迟</TableHead>
                  <TableHead className="hidden w-[13%] lg:table-cell">认证</TableHead>
                  <TableHead className="hidden w-[15%] lg:table-cell">状态</TableHead>
                  <TableHead className="w-[30%] text-right sm:w-44">操作</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {filtered.map((proxy) => {
                  const active = snapshot?.active_profile_id === proxy.id;
                  const SelectionIcon = active ? CircleDot : Circle;

                  return (
                    <TableRow key={proxy.id}>
                      <TableCell className="min-w-0 px-3 py-4 font-medium sm:px-4">
                        <span className="flex items-center gap-3 truncate">
                          <SelectionIcon
                            className={
                              active
                                ? "size-5 shrink-0 text-blue-500"
                                : "size-5 shrink-0 text-muted-foreground"
                            }
                            aria-label={active ? "默认代理" : "非默认代理"}
                          />
                          <span className="truncate">{proxy.name}</span>
                        </span>
                        <div className="mt-1 pl-8 lg:hidden">
                          <div className="flex items-center gap-2">
                            <Switch
                              checked={proxy.enabled}
                              disabled={busy}
                              onCheckedChange={(enabled) => void changeEnabled(proxy, enabled)}
                              aria-label={`${proxy.name}启用状态`}
                            />
                            <ProxyStatus active={active} enabled={proxy.enabled} />
                          </div>
                        </div>
                      </TableCell>
                      <TableCell className="hidden px-3 py-4 font-medium sm:table-cell sm:px-4">
                        {proxy.protocol.toUpperCase()}
                      </TableCell>
                      <TableCell className="hidden truncate px-3 py-4 text-muted-foreground md:table-cell sm:px-4">
                        {proxy.host}
                      </TableCell>
                      <TableCell className="hidden px-3 py-4 text-muted-foreground md:table-cell sm:px-4">
                        {proxy.port}
                      </TableCell>
                      <TableCell className="px-2 py-4 sm:px-3">
                        <LatencyCell result={latency.results[proxy.id]} />
                      </TableCell>
                      <TableCell className="hidden px-4 py-4 lg:table-cell">
                        {proxy.authentication_enabled ? "已配置" : "未启用"}
                      </TableCell>
                      <TableCell className="hidden px-4 py-4 font-medium lg:table-cell">
                        <div className="flex items-center gap-2">
                          <Switch
                            checked={proxy.enabled}
                            disabled={busy}
                            onCheckedChange={(enabled) => void changeEnabled(proxy, enabled)}
                            aria-label={`${proxy.name}启用状态`}
                          />
                          <ProxyStatus active={active} enabled={proxy.enabled} />
                        </div>
                      </TableCell>
                      <TableCell className="px-2 py-4 sm:px-4">
                        <div className="grid grid-cols-2 justify-items-end gap-1 sm:flex sm:justify-end">
                          {!active && (
                            <Button
                              variant="ghost"
                              size="icon-sm"
                              disabled={busy || !proxy.enabled}
                              aria-label={`设${proxy.name}为默认代理`}
                              onClick={() => void changeProfile("select_profile", proxy.id)}
                            >
                              <CircleDot className="size-4" aria-hidden="true" />
                            </Button>
                          )}
                          <Button
                            variant="ghost"
                            size="icon-sm"
                            aria-label={`测试${proxy.name}延迟`}
                            title={`测试${proxy.name}延迟`}
                            disabled={
                              !proxy.enabled ||
                              !!latency.results[proxy.id]?.pending ||
                              latency.batchPending
                            }
                            onClick={() => void latency.test(proxy.id)}
                          >
                            <Timer className="size-4" aria-hidden="true" />
                          </Button>
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
                            className="text-destructive hover:bg-destructive/10 hover:text-destructive"
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
                    <TableCell colSpan={8} className="text-center text-muted-foreground">
                      暂无匹配的代理档案
                    </TableCell>
                  </TableRow>
                )}
              </TableBody>
            </Table>
          </Card>
        </div>
      </div>

      <ProxyFormDialog
        open={dialogOpen}
        onOpenChange={(open) => {
          if (!open) closeDialog();
        }}
        isEditing={isEditing}
        busy={busy || credentialLoading || !!credentialError}
        credentialLoading={credentialLoading}
        credentialError={credentialError}
        onRetryCredential={() => {
          if (editingProxy) void loadCredential(editingProxy.id);
        }}
        draft={draft}
        setDraft={setDraft}
        proxyLink={proxyLink}
        linkError={linkError}
        onLinkChange={(value) => {
          setProxyLink(value);
          setLinkError(null);
        }}
        onParse={applyProxyLink}
        onSave={() => void saveProfile()}
      />
    </>
  );
}
