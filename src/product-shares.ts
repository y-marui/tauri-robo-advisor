// Rebalance: the weight of a product for one suggestion mode. The data is
// `Portfolio.product_shares` (only the weights other than 1 are stored); the calculations that use
// it are in Rust. This only looks a weight up.

import type { PortfolioView, SuggestionMode } from "./types.ts";

/** The weight of a product for a mode, as a decimal string; a product without a setting has 1. */
export function weightOf(view: PortfolioView, mode: SuggestionMode, product: number): string {
  const entry = (view.portfolio.product_shares ?? []).find((e) => e.mode === mode && e.product === product);
  return entry?.weight ?? "1";
}
