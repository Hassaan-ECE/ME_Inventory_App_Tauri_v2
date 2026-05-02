import { useEffect, useId, useState } from "react";
import type { ReactNode } from "react";

import { Badge } from "@/shared/components/ui/badge";
import { Input } from "@/shared/components/ui/input";
import { Textarea } from "@/shared/components/ui/textarea";
import { cn } from "@/shared/lib/utils";
import type {
  InventoryEntry,
  InventoryEntryEditContext,
  InventoryEntryInput,
  LifecycleStatus,
  WorkingStatus,
} from "@/features/inventory/types";

import { ContextRow, DialogActions } from "./entry-dialog/components";
import {
  ENTRY_BOOLEAN_FIELDS,
  ENTRY_CONDITION_FIELD,
  ENTRY_MAIN_INPUT_FIELDS,
  ENTRY_SELECT_FIELDS,
  buildEntryContextRows,
  type EntrySelectField,
} from "./entry-dialog/fieldMetadata";
import {
  buildFormState,
  formatOptionLabel,
  type EntryFormState,
  updateForm,
} from "./entry-dialog/form";
import { PicturePreviewPanel } from "./entry-dialog/PicturePreviewPanel";
import { useEntryDialogLayout } from "./entry-dialog/useEntryDialogLayout";
import { useEntryDialogSubmit } from "./entry-dialog/useEntryDialogSubmit";
import { useEntryPicturePreview } from "./entry-dialog/useEntryPicturePreview";
import { useMountedRef } from "./entry-dialog/useMountedRef";

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
  const formId = useId();
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
  const picturePreview = useEntryPicturePreview({
    isMountedRef,
    onPicturePathChange: (selectedPath) => updateForm(setForm, "picturePath", selectedPath),
    picturePath,
    setError,
  });
  const {
    showInlinePicturePreview,
    showSidebarPicturePreview,
    showsSidebarActions,
  } = useEntryDialogLayout({
    entry,
    mode,
    picturePath,
    readOnly,
  });

  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent): void {
      if (event.key === "Escape" && !isSaving) {
        onClose();
      }
    }

    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [isSaving, onClose]);

  function handleSelectChange(field: EntrySelectField, value: string): void {
    if (field.key === "lifecycleStatus") {
      updateForm(setForm, field.key, value as LifecycleStatus);
      return;
    }

    updateForm(setForm, field.key, value as WorkingStatus);
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
                {ENTRY_MAIN_INPUT_FIELDS.map((field) => (
                  <Field className={field.className} key={field.key} label={field.label}>
                    <Input
                      autoFocus={field.autoFocus}
                      inputMode={field.inputMode}
                      placeholder={field.placeholder}
                      value={form[field.key]}
                      onChange={(event) => updateForm(setForm, field.key, event.currentTarget.value)}
                    />
                  </Field>
                ))}

                {ENTRY_SELECT_FIELDS.map((field) => (
                  <Field key={field.key} label={field.label}>
                    <select
                      className={SELECT_CLASS}
                      value={form[field.key]}
                      onChange={(event) => handleSelectChange(field, event.currentTarget.value)}
                    >
                      {field.options.map((option) => (
                        <option className={OPTION_CLASS} key={option} value={option}>
                          {formatOptionLabel(option)}
                        </option>
                      ))}
                    </select>
                  </Field>
                ))}

                <Field className={ENTRY_CONDITION_FIELD.className} label={ENTRY_CONDITION_FIELD.label}>
                  <Input
                    placeholder={ENTRY_CONDITION_FIELD.placeholder}
                    value={form[ENTRY_CONDITION_FIELD.key]}
                    onChange={(event) => updateForm(setForm, ENTRY_CONDITION_FIELD.key, event.currentTarget.value)}
                  />
                </Field>

                {showInlinePicturePreview ? (
                  <div className="lg:col-span-2">
                    <PicturePreviewPanel picturePath={picturePath} preview={picturePreview} />
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
                {ENTRY_BOOLEAN_FIELDS.map((field) => (
                  <label className="flex items-center gap-2 text-sm text-foreground" key={field.key}>
                    <input
                      checked={form[field.key]}
                      className="size-4 accent-[var(--primary)]"
                      type="checkbox"
                      onChange={(event) => updateForm(setForm, field.key, event.currentTarget.checked)}
                    />
                    {field.label}
                  </label>
                ))}
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
                <PicturePreviewPanel compact picturePath={picturePath} preview={picturePreview} />
              ) : null}

              <div className={cn(showSidebarPicturePreview ? "mt-4" : "")}>
                <div>
                  <p className="text-[11px] font-semibold uppercase tracking-[0.08em] text-muted-foreground">Entry Context</p>
                  <h3 className="mt-1 text-base font-semibold text-foreground">Database Metadata</h3>
                </div>

                <div className="mt-4 space-y-4">
                  {buildEntryContextRows(entry).map((row) => (
                    <ContextRow key={row.label} label={row.label} value={row.value} />
                  ))}
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
