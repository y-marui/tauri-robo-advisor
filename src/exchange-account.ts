import type { Account, PortfolioView } from "./types.ts";

/** The slice of `Storage` used here, so the logic can be tested without a webview. */
export interface KeyValueStore {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
}

const KEY_PREFIX = "exchange-account:";

function webviewStore(): KeyValueStore | null {
  try {
    return localStorage;
  } catch {
    return null;
  }
}

/** The account last chosen for an exchange; `null` when none was or storage is unavailable. */
export function loadExchangeAccount(
  exchange: string,
  store: KeyValueStore | null = webviewStore(),
): string | null {
  try {
    return store?.getItem(KEY_PREFIX + exchange) ?? null;
  } catch {
    return null;
  }
}

export function saveExchangeAccount(
  exchange: string,
  accountId: string,
  store: KeyValueStore | null = webviewStore(),
): void {
  try {
    store?.setItem(KEY_PREFIX + exchange, accountId);
  } catch {
    // Only a convenience; the app works without storage.
  }
}

/** Accounts that hold crypto assets come first: that is where exchange balances belong. */
export function orderAccounts(view: PortfolioView): Account[] {
  const crypto = new Set(
    view.portfolio.holdings.filter((r) => r.account_type === "crypto").map((r) => r.account),
  );
  return [...view.portfolio.accounts].sort(
    (a, b) => Number(crypto.has(b.id)) - Number(crypto.has(a.id)),
  );
}

/**
 * The account to preselect: the one chosen in this session, else the remembered one,
 * each only while it still exists; otherwise `null` (the first option stays selected).
 */
export function pickAccount(
  ids: string[],
  current: string | null,
  saved: string | null,
): string | null {
  for (const candidate of [current, saved]) {
    if (candidate !== null && ids.includes(candidate)) {
      return candidate;
    }
  }
  return null;
}
