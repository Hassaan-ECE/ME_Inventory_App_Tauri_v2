import { useEffect, useId, useState } from "react";
import type { ReactNode } from "react";

import { Badge } from "@/shared/components/ui/badge";
import { Input } from "@/shared/components/ui/input";
import { Textarea } from "@/shared/components/ui/textarea";
import { cn } from "@/shared/lib/utils";
import {
  LIFECYCLE_OPTIONS,
  WORKING_STATUS_OPTIONS,
  type InventoryEntry,
  type InventoryEntryEditContext,
  type InventoryEntryInput,
  type LifecycleStatus,
  type WorkingStatus,
} from "@/features/inventory/types";

import { ContextRow, DialogActions, PicturePreviewCard } from "./entry-dialog/components";
import {
  buildFormState,
  formatOptionLabel,
  type EntryFormState,
  updateForm,
} from "./entry-dialog/form";
import { useEntryDialogSubmit } from "./entry-dialog/useEntryDialogSubmit";
import { useEntryPicturePreview } from "./entry-dialog/useEntryPicturePreview";
import { useMediaQuery } from "./entry-dialog/useMediaQuery";
import { useMountedRef } from "./entry-dialog/useMountedRef";

const LARGE_VIEWPORT_QUERY = "(min-width: 1024px)";
const SELECT_CLASS =
  "h-9 w-full rounded-lg border border-input bg-background px-3 text-sm text-foreground outline-none transition-shadow focus:border-ring focus:ring-[3px] focus:ring-ring/18 dark:bg-neutral-950 dark:text-neutral-100";
const OPTION_CLASS = "bg-background text-foreground dark:bg-neutral-950 dark:text-neutral-100";

interface EntryDialogProps {
  defaultArchived?: boolean;
  mode: "add" | "edit";
  onClose: () => void;
  onSave: (input: InventoryEntryInput, editContext?: InventoryEntryEditContext) => Promise<void> | void;
  readOnly?: boolean;
  entry?: InventoryEntry | null;
}

export function EntryDialog({ defaultArchived = false, mode, onClose, onSave, readOnly = false, entry }: EntryDialogProps) {
  const isMountedRef = useMountedRef();
  const [initialForm] = useState<EntryFormState>(() => buildFormState(entry, defaultArchived));
  const [form, setForm] = useState<EntryFormState>(initialForm);
  const [error, setError] = useState<string | null>(null);
  const isLargeViewport = useMediaQuery(LARGE_VIEWPORT_QUERY);
  const formId = useId();
  const showsSidebarActions = mode === "edit" && Boolean(entry) && isLargeViewport;
  const picturePath = form.picturePath.trim();
  const { handleSubmit, isSaving } = useEntryDialogSubmit({
    entry,
    form,
    initialForm,
    isMountedRef,
    mode,
    onSave,
    readOnly,
    setError,
  });
  const {
    canBrowsePicture,
    canOpenPicture,
    handleBrowsePicture,
    handleOpenPicture,
    handlePreviewError,
    handlePreviewLoad,
    picturePreviewSrc,
    picturePreviewState,
  } = useEntryPicturePreview({
    isMountedRef,
    onPicturePathChange: (selectedPath) => updateForm(setForm, "picturePath", selectedPath),
    picturePath,
    setError,
  });
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
                      onPreviewError={handlePreviewError}
                      onPreviewLoad={handlePreviewLoad}
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
                  onPreviewError={handlePreviewError}
                  onPreviewLoad={handlePreviewLoad}
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
