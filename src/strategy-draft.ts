// The strategy being edited in the settings is a draft: it is saved only when it is consistent, so
// the edit never passes through a state Rust rejects (for example blank ratios turning into typed
// ones one product at a time). Rust still validates everything on save.

import { cleanDecimal } from "./decimal.ts";

export interface DraftRow {
  id: number;
  checked: boolean;
  ratio: string;
}

export type DraftProblem = "no_product" | "mixed_ratios" | "invalid_number" | "not_positive";

export type DraftResult =
  | { ok: true; items: { product: number; ratio: string | null }[] }
  | { ok: false; problem: DraftProblem };

export function strategyFromDraft(rows: DraftRow[]): DraftResult {
  const ticked = rows.filter((row) => row.checked);
  if (ticked.length === 0) {
    return { ok: false, problem: "no_product" };
  }
  const given = ticked.filter((row) => row.ratio.trim() !== "");
  if (given.length !== 0 && given.length !== ticked.length) {
    return { ok: false, problem: "mixed_ratios" };
  }
  const items: { product: number; ratio: string | null }[] = [];
  for (const row of ticked) {
    const text = row.ratio.trim();
    if (text === "") {
      items.push({ product: row.id, ratio: null });
      continue;
    }
    const ratio = cleanDecimal(text);
    if (ratio === null || ratio.startsWith("-")) {
      return { ok: false, problem: ratio === null ? "invalid_number" : "not_positive" };
    }
    if (!/[1-9]/.test(ratio)) {
      return { ok: false, problem: "not_positive" };
    }
    items.push({ product: row.id, ratio });
  }
  return { ok: true, items };
}
