import { Field } from "@/components/forms/Field";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";

type ProxyLinkInputProps = {
  value: string;
  error: string | null;
  onChange: (value: string) => void;
  onParse: () => void;
};

export function ProxyLinkInput({ value, error, onChange, onParse }: ProxyLinkInputProps) {
  return (
    <div className="space-y-2 border-b border-border pb-5">
      <Field label="代理链接" htmlFor="proxy-link">
        <div className="flex gap-2">
          <Input
            id="proxy-link"
            type="text"
            className="min-w-0 flex-1"
            placeholder="socks5://用户:密码@服务器:端口"
            value={value}
            autoComplete="off"
            spellCheck={false}
            aria-invalid={!!error}
            aria-describedby={error ? "proxy-link-error" : undefined}
            onChange={(event) => onChange(event.target.value)}
          />
          <Button type="button" variant="secondary" onClick={onParse}>
            解析
          </Button>
        </div>
      </Field>
      {error && (
        <p id="proxy-link-error" role="alert" className="text-sm text-destructive">
          {error}
        </p>
      )}
    </div>
  );
}
