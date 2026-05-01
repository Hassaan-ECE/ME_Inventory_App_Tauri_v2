import { ExternalLinkIcon, FolderOpenIcon, ImageIcon, ImageOffIcon } from "lucide-react";

import { Badge } from "@/shared/components/ui/badge";
import { Button } from "@/shared/components/ui/button";
import { cn } from "@/shared/lib/utils";

import { handlePreviewKeyDown, type PicturePreviewState } from "./picturePreview";

interface PicturePreviewCardProps {
  canBrowse: boolean;
  canOpen: boolean;
  compact?: boolean;
  picturePath: string;
  previewSrc: string | null;
  previewState: PicturePreviewState;
  onBrowse: () => void;
  onOpen: () => void;
  onPreviewError: (previewSrc: string) => void;
  onPreviewLoad: (previewSrc: string) => void;
}

export function PicturePreviewCard({
  canBrowse,
  canOpen,
  compact = false,
  picturePath,
  previewSrc,
  previewState,
  onBrowse,
  onOpen,
  onPreviewError,
  onPreviewLoad,
}: PicturePreviewCardProps) {
  const trimmedPath = picturePath.trim();
  const hasPicture = Boolean(trimmedPath);

  return (
    <section className="rounded-2xl border border-border/70 bg-background/70 px-4 py-4">
      <div className="flex items-start justify-between gap-3">
        <div>
          <p className="text-[11px] font-semibold uppercase tracking-[0.08em] text-muted-foreground">Picture Preview</p>
        </div>
        <div className="flex shrink-0 items-center gap-2">
          {hasPicture ? (
            <Badge variant={previewState === "loaded" ? "success" : previewState === "missing" ? "warning" : "outline"}>
              {previewState === "loaded" ? "Ready" : previewState === "missing" ? "Missing" : "Selected"}
            </Badge>
          ) : null}
          <Button
            disabled={!canBrowse}
            size="sm"
            title={canBrowse ? "Browse for an entry picture" : "Desktop file picker unavailable"}
            variant="outline"
            onClick={onBrowse}
          >
            <FolderOpenIcon className="size-3.5" />
            Browse
          </Button>
        </div>
      </div>

      <div
        aria-disabled={!canOpen}
        aria-label={hasPicture ? "Picture preview" : "Picture preview unavailable"}
        className={cn(
          "group relative mt-3 flex overflow-hidden rounded-2xl border border-border/70 bg-card/70",
          compact ? "min-h-[14rem]" : "min-h-[17rem]",
          canOpen ? "cursor-zoom-in hover:border-primary/35" : "cursor-default",
        )}
        role={canOpen ? "button" : undefined}
        tabIndex={canOpen ? 0 : undefined}
        title={canOpen ? "Double-click to open in the default image viewer" : undefined}
        onDoubleClick={() => {
          if (canOpen) {
            onOpen();
          }
        }}
        onKeyDown={(event) => handlePreviewKeyDown(event, canOpen, onOpen)}
      >
        {previewSrc && previewState !== "missing" ? (
          <>
            <img
              alt="Entry picture preview"
              className={cn(
                "h-full w-full object-contain bg-background/40 transition-opacity",
                previewState === "loaded" ? "opacity-100" : "opacity-0",
              )}
              src={previewSrc}
              onError={() => onPreviewError(previewSrc)}
              onLoad={() => onPreviewLoad(previewSrc)}
            />
            {previewState !== "loaded" ? (
              <PreviewPlaceholder icon={ImageIcon} label="Loading preview..." />
            ) : canOpen ? (
              <div className="pointer-events-none absolute right-3 top-3 rounded-full bg-card/90 p-2 text-foreground shadow-sm">
                <ExternalLinkIcon className="size-4" />
              </div>
            ) : null}
          </>
        ) : (
          <PreviewPlaceholder icon={hasPicture ? ImageOffIcon : ImageIcon} label={hasPicture ? "Picture not found" : "No picture selected"} />
        )}
      </div>
    </section>
  );
}

interface PreviewPlaceholderProps {
  icon: typeof ImageIcon;
  label: string;
}

function PreviewPlaceholder({ icon: Icon, label }: PreviewPlaceholderProps) {
  return (
    <div className="absolute inset-0 flex flex-col items-center justify-center gap-2 px-4 text-center text-sm text-muted-foreground">
      <Icon className="size-7" />
      <p>{label}</p>
    </div>
  );
}

interface ContextRowProps {
  label: string;
  value: string;
}

export function ContextRow({ label, value }: ContextRowProps) {
  return (
    <div className="rounded-2xl border border-border/70 bg-card/70 px-3 py-3">
      <p className="text-[11px] font-semibold uppercase tracking-[0.08em] text-muted-foreground">{label}</p>
      <p className="mt-1 text-sm text-foreground">{value}</p>
    </div>
  );
}

interface DialogActionsProps {
  error: string | null;
  formId: string;
  isSaving: boolean;
  layout: "footer" | "sidebar";
  readOnly: boolean;
  onClose: () => void;
}

export function DialogActions({ error, formId, isSaving, layout, readOnly, onClose }: DialogActionsProps) {
  if (layout === "sidebar") {
    return (
      <>
        {error ? <p className="mb-3 text-sm text-destructive-foreground">{error}</p> : null}
        <div className="flex flex-col gap-2">
          <Button className="w-full" disabled={isSaving} variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button className="w-full" disabled={readOnly || isSaving} form={formId} type="submit">
            {isSaving ? "Saving..." : "Save Entry"}
          </Button>
        </div>
      </>
    );
  }

  return (
    <div className="flex flex-wrap items-center justify-end gap-2">
      {error ? <p className="mr-auto text-sm text-destructive-foreground">{error}</p> : null}
      <Button disabled={isSaving} variant="ghost" onClick={onClose}>
        Cancel
      </Button>
      <Button disabled={readOnly || isSaving} form={formId} type="submit">
        {isSaving ? "Saving..." : "Save Entry"}
      </Button>
    </div>
  );
}
