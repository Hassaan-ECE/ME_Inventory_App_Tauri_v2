import { APP_CREDIT } from "@/branding";

interface StatusStripProps {
  message: string;
}

export function StatusStrip({ message }: StatusStripProps) {
  return (
    <footer className="border-t border-border bg-card/80 px-3 py-3 text-xs text-muted-foreground sm:px-5">
      <div className="flex flex-wrap items-center justify-between gap-x-4 gap-y-1">
        <span>{message}</span>
        <span className="font-medium">{APP_CREDIT}</span>
      </div>
    </footer>
  );
}
