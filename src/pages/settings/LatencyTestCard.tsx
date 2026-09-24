import { useState } from "react";
import { Activity } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";

type Props = {
  url: string;
  busy: boolean;
  onSave: (url: string) => void;
};

export function LatencyTestCard({ url, busy, onSave }: Props) {
  const [draft, setDraft] = useState(url);

  return (
    <Card className="gap-0 border-border bg-card py-0 shadow-none">
      <CardHeader className="py-5">
        <CardTitle>代理延迟测试</CardTitle>
      </CardHeader>
      <CardContent className="border-t border-border py-5">
        <form
          className="flex flex-col gap-3 sm:flex-row"
          onSubmit={(event) => {
            event.preventDefault();
            onSave(draft);
          }}
        >
          <Input
            type="url"
            aria-label="延迟测试地址"
            placeholder="https://www.gstatic.com/generate_204"
            value={draft}
            onChange={(event) => setDraft(event.target.value)}
            disabled={busy}
            className="min-w-0 flex-1"
          />
          <Button type="submit" disabled={busy || draft === url || !draft}>
            <Activity className="size-4" aria-hidden="true" />
            保存测试地址
          </Button>
        </form>
      </CardContent>
    </Card>
  );
}
