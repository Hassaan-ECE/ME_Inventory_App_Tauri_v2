import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]): string {
  return twMerge(clsx(inputs));
}

export function formatRelativeTimestamp(isoTimestamp: string): string {
  const now = new Date();
  const updated = new Date(isoTimestamp);
  const deltaMs = now.getTime() - updated.getTime();
  const deltaHours = Math.max(0, Math.floor(deltaMs / (1000 * 60 * 60)));

  if (deltaHours < 1) {
    return "just now";
  }
  if (deltaHours < 24) {
    return `${deltaHours}h ago`;
  }
  const deltaDays = Math.floor(deltaHours / 24);
  return `${deltaDays}d ago`;
}
