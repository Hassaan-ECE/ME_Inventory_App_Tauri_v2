const SAFE_EXTERNAL_PROTOCOLS = new Set(["http:", "https:", "mailto:"]);
const WINDOWS_PATH_PATTERN = /^(?:[a-zA-Z]:[\\/]|\\\\)/;

interface ExternalUrlOptions {
  allowImplicitHttps?: boolean;
}

export function isSafeExternalUrl(value: string, options: ExternalUrlOptions = {}): boolean {
  return toSafeExternalUrl(value, options) !== null;
}

export function toSafeExternalUrl(
  value: string,
  { allowImplicitHttps = true }: ExternalUrlOptions = {},
): string | null {
  const parsed = parseExternalUrl(value, allowImplicitHttps);
  if (!parsed || !SAFE_EXTERNAL_PROTOCOLS.has(parsed.protocol)) {
    return null;
  }
  return parsed.toString();
}

function parseExternalUrl(value: string, allowImplicitHttps: boolean): URL | null {
  const trimmed = typeof value === "string" ? value.trim() : "";
  if (!trimmed || WINDOWS_PATH_PATTERN.test(trimmed)) {
    return null;
  }

  try {
    return new URL(trimmed);
  } catch {
    if (!allowImplicitHttps) {
      return null;
    }

    try {
      return new URL(`https://${trimmed}`);
    } catch {
      return null;
    }
  }
}
