import type { ComponentProps } from "react";

import { cn } from "@/lib/utils";

export function Empty({ className, ...props }: ComponentProps<"div">) {
  return (
    <div className={cn("flex min-w-0 flex-1 flex-col items-center justify-center gap-6 p-6 text-center md:p-12", className)} {...props} />
  );
}

export function EmptyHeader({ className, ...props }: ComponentProps<"div">) {
  return <div className={cn("flex max-w-sm flex-col items-center text-center", className)} {...props} />;
}

export function EmptyTitle({ className, ...props }: ComponentProps<"div">) {
  return <div className={cn("text-xl font-semibold", className)} {...props} />;
}

export function EmptyDescription({ className, ...props }: ComponentProps<"p">) {
  return <p className={cn("mt-1 text-sm text-muted-foreground", className)} {...props} />;
}
