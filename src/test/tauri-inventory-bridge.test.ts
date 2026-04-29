import { act } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { APP_VERSION } from "@/branding";
import type { UpdateState } from "@/types/inventory";

describe("tauri inventory bridge", () => {
  beforeEach(() => {
    delete window.inventoryDesktop;
  });

  it("does not register the desktop bridge in a normal browser session", async () => {
    const tauriGlobal = globalThis as { isTauri?: boolean };
    const originalIsTauri = tauriGlobal.isTauri;
    tauriGlobal.isTauri = false;
    delete window.inventoryDesktop;
    vi.resetModules();

    try {
      await import("@/lib/tauriInventoryBridge");
      expect(window.inventoryDesktop).toBeUndefined();
    } finally {
      if (originalIsTauri === undefined) {
        Reflect.deleteProperty(tauriGlobal, "isTauri");
      } else {
        tauriGlobal.isTauri = originalIsTauri;
      }
    }
  });

  it("registers and cleans up Tauri shared inventory change events", async () => {
    type SharedInventoryChangedEvent = {
      event: string;
      id: number;
      payload: unknown;
    };

    const sharedChangeHandlerRef: {
      current: ((event: SharedInventoryChangedEvent) => void) | null;
    } = { current: null };
    const unlisten = vi.fn();
    const listen = vi.fn((_eventName: string, handler: (event: SharedInventoryChangedEvent) => void) => {
      sharedChangeHandlerRef.current = handler;
      return Promise.resolve(unlisten);
    });
    const invoke = vi.fn();

    vi.resetModules();
    vi.doMock("@tauri-apps/api/core", () => ({
      convertFileSrc: (path: string) => `asset://${path}`,
      invoke,
      isTauri: () => true,
    }));
    vi.doMock("@tauri-apps/api/event", () => ({ listen }));

    try {
      await import("@/lib/tauriInventoryBridge");
      const callback = vi.fn();
      const desktopBridge = Reflect.get(window, "inventoryDesktop") as NonNullable<Window["inventoryDesktop"]> | undefined;
      const cleanup = desktopBridge?.onSharedInventoryChanged?.(callback);

      expect(cleanup).toEqual(expect.any(Function));
      expect(listen).toHaveBeenCalledWith("inventory:shared-changed", expect.any(Function));

      sharedChangeHandlerRef.current?.({ event: "inventory:shared-changed", id: 1, payload: null });
      expect(callback).toHaveBeenCalledTimes(1);

      await flushAsyncWork();
      cleanup?.();

      expect(unlisten).toHaveBeenCalledTimes(1);
    } finally {
      vi.doUnmock("@tauri-apps/api/core");
      vi.doUnmock("@tauri-apps/api/event");
      vi.resetModules();
    }
  });

  it("runs Tauri shared inventory cleanup after pending listener registration resolves", async () => {
    const deferredUnlisten = createDeferred<() => void>();
    const unlisten = vi.fn();
    const listen = vi.fn(() => deferredUnlisten.promise);

    vi.resetModules();
    vi.doMock("@tauri-apps/api/core", () => ({
      convertFileSrc: (path: string) => `asset://${path}`,
      invoke: vi.fn(),
      isTauri: () => true,
    }));
    vi.doMock("@tauri-apps/api/event", () => ({ listen }));

    try {
      await import("@/lib/tauriInventoryBridge");
      const desktopBridge = Reflect.get(window, "inventoryDesktop") as NonNullable<Window["inventoryDesktop"]> | undefined;
      const cleanup = desktopBridge?.onSharedInventoryChanged?.(() => undefined);

      cleanup?.();
      expect(unlisten).not.toHaveBeenCalled();

      await act(async () => {
        deferredUnlisten.resolve(unlisten);
        await deferredUnlisten.promise;
      });

      expect(unlisten).toHaveBeenCalledTimes(1);
    } finally {
      vi.doUnmock("@tauri-apps/api/core");
      vi.doUnmock("@tauri-apps/api/event");
      vi.resetModules();
    }
  });

  it("backs desktop update checks with Tauri updater progress events", async () => {
    const receivedStates: UpdateState[] = [];
    const update = {
      body: "Signed updater release",
      close: vi.fn().mockResolvedValue(undefined),
      currentVersion: APP_VERSION,
      date: "2026-04-29T00:00:00Z",
      download: vi.fn(async (onEvent?: (event: unknown) => void) => {
        onEvent?.({ event: "Started", data: { contentLength: 100 } });
        onEvent?.({ event: "Progress", data: { chunkLength: 25 } });
        onEvent?.({ event: "Finished" });
      }),
      install: vi.fn().mockResolvedValue(undefined),
      version: "0.9.8",
    };
    const check = vi.fn().mockResolvedValue(update);

    vi.resetModules();
    vi.doMock("@tauri-apps/api/core", () => ({
      convertFileSrc: (path: string) => `asset://${path}`,
      invoke: vi.fn(),
      isTauri: () => true,
    }));
    vi.doMock("@tauri-apps/api/event", () => ({
      listen: vi.fn(() => Promise.resolve(() => undefined)),
    }));
    vi.doMock("@tauri-apps/plugin-updater", () => ({ check }));

    try {
      await import("@/lib/tauriInventoryBridge");
      const desktopBridge = Reflect.get(window, "inventoryDesktop") as NonNullable<Window["inventoryDesktop"]> | undefined;
      const cleanup = desktopBridge?.onUpdateStateChanged?.((state) => {
        receivedStates.push(state);
      });

      const availableState = await desktopBridge?.checkForUpdate?.();
      expect(check).toHaveBeenCalledTimes(1);
      expect(availableState).toMatchObject({
        available: true,
        currentVersion: APP_VERSION,
        latestVersion: "0.9.8",
        notes: "Signed updater release",
        publishedAt: "2026-04-29T00:00:00Z",
        status: "available",
      });

      const readyState = await desktopBridge?.downloadUpdate?.();
      expect(update.download).toHaveBeenCalledTimes(1);
      expect(readyState).toMatchObject({
        available: true,
        downloadPhase: "ready",
        downloadProgress: 100,
        latestVersion: "0.9.8",
        status: "ready",
      });
      expect(receivedStates).toEqual(
        expect.arrayContaining([
          expect.objectContaining({ status: "checking" }),
          expect.objectContaining({ status: "available" }),
          expect.objectContaining({ downloadPhase: "copying", downloadProgress: 25 }),
          expect.objectContaining({ downloadPhase: "verifying", downloadProgress: 100 }),
          expect.objectContaining({ downloadPhase: "ready", downloadProgress: 100 }),
        ]),
      );

      await desktopBridge?.installUpdate?.();
      expect(update.install).toHaveBeenCalledTimes(1);
      cleanup?.();
    } finally {
      vi.doUnmock("@tauri-apps/api/core");
      vi.doUnmock("@tauri-apps/api/event");
      vi.doUnmock("@tauri-apps/plugin-updater");
      vi.resetModules();
    }
  });
});

function createDeferred<T>(): {
  promise: Promise<T>;
  reject: (reason?: unknown) => void;
  resolve: (value: T) => void;
} {
  let reject!: (reason?: unknown) => void;
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((promiseResolve, promiseReject) => {
    resolve = promiseResolve;
    reject = promiseReject;
  });

  return { promise, reject, resolve };
}

async function flushAsyncWork(): Promise<void> {
  await act(async () => {
    await Promise.resolve();
    await Promise.resolve();
  });
}
