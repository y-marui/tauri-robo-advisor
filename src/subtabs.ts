/** The sub-tabs of the Holdings tab. Only the visible area changes; the data stays the same. */
export const HOLDINGS_SUBTABS = ["holdings", "rebalance"] as const;
export type HoldingsSubTab = (typeof HOLDINGS_SUBTABS)[number];

/** Sub-tabs that no longer exist, and the one that took over their content. */
const RETIRED = new Map<string, HoldingsSubTab>([
  ["suggestions", "rebalance"],
  ["accounts", "rebalance"],
]);

/** The saved choice if it is one of the sub-tabs (a retired one maps to its successor), else the first one. */
export function pickSubTab(saved: string | null): HoldingsSubTab {
  return HOLDINGS_SUBTABS.find((name) => name === saved) ?? (saved === null ? undefined : RETIRED.get(saved)) ?? "holdings";
}

/** The sub-tab next to `current` in the direction of an arrow key, wrapping around; `null` for other keys. */
export function stepSubTab(current: HoldingsSubTab, key: string): HoldingsSubTab | null {
  const step = key === "ArrowRight" ? 1 : key === "ArrowLeft" ? -1 : 0;
  if (step === 0) {
    return null;
  }
  const count = HOLDINGS_SUBTABS.length;
  return HOLDINGS_SUBTABS[(HOLDINGS_SUBTABS.indexOf(current) + step + count) % count] ?? null;
}
