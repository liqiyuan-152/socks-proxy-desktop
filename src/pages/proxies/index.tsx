import { ProxyAction } from "./ProxyAction";
import { PageHeader } from "@/components/PageHeader";
import { ErrorAlert } from "@/components/ErrorAlert";
import { normalizeError } from "@/lib/error-handler";
import type { AppError } from "@/lib/generated/ipc";
import { useState } from "react";
import { Circle, CircleDot, Edit3, Timer, Trash2, Server, SearchX } from "lucide-react";
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
import { type ProxyProfile } from "@/lib/backend";
import { useBackendStore } from "@/store/backend-store";
import { useShallow } from "zustand/react/shallow";
import { ProxyFormDialog } from "./ProxyFormDialog";
import { ProxyStatus } from "./ProxyStatus";
import { ProxyToolbar } from "./ProxyToolbar";
import { useProxyLatency } from "./useProxyLatency";
import { LatencyCell } from "./LatencyCell";
import { useProxyEditor } from "./useProxyEditor";
import { ConfirmDeletionDialog } from "@/components/ConfirmDeletionDialog";

export default function ProxyList() {
  const {
    profiles,
    snapshot,
    loading,
    capabilities,
    deleteProfile,
    selectProfile,
    saveProfile: persistProfile,
  } = useBackendStore(
    useShallow((state) => ({
      profiles: state.profiles,
      snapshot: state.snapshot,
      loading: state.loading,
      capabilities: state.capabilities,
      deleteProfile: state.deleteProfile,
      selectProfile: state.selectProfile,
      saveProfile: state.saveProfile,
    })),
  );
  const [search, setSearch] = useState("");
  const [protocolFilter, setProtocolFilter] = useState("all");
  const [error, setError] = useState<AppError | null>(null);
  const [busy, setBusy] = useState(false);
  const [deletingProxy, setDeletingProxy] = useState<ProxyProfile | null>(null);
  const latency = useProxyLatency(profiles, capabilities?.proxy_latency ?? false);
  const {
    dialogOpen,
    editingProxy,
    draft,
    setDraft,
    proxyLink,
    setProxyLink,
    linkError,
    setLinkError,
    credentialLoading,
    credentialError,
    loadCredential,
    openDialog,
    closeDialog,
    applyProxyLink,
    saveProfile,
  } = useProxyEditor({
    busy,
    setBusy,
    setError,
    persistProfile,
    clearLatency: latency.clear,
  });

  const filtered = profiles.filter(
    (profile) =>
      (protocolFilter === "all" || profile.protocol === protocolFilter) &&
      `${profile.name} ${profile.host}`.toLocaleLowerCase().includes(search.toLocaleLowerCase()),
  );
  async function changeProfile(
    commandName: "select_profile" | "delete_profile",
    id: string | null,
  ) {
    setBusy(true);
    setError(null);
    try {
      if (commandName === "delete_profile") {
        if (id === null) return false;
        await deleteProfile(id);
      } else {
        await selectProfile(id);
      }
      if (commandName === "delete_profile" && id) latency.clear(id);
      return true;
    } catch (reason) {
      setError(normalizeError(reason));
      return false;
    } finally {
      setBusy(false);
    }
  }

  async function changeEnabled(proxy: ProxyProfile, enabled: boolean) {
    setBusy(true);
    setError(null);
    try {
      await persistProfile({
        id: proxy.id,
        name: proxy.name,
        protocol: proxy.protocol,
        host: proxy.host,
        port: proxy.port,
        authentication_enabled: proxy.authentication_enabled,
        enabled,
        credential: { action: "preserve" },
      });
      latency.clear(proxy.id);
    } catch (reason) {
      setError(normalizeError(reason));
    } finally {
      setBusy(false);
    }
  }

  const isEditing = editingProxy !== null;
  return (
    <>
      <PageHeader title="代理" description="管理多个 SOCKS5 或 HTTP 代理" />

      <div className="content-scroll animate-fade-in min-h-0 flex-1 overflow-y-auto px-4 py-4 sm:px-6 sm:py-6">
        <div className="mx-auto w-full max-w-7xl space-y-6">
          <ProxyToolbar
            search={search}
            onSearch={setSearch}
            protocolFilter={protocolFilter}
            onProtocolFilter={setProtocolFilter}
            batchPending={latency.batchPending}
            canTest={!!capabilities?.proxy_latency && profiles.some((profile) => profile.enabled)}
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
          {error && !dialogOpen && !deletingProxy && <ErrorAlert error={error} />}
          {loading && <p role="status">正在加载代理档案…</p>}

          <Card variant="elevated" className="gap-0 overflow-hidden py-0">
            <Table
              className="table-modern table-fixed text-sm"
              containerClassName="max-h-[60vh] overflow-auto"
            >
              <TableHeader className="bg-muted/50 [&_th]:h-12 [&_th]:px-3 [&_th]:text-xs [&_th]:font-medium [&_th]:text-muted-foreground sm:[&_th]:px-4">
                <TableRow className="hover:bg-transparent">
                  <TableHead className="w-[40%] sm:w-[22%]">名称</TableHead>
                  <TableHead className="hidden w-[12%] sm:table-cell">协议</TableHead>
                  <TableHead className="hidden w-[24%] md:table-cell">服务器</TableHead>
                  <TableHead className="hidden w-[10%] md:table-cell">端口</TableHead>
                  <TableHead className="w-[20%] sm:w-[16%]">延迟</TableHead>
                  <TableHead className="hidden w-[13%] lg:table-cell">认证</TableHead>
                  <TableHead className="hidden w-[15%] lg:table-cell">状态</TableHead>
                  <TableHead className="w-[40%] text-right sm:w-44">操作</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {filtered.map((proxy) => {
                  const active = snapshot?.active_profile_id === proxy.id;
                  const SelectionIcon = active ? CircleDot : Circle;

                  return (
                    <TableRow key={proxy.id} data-selected={active}>
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
                        <p
                          className="mt-1 truncate pl-8 text-xs text-muted-foreground md:hidden"
                          title={`${proxy.host}:${proxy.port}`}
                        >
                          {proxy.protocol.toUpperCase()} · {proxy.host}:{proxy.port}
                        </p>
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
                            <ProxyAction
                              variant="ghost"
                              size="icon-sm"
                              disabled={busy || !proxy.enabled}
                              aria-label={`设${proxy.name}为默认代理`}
                              onClick={() => void changeProfile("select_profile", proxy.id)}
                            >
                              <CircleDot className="size-4" aria-hidden="true" />
                            </ProxyAction>
                          )}
                          <ProxyAction
                            variant="ghost"
                            size="icon-sm"
                            aria-label={`测试${proxy.name}延迟`}
                            title={`测试${proxy.name}延迟`}
                            disabled={
                              !capabilities?.proxy_latency ||
                              !proxy.enabled ||
                              !!latency.results[proxy.id]?.pending ||
                              latency.batchPending
                            }
                            onClick={() => void latency.test(proxy.id)}
                          >
                            <Timer className="size-4" aria-hidden="true" />
                          </ProxyAction>
                          <ProxyAction
                            variant="ghost"
                            size="icon-sm"
                            aria-label={`编辑${proxy.name}`}
                            disabled={busy}
                            onClick={() => openDialog(proxy)}
                          >
                            <Edit3 className="size-4" aria-hidden="true" />
                          </ProxyAction>
                          <ProxyAction
                            variant="ghost"
                            size="icon-sm"
                            className="text-destructive hover:bg-destructive/10 hover:text-destructive"
                            aria-label={`删除${proxy.name}`}
                            disabled={busy}
                            onClick={() => {
                              setError(null);
                              setDeletingProxy(proxy);
                            }}
                          >
                            <Trash2 className="size-4" aria-hidden="true" />
                          </ProxyAction>
                        </div>
                      </TableCell>
                    </TableRow>
                  );
                })}
                {!loading && filtered.length === 0 && (
                  <TableRow>
                    <TableCell colSpan={8} className="text-center text-muted-foreground">
                      <div className="flex flex-col items-center gap-3 px-3 py-8">
                        {profiles.length ? (
                          <SearchX className="size-9 text-muted-foreground" aria-hidden="true" />
                        ) : (
                          <Server className="size-9 text-primary-text" aria-hidden="true" />
                        )}
                        <p className="font-medium text-foreground">
                          {profiles.length ? "暂无匹配的代理档案" : "还没有代理档案"}
                        </p>
                        <p className="text-sm">
                          {profiles.length
                            ? "试试其他关键词或协议筛选。"
                            : "添加一个 SOCKS5 或 HTTP 代理，开始配置你的网络出口。"}
                        </p>
                        {!profiles.length && (
                          <Button
                            variant="gradient"
                            onClick={() => openDialog()}
                            disabled={loading || busy}
                          >
                            添加第一个代理
                          </Button>
                        )}
                      </div>
                    </TableCell>
                  </TableRow>
                )}
              </TableBody>
            </Table>
          </Card>
        </div>
      </div>

      {deletingProxy && (
        <ConfirmDeletionDialog
          name={deletingProxy.name}
          resource="代理"
          error={error}
          onCancel={() => setDeletingProxy(null)}
          onConfirm={() => changeProfile("delete_profile", deletingProxy.id)}
        />
      )}
      <ProxyFormDialog
        open={dialogOpen}
        onOpenChange={(open) => {
          if (!open) closeDialog();
        }}
        isEditing={isEditing}
        busy={busy || credentialLoading || !!credentialError}
        error={error}
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
