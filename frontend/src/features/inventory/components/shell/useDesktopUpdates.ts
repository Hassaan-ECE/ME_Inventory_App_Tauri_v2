import { useCallback, useEffect, useRef, useState } from "react";

import type { UpdateState } from "@/features/inventory/types";

import { UPDATE_CHECK_INTERVAL_MS, buildIdleUpdateState, chooseFreshUpdateState } from "./helpers";

interface UseDesktopUpdatesOptions {
  announceStatus: (message: string) => void;
}

export function useDesktopUpdates({ announceStatus }: UseDesktopUpdatesOptions) {
  const [updateState, setUpdateState] = useState<UpdateState>(() => buildIdleUpdateState());
  const updateStateRef = useRef(updateState);

  useEffect(() => {
    updateStateRef.current = updateState;
  }, [updateState]);

  useEffect(() => {
    if (!window.inventoryDesktop?.checkForUpdate) {
      return undefined;
    }

    let active = true;
    const canCheckForUpdate = (): boolean =>
      !["downloading", "ready", "installing"].includes(updateStateRef.current.status);
    const runUpdateCheck = (): void => {
      if (!window.inventoryDesktop?.checkForUpdate || !canCheckForUpdate()) {
        return;
      }

      void window.inventoryDesktop
        .checkForUpdate()
        .then((state) => {
          if (active) {
            updateStateRef.current = chooseFreshUpdateState(updateStateRef.current, state);
            setUpdateState((current) => chooseFreshUpdateState(current, state));
          }
        })
        .catch(() => {
          if (active) {
            updateStateRef.current = buildIdleUpdateState();
            setUpdateState(buildIdleUpdateState());
          }
        });
    };
    const handleVisibilityChange = (): void => {
      if (document.visibilityState === "visible") {
        runUpdateCheck();
      }
    };
    const unsubscribe = window.inventoryDesktop.onUpdateStateChanged?.((state) => {
      if (active) {
        updateStateRef.current = state;
        setUpdateState(state);
      }
    });

    runUpdateCheck();
    const intervalId = window.setInterval(runUpdateCheck, UPDATE_CHECK_INTERVAL_MS);
    window.addEventListener("focus", runUpdateCheck);
    document.addEventListener("visibilitychange", handleVisibilityChange);

    return () => {
      active = false;
      window.clearInterval(intervalId);
      window.removeEventListener("focus", runUpdateCheck);
      document.removeEventListener("visibilitychange", handleVisibilityChange);
      unsubscribe?.();
    };
  }, []);

  const handleUpdateAction = useCallback(async (): Promise<void> => {
    if (!window.inventoryDesktop) {
      return;
    }

    try {
      if (updateState.status === "ready") {
        const nextState = await window.inventoryDesktop.installUpdate?.();
        if (nextState) {
          setUpdateState(nextState);
          if (nextState.status === "error" && nextState.error) {
            announceStatus(nextState.error);
          }
        }
        return;
      }

      if (updateState.status === "downloading" || updateState.status === "checking" || updateState.status === "installing") {
        return;
      }

      if (!window.inventoryDesktop.downloadUpdate) {
        announceStatus("Update download is only available in the desktop app.");
        return;
      }

      if (updateState.available) {
        setUpdateState((current) => ({ ...current, status: "downloading" }));
      }
      const nextState = await window.inventoryDesktop.downloadUpdate();
      setUpdateState((current) => chooseFreshUpdateState(current, nextState));
      if (nextState.status === "error" && nextState.error) {
        announceStatus(nextState.error);
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : "Update failed.";
      setUpdateState((current) => ({ ...current, error: message, status: "error" }));
      announceStatus(message);
    }
  }, [announceStatus, updateState.available, updateState.status]);

  return { handleUpdateAction, updateState };
}
