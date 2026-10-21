// The All tab of Rebalance: which (account, product) pairs take part in a suggestion. The data is
// the per-account exclusion list (`Account.excluded`): a product takes part when it is not excluded.

import type { PortfolioView } from "./types.ts";

/** The products an account holds (a record with a value above zero), in the order of the product list. */
export function heldProducts(view: PortfolioView, account: number): number[] {
  const held = new Set(
    view.portfolio.holdings
      .filter((h) => h.account === account && /[1-9]/.test(h.value) && !h.value.startsWith("-"))
      .map((h) => h.product),
  );
  return view.portfolio.products.filter((p) => held.has(p.id)).map((p) => p.id);
}

/** The exclusion list after every shown product takes part. Products that are not shown keep their state. */
export function selectAll(excluded: number[], shown: number[]): number[] {
  return excluded.filter((id) => !shown.includes(id));
}

/** The exclusion list after none of the shown products takes part. */
export function clearAll(excluded: number[], shown: number[]): number[] {
  return [...new Set([...excluded, ...shown])];
}

/** The exclusion list after one product is switched. */
export function toggle(excluded: number[], product: number, takesPart: boolean): number[] {
  const rest = excluded.filter((id) => id !== product);
  return takesPart ? rest : [...rest, product];
}
