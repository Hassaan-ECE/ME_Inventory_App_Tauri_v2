import type { ButtonHTMLAttributes } from "react";
import { cva, type VariantProps } from "class-variance-authority";

import { cn } from "@/lib/utils";

const toggleVariants = cva(
  "inline-flex shrink-0 items-center justify-center rounded-lg border outline-none transition-colors focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1 focus-visible:ring-offset-background disabled:pointer-events-none disabled:opacity-60",
  {
    variants: {
      size: {
        xs: "h-7 min-w-7 px-1",
        sm: "h-8 min-w-8 px-1.5",
      },
      variant: {
        outline: "border-input bg-background text-foreground hover:bg-accent/50 dark:bg-input/30 dark:hover:bg-input/60",
      },
      pressed: {
        true: "border-primary bg-primary text-primary-foreground shadow-sm hover:bg-primary/90",
        false: "",
      },
    },
    defaultVariants: {
      size: "xs",
      variant: "outline",
    },
  },
);

export interface ToggleProps
  extends Omit<ButtonHTMLAttributes<HTMLButtonElement>, "onChange">,
    Omit<VariantProps<typeof toggleVariants>, "pressed"> {
  pressed?: boolean;
  onPressedChange?: (nextPressed: boolean) => void;
}

export function Toggle({
  className,
  onPressedChange,
  pressed = false,
  size,
  variant,
  ...props
}: ToggleProps) {
  return (
    <button
      aria-pressed={pressed}
      className={cn(toggleVariants({ className, pressed, size, variant }))}
      type="button"
      onClick={() => onPressedChange?.(!pressed)}
      {...props}
    />
  );
}
