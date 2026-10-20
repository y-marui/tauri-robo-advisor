import type { BrowserKind } from "./types.ts";

/** The browser to preselect: the remembered one when it is still found, else the first found. */
export function pickBrowser(found: BrowserKind[], saved: BrowserKind | null): BrowserKind | null {
  if (saved !== null && found.includes(saved)) {
    return saved;
  }
  return found[0] ?? null;
}
