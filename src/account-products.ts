// The product list of one account in Rebalance: which products it shows and in what groups. Used
// for the account tabs and for each account on the All tab alike, so both look the same. The
// figures come from Rust (`AccountBalance`); this only arranges them and adds the products the
// balance leaves out (an account is listed with every product, held or not).

import type { Account, AccountBalance, PortfolioView, ProductBalance, ProductLine, TypeValue } from "./types.ts";

export interface ProductRow {
  product: number;
  /** JPY held in the account. */
  current: string;
  /** `current` by record type, in the order a sale takes them. */
  types: TypeValue[];
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
    types: figures?.types ?? [],
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

/** The key of an (account, product) pair, as the table and the suggestion share it. */
export const slotKey = (account: number, product: number): string => `${account}:${product}`;

/**
 * The suggestion's lines by (account, product). A purchase is one line; a sale is one line per
 * record type it takes from. Nothing is added up here: the figures come from Rust.
 */
export function tradesBySlot(lines: readonly ProductLine[]): Map<string, ProductLine[]> {
  const out = new Map<string, ProductLine[]>();
  for (const line of lines) {
    const key = slotKey(line.account, line.product);
    out.set(key, [...(out.get(key) ?? []), line]);
  }
  return out;
}

/** How the trades of one row are shown. */
export interface SlotTrade {
  /** The one line shown on the product's own row, or `null` when there is none or the sale is split. */
  inline: ProductLine | null;
  /** One sub-row per record type when a sale is shown by type, with the value that type holds now. */
  parts: { line: ProductLine; current: string }[];
}

/**
 * Decides how the trades of a row are shown: a sale of a product held in more than one record type
 * is shown per type (the row's own figures would not say which records give); anything else is a
 * single line on the row itself.
 */
export function slotTrade(lines: readonly ProductLine[] | undefined, types: readonly TypeValue[]): SlotTrade {
  if (!lines || lines.length === 0) {
    return { inline: null, parts: [] };
  }
  if (types.length > 1 && lines.every((line) => line.by_type)) {
    return {
      inline: null,
      parts: lines.map((line) => ({
        line,
        current: types.find((t) => (t.account_type ?? null) === (line.account_type ?? null))?.current ?? "0",
      })),
    };
  }
  return { inline: lines[0] ?? null, parts: [] };
}

/**
 * The lines whose pair has no row in the table (`shown`), among the accounts the table covers, so
 * a trade is never dropped from the screen.
 */
export function unlistedTrades(
  lines: readonly ProductLine[],
  shown: ReadonlySet<string>,
  accounts: readonly number[],
): ProductLine[] {
  return lines.filter((line) => accounts.includes(line.account) && !shown.has(slotKey(line.account, line.product)));
}
