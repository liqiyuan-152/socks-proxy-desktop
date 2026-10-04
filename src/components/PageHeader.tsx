import { SidebarTrigger } from "@/components/ui/sidebar";

export function PageHeader({ title, description }: { title: string; description: string }) {
  return (
    <header className="relative shrink-0 overflow-hidden border-b border-sidebar-border bg-gradient-to-br from-sidebar via-sidebar to-sidebar-accent/30 px-4 py-4 sm:py-5 text-sidebar-foreground backdrop-blur-sm sm:px-6">
      <div
        aria-hidden="true"
        className="pointer-events-none absolute inset-0 bg-gradient-to-br from-primary/5 to-transparent"
      />
      <div className="relative flex items-start gap-3">
        <SidebarTrigger className="mt-0.5 shrink-0 md:hidden" />
        <div className="min-w-0">
          <h1 className="text-xl font-bold md:text-2xl tracking-tight">{title}</h1>
          <p className="mt-1.5 text-sm text-muted-foreground">{description}</p>
        </div>
      </div>
    </header>
  );
}
