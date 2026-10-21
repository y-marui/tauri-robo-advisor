// The product list of one account in Rebalance: which products it shows and in what groups. Used
// for the account tabs and for each account on the All tab alike, so both look the same. The
// figures come from Rust (`AccountBalance`); this only arranges them and adds the products the
// balance leaves out (an account is listed with every product, held or not).

import type { Account, AccountBalance, PortfolioView, ProductBalance } from "./types.ts";

export interface ProductRow {
  product: number;
  /** JPY held in the account. */
  current: string;
  target: string | null;
  shortfall: string | null;
  /** False when the account leaves the product out of trading. */
  takesPart: boolean;
}

/** Whether the figures show a holding: any non-zero digit in the yen amount. */
export function isHeld(current: string): boolean {
  return /[1-9]/.test(current);
}

export interface ProductGroup {
  /** `null` for a product strategy, which has no categories. */
  category: number | null;
  total: string | null;
  rows: ProductRow[];
}

/**
 * The groups to show. "Show all" lists every product the account holds; otherwise only those that
 * also take part. Products the account does not hold are never candidates (not every account can
 * trade every product). `forAll` reads the list kept for the All tab. A product strategy lists the products it names, held or not.
 */
export function productGroups(
  view: PortfolioView,
  account: Account,
  balance: AccountBalance | null,
  showAll: boolean,
  forAll = false,
): ProductGroup[] {
  const excluded = new Set((forAll ? account.excluded_all : account.excluded) ?? []);
  const row = (product: number, figures: ProductBalance | undefined): ProductRow => ({
    product,
    current: figures?.current ?? "0",
    target: figures?.target ?? null,
    shortfall: figures?.shortfall ?? null,
    takesPart: !excluded.has(product),
  });
  const wanted = (r: ProductRow): boolean => showAll || r.takesPart;

  if (account.strategy?.type === "products") {
    const figures = new Map((balance?.strategy?.products ?? []).map((p) => [p.product, p]));
    const rows = account.strategy.items.map((item) => row(item.product, figures.get(item.product))).filter(wanted);
    return rows.length === 0 ? [] : [{ category: null, total: balance?.strategy?.total ?? null, rows }];
  }

  const byCategory = new Map((balance?.categories ?? []).map((c) => [c.category, c]));
  const groups: ProductGroup[] = [];
  for (const category of view.portfolio.categories) {
    const known = byCategory.get(category.id);
    const figures = new Map((known?.products ?? []).map((p) => [p.product, p]));
    const rows = view.portfolio.products
      .filter((p) => p.category === category.id)
      .map((p) => row(p.id, figures.get(p.id)))
      .filter((r) => isHeld(r.current) && wanted(r));
    if (rows.length > 0) {
      groups.push({ category: category.id, total: known?.total ?? null, rows });
    }
  }
  return groups;
}
