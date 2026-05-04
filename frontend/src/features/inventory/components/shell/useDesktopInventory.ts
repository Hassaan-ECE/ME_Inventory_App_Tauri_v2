import { useCallback, useEffect, useRef, useState } from "react";

import { MOCK_INVENTORY } from "@/features/inventory/data/mockInventory";
import type { InventoryEntry, InventorySharedStatus } from "@/features/inventory/types";
import type { InventorySyncResult } from "@/integrations/tauri/desktop-bridge";

import {
  DESKTOP_SHARED_PENDING_STATUS,
  MOCK_SHARED_STATUS,
  clampSharedSyncIntervalMs,
  hasDesktopBridge,
  normalizeSharedStatus,
  sharedStatusesMatch,
} from "./helpers";

const LOCAL_MUTATION_SYNC_DELAY_MS = 75;

interface UseDesktopInventoryOptions {
  announceStatus: (message: string) => void;
}

interface RefreshDesktopEntriesOptions {
  applyResult?: boolean;
  keepLoading?: boolean;
  showLoading?: boolean;
}

export function useDesktopInventory({ announceStatus }: UseDesktopInventoryOptions) {
  const [entries, setEntries] = useState<InventoryEntry[]>(() => (hasDesktopBridge() ? [] : MOCK_INVENTORY));
  const [dataSource, setDataSource] = useState<"desktop" | "mock">(() => (hasDesktopBridge() ? "desktop" : "mock"));
  const [isLoading, setIsLoading] = useState<boolean>(() => hasDesktopBridge());
  const [sharedStatus, setSharedStatus] = useState<InventorySharedStatus>(() =>
    hasDesktopBridge() ? DESKTOP_SHARED_PENDING_STATUS : MOCK_SHARED_STATUS,
  );
  const syncInFlightRef = useRef(false);
  const syncFollowUpRequestedRef = useRef(false);
  const delayedSyncTimeoutRef = useRef<number | null>(null);
  const initialSyncStartedRef = useRef(false);
  const queryRequestRef = useRef(0);
  const mountedRef = useRef(false);

  const canApplyDesktopResult = useCallback(
    (applyResult: boolean, requestId?: number): boolean =>
      applyResult && mountedRef.current && (requestId === undefined || requestId === queryRequestRef.current),
    [],
  );

  const markSharedUnavailable = useCallback((message = "Shared workspace unavailable. Saving changes locally."): void => {
    setSharedStatus((current) => ({
      ...current,
      available: false,
      canModify: true,
      enabled: true,
      hasLocalOnlyChanges: current.hasLocalOnlyChanges,
      message,
      mutationMode: "local",
      syncIntervalMs: clampSharedSyncIntervalMs(current.syncIntervalMs),
    }));
  }, []);

  const refreshDesktopEntries = useCallback(
    async ({
      applyResult = true,
      keepLoading = false,
      showLoading = false,
    }: RefreshDesktopEntriesOptions = {}): Promise<InventorySyncResult | null> => {
      const desktopBridge = window.inventoryDesktop;
      if (!desktopBridge?.loadInventory) {
        return null;
      }

      const requestId = queryRequestRef.current + 1;
      queryRequestRef.current = requestId;
      if (showLoading && canApplyDesktopResult(applyResult, requestId)) {
        setIsLoading(true);
      }
      try {
        const payload = await desktopBridge.loadInventory();
        if (!canApplyDesktopResult(applyResult, requestId)) {
          return null;
        }
        const shared = normalizeSharedStatus(payload.shared);
        setEntries(payload.entries);
        setDataSource("desktop");
        setSharedStatus((current) => (sharedStatusesMatch(current, shared) ? current : shared));
        return shared === payload.shared ? payload : { ...payload, shared };
      } catch {
        if (canApplyDesktopResult(applyResult, requestId)) {
          setEntries([]);
          setDataSource("desktop");
          markSharedUnavailable("Inventory database unavailable. Restart the app or check app data permissions.");
          announceStatus("Could not load the ME Inventory database.");
        }
        return null;
      } finally {
        if (!keepLoading && canApplyDesktopResult(applyResult, requestId)) {
          setIsLoading(false);
        }
      }
    },
    [announceStatus, canApplyDesktopResult, markSharedUnavailable],
  );

  const syncEntriesFromDesktop = useCallback(
    async function syncEntriesFromDesktopImpl({
      applyResult = true,
    }: { applyResult?: boolean } = {}): Promise<void> {
      if (!canApplyDesktopResult(applyResult)) {
        return;
      }

      const desktopBridge = window.inventoryDesktop;
      if (!desktopBridge?.syncInventory) {
        return;
      }
      if (syncInFlightRef.current) {
        syncFollowUpRequestedRef.current = true;
        return;
      }

      const startingRequestId = queryRequestRef.current;
      try {
        syncInFlightRef.current = true;
        const payload = await desktopBridge.syncInventory();
        if (!canApplyDesktopResult(applyResult)) {
          return;
        }
        const shared = normalizeSharedStatus(payload.shared);
        setSharedStatus((current) => (sharedStatusesMatch(current, shared) ? current : shared));
        if (payload.entriesChanged === true && startingRequestId === queryRequestRef.current) {
          setEntries(payload.entries);
          setDataSource("desktop");
        } else if (payload.entriesChanged === undefined && startingRequestId === queryRequestRef.current) {
          await refreshDesktopEntries({ applyResult });
        }
      } catch {
        if (canApplyDesktopResult(applyResult)) {
          markSharedUnavailable();
          if (startingRequestId === queryRequestRef.current) {
            await refreshDesktopEntries({ applyResult });
          }
        }
      } finally {
        syncInFlightRef.current = false;
        if (syncFollowUpRequestedRef.current && canApplyDesktopResult(applyResult)) {
          syncFollowUpRequestedRef.current = false;
          void syncEntriesFromDesktopImpl({ applyResult });
        }
      }
    },
    [canApplyDesktopResult, markSharedUnavailable, refreshDesktopEntries],
  );

  const scheduleDesktopSync = useCallback((): void => {
    if (!window.inventoryDesktop?.syncInventory) {
      return;
    }
    if (delayedSyncTimeoutRef.current !== null) {
      window.clearTimeout(delayedSyncTimeoutRef.current);
    }
    delayedSyncTimeoutRef.current = window.setTimeout(() => {
      delayedSyncTimeoutRef.current = null;
      void syncEntriesFromDesktop();
    }, LOCAL_MUTATION_SYNC_DELAY_MS);
  }, [syncEntriesFromDesktop]);

  useEffect(() => {
    mountedRef.current = true;

    return () => {
      mountedRef.current = false;
      queryRequestRef.current += 1;
      if (delayedSyncTimeoutRef.current !== null) {
        window.clearTimeout(delayedSyncTimeoutRef.current);
      }
    };
  }, []);

  useEffect(() => {
    let active = true;

    async function loadEntriesFromDesktop(): Promise<void> {
      if (!window.inventoryDesktop?.loadInventory) {
        return;
      }

      const payload = await refreshDesktopEntries({ applyResult: active, keepLoading: true, showLoading: true });

      if (active && payload?.shared.enabled && !initialSyncStartedRef.current) {
        initialSyncStartedRef.current = true;
        await syncEntriesFromDesktop({ applyResult: active });
      }

      if (active) {
        setIsLoading(false);
      }
    }

    void loadEntriesFromDesktop();

    return () => {
      active = false;
    };
  }, [refreshDesktopEntries, syncEntriesFromDesktop]);

  useEffect(() => {
    if (!window.inventoryDesktop?.syncInventory || !sharedStatus.enabled) {
      return undefined;
    }

    let active = true;
    const syncIntervalMs = clampSharedSyncIntervalMs(sharedStatus.syncIntervalMs);
    const intervalId = window.setInterval(() => {
      void syncEntriesFromDesktop({ applyResult: active });
    }, syncIntervalMs);
    const unsubscribe = window.inventoryDesktop.onSharedInventoryChanged?.(() => {
      void syncEntriesFromDesktop({ applyResult: active });
    });

    return () => {
      active = false;
      window.clearInterval(intervalId);
      unsubscribe?.();
    };
  }, [sharedStatus.enabled, sharedStatus.syncIntervalMs, syncEntriesFromDesktop]);

  return {
    dataSource,
    entries,
    isLoading,
    scheduleDesktopSync,
    setEntries,
    setSharedStatus,
    sharedStatus,
  };
}
