import { CheckCircle2Icon, DownloadIcon, LoaderCircleIcon } from "lucide-react";

import type { UpdateState } from "@/features/inventory/types";

interface UpdateActionButtonProps {
  onClick: () => void;
  state: UpdateState;
}

export function UpdateActionButton({ onClick, state }: UpdateActionButtonProps) {
  if (!state.available && state.status !== "ready") {
    return null;
  }

  const label = getUpdateActionLabel(state);
  if (!label) {
    return null;
  }
  const progress = getUpdateProgress(state);
  const showProgress = progress !== null;
  const isBusy = state.status === "downloading" || state.status === "checking" || state.status === "installing";

  return (
    <button
      className="relative ml-1 inline-flex h-7 min-w-[9.75rem] shrink-0 items-center justify-center overflow-hidden rounded-lg border border-sky-500 bg-sky-100 px-2.5 text-xs font-semibold text-sky-700 transition-colors hover:bg-sky-200 disabled:cursor-default disabled:opacity-90 dark:border-sky-400/70 dark:bg-sky-950/50 dark:text-sky-200 dark:hover:bg-sky-900/70"
      disabled={isBusy}
      type="button"
      onClick={onClick}
    >
      {showProgress ? (
        <span
          aria-hidden="true"
          className="absolute inset-y-0 left-0 bg-sky-300/70 transition-[width] duration-200 dark:bg-sky-500/30"
          style={{ width: `${progress}%` }}
        />
      ) : null}
      <span className="relative z-10 inline-flex min-w-0 items-center gap-1.5">
        {renderUpdateActionIcon(state)}
        <span className="truncate">{label}</span>
      </span>
    </button>
  );
}

function getUpdateActionLabel(state: UpdateState): string {
  switch (state.status) {
    case "available":
      return state.latestVersion ? `Update ${state.latestVersion}` : "Update available";
    case "downloading":
      return typeof state.downloadProgress === "number" ? `Downloading ${state.downloadProgress}%` : "Downloading update...";
    case "ready":
      return "Install update";
    case "installing":
      return "Installing update...";
    case "error":
      return "Retry update";
    default:
      return "";
  }
}

function renderUpdateActionIcon(state: UpdateState) {
  switch (state.status) {
    case "ready":
      return <CheckCircle2Icon className="size-3.5" />;
    case "downloading":
    case "installing":
      return <LoaderCircleIcon className="size-3.5 animate-spin" />;
    default:
      return <DownloadIcon className="size-3.5" />;
  }
}

function getUpdateProgress(state: UpdateState): number | null {
  if (state.status === "installing") {
    return 100;
  }

  if (state.status !== "downloading") {
    return null;
  }

  if (typeof state.downloadProgress !== "number" || !Number.isFinite(state.downloadProgress)) {
    return 10;
  }

  return Math.max(3, Math.min(100, state.downloadProgress));
}
