import type { HTMLAttributes } from "react";
import { cva, type VariantProps } from "class-variance-authority";

import { cn } from "@/lib/utils";

const badgeVariants = cva("inline-flex items-center gap-1 whitespace-nowrap rounded-sm border font-medium", {
  variants: {
    size: {
      default: "h-5.5 px-1.5 text-xs",
      sm: "h-5 rounded-[.25rem] px-1 text-[10px]",
    },
    variant: {
      default: "border-primary bg-primary text-primary-foreground",
      outline: "border-input bg-background text-foreground dark:bg-input/30",
      secondary: "border-transparent bg-secondary text-secondary-foreground",
      success: "border-transparent bg-success/10 text-success-foreground",
      warning: "border-transparent bg-warning/10 text-warning-foreground",
      error: "border-transparent bg-destructive/10 text-destructive-foreground",
    },
  },
  defaultVariants: {
    size: "default",
    variant: "outline",
  },
});

export interface BadgeProps extends HTMLAttributes<HTMLSpanElement>, VariantProps<typeof badgeVariants> {}

export function Badge({ className, size, variant, ...props }: BadgeProps) {
  return <span className={cn(badgeVariants({ className, size, variant }))} {...props} />;
}
