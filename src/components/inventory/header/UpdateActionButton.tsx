import type { UpdateState } from "@/types/inventory";

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

  return (
    <button
      className="ml-1 inline-flex h-7 shrink-0 items-center justify-center rounded-lg border border-sky-500 bg-sky-100 px-2.5 text-xs font-semibold text-sky-700 transition-colors hover:bg-sky-200 disabled:cursor-default disabled:opacity-80 dark:border-sky-400/70 dark:bg-sky-950/50 dark:text-sky-200 dark:hover:bg-sky-900/70"
      disabled={state.status === "downloading" || state.status === "checking" || state.status === "installing"}
      type="button"
      onClick={onClick}
    >
      {label}
    </button>
  );
}

function getUpdateActionLabel(state: UpdateState): string {
  switch (state.status) {
    case "available":
      return "Update available";
    case "downloading":
      return "Downloading update...";
    case "ready":
      return "Install update";
    case "installing":
      return "Installer opened";
    case "error":
      return "Retry update";
    default:
      return "";
  }
}
