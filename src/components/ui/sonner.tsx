import { Toaster as SonnerToaster } from "sonner";

export function Toaster() {
  return (
    <SonnerToaster
      position="top-right"
      closeButton
      toastOptions={{
        duration: 3000,
        style: {
          background: "var(--card)",
          color: "var(--card-foreground)",
          border: "1px solid var(--border)",
        },
        actionButtonStyle: {
          background: "var(--primary)",
          color: "var(--primary-foreground)",
        },
      }}
    />
  );
}
