import * as React from "react";
import { Switch as SwitchPrimitive } from "radix-ui";
import { cn } from "cn";

function Switch({ className, ...props }: React.ComponentProps<typeof SwitchPrimitive.Root>) {
  return (
    <SwitchPrimitive.Root
      data-slot="switch"
      className={cn(
        "peer inline-flex h-6 w-11 shrink-0 items-center rounded-full border border-transparent bg-input p-0.5 shadow-xs outline-none transition-[background-color,box-shadow] duration-200 motion-reduce:transition-none disabled:cursor-not-allowed disabled:opacity-50 focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/30 data-[state=checked]:bg-success data-[state=checked]:shadow-sm data-[state=checked]:shadow-success/25",
        className,
      )}
      {...props}
    >
      <SwitchPrimitive.Thumb
        data-slot="switch-thumb"
        className="pointer-events-none block size-5 rounded-full bg-white shadow-sm transition-transform duration-200 motion-reduce:transition-none data-[state=checked]:translate-x-5 data-[state=unchecked]:translate-x-0"
      />
    </SwitchPrimitive.Root>
  );
}

export { Switch };
