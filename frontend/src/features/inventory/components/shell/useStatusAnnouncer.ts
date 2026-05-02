import { useCallback, useEffect, useRef, useState } from "react";

const DEFAULT_STATUS_TIMEOUT_MS = 2400;

export function useStatusAnnouncer(timeoutMs = DEFAULT_STATUS_TIMEOUT_MS) {
  const [statusOverride, setStatusOverride] = useState<string | null>(null);
  const statusTimeoutRef = useRef<number | null>(null);

  const announceStatus = useCallback(
    (message: string): void => {
      setStatusOverride(message);

      if (statusTimeoutRef.current !== null) {
        window.clearTimeout(statusTimeoutRef.current);
      }

      statusTimeoutRef.current = window.setTimeout(() => {
        setStatusOverride(null);
      }, timeoutMs);
    },
    [timeoutMs],
  );

  useEffect(
    () => () => {
      if (statusTimeoutRef.current !== null) {
        window.clearTimeout(statusTimeoutRef.current);
      }
    },
    [],
  );

  return { announceStatus, statusOverride };
}
