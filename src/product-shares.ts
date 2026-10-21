// Rebalance: how a product's target inside its category is set for one suggestion mode. The data
// is `Portfolio.product_shares` (only the products that are not an equal share are stored); the
// calculations that use it are in Rust. This only looks a setting up and builds the edit.

import type { Edit, PortfolioView, ProductShare, SuggestionMode } from "./types.ts";

export type ShareKind = ProductShare["kind"];

export const SHARE_KINDS: ShareKind[] = ["equal", "weighted", "keep_current"];

/** The setting of a product for a mode; a product without one has an equal share. */
export function shareOf(view: PortfolioView, mode: SuggestionMode, product: number): ProductShare {
  const entry = (view.portfolio.product_shares ?? []).find((e) => e.mode === mode && e.product === product);
  return entry?.share ?? { kind: "equal" };
}

/**
 * The edit that switches a product to another kind. A weighted product keeps the weight it had, or
 * starts at 1 (the weight must be positive).
 */
export function switchKind(
  view: PortfolioView,
  mode: SuggestionMode,
  product: number,
  kind: ShareKind,
  lastWeight = "1",
): Edit {
  const old = shareOf(view, mode, product);
  const share: ProductShare =
    kind === "weighted"
      ? { kind, weight: old.kind === "weighted" ? old.weight : lastWeight }
      : { kind };
  return { type: "set_product_share", mode, product, share };
}
