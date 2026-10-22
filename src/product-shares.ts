// Rebalance: the weight of a record (account, product, type) for one suggestion mode, per tab. The
// data is `Portfolio.product_shares` (only the weights other than 1 are stored; an entry without a
// `tab` is the weight of the All tab); the calculations that use it are in Rust. This only looks a
// weight up.

import type { PortfolioView, SuggestionMode } from "./types.ts";

/**
 * The weight of a record for a mode on a tab (`null`: the All tab), as a decimal string; a record
 * without a setting has 1.
 */
export function weightOf(
  view: PortfolioView,
  mode: SuggestionMode,
  tab: number | null,
  account: number,
  product: number,
  accountType: string | null,
): string {
  const entry = (view.portfolio.product_shares ?? []).find(
    (e) =>
      e.mode === mode &&
      (e.tab ?? null) === tab &&
      e.account === account &&
      e.product === product &&
      (e.account_type ?? null) === accountType,
  );
  return entry?.weight ?? "1";
}
