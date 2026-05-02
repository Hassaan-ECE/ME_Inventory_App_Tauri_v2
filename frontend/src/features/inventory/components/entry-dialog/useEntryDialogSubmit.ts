import { useCallback, useState } from "react";
import type { FormEvent, MutableRefObject } from "react";

import type {
  InventoryEntry,
  InventoryEntryEditContext,
  InventoryEntryInput,
} from "@/features/inventory/types";

import { buildEditContext } from "./editContext";
import { buildEntryInput, type EntryFormState } from "./form";

interface UseEntryDialogSubmitOptions {
  entry?: InventoryEntry | null;
  form: EntryFormState;
  initialForm: EntryFormState;
  isMountedRef: MutableRefObject<boolean>;
  mode: "add" | "edit";
  onSave: (input: InventoryEntryInput, editContext?: InventoryEntryEditContext) => Promise<void> | void;
  readOnly: boolean;
  setError: (message: string | null) => void;
}

export function useEntryDialogSubmit({
  entry,
  form,
  initialForm,
  isMountedRef,
  mode,
  onSave,
  readOnly,
  setError,
}: UseEntryDialogSubmitOptions) {
  const [isSaving, setIsSaving] = useState(false);

  const handleSubmit = useCallback(
    async (event: FormEvent<HTMLFormElement>): Promise<void> => {
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
        await onSave(result.value, buildEditContext(mode, entry, initialForm, result.value));
      } catch (submissionError) {
        if (!isMountedRef.current) {
          return;
        }
        setIsSaving(false);
        setError(submissionError instanceof Error ? submissionError.message : "Could not save this entry.");
      }
    },
    [entry, form, initialForm, isMountedRef, mode, onSave, readOnly, setError],
  );

  return { handleSubmit, isSaving };
}
