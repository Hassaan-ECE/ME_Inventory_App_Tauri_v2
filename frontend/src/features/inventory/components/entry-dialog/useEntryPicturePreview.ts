import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import type { MutableRefObject } from "react";

import {
  buildPicturePreviewSource,
  openPictureTarget,
  shouldLoadNativePicturePreview,
  type PicturePreviewState,
} from "./picturePreview";

interface UseEntryPicturePreviewOptions {
  isMountedRef: MutableRefObject<boolean>;
  onPicturePathChange: (path: string) => void;
  picturePath: string;
  setError: (message: string | null) => void;
}

export interface EntryPicturePreviewControls {
  canBrowsePicture: boolean;
  canOpenPicture: boolean;
  handleBrowsePicture: () => Promise<void>;
  handleOpenPicture: () => Promise<void>;
  handlePreviewError: (previewSrc: string) => void;
  handlePreviewLoad: (previewSrc: string) => void;
  picturePreviewSrc: string | null;
  picturePreviewState: PicturePreviewState;
}

export function useEntryPicturePreview({
  isMountedRef,
  onPicturePathChange,
  picturePath,
  setError,
}: UseEntryPicturePreviewOptions): EntryPicturePreviewControls {
  const [picturePreviewSrc, setPicturePreviewSrc] = useState<string | null>(null);
  const [picturePreviewState, setPicturePreviewState] = useState<PicturePreviewState>("empty");
  const picturePreviewSrcRef = useRef<string | null>(null);
  const canBrowsePicture = Boolean(window.inventoryDesktop?.pickPicturePath);
  const canOpenPicture = Boolean(picturePath) && picturePreviewState === "loaded";

  useLayoutEffect(() => {
    picturePreviewSrcRef.current = picturePreviewSrc;
  }, [picturePreviewSrc]);

  useEffect(() => {
    let active = true;
    const canSetPreviewState = (): boolean => active && isMountedRef.current;

    async function loadPicturePreview(): Promise<void> {
      if (!picturePath) {
        if (!canSetPreviewState()) {
          return;
        }
        setPicturePreviewSrc(null);
        setPicturePreviewState("empty");
        return;
      }

      if (window.inventoryDesktop?.loadPicturePreview && shouldLoadNativePicturePreview(picturePath)) {
        setPicturePreviewSrc(null);
        setPicturePreviewState("loading");

        try {
          const previewSrc = await window.inventoryDesktop.loadPicturePreview(picturePath);
          if (!canSetPreviewState()) {
            return;
          }
          setPicturePreviewSrc(previewSrc);
          setPicturePreviewState(previewSrc ? "loading" : "missing");
        } catch {
          if (canSetPreviewState()) {
            setPicturePreviewSrc(null);
            setPicturePreviewState("missing");
          }
        }
        return;
      }

      const previewSrc = buildPicturePreviewSource(picturePath);
      if (!canSetPreviewState()) {
        return;
      }
      setPicturePreviewSrc(previewSrc);
      setPicturePreviewState(previewSrc ? "loading" : "missing");
    }

    void loadPicturePreview();

    return () => {
      active = false;
    };
  }, [isMountedRef, picturePath]);

  const handleBrowsePicture = useCallback(async (): Promise<void> => {
    if (!window.inventoryDesktop?.pickPicturePath) {
      return;
    }

    try {
      const selectedPath = await window.inventoryDesktop.pickPicturePath();
      if (!isMountedRef.current) {
        return;
      }
      if (!selectedPath) {
        return;
      }

      setError(null);
      onPicturePathChange(selectedPath);
    } catch (browseError) {
      if (!isMountedRef.current) {
        return;
      }
      setError(browseError instanceof Error ? browseError.message : "Could not browse for a picture.");
    }
  }, [isMountedRef, onPicturePathChange, setError]);

  const handleOpenPicture = useCallback(async (): Promise<void> => {
    const targetPath = picturePath.trim();
    if (!targetPath) {
      return;
    }

    const opened = await openPictureTarget(targetPath);
    if (!isMountedRef.current) {
      return;
    }
    if (!opened) {
      setError("Could not open the selected picture.");
      return;
    }

    setError(null);
  }, [isMountedRef, picturePath, setError]);

  const handlePreviewError = useCallback((previewSrc: string): void => {
    if (!isMountedRef.current || picturePreviewSrcRef.current !== previewSrc) {
      return;
    }

    setPicturePreviewSrc(null);
    setPicturePreviewState("missing");
  }, [isMountedRef]);

  const handlePreviewLoad = useCallback((previewSrc: string): void => {
    if (!isMountedRef.current || picturePreviewSrcRef.current !== previewSrc) {
      return;
    }

    setPicturePreviewState("loaded");
  }, [isMountedRef]);

  return {
    canBrowsePicture,
    canOpenPicture,
    handleBrowsePicture,
    handleOpenPicture,
    handlePreviewError,
    handlePreviewLoad,
    picturePreviewSrc,
    picturePreviewState,
  };
}
