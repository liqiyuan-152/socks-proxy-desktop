import { useState } from "react";
import { Circle, CircleDot, Edit3, Eye, EyeOff, Plus, Search, Trash2, X } from "lucide-react";
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
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
const proxies = [
  {
    name: "公司代理",
    protocol: "SOCKS5",
    server: "proxy.example.com",
    port: "1080",
    authentication: "已启用",
    status: "当前使用",
    active: true,
  },
  {
    name: "测试代理",
    protocol: "HTTP",
    server: "198.51.100.10",
    port: "8080",
    authentication: "未启用",
    status: "可用",
  },
  {
    name: "备用代理",
    protocol: "SOCKS5",
    server: "203.0.113.5",
    port: "1080",
    authentication: "已启用",
    status: "可用",
  },
  {
    name: "海外代理",
    protocol: "HTTP",
    server: "us.example.com",
    port: "3128",
    authentication: "未启用",
    status: "连接失败",
  },
  {
    name: "临时代理",
    protocol: "SOCKS5",
    server: "192.0.2.1",
    port: "1080",
    authentication: "已启用",
    status: "可用",
  },
];

type Proxy = (typeof proxies)[number];

type ProxyDraft = {
  name: string;
  protocol: string;
  server: string;
  port: string;
  authentication: boolean;
  username: string;
  password: string;
};

function createProxyDraft(proxy?: Proxy): ProxyDraft {
  const authentication = proxy?.authentication === "已启用";

  return {
    name: proxy?.name ?? "",
    protocol: proxy?.protocol ?? "SOCKS5",
    server: proxy?.server ?? "",
    port: proxy?.port ?? "",
    authentication,
    username: authentication ? "user123" : "",
    password: authentication ? "password123" : "",
  };
}

export default function ProxyList() {
  const [dialogOpen, setDialogOpen] = useState(false);
  const [editingProxy, setEditingProxy] = useState<Proxy | null>(null);
  const [draft, setDraft] = useState<ProxyDraft>(() => createProxyDraft());
  const [passwordVisible, setPasswordVisible] = useState(false);

  const openDialog = (proxy?: Proxy) => {
    setEditingProxy(proxy ?? null);
    setDraft(createProxyDraft(proxy));
    setPasswordVisible(false);
    setDialogOpen(true);
  };

  const closeDialog = () => setDialogOpen(false);
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
              <Input className="h-11 bg-card pl-10" placeholder="搜索代理名称、服务器地址..." />
            </div>
            <Select defaultValue="all">
              <SelectTrigger aria-label="按协议筛选" className="h-11 bg-card sm:w-44">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="all">全部协议</SelectItem>
                <SelectItem value="socks5">SOCKS5</SelectItem>
                <SelectItem value="http">HTTP</SelectItem>
              </SelectContent>
            </Select>
            <Button className="h-11 sm:min-w-36" onClick={() => openDialog()}>
              <Plus className="size-5" aria-hidden="true" />
              添加代理
            </Button>
          </div>

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
                {proxies.map((proxy) => {
                  const SelectionIcon = proxy.active ? CircleDot : Circle;

                  return (
                    <TableRow key={proxy.name} className="border-white/10 hover:bg-white/[0.035]">
                      <TableCell className="px-3 py-4 font-medium sm:px-4">
                        <span className="flex items-center gap-3 truncate">
                          <SelectionIcon
                            className={
                              proxy.active
                                ? "size-5 shrink-0 text-blue-500"
                                : "size-5 shrink-0 text-muted-foreground"
                            }
                            aria-label={proxy.active ? "当前使用的代理" : "未选中的代理"}
                          />
                          <span className="truncate">{proxy.name}</span>
                        </span>
                      </TableCell>
                      <TableCell className="px-3 py-4 font-medium sm:px-4">
                        {proxy.protocol}
                      </TableCell>
                      <TableCell className="truncate px-3 py-4 text-muted-foreground sm:px-4">
                        {proxy.server}
                      </TableCell>
                      <TableCell className="px-3 py-4 text-muted-foreground sm:px-4">
                        {proxy.port}
                      </TableCell>
                      <TableCell className="hidden px-4 py-4 lg:table-cell">
                        {proxy.authentication}
                      </TableCell>
                      <TableCell
                        className={`hidden px-4 py-4 font-medium lg:table-cell ${
                          proxy.status === "连接失败" ? "text-rose-400" : "text-emerald-400"
                        }`}
                      >
                        {proxy.status}
                      </TableCell>
                      <TableCell className="hidden px-4 py-4 lg:table-cell">
                        <div className="flex justify-end gap-1">
                          <Button
                            variant="ghost"
                            size="icon-sm"
                            aria-label={`编辑${proxy.name}`}
                            onClick={() => openDialog(proxy)}
                          >
                            <Edit3 className="size-4" aria-hidden="true" />
                          </Button>
                          <Button
                            variant="ghost"
                            size="icon-sm"
                            className="text-rose-400 hover:bg-rose-500/10 hover:text-rose-300"
                            aria-label={`删除${proxy.name}`}
                          >
                            <Trash2 className="size-4" aria-hidden="true" />
                          </Button>
                        </div>
                      </TableCell>
                    </TableRow>
                  );
                })}
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
              closeDialog();
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
                      <SelectItem value="SOCKS5">SOCKS5</SelectItem>
                      <SelectItem value="HTTP">HTTP</SelectItem>
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

              <div className="border-t border-white/10 pt-5">
                <div className="flex items-center gap-4">
                  <Label htmlFor="proxy-authentication">启用认证</Label>
                  <Switch
                    id="proxy-authentication"
                    checked={draft.authentication}
                    onCheckedChange={(authentication) =>
                      setDraft((current) => ({ ...current, authentication }))
                    }
                  />
                </div>

                {draft.authentication && (
                  <div className="mt-5 grid gap-5 sm:grid-cols-2">
                    <Field label="用户名" htmlFor="proxy-username">
                      <Input
                        id="proxy-username"
                        placeholder="请输入用户名"
                        value={draft.username}
                        onChange={(event) =>
                          setDraft((current) => ({ ...current, username: event.target.value }))
                        }
                      />
                    </Field>
                    <Field label="密码" htmlFor="proxy-password">
                      <div className="relative">
                        <Input
                          id="proxy-password"
                          type={passwordVisible ? "text" : "password"}
                          className="pr-10"
                          placeholder="请输入密码"
                          value={draft.password}
                          onChange={(event) =>
                            setDraft((current) => ({ ...current, password: event.target.value }))
                          }
                        />
                        <Button
                          type="button"
                          variant="ghost"
                          size="icon-sm"
                          className="absolute top-1/2 right-1 -translate-y-1/2 text-muted-foreground"
                          aria-label={passwordVisible ? "隐藏密码" : "显示密码"}
                          onClick={() => setPasswordVisible((visible) => !visible)}
                        >
                          {passwordVisible ? (
                            <EyeOff className="size-4" aria-hidden="true" />
                          ) : (
                            <Eye className="size-4" aria-hidden="true" />
                          )}
                        </Button>
                      </div>
                    </Field>
                  </div>
                )}
              </div>
            </div>

            <DialogFooter className="border-t border-white/10 px-6 py-5 sm:px-7">
              <DialogClose asChild>
                <Button type="button" variant="secondary" className="min-w-28">
                  取消
                </Button>
              </DialogClose>
              <Button type="submit" className="min-w-28">
                保存
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>
    </>
  );
}
