import type { ButtonHTMLAttributes } from "react";
import { cva, type VariantProps } from "class-variance-authority";

import { cn } from "@/lib/utils";

const buttonVariants = cva(
  "inline-flex shrink-0 items-center justify-center gap-2 whitespace-nowrap rounded-lg border font-medium outline-none transition-colors focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1 focus-visible:ring-offset-background disabled:pointer-events-none disabled:opacity-60",
  {
    variants: {
      size: {
        default: "h-9 px-3 text-sm",
        sm: "h-8 px-2.5 text-xs",
        xs: "h-7 rounded-md px-2 text-xs",
        icon: "size-9",
      },
      variant: {
        default: "border-primary bg-primary text-primary-foreground hover:bg-primary/90",
        outline: "border-input bg-background text-foreground hover:bg-accent/50 dark:bg-input/30",
        ghost: "border-transparent text-foreground hover:bg-accent",
        secondary: "border-transparent bg-secondary text-secondary-foreground hover:bg-secondary/90",
        "destructive-outline":
          "border-destructive/25 bg-background text-destructive-foreground hover:bg-destructive/8 dark:bg-input/30",
      },
    },
    defaultVariants: {
      size: "default",
      variant: "default",
    },
  },
);

export interface ButtonProps
  extends ButtonHTMLAttributes<HTMLButtonElement>,
    VariantProps<typeof buttonVariants> {}

export function Button({ className, size, type = "button", variant, ...props }: ButtonProps) {
  return <button className={cn(buttonVariants({ className, size, variant }))} type={type} {...props} />;
}
