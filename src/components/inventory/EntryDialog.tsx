import { ExternalLinkIcon, FolderOpenIcon, ImageIcon, ImageOffIcon } from "lucide-react";
import { useEffect, useId, useState, useSyncExternalStore } from "react";
import type { Dispatch, FormEvent, KeyboardEvent as ReactKeyboardEvent, ReactNode, SetStateAction } from "react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { toSafeExternalUrl } from "@/lib/externalUrl";
import { cn } from "@/lib/utils";
import {
  LIFECYCLE_OPTIONS,
  WORKING_STATUS_OPTIONS,
  type InventoryEntry,
  type InventoryEntryInput,
  type LifecycleStatus,
  type WorkingStatus,
} from "@/types/inventory";

const LARGE_VIEWPORT_QUERY = "(min-width: 1024px)";
const SELECT_CLASS =
  "h-9 w-full rounded-lg border border-input bg-background px-3 text-sm text-foreground outline-none transition-shadow focus:border-ring focus:ring-[3px] focus:ring-ring/18 dark:bg-neutral-950 dark:text-neutral-100";
const OPTION_CLASS = "bg-background text-foreground dark:bg-neutral-950 dark:text-neutral-100";

type PicturePreviewState = "empty" | "loading" | "loaded" | "missing";

interface EntryDialogProps {
  defaultArchived?: boolean;
  mode: "add" | "edit";
  onClose: () => void;
  onSave: (input: InventoryEntryInput) => Promise<void> | void;
  readOnly?: boolean;
  entry?: InventoryEntry | null;
}

interface EntryFormState {
  archived: boolean;
  assetNumber: string;
  assignedTo: string;
  condition: string;
  description: string;
  lifecycleStatus: LifecycleStatus;
  links: string;
  location: string;
  manufacturer: string;
  model: string;
  notes: string;
  picturePath: string;
  projectName: string;
  qty: string;
  serialNumber: string;
  verifiedInSurvey: boolean;
  workingStatus: WorkingStatus;
}

export function EntryDialog({ defaultArchived = false, mode, onClose, onSave, readOnly = false, entry }: EntryDialogProps) {
  const [form, setForm] = useState<EntryFormState>(() => buildFormState(entry, defaultArchived));
  const [error, setError] = useState<string | null>(null);
  const [isSaving, setIsSaving] = useState(false);
  const isLargeViewport = useMediaQuery(LARGE_VIEWPORT_QUERY);
  const formId = useId();
  const showsSidebarActions = mode === "edit" && Boolean(entry) && isLargeViewport;
  const picturePath = form.picturePath.trim();
  const picturePreviewSrc = buildPicturePreviewSource(picturePath);
  const [loadedPreviewSrc, setLoadedPreviewSrc] = useState<string | null>(null);
  const [failedPreviewSrc, setFailedPreviewSrc] = useState<string | null>(null);
  const picturePreviewState = getPicturePreviewState({
    failedPreviewSrc,
    loadedPreviewSrc,
    picturePath,
    picturePreviewSrc,
  });
  const canBrowsePicture = Boolean(window.inventoryDesktop?.pickPicturePath);
  const canOpenPicture = Boolean(picturePath) && picturePreviewState === "loaded";
  const showInlinePicturePreview = (!showsSidebarActions && !readOnly) || (!showsSidebarActions && Boolean(picturePath));
  const showSidebarPicturePreview = showsSidebarActions && (!readOnly || Boolean(picturePath));

  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent): void {
      if (event.key === "Escape" && !isSaving) {
        onClose();
      }
    }

    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [isSaving, onClose]);

  async function handleBrowsePicture(): Promise<void> {
    if (!window.inventoryDesktop?.pickPicturePath) {
      return;
    }

    try {
      const selectedPath = await window.inventoryDesktop.pickPicturePath();
      if (!selectedPath) {
        return;
      }

      setError(null);
      updateForm(setForm, "picturePath", selectedPath);
    } catch (browseError) {
      setError(browseError instanceof Error ? browseError.message : "Could not browse for a picture.");
    }
  }

  async function handleOpenPicture(): Promise<void> {
    const targetPath = form.picturePath.trim();
    if (!targetPath) {
      return;
    }

    const opened = await openPictureTarget(targetPath);
    if (!opened) {
      setError("Could not open the selected picture.");
      return;
    }

    setError(null);
  }

  async function handleSubmit(event: FormEvent<HTMLFormElement>): Promise<void> {
    event.preventDefault();

    if (readOnly) {
      return;
    }

    const result = buildEntryInput(form);
    if ("error" in result) {
      setError(result.error);
      return;
    }

    try {
      setIsSaving(true);
      setError(null);
      await onSave(result.value);
    } catch (submissionError) {
      setIsSaving(false);
      setError(submissionError instanceof Error ? submissionError.message : "Could not save this entry.");
    }
  }

  return (
    <div
      aria-modal="true"
      className="fixed inset-0 z-40 flex items-center justify-center bg-black/45 p-4 backdrop-blur-[2px]"
      role="dialog"
      onClick={(event) => {
        if (event.target === event.currentTarget && !isSaving) {
          onClose();
        }
      }}
    >
      <div className="flex max-h-[92vh] w-full max-w-[72rem] overflow-hidden rounded-[1.75rem] border border-border/70 bg-card text-card-foreground shadow-2xl lg:max-h-[94vh]">
        <form
          className={cn("min-w-0 flex flex-1 flex-col overflow-hidden", showsSidebarActions ? "lg:border-r lg:border-border/70" : "")}
          id={formId}
          onSubmit={handleSubmit}
        >
          <div className="shrink-0 border-b border-border/70 px-5 py-4 lg:py-3.5">
            <div className="flex items-center justify-between gap-3">
              <div>
                <p className="text-[11px] font-semibold uppercase tracking-[0.08em] text-muted-foreground">
                  {mode === "edit" ? "Open Full Entry" : "Add Entry"}
                </p>
                <h2 className="text-xl font-semibold tracking-tight text-foreground">
                  {mode === "edit" ? "Edit Entry" : "Add Entry"}
                </h2>
              </div>
              <div className="flex items-center gap-2">
                <Badge variant={form.archived ? "warning" : "secondary"}>{form.archived ? "Archive" : "Inventory"}</Badge>
                <Badge variant={form.verifiedInSurvey ? "success" : "outline"}>
                  {form.verifiedInSurvey ? "Verified" : "Pending"}
                </Badge>
              </div>
            </div>
          </div>

          <fieldset className="contents" disabled={readOnly || isSaving}>
            <div className="min-h-0 flex-1 overflow-y-auto px-5 py-4 lg:py-4">
              <div className="grid gap-4 lg:grid-cols-2 lg:gap-5">
                <Field label="Asset Number">
                  <Input
                    autoFocus
                    placeholder="Optional asset tag"
                    value={form.assetNumber}
                    onChange={(event) => updateForm(setForm, "assetNumber", event.currentTarget.value)}
                  />
                </Field>

                <Field label="Serial / Internal ID">
                  <Input
                    placeholder="Serial or internal ID"
                    value={form.serialNumber}
                    onChange={(event) => updateForm(setForm, "serialNumber", event.currentTarget.value)}
                  />
                </Field>

                <Field label="Manufacturer / Brand">
                  <Input
                    placeholder="Maker, brand, or supplier"
                    value={form.manufacturer}
                    onChange={(event) => updateForm(setForm, "manufacturer", event.currentTarget.value)}
                  />
                </Field>

                <Field label="Model / Part No.">
                  <Input
                    placeholder="Model or part number"
                    value={form.model}
                    onChange={(event) => updateForm(setForm, "model", event.currentTarget.value)}
                  />
                </Field>

                <Field label="Quantity">
                  <Input
                    inputMode="decimal"
                    placeholder="Quantity on hand"
                    value={form.qty}
                    onChange={(event) => updateForm(setForm, "qty", event.currentTarget.value)}
                  />
                </Field>

                <Field label="Project">
                  <Input
                    placeholder="Project this entry supports"
                    value={form.projectName}
                    onChange={(event) => updateForm(setForm, "projectName", event.currentTarget.value)}
                  />
                </Field>

                <Field className="lg:col-span-2" label="Description">
                  <Input
                    placeholder="Part or entry description"
                    value={form.description}
                    onChange={(event) => updateForm(setForm, "description", event.currentTarget.value)}
                  />
                </Field>

                <Field label="Location">
                  <Input
                    placeholder="Shelf, room, bin, or area"
                    value={form.location}
                    onChange={(event) => updateForm(setForm, "location", event.currentTarget.value)}
                  />
                </Field>

                <Field label="Used By / Assigned To">
                  <Input
                    placeholder="Person or team using it"
                    value={form.assignedTo}
                    onChange={(event) => updateForm(setForm, "assignedTo", event.currentTarget.value)}
                  />
                </Field>

                <Field className="lg:col-span-2" label="Links">
                  <Input
                    placeholder="Product, vendor, or reference link"
                    value={form.links}
                    onChange={(event) => updateForm(setForm, "links", event.currentTarget.value)}
                  />
                </Field>

                <Field label="Lifecycle">
                  <select
                    className={SELECT_CLASS}
                    value={form.lifecycleStatus}
                    onChange={(event) =>
                      updateForm(setForm, "lifecycleStatus", event.currentTarget.value as LifecycleStatus)
                    }
                  >
                    {LIFECYCLE_OPTIONS.map((option) => (
                      <option className={OPTION_CLASS} key={option} value={option}>
                        {formatOptionLabel(option)}
                      </option>
                    ))}
                  </select>
                </Field>

                <Field label="Working Status">
                  <select
                    className={SELECT_CLASS}
                    value={form.workingStatus}
                    onChange={(event) =>
                      updateForm(setForm, "workingStatus", event.currentTarget.value as WorkingStatus)
                    }
                  >
                    {WORKING_STATUS_OPTIONS.map((option) => (
                      <option className={OPTION_CLASS} key={option} value={option}>
                        {formatOptionLabel(option)}
                      </option>
                    ))}
                  </select>
                </Field>

                <Field className="lg:col-span-2" label="Condition">
                  <Input
                    placeholder="Condition or operating note"
                    value={form.condition}
                    onChange={(event) => updateForm(setForm, "condition", event.currentTarget.value)}
                  />
                </Field>

                {showInlinePicturePreview ? (
                  <div className="lg:col-span-2">
                    <PicturePreviewCard
                      canBrowse={canBrowsePicture}
                      canOpen={canOpenPicture}
                      compact={false}
                      picturePath={picturePath}
                      previewSrc={picturePreviewSrc}
                      previewState={picturePreviewState}
                      onBrowse={() => {
                        void handleBrowsePicture();
                      }}
                      onOpen={() => {
                        void handleOpenPicture();
                      }}
                      onPreviewError={() => {
                        if (!picturePreviewSrc) {
                          return;
                        }

                        setFailedPreviewSrc(picturePreviewSrc);
                        setLoadedPreviewSrc((current) => (current === picturePreviewSrc ? null : current));
                      }}
                      onPreviewLoad={() => {
                        if (!picturePreviewSrc) {
                          return;
                        }

                        setLoadedPreviewSrc(picturePreviewSrc);
                        setFailedPreviewSrc((current) => (current === picturePreviewSrc ? null : current));
                      }}
                    />
                  </div>
                ) : null}

                <Field className="lg:col-span-2" label="Notes">
                  <Textarea
                    placeholder="Operational notes, repair history, or provenance"
                    value={form.notes}
                    onChange={(event) => updateForm(setForm, "notes", event.currentTarget.value)}
                  />
                </Field>
              </div>

              <div className="mt-4 flex flex-wrap items-center gap-4 rounded-2xl border border-border/70 bg-background/70 px-4 py-3">
                <label className="flex items-center gap-2 text-sm text-foreground">
                  <input
                    checked={form.verifiedInSurvey}
                    className="size-4 accent-[var(--primary)]"
                    type="checkbox"
                    onChange={(event) => updateForm(setForm, "verifiedInSurvey", event.currentTarget.checked)}
                  />
                  Verified in survey
                </label>
                <label className="flex items-center gap-2 text-sm text-foreground">
                  <input
                    checked={form.archived}
                    className="size-4 accent-[var(--primary)]"
                    type="checkbox"
                    onChange={(event) => updateForm(setForm, "archived", event.currentTarget.checked)}
                  />
                  Archived entry
                </label>
              </div>
            </div>
          </fieldset>

          {showsSidebarActions ? null : (
            <div className="shrink-0 border-t border-border/70 px-5 py-4">
              <DialogActions error={error} formId={formId} isSaving={isSaving} layout="footer" readOnly={readOnly} onClose={onClose} />
            </div>
          )}
        </form>

        {showsSidebarActions && entry ? (
          <aside className="flex w-[19rem] shrink-0 flex-col bg-background/60 px-5 py-4">
            <div className="min-h-0 flex-1 overflow-y-auto pr-1">
              {showSidebarPicturePreview ? (
                <PicturePreviewCard
                  canBrowse={canBrowsePicture}
                  canOpen={canOpenPicture}
                  compact
                  picturePath={picturePath}
                  previewSrc={picturePreviewSrc}
                  previewState={picturePreviewState}
                  onBrowse={() => {
                    void handleBrowsePicture();
                  }}
                  onOpen={() => {
                    void handleOpenPicture();
                  }}
                  onPreviewError={() => {
                    if (!picturePreviewSrc) {
                      return;
                    }

                    setFailedPreviewSrc(picturePreviewSrc);
                    setLoadedPreviewSrc((current) => (current === picturePreviewSrc ? null : current));
                  }}
                  onPreviewLoad={() => {
                    if (!picturePreviewSrc) {
                      return;
                    }

                    setLoadedPreviewSrc(picturePreviewSrc);
                    setFailedPreviewSrc((current) => (current === picturePreviewSrc ? null : current));
                  }}
                />
              ) : null}

              <div className={cn(showSidebarPicturePreview ? "mt-4" : "")}>
                <div>
                  <p className="text-[11px] font-semibold uppercase tracking-[0.08em] text-muted-foreground">Entry Context</p>
                  <h3 className="mt-1 text-base font-semibold text-foreground">Database Metadata</h3>
                </div>

                <div className="mt-4 space-y-4">
                  <ContextRow label="Entry ID" value={entry.id} />
                  <ContextRow label="Created" value={entry.createdAt || "-"} />
                  <ContextRow label="Updated" value={entry.updatedAt || "-"} />
                  <ContextRow label="Status" value={entry.archived ? "Archived" : "Inventory"} />
                  <ContextRow label="Verified" value={entry.verifiedInSurvey ? "Verified" : "Pending"} />
                  <ContextRow label="Manual Entry" value={entry.manualEntry ? "Yes" : "No"} />
                </div>
              </div>
            </div>

            <div className="mt-4 shrink-0 border-t border-border/70 pt-4">
              <DialogActions error={error} formId={formId} isSaving={isSaving} layout="sidebar" readOnly={readOnly} onClose={onClose} />
            </div>
          </aside>
        ) : null}
      </div>
    </div>
  );
}

interface FieldProps {
  children: ReactNode;
  className?: string;
  label: string;
}

function Field({ children, className, label }: FieldProps) {
  return (
    <label className={cn("block", className)}>
      <span className="mb-1.5 block text-[11px] font-semibold uppercase tracking-[0.08em] text-muted-foreground">
        {label}
      </span>
      {children}
    </label>
  );
}

interface PicturePreviewCardProps {
  canBrowse: boolean;
  canOpen: boolean;
  compact?: boolean;
  picturePath: string;
  previewSrc: string | null;
  previewState: PicturePreviewState;
  onBrowse: () => void;
  onOpen: () => void;
  onPreviewError: () => void;
  onPreviewLoad: () => void;
}

function PicturePreviewCard({
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
              onError={onPreviewError}
              onLoad={onPreviewLoad}
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

function ContextRow({ label, value }: ContextRowProps) {
  return (
    <div className="rounded-2xl border border-border/70 bg-card/70 px-3 py-3">
      <p className="text-[11px] font-semibold uppercase tracking-[0.08em] text-muted-foreground">{label}</p>
      <p className="mt-1 text-sm text-foreground">{value}</p>
    </div>
  );
}

function useMediaQuery(query: string): boolean {
  return useSyncExternalStore(
    (onStoreChange) => {
      if (typeof window === "undefined" || typeof window.matchMedia !== "function") {
        return () => undefined;
      }

      const mediaQuery = window.matchMedia(query);
      const handleChange = (): void => onStoreChange();
      mediaQuery.addEventListener("change", handleChange);
      return () => mediaQuery.removeEventListener("change", handleChange);
    },
    () => {
      if (typeof window === "undefined" || typeof window.matchMedia !== "function") {
        return false;
      }

      return window.matchMedia(query).matches;
    },
    () => false,
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

function DialogActions({ error, formId, isSaving, layout, readOnly, onClose }: DialogActionsProps) {
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

function buildFormState(entry: InventoryEntry | null | undefined, defaultArchived: boolean): EntryFormState {
  return {
    archived: entry?.archived ?? defaultArchived,
    assetNumber: entry?.assetNumber ?? "",
    assignedTo: entry?.assignedTo ?? "",
    condition: entry?.condition ?? "",
    description: entry?.description ?? "",
    lifecycleStatus: entry?.lifecycleStatus ?? "active",
    links: entry?.links ?? "",
    location: entry?.location ?? "",
    manufacturer: entry?.manufacturer ?? "",
    model: entry?.model ?? "",
    notes: entry?.notes ?? "",
    picturePath: entry?.picturePath ?? "",
    projectName: entry?.projectName ?? "",
    qty: entry?.qty == null ? "" : String(entry.qty),
    serialNumber: entry?.serialNumber ?? "",
    verifiedInSurvey: entry?.verifiedInSurvey ?? false,
    workingStatus: entry?.workingStatus ?? "unknown",
  };
}

function buildEntryInput(
  form: EntryFormState,
): { value: InventoryEntryInput } | { error: string } {
  const qtyText = form.qty.trim();
  let qty: number | null = null;

  if (qtyText) {
    qty = Number(qtyText);
    if (!Number.isFinite(qty)) {
      return { error: "Enter quantity as a number, for example 4 or 4.5." };
    }
  }

  if (!hasIdentity(form)) {
    return {
      error: "Provide at least an asset number, serial number, manufacturer, model, or description before saving.",
    };
  }

  return {
    value: {
      archived: form.archived,
      assetNumber: form.assetNumber.trim(),
      assignedTo: form.assignedTo.trim(),
      condition: form.condition.trim(),
      description: form.description.trim(),
      lifecycleStatus: form.lifecycleStatus,
      links: form.links.trim(),
      location: form.location.trim(),
      manufacturer: form.manufacturer.trim(),
      model: form.model.trim(),
      notes: form.notes.trim(),
      picturePath: form.picturePath.trim(),
      projectName: form.projectName.trim(),
      qty,
      serialNumber: form.serialNumber.trim(),
      verifiedInSurvey: form.verifiedInSurvey,
      workingStatus: form.workingStatus,
    },
  };
}

function hasIdentity(form: EntryFormState): boolean {
  return Boolean(
    form.assetNumber.trim() ||
      form.serialNumber.trim() ||
      form.manufacturer.trim() ||
      form.model.trim() ||
      form.description.trim(),
  );
}

function updateForm<Key extends keyof EntryFormState>(
  setForm: Dispatch<SetStateAction<EntryFormState>>,
  key: Key,
  value: EntryFormState[Key],
): void {
  setForm((current) => ({ ...current, [key]: value }));
}

function formatOptionLabel(option: string): string {
  return option.replaceAll("_", " ").replace(/\b\w/g, (character) => character.toUpperCase());
}

function buildPicturePreviewSource(picturePath: string): string | null {
  const trimmedPath = picturePath.trim();
  if (!trimmedPath) {
    return null;
  }

  if (/^(?:https?:|file:|data:)/i.test(trimmedPath)) {
    return trimmedPath;
  }

  const normalizedPath = trimmedPath.replaceAll("\\", "/");
  if (/^(?:[a-zA-Z]:\/|\/\/)/.test(normalizedPath)) {
    return encodeURI(`file:${normalizedPath.startsWith("//") ? normalizedPath : `///${normalizedPath}`}`);
  }

  return null;
}

function getPicturePreviewState({
  failedPreviewSrc,
  loadedPreviewSrc,
  picturePath,
  picturePreviewSrc,
}: {
  failedPreviewSrc: string | null;
  loadedPreviewSrc: string | null;
  picturePath: string;
  picturePreviewSrc: string | null;
}): PicturePreviewState {
  if (!picturePath) {
    return "empty";
  }

  if (!picturePreviewSrc) {
    return "missing";
  }

  if (failedPreviewSrc === picturePreviewSrc) {
    return "missing";
  }

  if (loadedPreviewSrc === picturePreviewSrc) {
    return "loaded";
  }

  return "loading";
}

function handlePreviewKeyDown(
  event: ReactKeyboardEvent<HTMLDivElement>,
  canOpen: boolean,
  onOpen: () => void,
): void {
  if (!canOpen) {
    return;
  }

  if (event.key === "Enter" || event.key === " ") {
    event.preventDefault();
    onOpen();
  }
}

async function openPictureTarget(targetPath: string): Promise<boolean> {
  const trimmedTargetPath = targetPath.trim();
  if (!trimmedTargetPath) {
    return false;
  }

  const externalUrl = toSafeExternalUrl(trimmedTargetPath);
  if (externalUrl) {
    if (window.inventoryDesktop?.openExternal) {
      return window.inventoryDesktop.openExternal(externalUrl);
    }

    window.open(externalUrl, "_blank", "noopener,noreferrer");
    return true;
  }

  if (window.inventoryDesktop?.openPath) {
    return window.inventoryDesktop.openPath(trimmedTargetPath);
  }

  return false;
}
