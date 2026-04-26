import { describe, expect, it } from "vitest";

import { isSafeExternalUrl, toSafeExternalUrl } from "@/lib/externalUrl";

describe("external URL safety helpers", () => {
  it("allows browser and email URL schemes", () => {
    expect(isSafeExternalUrl("https://example.com/item")).toBe(true);
    expect(isSafeExternalUrl("http://example.com/item")).toBe(true);
    expect(isSafeExternalUrl("mailto:lab@example.com")).toBe(true);
  });

  it("adds https to plain saved links by default", () => {
    expect(toSafeExternalUrl("example.com/item")).toBe("https://example.com/item");
  });

  it("rejects unsafe schemes and local filesystem paths", () => {
    expect(isSafeExternalUrl("javascript:alert(1)")).toBe(false);
    expect(isSafeExternalUrl("data:text/html,hello")).toBe(false);
    expect(isSafeExternalUrl("file:///C:/Temp/image.png")).toBe(false);
    expect(isSafeExternalUrl("vscode://file/C:/Temp/file.txt")).toBe(false);
    expect(isSafeExternalUrl("C:\\Temp\\image.png")).toBe(false);
    expect(isSafeExternalUrl("\\\\server\\share\\image.png")).toBe(false);
  });

  it("can require explicit schemes for already-built URLs", () => {
    expect(toSafeExternalUrl("https://www.google.com/search?q=fixture", { allowImplicitHttps: false })).toBe(
      "https://www.google.com/search?q=fixture",
    );
    expect(toSafeExternalUrl("example.com/item", { allowImplicitHttps: false })).toBeNull();
  });
});
