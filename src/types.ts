// Mirrors the JSON of the Rust `portfolio` crate. Amounts and weights are
// decimal strings: they are never converted to numbers, so nothing is rounded.

export type ProductTargeting = "free" | "equal" | "keep_current" | "weighted";
export type AccountKind =
  | "taxable"
  | "nisa"
  | "ideco"
  | "points"
  | "crypto"
  | "deposit"
  | "other";
export type Funding = "cash" | "points";

export interface Category {
  id: number;
  name: string;
  target_weight: string;
  targeting: ProductTargeting;
}

export interface Product {
  id: number;
  name: string;
  category: number;
  currency: string;
  weight: string | null;
  aliases: { source: string; name: string }[];
}

export interface Account {
  id: number;
  name: string;
  kind: AccountKind;
  sellable: boolean;
  /** `null` means the account can buy any product. */
  tradable: number[] | null;
  funding: Funding;
  rounding_unit: string;
}

export interface Holding {
  account: number;
  product: number;
  value: string;
  quantity: string | null;
}

export interface Settings {
  usd_jpy: string | null;
  category_tolerance: string;
  product_tolerance: string;
  target_account: number | null;
}

export interface Portfolio {
  schema_version: number;
  next_id: number;
  categories: Category[];
  products: Product[];
  accounts: Account[];
  holdings: Holding[];
  settings: Settings;
}

export type Issue =
  | { type: "category_weights_do_not_sum_to_one"; sum: string }
  | { type: "weighted_product_missing_weight"; category: number }
  | { type: "category_has_no_products"; category: number }
  | { type: "missing_exchange_rate"; currency: string };

export interface PortfolioView {
  portfolio: Portfolio;
  issues: Issue[];
}

export type Edit =
  | { type: "add_category"; name: string; target_weight: string }
  | {
      type: "update_category";
      id: number;
      name?: string;
      target_weight?: string;
      targeting?: ProductTargeting;
    }
  | { type: "remove_category"; id: number }
  | { type: "add_product"; name: string; category: number; currency: string }
  | {
      type: "update_product";
      id: number;
      name?: string;
      category?: number;
      currency?: string;
    }
  | { type: "set_product_weight"; id: number; weight: string | null }
  | { type: "remove_product"; id: number }
  | { type: "add_account"; name: string; kind: AccountKind }
  | {
      type: "update_account";
      id: number;
      name?: string;
      kind?: AccountKind;
      sellable?: boolean;
      funding?: Funding;
      rounding_unit?: string;
    }
  | { type: "set_tradable"; id: number; products: number[] | null }
  | { type: "remove_account"; id: number }
  | {
      type: "set_holding";
      account: number;
      product: number;
      value: string;
      quantity: string | null;
    }
  | { type: "remove_holding"; account: number; product: number }
  | { type: "set_exchange_rate"; usd_jpy: string | null }
  | { type: "set_tolerances"; category: string; product: string }
  | { type: "set_target_account"; account: number | null };
