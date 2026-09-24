import type { ReactNode } from "react";
import { Label } from "@/components/ui/label";

type FieldProps = {
  label: string;
  required?: boolean;
  htmlFor: string;
  children: ReactNode;
};

export function Field({ label, required = false, htmlFor, children }: FieldProps) {
  return (
    <div className="grid gap-2">
      <Label htmlFor={htmlFor}>
        {label}
        {required && <span className="text-destructive">*</span>}
      </Label>
      {children}
    </div>
  );
}
