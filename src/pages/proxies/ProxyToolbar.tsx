import { Plus, Search, Timer } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";

type Props = {
  search: string;
  onSearch: (value: string) => void;
  protocolFilter: string;
  onProtocolFilter: (value: string) => void;
  batchPending: boolean;
  canTest: boolean;
  loading: boolean;
  busy: boolean;
  onTestAll: () => void;
  onAdd: () => void;
};

export function ProxyToolbar({
  search,
  onSearch,
  protocolFilter,
  onProtocolFilter,
  batchPending,
  canTest,
  loading,
  busy,
  onTestAll,
  onAdd,
}: Props) {
  return (
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
          onChange={(event) => onSearch(event.target.value)}
        />
      </div>
      <Select value={protocolFilter} onValueChange={onProtocolFilter}>
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
        variant="secondary"
        className="h-11"
        onClick={onTestAll}
        disabled={loading || batchPending || !canTest}
      >
        <Timer className="size-4" aria-hidden="true" />
        {batchPending ? "测试中…" : "测试全部"}
      </Button>
      <Button className="h-11 sm:min-w-36" onClick={onAdd} disabled={loading || busy}>
        <Plus className="size-5" aria-hidden="true" />
        添加代理
      </Button>
    </div>
  );
}
