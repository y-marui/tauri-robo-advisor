// Rebalance: what each tab (All, or one account) was last set to, kept between visits and restarts.
// Only a convenience: without storage everything starts from its default.

import type { KeyValueStore } from "./exchange-account.ts";
import type { SuggestMethod } from "./types.ts";

export type ListMode = "all" | "tradable";

const METHODS: SuggestMethod[] = ["follow_portfolio", "by_product_weights"];
const LIST_MODES: ListMode[] = ["all", "tradable"];

function webviewStore(): KeyValueStore | null {
  try {
    return localStorage;
  } catch {
    return null;
  }
}

function load<T extends string>(
  kind: string,
  tab: number | "all",
  allowed: T[],
  store: KeyValueStore | null,
): T | null {
  try {
    const saved = store?.getItem(`rebalance-${kind}:${tab}`) ?? null;
    return allowed.find((value) => value === saved) ?? null;
  } catch {
    return null;
  }
}

function save(kind: string, tab: number | "all", value: string, store: KeyValueStore | null): void {
  try {
    store?.setItem(`rebalance-${kind}:${tab}`, value);
  } catch {
    // Only a convenience; the app works without storage.
  }
}

export const loadMethod = (tab: number | "all", store: KeyValueStore | null = webviewStore()): SuggestMethod | null =>
  load("method", tab, METHODS, store);

export const saveMethod = (
  tab: number | "all",
  value: SuggestMethod,
  store: KeyValueStore | null = webviewStore(),
): void => save("method", tab, value, store);

export const loadListMode = (tab: number | "all", store: KeyValueStore | null = webviewStore()): ListMode | null =>
  load("list", tab, LIST_MODES, store);

export const saveListMode = (
  tab: number | "all",
  value: ListMode,
  store: KeyValueStore | null = webviewStore(),
): void => save("list", tab, value, store);
