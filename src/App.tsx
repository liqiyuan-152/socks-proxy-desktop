import { ShieldCheck } from "lucide-react";
import { Button } from "@/components/ui/button";

export default function App() {
  return (
    <main className="flex min-h-screen items-center justify-center bg-background p-6 text-foreground">
      <section className="w-full max-w-md space-y-5 rounded-lg border bg-card p-6 shadow-sm">
        <ShieldCheck className="size-8 text-primary" aria-hidden="true" />
        <div className="space-y-1">
          <h1 className="text-xl font-semibold">Socks Proxy</h1>
          <p className="text-sm text-muted-foreground">Tauri UI foundation is ready.</p>
        </div>
        <Button>Get started</Button>
      </section>
    </main>
  );
}
