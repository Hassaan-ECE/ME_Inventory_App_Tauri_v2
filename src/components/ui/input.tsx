import type { InputHTMLAttributes } from "react";

import { cn } from "@/lib/utils";

export interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  inputClassName?: string;
}

export function Input({ className, inputClassName, ...props }: InputProps) {
  return (
    <span
      className={cn(
        "relative inline-flex w-full rounded-lg border border-input bg-background text-foreground transition-shadow before:pointer-events-none before:absolute before:inset-0 before:rounded-[calc(var(--radius-lg)-1px)] before:shadow-[inset_0_1px_0_rgba(255,255,255,0.08)] has-focus-within:border-ring has-focus-within:ring-[3px] has-focus-within:ring-ring/18 dark:bg-input/30 dark:before:shadow-[inset_0_-1px_0_rgba(255,255,255,0.06)]",
        className,
      )}
    >
      <input
        className={cn(
          "relative z-10 h-8.5 w-full rounded-[inherit] bg-transparent px-3 leading-8.5 outline-none placeholder:text-muted-foreground/72",
          inputClassName,
        )}
        {...props}
      />
    </span>
  );
}
