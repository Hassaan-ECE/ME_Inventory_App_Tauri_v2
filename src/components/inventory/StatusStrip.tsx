interface StatusStripProps {
  message: string;
}

export function StatusStrip({ message }: StatusStripProps) {
  return (
    <footer className="border-t border-border bg-card/80 px-3 py-3 text-xs text-muted-foreground sm:px-5">
      {message}
    </footer>
  );
}
