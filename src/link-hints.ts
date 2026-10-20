import type { LinkSuggestion } from "./types.ts";

// Small helpers for the link step of an import. The matching itself (exact and similar names)
// is done in Rust and arrives in the plan; these only pick what to show.

/** The suggestion for an unmatched name (the plan trims names, so the lookup does too). */
export function suggestionFor(suggestions: LinkSuggestion[] | undefined, name: string): LinkSuggestion | undefined {
  const trimmed = name.trim();
  return suggestions?.find((s) => s.name === trimmed);
}

/** The product the row should offer first: the same name, else a similar one. */
export function suggestedProduct(suggestion: LinkSuggestion | undefined): number | undefined {
  return suggestion?.exact ?? suggestion?.similar ?? undefined;
}

/** `items` with the one that has `first` as id moved to the front; the rest keep their order. */
export function offerFirst<T extends { id: number }>(items: T[], first: number | undefined): T[] {
  const index = items.findIndex((item) => item.id === first);
  return index <= 0 ? items : [items[index], ...items.slice(0, index), ...items.slice(index + 1)];
}

/** Whether an edit was refused because a product with that name exists (the text comes from Rust). */
export function isNameConflict(reason: string): boolean {
  return reason.includes("conflict: a product with this name exists");
}
